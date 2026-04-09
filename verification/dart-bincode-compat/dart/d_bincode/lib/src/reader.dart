// Copyright (c) 2025 Binurie
//
// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files (the "Software"), to deal
// in the Software without restriction, including without limitation the rights
// to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
// copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions:
//
// The above copyright notice and this permission notice shall be included in all
// copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
// SOFTWARE.

import 'dart:convert';
import 'dart:typed_data';

import '../d_bincode.dart';
import 'builder.dart';
import 'exception/exception.dart';

/// A high-performance binary deserializer for Rust-compatible Bincode data.
///
/// `BincodeReader` provides low-level, cursor-based access to binary data encoded
/// using Rust's `bincode` crates. It enables direct decoding of primitive
/// types, strings, optionals, collections, and nested structures — all with fine-grained
/// control over buffer position and alignment handling.
///
/// ### Key Features
/// - Supports `u8`, `u16`, `u32`, `u64`, `i8`, `i16`, `i32`, `i64`, `f32`, `f64`
/// - Decodes `String`, fixed-length strings, `Option<T>`, `Vec<T>`, and `Map<K,V>`
/// - Reads nested objects from both fixed-size and length-prefixed layouts
///
/// ### Memory & Performance
/// - **Zero-copy** for fixed-size, aligned types using `TypedData.view`
/// - Falls back to per-element decoding if buffer alignment is not satisfied
/// - Variable-length fields like strings and maps incur allocations
/// - Use `isAligned(n)` to check alignment before numeric list reads
/// - `measureFixedSize()` internally encodes to compute size — avoid in tight loops
///
/// ### Example
/// ```dart
/// final reader = BincodeReader.fromBytes(data);
/// final vec3 = reader.readNestedObjectForFixed(Vec3());
/// final name = reader.readString();
/// ```
///
/// ### When Zero-Copy Applies
/// - Fixed-size primitives and aligned numeric arrays: `Float32List`, `Int16List`, etc.
/// - Manual alignment via [align()] allows optimal performance
/// - No memory is copied for raw byte views (`readRawBytes`, `readUint8List`)
///
/// ### When Fallback Occurs
/// - If the buffer is misaligned (e.g. unaligned `u32` or `f64`)
/// - If the type is variable-length (e.g. `String`, `Vec<T>`)
/// - If decoding into high-level Dart types like `List<T>` or `Map<K,V>`
///
class BincodeReader implements BincodeReaderBuilder {
  final Uint8List _bytes;
  late final ByteData _view;
  int _pos = 0; // read cursor
  final _tmpWriter = BincodeWriter();
  final Map<Type, int> _fixedSizeCache = {};
  late int _limit;

  /// Wrap an existing byte buffer.
  BincodeReader(this._bytes) {
    _view = ByteData.view(
      _bytes.buffer,
      _bytes.offsetInBytes,
      _bytes.lengthInBytes,
    );
    _limit = _bytes.lengthInBytes;
  }

  /// Checks whether [bytes] can be successfully decoded into the given [instance].
  ///
  /// Internally attempts to decode [bytes] using [decode] with the provided
  /// [BincodeDecodable] instance. If decoding throws, returns `false`; otherwise, `true`.
  ///
  /// This method is useful for pre-validation before decoding, e.g. when loading
  /// user-supplied or external binary data.
  ///
  /// #### Example
  /// ```dart
  /// final data = ...; // Uint8List from disk/network
  /// final ok = BincodeReader.isValidBincode(data, MyStruct());
  /// if (ok) {
  ///   final instance = BincodeReader.decode(data, MyStruct());
  /// }
  /// ```
  ///
  /// #### Warning
  /// This method **runs decoding logic** internally — it's not a lightweight check.
  /// Avoid using it in tight loops or performance-critical paths.
  static bool isValidBincode(Uint8List bytes, BincodeDecodable instance) {
    try {
      decode(bytes, instance);
      return true;
    } catch (_) {
      return false;
    }
  }

  /// Deserializes a fixed-size [instance] from [bytes] using the Bincode layout.
  ///
  /// This is used when the layout of the object is known and does **not** include
  /// any dynamic-length fields (e.g. no `Vec`, `String`, or slices).
  ///
  /// Internally, this wraps the provided [bytes] in a [BincodeReader] and calls
  /// [readNestedObjectForFixed] to decode the instance directly.
  ///
  /// #### Rust Context Example:
  /// ```rust
  /// #[derive(Serialize, Deserialize)]
  /// struct Vec3 {
  ///     x: f32,
  ///     y: f32,
  ///     z: f32,
  /// }
  /// ```
  ///
  /// #### Example
  /// ```dart
  /// final bytes = ...; // [12, 0, 0, 65, ...]
  /// final vec = BincodeReader.decodeFixed(bytes, Vec3());
  /// print(vec.x); // decoded f32
  /// ```
  ///
  /// **Layout:** `[field bytes...`] with no length prefix
  static T decodeFixed<T extends BincodeCodable>(Uint8List bytes, T instance) {
    final reader = BincodeReader(bytes);
    return reader.readNestedObjectForFixed(instance);
  }

  /// Deserializes a dynamically-sized [instance] from [bytes] using Bincode layout.
  ///
  /// This is used when the object may contain dynamic-length fields such as
  /// `Vec`, `String`, or nested optional values. The caller supplies an empty
  /// instance of type [T], which is populated via `decode()` logic.
  ///
  /// Internally, wraps the bytes in a [BincodeReader] and passes it to the instance’s
  /// `decode(reader)` method.
  ///
  /// #### Rust Context Example:
  /// ```rust
  /// #[derive(Serialize, Deserialize)]
  /// struct User {
  ///     name: String,
  ///     scores: Vec<u32>,
  /// }
  /// ```
  ///
  /// #### Example
  /// ```dart
  /// final bytes = ...; // serialized `User`
  /// final user = BincodeReader.decode(bytes, User());
  /// print(user.name); // "Alice"
  /// ```
  ///
  /// **Layout:** Bincode-encoded structure as `[field1][field2]...` with internal lengths if needed
  static T decode<T extends BincodeDecodable>(Uint8List bytes, T instance) {
    final reader = BincodeReader(bytes);
    instance.decode(reader);
    return instance;
  }

  /// Wraps a [Uint8List] into a [BincodeReader] for reading.
  ///
  /// Creates a new [BincodeReader] that reads from the provided [data] buffer.
  /// The reader starts at position 0 and provides access to all decoding
  /// utilities over the raw binary.
  ///
  /// This is the standard constructor when you already have a byte array,
  /// such as from file I/O, sockets, or another deserializer.
  ///
  /// #### Example
  /// ```dart
  /// final data = file.readAsBytesSync(); // returns Uint8List
  /// final reader = BincodeReader.fromBytes(data);
  /// final id = reader.readU32();
  /// ```
  ///
  /// **Layout:** Uses entire byte list as-is with no slicing or copying.
  static BincodeReader fromBytes(Uint8List data) => BincodeReader(data);

  /// Wraps a [ByteBuffer] into a [BincodeReader] with optional offset and length.
  ///
  /// This is useful when decoding from shared memory, slices, or typed views
  /// such as `Float32List.buffer`. It allows you to read from a subregion of a buffer
  /// without copying or reallocating data.
  ///
  /// - [offset]: byte offset into the buffer where reading should begin.
  /// - [length]: optional number of bytes to read. If omitted, reads until end of buffer.
  ///
  /// #### Example
  /// ```dart
  /// final full = ByteData(64);
  /// final reader = BincodeReader.fromBuffer(full.buffer, 16, 32); // read 32 bytes from offset 16
  /// final val = reader.readF64();
  /// ```
  ///
  /// #### Performance
  /// This method avoids allocation by creating a [Uint8List.view] over the buffer.
  ///
  /// **Layout:** `[buffer[offset] ... buffer[offset + length]]`
  static BincodeReader fromBuffer(ByteBuffer buffer,
      [int offset = 0, int? length]) {
    final view =
        Uint8List.view(buffer, offset, length ?? buffer.lengthInBytes - offset);
    return BincodeReader(view);
  }

  void _check(int length, [int? at]) {
    final start = at ?? _pos;
    if (start < 0 || start + length > _limit) {
      assert(() {
        throw RangeError(
            'Read out of bounds: Trying to read $length bytes at $start (limit: $_limit, total: ${_bytes.lengthInBytes}).');
        // ignore: dead_code
        return true;
      }());
      if (start < 0 || start + length > _limit) {
        throw RangeError(
            'Read out of bounds: Trying to read $length bytes at $start (limit: $_limit, total: ${_bytes.lengthInBytes}).');
      }
    }
  }

  /// Internal helper to get the byte size of a fixed-size object, using a cache.
  ///
  /// Checks the `_fixedSizeCache` for the runtime type of the [instance].
  /// - If found (cache hit), returns the cached size immediately.
  /// - If not found (cache miss):
  ///   - Rewinds the internal temporary `_tmpWriter`.
  ///   - Calls `instance.encode()` on the temporary writer to measure the size.
  ///   - Stores the measured size in the `_fixedSizeCache` for the type.
  ///   - Returns the measured size.
  ///
  /// This avoids repeatedly encoding objects just to measure their size, significantly
  /// improving performance for repeated reads of the same fixed-size type.
  /// Primarily used by `readNestedObjectForFixed` and `readOptionNestedObjectForFixed`.
  int _getFixedSize<T extends BincodeCodable>(T instance) {
    final type = instance.runtimeType;
    final cachedSize = _fixedSizeCache[type];
    if (cachedSize != null) {
      return cachedSize;
    } else {
      _tmpWriter.rewind();
      instance.encode(_tmpWriter);
      final measuredSize = _tmpWriter.getBytesWritten();
      _fixedSizeCache[type] = measuredSize;
      return measuredSize;
    }
  }

  // ── Positioning ─────────────────────────────────────────────────────────
  /// Gets or sets the current read cursor position (in bytes).
  ///
  /// Use this to inspect or manipulate the position during deserialization,
  /// such as for rewinding, skipping, or jumping back to a previous offset.
  ///
  /// #### Example
  /// ```dart
  /// final reader = BincodeReader.fromBytes(bytes);
  /// print(reader.position); // 0
  /// reader.readU32();
  /// print(reader.position); // 4
  /// reader.position = 0;    // rewind manually
  /// ```
  ///
  /// Throws [RangeError] if the new position is outside of the valid range.
  @override
  int get position => _pos;
  @override
  set position(int v) {
    if (v < 0 || v > _limit) {
      throw RangeError('Cannot set position $v outside bounds [0..$_limit]');
    }
    _pos = v;
  }

  /// Moves the read cursor forward by [offset] bytes.
  ///
  /// This is a relative move operation. Negative values are allowed
  /// to move backwards within the buffer, as long as the resulting position
  /// is within bounds.
  ///
  /// #### Example
  /// ```dart
  /// reader.seek(8);  // skip 8 bytes forward
  /// reader.seek(-4); // rewind 4 bytes
  /// ```
  ///
  /// Throws [RangeError] if the resulting position is out of bounds.
  @override
  void seek(int offset) => position += offset;

  /// Sets the read cursor to the absolute byte offset [absolute].
  ///
  /// This is equivalent to `reader.position = offset`, but may improve
  /// code clarity for forward jumps or manual pointer positioning.
  ///
  /// #### Example
  /// ```dart
  /// reader.seekTo(16); // jump to byte index 16
  /// ```
  ///
  /// Throws [RangeError] if the position is invalid.
  @override
  void seekTo(int absolute) => position = absolute;

  /// Resets the read cursor to the beginning (byte index 0).
  ///
  /// This is a convenience method for restarting a read session
  /// from the start of the buffer.
  ///
  /// #### Example
  /// ```dart
  /// reader.readU64();
  /// reader.rewind();       // go back to start
  /// final again = reader.readU64();
  /// ```
  ///
  /// Does not modify or clear any bytes.
  @override
  void rewind() => _reset();

  /// Internal: Resets the read cursor to 0 without modifying buffer contents.
  void _reset() {
    _pos = 0;
  }

  /// Replaces the current buffer with [newBytes] and resets position.
  ///
  /// This allows reusing the same reader instance with new binary data
  /// without allocating a new [BincodeReader].
  ///
  /// #### Example
  /// ```dart
  /// final reader = BincodeReader(bytesA);
  /// reader.readU32();
  /// reader.resetWith(bytesB); // reuse same reader for different data
  /// ```
  ///
  /// The read position is reset to 0. Throws if [newBytes] is shorter than required reads.
  void resetWith(Uint8List newBytes) {
    _pos = 0;
    _bytes.setAll(0, newBytes);
    _view = ByteData.view(
      _bytes.buffer,
      _bytes.offsetInBytes,
      _bytes.lengthInBytes,
    );
  }

  /// Moves the read cursor to the end of the buffer.
  ///
  /// Useful for fast‑forwarding to the end after partial reads,
  /// or for skipping trailing content.
  ///
  /// #### Example
  /// ```dart
  /// reader.skipToEnd();
  /// assert(reader.remainingBytes == 0);
  /// ```
  @override
  void skipToEnd() => _pos = _bytes.lengthInBytes;

  /// Removes all null (`\x00`) characters from a [String].
  ///
  /// Useful for cleaning up fixed‑length strings that were padded
  /// with nulls during encoding.
  ///
  /// #### Example
  /// ```dart
  /// final s = 'User\x00\x00\x00';
  /// final clean = BincodeReader.stripNulls(s);
  /// print(clean); // 'User'
  /// ```
  static String stripNulls(String s) => s.replaceAll('\x00', '');

  /// Overwrites the current buffer with [data] and resets the read cursor.
  ///
  /// Reuses the reader for new binary data without allocating a new instance.
  /// The buffer must be at least as large as the new data.
  ///
  /// #### Example
  /// ```dart
  /// reader.copyFrom(Uint8List.fromList([1, 2, 3]));
  /// print(reader.readU8()); // 1
  /// ```
  ///
  /// Warning: Only use this if the internal `_bytes` buffer is large enough to hold [data].
  void copyFrom(Uint8List data) {
    _bytes.setAll(0, data);
    _pos = 0;
  }

  /// Returns the number of unread bytes left in the buffer.
  ///
  /// This can be used to detect under‑reads or to check availability
  /// before attempting a read.
  ///
  /// #### Example
  /// ```dart
  /// print('Remaining: ${reader.remainingBytes} bytes');
  /// ```
  @override
  int get remainingBytes => _limit - _pos;

  /// Returns true if the current read position is aligned to [alignment] bytes.
  ///
  /// Useful before reading multi‑byte primitives to avoid misalignment traps,
  /// especially when using `TypedData.view` directly.
  ///
  /// #### Example
  /// ```dart
  /// if (!reader.isAligned(4)) {
  ///   reader.align(4); // optional
  /// }
  /// ```
  bool isAligned(int alignment) => (_pos % alignment) == 0;

  /// Aligns the read cursor to the next [alignment]‑byte boundary.
  ///
  /// If the current position is not aligned (i.e., `position % alignment != 0`),
  /// this method advances the cursor forward by the required number of bytes
  /// to reach the next aligned offset.
  ///
  /// This is useful when reading native types that require aligned memory
  /// access (e.g., `Float64List.view`) or matching formats that include padding.
  ///
  /// #### Example
  /// ```dart
  /// reader.seek(3);        // misaligned for 4‑byte access
  /// reader.align(4);       // moves to byte offset 4
  /// print(reader.position); // 4
  /// ```
  ///
  /// Throws a [RangeError] if alignment padding would exceed buffer length.
  void align(int alignment) {
    final misalignment = _pos % alignment;
    if (misalignment != 0) {
      final pad = alignment - misalignment;
      _check(pad);
      _pos += pad;
    }
  }

  /// Peeks the data by saving the current read position and executing the provided [body].
  /// This allows for peeking multiple values without advancing the reader position.
  ///
  /// The position is restored after the operation, ensuring no side effects on the reader's state.
  ///
  /// #### Example Usage:
  ///
  /// ```dart
  /// final reader = BincodeReader(bytes);
  /// final values = reader.peekSession(() {
  ///   final u8Value = reader.readU8();   // Peek a U8 value
  ///   final u16Value = reader.readU16(); // Peek a U16 value
  ///   final u32Value = reader.readU32(); // Peek a U32 value
  ///   return [u8Value, u16Value, u32Value]; // Return the values
  /// });
  /// print(values);  // Output: [value1, value2, value3]
  /// ```
  ///
  /// This method is useful when you need to peek multiple values, while maintaining
  /// the current reader position intact.
  T peekSession<T>(T Function() body) {
    final savedPosition = _pos;
    try {
      return body();
    } finally {
      _pos = savedPosition;
    }
  }

  /// Static utility to peek a `u64` length prefix from the start of a buffer.
  ///
  /// Does **not** require a full reader, useful for inspecting list or blob lengths before allocation.
  ///
  /// #### Example
  /// ```dart
  /// final len = BincodeReader.peekLength(myBytes);
  /// ```
  static int peekLength(Uint8List bytes) {
    final view = ByteData.view(bytes.buffer, bytes.offsetInBytes);
    return view.getUint64(0, Endian.little); // Read the 8 bytes from the start
  }

  /// Skips an entire optional value (Bincode `Option<T>`).
  ///
  /// Reads the 1-byte tag (`0` = None, `1` = Some), and if the tag indicates a value,
  /// executes [skipValue] to advance past the encoded bytes of `T`.
  /// If the tag is invalid (neither 0 nor 1), throws [InvalidOptionTagException].
  ///
  /// #### Example
  /// ```dart
  /// reader.skipOption(() => reader.skipF64()); // skip Option<f64>
  /// reader.skipOption(() => reader.skipString()); // skip Option<String>
  /// ```
  void skipOption<T>(void Function() skipValue) {
    final tag = readU8();
    if (tag == 1) {
      skipValue();
    } else if (tag != 0) {
      throw InvalidOptionTagException(tag);
    }
  }

  /// Skips a single byte (unsigned 8-bit, `u8`) in the buffer.
  ///
  /// Advances the cursor by 1 byte.
  ///
  /// #### Example
  /// ```dart
  /// reader.skipU8(); // skip 1 byte
  /// ```
  void skipU8() => seek(1);

  /// Skips a 2-byte unsigned integer (`u16`) in the buffer.
  ///
  /// Advances the cursor by 2 bytes.
  ///
  /// #### Example
  /// ```dart
  /// reader.skipU16(); // skip 2 bytes
  /// ```
  void skipU16() => seek(2);

  /// Skips a 4-byte unsigned integer (`u32`) in the buffer.
  ///
  /// Advances the cursor by 4 bytes.
  ///
  /// #### Example
  /// ```dart
  /// reader.skipU32(); // skip 4 bytes
  /// ```
  void skipU32() => seek(4);

  /// Skips an 8-byte unsigned integer (`u64`) in the buffer.
  ///
  /// Advances the cursor by 8 bytes.
  ///
  /// #### Example
  /// ```dart
  /// reader.skipU64(); // skip 8 bytes
  /// ```
  void skipU64() => seek(8);

  /// Skips a 4-byte IEEE-754 floating point value (`f32`) in the buffer.
  ///
  /// Advances the cursor by 4 bytes.
  ///
  /// #### Example
  /// ```dart
  /// reader.skipF32(); // skip f32
  /// ```
  void skipF32() => seek(4);

  /// Skips an 8-byte IEEE-754 floating point value (`f64`) in the buffer.
  ///
  /// Advances the cursor by 8 bytes.
  ///
  /// #### Example
  /// ```dart
  /// reader.skipF64(); // skip f64
  /// ```
  void skipF64() => seek(8);

  /// Skips the next [n] raw bytes from the buffer.
  ///
  /// No decoding is performed. Useful for skipping unknown or irrelevant binary sections.
  ///
  /// #### Example
  /// ```dart
  /// reader.skipBytes(16); // skip 16 bytes
  /// ```
  void skipBytes(int n) => seek(n);

  /// Skips a dynamic UTF‑8 string field.
  ///
  /// Reads a `u64` length prefix and advances the cursor by that many bytes,
  /// without decoding the string.
  ///
  /// #### Example
  /// ```dart
  /// reader.skipString(); // skip a Bincode-encoded string
  /// ```
  void skipString() {
    final len = readU64();
    seek(len);
  }

  /// Skips a fixed-length UTF‑8 string field of exactly [n] bytes.
  ///
  /// Advances the cursor by [n] bytes without decoding. Does not trim nulls.
  ///
  /// #### Example
  /// ```dart
  /// reader.skipFixedString(32); // skip a 32-byte label field
  /// ```
  void skipFixedString(int n) => seek(n);

  /// Computes the total byte size of a Bincode list `(Vec<T>)` given [bytes] and per-element size.
  ///
  /// Reads the first 8 bytes as a `u64` length prefix from [bytes] (without mutation),
  /// then returns the total number of bytes the full list would occupy in memory,
  /// including the prefix and all element payloads.
  ///
  /// Does **not** validate or decode actual elements; it only calculates size.
  ///
  /// #### Example
  /// ```dart
  /// final bytes = Uint8List.fromList([3, 0, 0, 0, 0, 0, 0, 0, ...]);
  /// final totalSize = BincodeReader.measureListByteSize(bytes, 4); // 8 + 3 * 4 = 20
  /// ```
  ///
  /// #### Use Case
  /// Useful for buffer pre-checks, slicing payloads, or skipping over a serialized list.
  static int measureListByteSize(Uint8List bytes, int elementSize) {
    final len = peekLength(bytes);
    return 8 + len * elementSize;
  }

  /// Checks whether at least [count] bytes remain to be read in the buffer.
  ///
  /// Returns `true` if there are enough bytes starting from the current [position]
  /// to safely read [count] bytes without hitting the end of the buffer.
  ///
  /// #### Example
  /// ```dart
  /// if (reader.hasBytes(4)) {
  ///   final value = reader.readU32();
  /// }
  /// ```
  bool hasBytes(int count) => remainingBytes >= count;

  // ── Primitive Reads ───────────────────────────────────────────────────────

  /// Reads an unsigned 8‑bit integer (`u8`) from the buffer.
  ///
  /// 1. Checks that at least 1 byte is available.
  /// 2. Reads the byte at the current position.
  /// 3. Advances the cursor by 1.
  ///
  /// #### Rust Context Example:
  /// ```rust
  /// #[derive(Deserialize)]
  /// struct Packet { id: u8 }
  /// // [0xFF] deserializes to Packet { id: 255 }
  /// ```
  ///
  /// **Layout:** `[u8 byte]`
  @override
  @pragma('vm:always-inline')
  int readU8() {
    _check(1);
    final p = _pos;
    _pos = p + 1;
    return _view.getUint8(p);
  }

  /// Reads an unsigned 16‑bit integer (`u16`) in little-endian order.
  ///
  /// 1. Checks that at least 2 bytes are available.
  /// 2. Interprets bytes at the current position as a `u16`.
  /// 3. Advances the cursor by 2.
  ///
  /// #### Rust Context Example:
  /// ```rust
  /// #[derive(Deserialize)]
  /// struct Header { version: u16 }
  /// ```
  ///
  /// **Layout:** `[low byte][high byte]`
  @override
  @pragma('vm:always-inline')
  int readU16() {
    _check(2);
    final p = _pos;
    _pos = p + 2;
    return _view.getUint16(p, Endian.little);
  }

  /// Reads an unsigned 32‑bit integer (`u32`) in little-endian order.
  ///
  /// 1. Checks that at least 4 bytes are available.
  /// 2. Decodes the bytes at current position as a `u32`.
  /// 3. Advances the cursor by 4.
  ///
  /// #### Rust Context Example:
  /// ```rust
  /// #[derive(Deserialize)]
  /// struct Record { count: u32 }
  /// ```
  ///
  /// **Layout:** `[byte0][byte1][byte2][byte3]` (little-endian)
  @override
  @pragma('vm:always-inline')
  int readU32() {
    _check(4);
    final p = _pos;
    _pos = p + 4;
    return _view.getUint32(p, Endian.little);
  }

  /// Reads an unsigned 64‑bit integer (`u64`) in little-endian order.
  ///
  /// 1. Checks for at least 8 bytes.
  /// 2. Interprets 8 bytes at position as a `u64`.
  /// 3. Advances the cursor by 8.
  ///
  /// #### Rust Context Example:
  /// ```rust
  /// #[derive(Deserialize)]
  /// struct Timestamp { nanos: u64 }
  /// // [0x08, 0x07, ..., 0x00] → 64-bit integer
  /// ```
  ///
  /// **Layout:** `[b0][b1][b2][b3][b4][b5][b6][b7]` (little-endian)
  @override
  @pragma('vm:always-inline')
  int readU64() {
    _check(8);
    final p = _pos;
    _pos = p + 8;
    return _view.getUint64(p, Endian.little);
  }

  /// Reads a signed 8‑bit integer (`i8`) from the buffer.
  ///
  /// Internally uses [readU8] and converts the byte using two’s complement.
  ///
  /// #### Rust Context Example:
  /// ```rust
  /// #[derive(Deserialize)]
  /// struct Signed { delta: i8 }
  /// // [0xFF] → -1
  /// ```
  ///
  /// **Layout:** `[i8 byte]`
  @override
  int readI8() => readU8().toSigned(8);

  /// Reads a signed 16‑bit integer (`i16`) in little-endian format.
  ///
  /// Uses [readU16] and converts using two’s complement.
  ///
  /// #### Rust Context Example:
  /// ```rust
  /// #[derive(Deserialize)]
  /// struct Coord { y: i16 }
  /// // [0xFC, 0xFF] → -4 (0xFFFC)
  /// ```
  ///
  /// **Layout:** `[low byte][high byte]` (two’s complement)
  @override
  int readI16() => readU16().toSigned(16);

  /// Reads a signed 32‑bit integer (`i32`) in little-endian format.
  ///
  /// Reads with [readU32] and interprets with two’s complement.
  ///
  /// #### Rust Context Example:
  /// ```rust
  /// #[derive(Deserialize)]
  /// struct Vector { dx: i32 }
  /// // [0xFF, 0xFF, 0xFF, 0xFF] → -1
  /// ```
  ///
  /// **Layout:** `[b0][b1][b2][b3]` (two’s complement, little-endian)
  @override
  int readI32() => readU32().toSigned(32);

  /// Reads a signed 64‑bit integer (`i64`) in little-endian format.
  ///
  /// Reads using [readU64], then converts using two’s complement.
  ///
  /// #### Rust Context Example:
  /// ```rust
  /// #[derive(Deserialize)]
  /// struct Delta { offset: i64 }
  /// // [0xFF ... x8] → -1
  /// ```
  ///
  /// **Layout:** `[b0..b7]` (two’s complement, little-endian)
  @override
  int readI64() => readU64().toSigned(64);

  /// Reads a 32‑bit floating point value (`f32`) in little-endian format.
  ///
  /// 1. Reads 4 bytes as IEEE 754 `f32`.
  /// 2. Advances cursor by 4.
  ///
  /// #### Rust Context Example:
  /// ```rust
  /// #[derive(Deserialize)]
  /// struct Vec2 { x: f32 }
  /// // [0x00, 0x00, 0x80, 0x3F] → 1.0
  /// ```
  ///
  /// **Layout:** IEEE 754 binary32 (little-endian)
  @override
  @pragma('vm:always-inline')
  double readF32() {
    _check(4);
    final p = _pos;
    _pos = p + 4;
    return _view.getFloat32(p, Endian.little);
  }

  /// Reads a 32‑bit floating point value (`f32`) in little-endian format.
  ///
  /// 1. Reads 4 bytes as IEEE 754 `f32`.
  /// 2. Advances cursor by 4.
  ///
  /// #### Rust Context Example:
  /// ```rust
  /// #[derive(Deserialize)]
  /// struct Vec2 { x: f32 }
  /// // [0x00, 0x00, 0x80, 0x3F] → 1.0
  /// ```
  ///
  /// **Layout:** IEEE 754 binary32 (little-endian)
  @override
  @pragma('vm:always-inline')
  double readF64() {
    _check(8);
    final p = _pos;
    _pos = p + 8;
    return _view.getFloat64(p, Endian.little);
  }

  // ── Common Reads ───────────────────────────────────────────────────────────

  /// Reads a boolean value (`bool`) from a single byte.
  ///
  /// 1. Reads 1 byte using [readU8].
  /// 2. Interprets `0` as `false`, `1` as `true`.
  /// 3. Throws [InvalidBooleanValueException] if value is neither `0` nor `1`.
  ///
  /// #### Rust Context Example:
  /// ```rust
  /// #[derive(Deserialize)]
  /// struct Flags {
  ///     active: bool,
  /// }
  /// // [0x01] → true, [0x00] → false, [0x42] → error
  /// ```
  ///
  /// **Layout:** `[1]` for true, `[0]` for false
  @override
  bool readBool() {
    final b = readU8();
    if (b > 1) throw InvalidBooleanValueException(b);
    return b == 1;
  }

  /// Reads a raw sequence of [count] bytes from the buffer as `Uint8List`.
  ///
  /// 1. Checks that [count] bytes are available.
  /// 2. Returns a view over the exact bytes starting at current cursor.
  /// 3. Advances the cursor by [count].
  ///
  /// #### Rust Context Example:
  /// ```rust
  /// #[derive(Deserialize)]
  /// struct Blob {
  ///     raw: [u8; 4],
  /// }
  /// ```
  ///
  /// **Layout:** `[u8, u8, ..., u8]` (exactly [count] bytes)
  @override
  @pragma('vm:prefer-inline')
  Uint8List readRawBytes(int count) {
    _check(count);
    final p = _pos;
    _pos = p + count;
    return Uint8List.view(
      _bytes.buffer,
      _bytes.offsetInBytes + p,
      count,
    );
  }

  /// Reads [count] bytes and returns them as a list of integers.
  ///
  /// This method is functionally identical to [readRawBytes], but
  /// explicitly returns a `List<int>` instead of `Uint8List`.
  ///
  /// #### Rust Context Example:
  /// ```rust
  /// #[derive(Deserialize)]
  /// struct RawData {
  ///     content: Vec<u8>,
  /// }
  /// ```
  ///
  /// **Layout:** `[u8, u8, ..., u8]`
  @override
  List<int> readBytes(int count) => readRawBytes(count);

  /// Reads a UTF‑8 encoded string prefixed by a `u64` length header.
  ///
  /// #### Rust Context Example:
  /// ```rust
  /// #[derive(Deserialize)]
  /// struct Message {
  ///     text: String,
  /// }
  /// ```
  ///
  /// **Layout:** `[len: u64][utf8-bytes...]`
  @override
  String readString() {
    final len = readU64();
    return utf8.decode(readRawBytes(len));
  }

  /// Reads a fixed-length UTF-8 encoded string of exactly [length] bytes.
  ///
  /// This method does not strip trailing nulls or check UTF-8 termination.
  /// Use [readCleanFixedString] for zero-padded fields.
  ///
  /// #### Rust Context Example:
  /// ```rust
  /// #[derive(Deserialize)]
  /// struct Header {
  ///     label: [u8; 8],
  /// }
  /// // [b"test\x00\x00\x00\x00"] → "test\u0000\u0000\u0000\u0000"
  /// ```
  ///
  /// **Layout:** `[utf8...][0x00 0x00 ...]` (exactly [length] bytes)
  @override
  String readFixedString(int length) {
    return utf8.decode(readRawBytes(length));
  }

  /// Reads a Rust `char` (encoded as u32 for Bincode v1/legacy) and returns it as a single-character Dart String.
  ///
  /// Reads 4 bytes from the current position, interprets them as a little-endian `u32`
  /// representing a Unicode code point, and returns the corresponding character.
  /// Advances the cursor by 4 bytes. Throws if the decoded code point is invalid
  /// (e.g., a surrogate pair code point).
  ///
  /// #### Rust Type: `char`
  /// #### Rust Context Example:
  /// ```rust
  /// #[derive(Deserialize)]
  /// struct Message { first_initial: char }
  /// ```
  /// **Layout:** `[u32 rune]` (little-endian)
  @override
  String readChar() {
    final rune = readU32();

    if (rune < 0 || rune > 0x10FFFF || (rune >= 0xD800 && rune <= 0xDFFF)) {
      throw BincodeException(
          'Invalid Unicode code point read for char: $rune (0x${rune.toRadixString(16)})');
    }

    return String.fromCharCode(rune);
  }

  /// Reads a fixed-length string of [length] bytes, and removes null padding.
  ///
  /// 1. Reads [length] bytes as UTF-8.
  /// 2. Strips all `\x00` null characters.
  /// 3. Useful for fixed-size fields like `[u8; 16]` in Rust.
  ///
  /// #### Rust Context Example:
  /// ```rust
  /// #[derive(Deserialize)]
  /// struct Label {
  ///     name: [u8; 8],
  /// }
  /// // [b"abc\x00\x00\x00\x00\x00"] → "abc"
  /// ```
  ///
  /// **Layout:** `[utf8...][padding...]` → cleaned
  @override
  String readCleanFixedString(int length) {
    return readFixedString(length).replaceAll('\x00', '');
  }

  // ── Optionals ────────────────────────────────────────────────────────────

  /// Reads an optional value using Bincode’s `Option<T>` encoding format.
  ///
  /// 1. Reads a tag byte:
  ///    - `0` → returns `null`
  ///    - `1` → calls [reader] to read and return a value
  ///    - anything else → throws [InvalidOptionTagException]
  ///
  /// This method underpins all `readOption*` methods and enforces strict tag validation.
  ///
  /// #### Rust Context Example:
  /// ```rust
  /// #[derive(Deserialize)]
  /// struct Config {
  ///     maybe_value: Option<u32>,
  /// }
  /// // [0] → None
  /// // [1][value bytes...] → Some(value)
  /// ```
  ///
  /// **Layout:** `[tag: u8][value?]`
  @pragma('vm:always-inline')
  T? _readTagged<T>(T Function() reader) {
    final tag = readU8();
    if (tag == 0) return null;
    if (tag != 1) throw InvalidOptionTagException(tag);
    return reader();
  }

  /// Reads an optional `bool` value using Bincode’s `Option<bool>` layout.
  ///
  /// Delegates to `_readTagged` and reads one byte if tag is `1`.
  ///
  /// #### Rust Context Example:
  /// ```rust
  /// #[derive(Deserialize)]
  /// struct Flags {
  ///     active: Option<bool>,
  /// }
  /// ```
  ///
  /// **Layout:** `[0]` if null, `[1][bool byte]` if present
  @override
  bool? readOptionBool() => _readTagged(readBool);

  /// Reads an optional unsigned 8-bit integer (`u8`) from the buffer.
  ///
  /// #### Rust Context Example:
  /// ```rust
  /// #[derive(Deserialize)]
  /// struct Config {
  ///     level: Option<u8>,
  /// }
  /// ```
  ///
  /// **Layout:** `[0]` if null, `[1][u8 byte]` if present
  @override
  int? readOptionU8() => _readTagged(readU8);

  /// Reads an optional `u16` value from the buffer in little-endian format.
  ///
  /// #### Rust Context Example:
  /// ```rust
  /// #[derive(Deserialize)]
  /// struct Packet {
  ///     checksum: Option<u16>,
  /// }
  /// ```
  ///
  /// **Layout:** `[0]` if null, `[1][u16 bytes]` if present
  @override
  int? readOptionU16() => _readTagged(readU16);

  /// Reads an optional `u32` value from the buffer.
  ///
  /// #### Rust Context Example:
  /// ```rust
  /// #[derive(Deserialize)]
  /// struct Record {
  ///     id: Option<u32>,
  /// }
  /// ```
  ///
  /// **Layout:** `[0]` if null, `[1][u32 bytes]` if present
  @override
  int? readOptionU32() => _readTagged(readU32);

  /// Reads an optional `u64` value from the buffer.
  ///
  /// #### Rust Context Example:
  /// ```rust
  /// #[derive(Deserialize)]
  /// struct Timestamp {
  ///     updated_at: Option<u64>,
  /// }
  /// ```
  ///
  /// **Layout:** `[0]` if null, `[1][u64 bytes]` if present
  @override
  int? readOptionU64() => _readTagged(readU64);

  /// Reads an optional signed 8-bit integer (`i8`) from the buffer.
  ///
  /// #### Rust Context Example:
  /// ```rust
  /// #[derive(Deserialize)]
  /// struct Delta {
  ///     offset: Option<i8>,
  /// }
  /// ```
  ///
  /// **Layout:** `[0]` if null, `[1][i8 byte]` if present
  @override
  int? readOptionI8() => _readTagged(readI8);

  /// Reads an optional signed 16-bit integer (`i16`) from the buffer.
  ///
  /// #### Rust Context Example:
  /// ```rust
  /// #[derive(Deserialize)]
  /// struct Point {
  ///     z: Option<i16>,
  /// }
  /// ```
  ///
  /// **Layout:** `[0]` if null, `[1][i16 bytes]` if present
  @override
  int? readOptionI16() => _readTagged(readI16);

  /// Reads an optional signed 32-bit integer (`i32`) from the buffer.
  ///
  /// #### Rust Context Example:
  /// ```rust
  /// #[derive(Deserialize)]
  /// struct Vec2 {
  ///     x: Option<i32>,
  /// }
  /// ```
  ///
  /// **Layout:** `[0]` if null, `[1][i32 bytes]` if present
  @override
  int? readOptionI32() => _readTagged(readI32);

  /// Reads an optional signed 64-bit integer (`i64`) from the buffer.
  ///
  /// #### Rust Context Example:
  /// ```rust
  /// #[derive(Deserialize)]
  /// struct Account {
  ///     balance: Option<i64>,
  /// }
  /// ```
  ///
  /// **Layout:** `[0]` if null, `[1][i64 bytes]` if present
  @override
  int? readOptionI64() => _readTagged(readI64);

  /// Reads an optional 32-bit float (`f32`) using IEEE-754 format.
  ///
  /// #### Rust Context Example:
  /// ```rust
  /// #[derive(Deserialize)]
  /// struct Sensor {
  ///     temperature: Option<f32>,
  /// }
  /// ```
  ///
  /// **Layout:** `[0]` if null, `[1][f32 bytes]` if present
  @override
  double? readOptionF32() => _readTagged(readF32);

  /// Reads an optional 64-bit float (`f64`) using IEEE-754 format.
  ///
  /// #### Rust Context Example:
  /// ```rust
  /// #[derive(Deserialize)]
  /// struct DataPoint {
  ///     value: Option<f64>,
  /// }
  /// ```
  ///
  /// **Layout:** `[0]` if null, `[1][f64 bytes]` if present
  @override
  double? readOptionF64() => _readTagged(readF64);

  /// Reads an optional UTF‑8 string with a `u64` length prefix if present.
  ///
  /// #### Rust Context Example:
  /// ```rust
  /// #[derive(Deserialize)]
  /// struct Message {
  ///     text: Option<String>,
  /// }
  /// ```
  ///
  /// **Layout:** `[0]` if null, `[1][len: u64][utf8-bytes]` if present
  @override
  String? readOptionString() => _readTagged(() => readString());

  /// Reads an optional fixed-length UTF‑8 string of exactly [length] bytes.
  ///
  /// #### Rust Context Example:
  /// ```rust
  /// #[derive(Deserialize)]
  /// struct Label {
  ///     name: Option<[u8; 8]>,
  /// }
  /// ```
  ///
  /// **Layout:** `[0]` if null, `[1][fixed-bytes]` if present
  @override
  String? readOptionFixedString(int length) =>
      _readTagged(() => readFixedString(length));

  /// Reads an optional Rust `char` (encoded as u32 for Bincode v1/legacy).
  ///
  /// Reads the tag (u8). If 1, reads the `u32` rune and returns the character String.
  /// If 0, returns null. Throws on invalid tag or invalid rune.
  ///
  /// #### Rust Type: `Option<char>`
  /// #### Rust Context Example:
  /// ```rust
  /// #[derive(Deserialize)]
  /// struct MaybeInitial { initial: Option<char> }
  /// ```
  /// **Layout:** `[0]` or `[1][u32 rune]`
  @override
  String? readOptionChar() => _readTagged(readChar);

  /// Reads an optional fixed-length UTF‑8 string and strips null padding.
  ///
  /// Useful for padded fields like `[u8; 16]` where content may be shorter.
  ///
  /// #### Rust Context Example:
  /// ```rust
  /// #[derive(Deserialize)]
  /// struct Record {
  ///     tag: Option<[u8; 16]>,
  /// }
  /// ```
  ///
  /// **Layout:** `[0]` if null, `[1][fixed-bytes]` with `\x00` stripped
  @override
  String? readCleanOptionFixedString(int length) =>
      _readTagged(() => readFixedString(length).replaceAll('\x00', ''));

  /// Reads an optional [Duration] value using the defined format.
  ///
  /// Reads the tag (u8). If 1, reads the Duration as seconds (i64) + positive nanos (u32).
  /// If 0, returns null. Throws on invalid tag or invalid Duration format.
  ///
  /// #### Rust Type: `Option<chrono::Duration>` (via serde)
  /// #### Rust Context Example:
  /// ```rust
  /// #[derive(Deserialize)]
  /// struct Task { estimated_time: Option<chrono::Duration> }
  /// ```
  /// **Layout:** `[0]` or `[1][i64 seconds][u32 positive_nanos]`
  @override
  Duration? readOptionDuration() {
    return _readTagged(readDuration);
  }

  /// Reads an optional enum discriminant (variant index).
  ///
  /// Reads the `u8` tag. If the tag is `1`, reads the `u32` discriminant and returns it.
  /// If the tag is `0`, returns `null`. Throws [InvalidOptionTagException] for other tag values.
  /// This corresponds to reading the discriminant part of an `Option<Enum>`.
  ///
  /// #### Rust Type: Part of `Option<Enum>` deserialization
  /// #### Rust Context Example:
  /// ```rust
  /// enum MaybeStatus { None, Some(Status) }
  /// let opt_discriminant = reader.read_option_enum_discriminant(); // Reads tag + Option<u32>
  /// // Based on result, might read Status payload
  /// ```
  /// **Layout:** `[0]` or `[1][u32 discriminant]`
  @override
  int? readOptionEnumDiscriminant() {
    return _readTagged(readEnumDiscriminant);
  }

  // ── Collections ──────────────────────────────────────────────────────────

  /// Reads a Bincode sequence of elements into a `List<T>`, using the provided [readElement] function.
  ///
  /// 1. Reads a `u64` length prefix.
  /// 2. Calls [readElement] once for each element, appending to the result list.
  ///
  /// #### Rust Context Example:
  /// ```rust
  /// #[derive(Deserialize)]
  /// struct Scene {
  ///     objects: Vec<SceneObject>,
  /// }
  /// ```
  ///
  /// **Layout:** `[u64 length][element 1 bytes][element 2 bytes]...`
  ///
  /// #### Example
  /// ```dart
  /// final list = reader.readList(() => reader.readU32());
  /// ```
  ///
  /// **Performance Warning:**
  /// This method allocates and decodes one element at a time using [readElement].
  /// It is **computationally expensive** for large sequences due to Dart VM function call overhead and dynamic dispatch.
  ///
  /// Consider using specialized numeric list readers (like `readUint32List`) when possible.
  @override
  List<T> readList<T>(T Function() readElement) {
    final length = readU64();
    if (length > 0 && remainingBytes < length) {
      // Heuristic check
      _check(length);
    }
    return List<T>.generate(length, (_) => readElement(), growable: false);
  }

  /// Reads a Bincode-encoded map into a `Map<K, V>`, using provided reader functions for keys and values.
  ///
  /// 1. Reads a `u64` length prefix.
  /// 2. Reads `[key][value]` pairs `length` times using [readKey] and [readValue].
  ///
  /// #### Rust Context Example:
  /// ```rust
  /// use std::collections::HashMap;
  ///
  /// #[derive(Deserialize)]
  /// struct GameState {
  ///     players: HashMap<u32, Player>,
  /// }
  /// ```
  ///
  /// **Layout:** `[u64 num_entries][key1][val1][key2][val2]...`
  ///
  /// #### Example
  /// ```dart
  /// final map = reader.readMap(
  ///   () => reader.readU32(),
  ///   () => reader.readNestedObjectForFixed(player),
  /// );
  /// ```
  ///
  /// **Performance Warning:**
  /// This method is **computationally expensive** for large maps due to repeated function calls.
  /// Prefer using flat representations with list pairs if you can optimize for size or speed.
  @override
  Map<K, V> readMap<K, V>(K Function() readKey, V Function() readValue) {
    final length = readU64();
    final result = <K, V>{};
    for (var i = 0; i < length; i++) {
      final K key = readKey();
      final V value = readValue();
      result[key] = value;
    }
    return result;
  }

  /// Reads a fixed-size array of elements without a length prefix.
  ///
  /// Reads exactly [length] items sequentially using the [readElement] callback.
  /// Corresponds to deserializing Rust's fixed-size array type `[T; N]`.
  ///
  /// #### Rust Type: `[T; N]`
  /// #### Rust Context Example:
  /// ```rust
  /// #[derive(Deserialize)]
  /// struct Header { checksum: [u16; 4] } // N = 4, T = u16
  /// ```
  /// **Layout:** `[element 1 bytes][element 2 bytes]...[element N bytes]` (no length prefix)
  @override
  List<T> readFixedArray<T>(int length, T Function() readElement) {
    if (length < 0) {
      throw BincodeException(
          "Cannot read fixed array with negative length: $length");
    }
    return List<T>.generate(length, (_) => readElement(), growable: false);
  }

  /// Reads a `Set<T>` from a Bincode sequence (length + elements).
  ///
  /// Reads a U64 length prefix, then reads that many elements using the
  /// [readElement] callback, adding them to the returned `Set<T>`.
  /// Corresponds to deserializing Rust's `HashSet<T>` or `BTreeSet<T>` (which
  /// are typically serialized identically to `Vec<T>`).
  ///
  /// **Performance Warning:** Similar to [readList], performance depends on the
  /// number of elements and the complexity of the [readElement] callback.
  ///
  /// #### Rust Type: `HashSet<T>`, `BTreeSet<T>` (Deserialized from `Vec<T>` format)
  /// #### Rust Context Example:
  /// ```rust
  /// use std::collections::HashSet;
  /// #[derive(Deserialize)]
  /// struct UniqueIds { ids: HashSet<u64> }
  /// ```
  /// **Layout:** `[u64 length][element 1 bytes][element 2 bytes]...`
  @override
  Set<T> readSet<T>(T Function() readElement) {
    final length = readU64();
    final Set<T> result = {};
    for (int i = 0; i < length; i++) {
      result.add(readElement());
    }
    return result;
  }

  // ── Numeric Lists ────────────────────────────────────────────────────────

  /// Reads a `Vec<u8>` as `[u64 length][u8, u8, ...]`.
  ///
  /// Returns a `Uint8List.view` directly over the buffer for maximum efficiency.
  ///
  /// #### Rust Context:
  /// ```rust
  /// #[derive(Deserialize)]
  /// struct Blob {
  ///     data: Vec<u8>,
  /// }
  /// ```
  ///
  /// #### Example:
  /// ```dart
  /// final bytes = reader.readUint8List();
  /// ```
  ///
  /// **Layout:** `[u64 length][u8 * length]`
  ///
  /// This method is always zero-copy and alignment-safe.
  @override
  @pragma('vm:prefer-inline')
  Uint8List readUint8List() {
    final length = readU64();
    return readRawBytes(length);
  }

  /// Reads a `Vec<i8>` as `[u64 length][i8, i8, ...]`.
  ///
  /// Returns a `Int8List.view` over the buffer directly.
  ///
  /// #### Rust Context:
  /// ```rust
  /// #[derive(Deserialize)]
  /// struct Audio {
  ///     samples: Vec<i8>,
  /// }
  /// ```
  ///
  /// **Layout:** `[u64 length][i8 * length]`
  ///
  /// This method is always zero-copy and alignment-safe.
  @override
  @pragma('vm:prefer-inline')
  Int8List readInt8List() {
    final length = readU64();
    _check(length);
    final p = _pos;
    _pos = p + length;
    return Int8List.view(
      _bytes.buffer,
      _bytes.offsetInBytes + p,
      length,
    );
  }

  /// Reads a Rust `Vec<u16>` as `[u64 length][u16, u16, ...]` (little-endian).
  ///
  /// Attempts to return a `Uint16List.view`.
  /// Falls back to manual per-element reads if the buffer is not 2-byte aligned.
  ///
  /// **Performance Warning:**
  /// Fallback reduces performance. Use `reader.isAligned(2)` to check alignment before calling.
  ///
  /// #### Rust Context Example:
  /// ```rust
  /// #[derive(Deserialize)]
  /// struct Sample { values: Vec<u16> }
  /// ```
  ///
  /// **Layout:** `[u64 length][u16 * length]`
  @override
  @pragma('vm:prefer-inline')
  Uint16List readUint16List() {
    final length = readU64();
    final byteCount = length * 2;
    _check(byteCount);
    final p = _pos;
    final offset = _bytes.offsetInBytes + p;
    _pos = p + byteCount;

    if ((offset & 1) == 0) {
      return Uint16List.view(_bytes.buffer, offset, length);
    } else {
      final dataView = ByteData.view(_bytes.buffer, offset, byteCount);
      final list = Uint16List(length);
      for (var i = 0; i < length; i++) {
        list[i] = dataView.getUint16(i * 2, Endian.little);
      }
      return list;
    }
  }

  /// Reads a Rust `Vec<u32>` as `[u64 length][u32, u32, ...]` (little-endian).
  ///
  /// Uses zero-copy `Uint32List.view` if the buffer is 4-byte aligned.
  /// Otherwise falls back to slower per-element decoding using `readU32()`.
  ///
  /// **Performance Warning:**
  /// Use `reader.isAligned(4)` to avoid fallback and preserve performance.
  ///
  /// #### Rust Context Example:
  /// ```rust
  /// #[derive(Deserialize)]
  /// struct Flags { bits: Vec<u32> }
  /// ```
  ///
  /// **Layout:** `[u64 length][u32 * length]`
  @override
  @pragma('vm:prefer-inline')
  List<int> readUint32List() {
    final length = readU64();
    final byteCount = length * 4;
    _check(byteCount);
    final p = _pos;
    final offsetBytes = _bytes.offsetInBytes + p;
    _pos = p + byteCount;

    if ((offsetBytes & 3) == 0) {
      return Uint32List.view(_bytes.buffer, offsetBytes, length);
    } else {
      // Unaligned
      final dataView = ByteData.view(_bytes.buffer, offsetBytes, byteCount);
      final list = List<int>.filled(length, 0, growable: false);
      for (int i = 0; i < length; ++i) {
        list[i] = dataView.getUint32(i * 4, Endian.little);
      }
      return list;
    }
  }

  /// Reads a Rust `Vec<f32>` as `[u64 length][f32, f32, ...]` (little-endian).
  ///
  /// Uses `Float32List.view` if buffer is 4-byte aligned.
  /// Falls back to `readF32()` per element on unaligned memory.
  ///
  /// **Performance Warning:**
  /// Use `reader.isAligned(4)` to prevent performance degradation from fallback mode.
  ///
  /// #### Rust Context Example:
  /// ```rust
  /// #[derive(Deserialize)]
  /// struct Curve { points: Vec<f32> }
  /// ```
  ///
  /// **Layout:** `[u64 length][f32 * length]`

  @override
  @pragma('vm:prefer-inline')
  List<double> readFloat32List() {
    final length = readU64();
    final byteCount = length * 4;
    _check(byteCount);
    final p = _pos;
    final offset = _bytes.offsetInBytes + p;
    _pos = p + byteCount;

    if ((offset & 3) == 0) {
      return Float32List.view(_bytes.buffer, offset, length);
    } else {
      // Unaligned
      final dataView = ByteData.view(_bytes.buffer, offset, byteCount);
      final list = Float32List(length);
      for (var i = 0; i < length; i++) {
        list[i] = dataView.getFloat32(i * 4, Endian.little);
      }
      return list;
    }
  }

  /// Reads a Rust `Vec<f64>` as `[u64 length][f64, f64, ...]` (little-endian).
  ///
  /// Uses `Float64List.view` on 8-byte aligned buffers.
  /// If unaligned, falls back to calling `readF64()` repeatedly.
  ///
  /// **Performance Warning:**
  /// Fallback is costly. Use `reader.isAligned(8)` in critical code paths.
  ///
  /// #### Rust Context Example:
  /// ```rust
  /// #[derive(Deserialize)]
  /// struct Measurements { values: Vec<f64> }
  /// ```
  ///
  /// **Layout:** `[u64 length][f64 * length]`

  @override
  @pragma('vm:prefer-inline')
  List<double> readFloat64List() {
    final length = readU64();
    final byteCount = length * 8;
    _check(byteCount);
    final p = _pos;
    final offsetBytes = _bytes.offsetInBytes + p;
    _pos = p + byteCount;

    if ((offsetBytes & 7) == 0) {
      return Float64List.view(_bytes.buffer, offsetBytes, length);
    } else {
      // Unaligned
      final dataView = ByteData.view(_bytes.buffer, offsetBytes, byteCount);
      final list = Float64List(length);
      for (var i = 0; i < length; i++) {
        list[i] = dataView.getFloat64(i * 8, Endian.little);
      }
      return list;
    }
  }

  /// Reads a Rust `Vec<i16>` as `[u64 length][i16, i16, ...]` (little-endian).
  ///
  /// Uses `Int16List.view` if the buffer is 2-byte aligned.
  /// Falls back to element-by-element decoding on unaligned data.
  ///
  /// **Performance Warning:**
  /// Fallback reduces performance. Use `reader.isAligned(2)` before reading.
  ///
  /// #### Rust Context Example:
  /// ```rust
  /// #[derive(Deserialize)]
  /// struct PCM { data: Vec<i16> }
  /// ```
  ///
  /// **Layout:** `[u64 length][i16 * length]`
  @override
  @pragma('vm:prefer-inline')
  List<int> readInt16List() {
    final length = readU64();
    final byteCount = length * 2;
    _check(byteCount);
    final p = _pos;
    final offset = _bytes.offsetInBytes + p;
    _pos = p + byteCount;

    if ((offset & 1) == 0) {
      return Int16List.view(_bytes.buffer, offset, length);
    } else {
      // Unaligned
      final dataView = ByteData.view(_bytes.buffer, offset, byteCount);
      final list = Int16List(length);
      for (var i = 0; i < length; i++) {
        list[i] = dataView.getInt16(i * 2, Endian.little);
      }
      return list;
    }
  }

  /// Reads a Rust `Vec<i32>` as `[u64 length][i32, i32, ...]` (little-endian).
  ///
  /// Zero-copy if the buffer is 4-byte aligned.
  /// Fallback occurs if alignment is off, falling back to `readI32()` per element.
  ///
  /// **Performance Warning:**
  /// Use `reader.isAligned(4)` to check alignment and avoid fallback mode.
  ///
  /// #### Rust Context Example:
  /// ```rust
  /// #[derive(Deserialize)]
  /// struct Signal { deltas: Vec<i32> }
  /// ```
  ///
  /// **Layout:** `[u64 length][i32 * length]`
  @override
  @pragma('vm:prefer-inline')
  List<int> readInt32List() {
    final length = readU64();
    final byteCount = length * 4;
    _check(byteCount);
    final p = _pos;
    final offsetBytes = _bytes.offsetInBytes + p;
    _pos = p + byteCount;

    if ((offsetBytes & 3) == 0) {
      return Int32List.view(_bytes.buffer, offsetBytes, length);
    } else {
      // Unaligned
      final dataView = ByteData.view(_bytes.buffer, offsetBytes, byteCount);
      final list = Int32List(length);
      for (var i = 0; i < length; i++) {
        list[i] = dataView.getInt32(i * 4, Endian.little);
      }
      return list;
    }
  }

  /// Reads a Rust `Vec<i64>` as `[u64 length][i64, i64, ...]` (little-endian).
  ///
  /// Uses `Int64List.view` on 8-byte aligned buffers.
  /// Falls back to slower `readI64()` if unaligned.
  ///
  /// **Performance Warning:**
  /// Use `reader.isAligned(8)` to avoid the fallback path.
  ///
  /// #### Rust Context Example:
  /// ```rust
  /// #[derive(Deserialize)]
  /// struct Timeline { timestamps: Vec<i64> }
  /// ```
  ///
  /// **Layout:** `[u64 length][i64 * length]`

  @override
  @pragma('vm:prefer-inline')
  List<int> readInt64List() {
    final length = readU64();
    final byteCount = length * 8;
    _check(byteCount);
    final p = _pos;
    final offsetBytes = _bytes.offsetInBytes + p;
    _pos = p + byteCount;

    if ((offsetBytes & 7) == 0) {
      return Int64List.view(_bytes.buffer, offsetBytes, length);
    } else {
      // Unaligned
      final dataView = ByteData.view(_bytes.buffer, offsetBytes, byteCount);
      final list = Int64List(length);
      for (var i = 0; i < length; i++) {
        list[i] = dataView.getInt64(i * 8, Endian.little);
      }
      return list;
    }
  }

  /// Reads a Rust `Vec<u64>` as `[u64 length][u64, u64, ...]` (little-endian).
  ///
  /// Returns a `Uint64List.view` when 8-byte alignment is satisfied.
  /// Falls back to calling `readU64()` per element otherwise.
  ///
  /// **Performance Warning:**
  /// Fallback is slower. Use `reader.isAligned(8)` before calling for performance-sensitive code.
  ///
  /// #### Rust Context Example:
  /// ```rust
  /// #[derive(Deserialize)]
  /// struct BigInts { values: Vec<u64> }
  /// ```
  ///
  /// **Layout:** `[u64 length][u64 * length]`
  @override
  @pragma('vm:prefer-inline')
  List<int> readUint64List() {
    final length = readU64();
    final byteCount = length * 8;
    _check(byteCount);
    final p = _pos;
    final offsetBytes = _bytes.offsetInBytes + p;
    _pos = p + byteCount;

    if ((offsetBytes & 7) == 0) {
      return Uint64List.view(_bytes.buffer, offsetBytes, length);
    } else {
      // Unaligned
      final dataView = ByteData.view(_bytes.buffer, offsetBytes, byteCount);
      final list = Uint64List(length);
      for (var i = 0; i < length; i++) {
        list[i] = dataView.getUint64(i * 8, Endian.little);
      }
      return list;
    }
  }

  Uint8List toBytes() => _bytes.buffer.asUint8List();

  // ── Nested Objects ───────────────────────────────────────────────────────

  /// Reads a nested object from a Rust collection-style layout: `[u64 length][...payload bytes]`.
  ///
  /// Internally:
  /// - Reads the payload length using `readU64()`.
  /// - Extracts a sub-slice of the buffer from the current position.
  /// — Constructs a new temporary `BincodeReader` for the slice.
  /// - Passes it to the given [instance]'s `decode()` method.
  ///
  /// Used when decoding objects that are wrapped in `Vec<T>` or other length-prefixed containers.
  ///
  /// #### Rust Context Example:
  /// ```rust
  /// #[derive(Deserialize)]
  /// struct Wrapper { inner: Vec<MyStruct> }
  /// ```
  ///
  /// #### Dart Example:
  /// ```dart
  /// final user = reader.readNestedObjectForCollection(User());
  /// ```
  ///
  /// **Layout:** `[u64 length][nested bytes]`
  ///
  /// **Performance Tip:**
  /// This method allocates a sub-reader and a `Uint8List.view`, but avoids copying.
  @override
  T readNestedObjectForCollection<T extends BincodeDecodable>(T instance) {
    final length = readU64();
    _check(length);
    final slice = Uint8List.sublistView(_bytes, _pos, _pos + length);
    _pos += length;
    final sub = BincodeReader(slice);
    instance.decode(sub);
    return instance;
  }

  /// Reads a fixed-size nested object, assuming its byte size can be calculated ahead of time.
  ///
  /// Internally:
  /// - Uses a cache to retrieve the object's fixed size. If not cached, measures
  ///   it by temporarily encoding a sample instance.
  /// - Checks if enough bytes are available within the current read limit.
  /// - **Decodes the object directly using the current reader instance** by temporarily
  ///   restricting the read limit (`_limit`) to the object's exact size.
  /// - Verifies that the decode consumed the expected number of bytes.
  ///
  /// This method is for Rust structs where the size is known deterministically
  /// (no variable-length collections or strings).
  ///
  /// #### Rust Context Example:
  /// ```rust
  /// #[derive(Deserialize)]
  /// struct Vec3 { x: f32, y: f32, z: f32 } // Fixed 12 bytes
  /// ```
  ///
  /// #### Dart Example:
  /// ```dart
  /// final vec = reader.readNestedObjectForFixed(Vec3());
  /// ```
  ///
  /// **Layout:** `[fixed bytes]` (no length prefix)
  ///
  /// **Performance:**
  /// Generally very fast due to avoiding sub-reader allocation. The primary cost
  /// on the first encounter with a type is measuring its size via `encode()`,
  /// but subsequent reads of the same type benefit from caching.
  @override
  T readNestedObjectForFixed<T extends BincodeCodable>(T instance) {
    final toRead = _getFixedSize(instance);
    _check(toRead);

    final originalLimit = _limit;
    final nestedLimit = _pos + toRead;

    try {
      _limit = nestedLimit;
      instance.decode(this);

      if (_pos != nestedLimit) {
        final consumed = _pos - (nestedLimit - toRead);
        _limit = originalLimit;
        throw BincodeException(
            'Fixed object decode consumed incorrect number of bytes for type ${instance.runtimeType}. Expected: $toRead, Actual: $consumed. Reader state: pos=$_pos, limit=$_limit');
      }
    } finally {
      _limit = originalLimit;
    }
    return instance;
  }

  /// Reads an optional nested object encoded with a tag + length-prefixed payload.
  /// Layout: `[u8 tag][u64 length][...payload]`
  ///
  /// - If [tag] is `0`, returns `null`.
  /// - If [tag] is `1`, reads the next `u64` bytes and decodes it as a nested object.
  /// - Throws [InvalidOptionTagException] if tag is not `0` or `1`.
  ///
  /// Uses a fresh `BincodeReader` over the slice for decoding.
  /// The [creator] function is called *only* if the option is present.
  ///
  /// #### Rust Context Example:
  /// ```rust
  /// #[derive(Deserialize)]
  /// struct Item { child: Option<MyStruct> }
  /// ```
  ///
  /// #### Dart Example:
  /// ```dart
  /// final item = reader.readOptionNestedObjectForCollection(() => Item());
  /// ```
  ///
  /// **Performance Tip:**
  /// Allocates a sub-slice and reader, but avoids deep allocation.
  /// Use only with collection-style nested encoding.
  @override
  T? readOptionNestedObjectForCollection<T extends BincodeDecodable>(
      T Function() creator) {
    final tag = readU8();
    if (tag == 0) return null;
    if (tag != 1) throw InvalidOptionTagException(tag);
    final length = readU64();
    _check(length);
    final slice = Uint8List.sublistView(_bytes, _pos, _pos + length);
    _pos += length;
    final inst = creator();
    final sub = BincodeReader(slice);
    inst.decode(sub);
    return inst;
  }

  /// Reads an optional fixed-size nested object encoded with a tag only (no length).
  /// Layout: `[u8 tag][fixed object bytes]`
  ///
  /// - If [tag] is `0`, returns `null`.
  /// - If [tag] is `1`:
  ///   - Uses a cache to retrieve the object's fixed size (measuring on miss).
  ///   - Checks if enough bytes are available within the current read limit.
  ///   - **Decodes the object directly using the current reader instance** by temporarily
  ///     restricting the read limit (`_limit`) to the object's exact size.
  ///   - Verifies that the decode consumed the expected number of bytes.
  ///
  /// #### Rust Context Example:
  /// ```rust
  /// #[derive(Deserialize)]
  /// struct Header { optional_id: Option<[u8; 8]> }
  /// ```
  ///
  /// #### Dart Example:
  /// ```dart
  /// final header = reader.readOptionNestedObjectForFixed(() => Header());
  /// ```
  ///
  /// **Performance:**
  /// Efficient as it avoids sub-reader allocation when the value is present.
  /// Size measurement via `encode()` occurs only on the first encounter with the type.
  @override
  T? readOptionNestedObjectForFixed<T extends BincodeCodable>(
      T Function() creator) {
    final tag = readU8();
    if (tag == 0) return null;
    if (tag != 1) throw InvalidOptionTagException(tag);

    final sample = creator();
    final toRead = _getFixedSize(sample);

    _check(toRead);

    final originalLimit = _limit;
    final nestedLimit = _pos + toRead;
    final inst = creator();

    try {
      _limit = nestedLimit;
      inst.decode(this);

      if (_pos != nestedLimit) {
        final consumed = _pos - (nestedLimit - toRead);
        _limit = originalLimit;
        throw BincodeException(
            'Fixed option object decode consumed incorrect number of bytes for type ${inst.runtimeType}. Expected: $toRead, Actual: $consumed. Reader state: pos=$_pos, limit=$_limit');
      }
    } finally {
      _limit = originalLimit;
    }
    return inst;
  }

  /// Reads a fixed-size nested object where the exact byte size is provided by the caller.
  /// Bypasses the internal size calculation/cache (_getFixedSize).
  ///
  /// Use this when the size of T is known at compile time (e.g., via a static const)
  /// for potential performance improvement. The caller MUST ensure [knownSize] is correct.
  ///
  /// Throws if the decode operation does not consume exactly [knownSize] bytes or if
  /// [knownSize] exceeds the remaining buffer limit.
  ///
  /// Example:
  /// ```dart
  /// final myStruct = reader.readNestedObjectWithKnownSize(MyFixedStruct(), MyFixedStruct.knownSize);
  /// ```
  @override
  T readNestedObjectWithKnownSize<T extends BincodeDecodable>(
      T instance, int knownSize) {
    if (knownSize < 0) {
      throw BincodeException("Known size cannot be negative: $knownSize");
    }
    _check(knownSize);

    final originalLimit = _limit;
    final startPos = _pos;
    final nestedLimit = startPos + knownSize;

    if (nestedLimit > _limit) {
      throw RangeError(
          'Known size $knownSize read starting at $startPos would exceed reader limit $_limit');
    }

    try {
      _limit = nestedLimit;
      instance.decode(this);

      if (_pos != nestedLimit) {
        final consumed = _pos - startPos;
        _limit = originalLimit;
        throw BincodeException(
            'Decode for ${instance.runtimeType} with knownSize=$knownSize consumed incorrect bytes. Actual: $consumed. Final position: $_pos, Expected end: $nestedLimit');
      }
    } finally {
      _limit = originalLimit;
    }
    return instance;
  }

  /// Reads an optional fixed-size nested object where the exact byte size is provided.
  /// Layout: `[u8 tag][fixed object bytes]`
  /// Bypasses the internal size calculation/cache (_getFixedSize).
  ///
  /// The caller MUST ensure [knownSize] is the correct size for type `T`.
  ///
  /// Example:
  /// ```dart
  /// final myStructOpt = reader.readOptionNestedObjectWithKnownSize(
  ///   () => MyFixedStruct(),
  ///   MyFixedStruct.knownSize
  /// );
  /// ```
  @override
  T? readOptionNestedObjectWithKnownSize<T extends BincodeDecodable>(
      T Function() creator, int knownSize) {
    final tag = readU8();

    if (tag == 0) {
      return null;
    } else if (tag == 1) {
      final instance = creator();
      return readNestedObjectWithKnownSize(instance, knownSize);
    } else {
      throw InvalidOptionTagException(tag);
    }
  }

  /// Reads a [Duration] value using the defined format compatible with Rust's `chrono::Duration`.
  ///
  /// Reads seconds (`i64`) and positive nanoseconds within the second (`u32`)
  /// and reconstructs a Dart `Duration` object.
  ///
  /// #### Rust Type: `chrono::Duration` (via serde)
  /// #### Rust Context Example:
  /// ```rust
  /// #[derive(Deserialize)]
  /// struct Timer { elapsed: chrono::Duration }
  /// ```
  /// **Layout:** `[i64 seconds][u32 positive_nanos]` (little-endian)
  @override
  Duration readDuration() {
    final seconds = readI64();
    final nanoseconds = readU32();

    if (nanoseconds >= 1000000000) {
      throw BincodeException(
          'Invalid nanosecond value read for Duration: $nanoseconds');
    }

    final totalMicroseconds = (seconds * 1000000) + (nanoseconds ~/ 1000);

    return Duration(microseconds: totalMicroseconds);
  }

  /// Reads an enum discriminant (variant index) encoded as a `u32`.
  ///
  /// The caller is responsible for subsequently reading any payload based on the
  /// returned discriminant. Matches Bincode Fixint/legacy mode.
  ///
  /// #### Rust Type: Part of `enum` deserialization
  /// #### Rust Context Example:
  /// ```rust
  /// enum Status { Running, Stopped, Errored = 255 }
  /// let discriminant = reader.read_enum_discriminant(); // Reads 0, 1, or 255 (as u32)
  /// ```
  /// **Layout:** `[u32 discriminant]` (little-endian)
  @override
  int readEnumDiscriminant() {
    // Format: u32 index
    return readU32();
  }

  /// Measures the exact number of bytes a fixed-size object would write using `encode()`.
  ///
  /// This method is used internally to determine how many bytes to read for a fixed-size struct
  /// that doesn't include a length prefix (e.g., Rust types with known memory layout).
  ///
  /// - Clears the temporary writer.
  /// - Calls `encode()` on the [instance].
  /// - Returns the number of bytes written during encoding.
  ///
  /// #### Usage Context:
  /// Used by [readNestedObjectForFixed] and [readOptionNestedObjectForFixed] to calculate
  /// how many bytes to read from the stream when deserializing fixed-size objects.
  ///
  /// #### Rust Context Example:
  /// ```rust
  /// #[derive(Serialize, Deserialize)]
  /// struct Vec3 { x: f32, y: f32, z: f32 } // Always 12 bytes
  /// ```
  ///
  /// #### Dart Example:
  /// ```dart
  /// final reader = BincodeReader(bytes);
  /// final vec = reader.readNestedObjectForFixed(Vec3());
  /// ```
  ///
  /// **Performance Warning:**
  /// This method encodes the object just to measure its size.
  /// Avoid calling frequently in tight loops. If possible, calculate the size once and cache it.
  @pragma('vm:always-inline')
  @Deprecated(
      "Superseded by improved internal size handling and '*...WithKnownSize' methods. "
      "Use 'BincodeReader.readNestedObjectWithKnownSize' or 'BincodeWriter.writeNestedObjectWithKnownSize' "
      "if explicit size validation is needed. This method may be removed in a future release (e.g., v4.0.0).")
  int measureFixedSize<T extends BincodeCodable>(T instance) {
    _tmpWriter.rewind();
    instance.encode(_tmpWriter);
    return _tmpWriter.getBytesWritten();
  }
}
