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
import 'dart:io';
import 'dart:typed_data';

import 'package:d_bincode/src/exception/exception.dart';

import 'builder.dart';
import 'codable.dart';

/// A fast, flexible binary serializer for Rust-compatible Bincode data.
///
/// `BincodeWriter` provides low-level control over binary encoding of Dart objects
/// using the same layout as Rust’s `bincode` format. It is optimized for
/// performance, alignment correctness, and composability with other low-level APIs.
class BincodeWriter implements BincodeWriterBuilder {
  // — internal buffer and cursors —
  Uint8List _bytes;

  int _pos = 0; // write cursor
  int _length = 0; // high‑water mark
  int _capacity; // buffer size

  /// The managed ByteData view, updated when _bytes changes.
  ByteData? _dataView;

  /// A fast, flexible binary serializer for Rust-compatible Bincode data.
  ///
  /// `BincodeWriter` provides low-level control over binary encoding of Dart objects
  /// using the same layout as Rust’s `bincode` format. It is optimized for
  /// performance, alignment correctness, and composability with other low-level APIs.
  ///
  /// ### Core Capabilities
  /// - Encodes primitive values (`u8`, `i32`, `f64`, etc.) with little-endian layout
  /// - Writes Rust-style `Option<T>` types using 1-byte tags
  /// - Supports strings (length-prefixed or fixed-length)
  /// - Writes nested structs using fixed-size or collection-style layout
  /// - Serializes `Vec<T>` and `HashMap<K, V>` as sequences with length prefix
  ///
  /// ### Performance & Safety
  /// - Uses a dynamically growing internal buffer (default 128 bytes, expandable)
  /// - Avoids allocations for primitives and numeric arrays when possible
  /// - Uses `TypedData.view` for bulk writes (e.g. `Float64List`) when input is typed
  /// - `reserve()` allows manual preallocation for large payloads
  /// - Reusable: call `reset()` or `rewind()` to reuse the same writer instance
  ///
  /// ### Memory Behavior
  /// - Zero-copy for writing raw views like `Uint8List`, `Int16List`, etc.
  /// - Scalar types and strings are encoded by value, with no reference retained
  /// - Temporary shared buffer is used for float conversion (no allocation per write)
  ///
  /// ### Example
  /// ```dart
  /// final writer = BincodeWriter();
  /// writer.writeU8(1);
  /// writer.writeF64(3.14);
  /// writer.writeString("hello");
  /// final encoded = writer.toBytes(); // Uint8List of written data
  /// ```
  ///
  /// ### Supported Rust Layouts
  /// - `Vec<T>` → `[u64 length][T, T, ...]`
  /// - `Option<T>` → `[0]` if null, `[1][T]` if value exists
  /// - Structs → fixed layout or collection-style nested with length prefix
  BincodeWriter({
    int initialCapacity = 128,
  })  : assert(initialCapacity > 0),
        _capacity = initialCapacity,
        _bytes = Uint8List(initialCapacity) {
    _updateView();
  }

  void _updateView() {
    _dataView = ByteData.view(_bytes.buffer, _bytes.offsetInBytes, _capacity);
  }

  // ─── Growth & Tracking ──────────────────────────────────────────────────────

  @pragma('vm:prefer-inline')
  void _ensureCapacity(int needed) {
    final minRequired = _pos + needed;
    if (minRequired > _capacity) {
      int newCap = _capacity * 2;
      if (newCap < minRequired) newCap = minRequired;

      final newBytes = Uint8List(newCap);
      newBytes.setRange(0, _length, _bytes, 0);

      _bytes = newBytes;
      _capacity = newCap;
      _updateView();
    }
  }

  @pragma('vm:prefer-inline')
  void _track(int newPos) {
    _pos = newPos;
    if (newPos > _length) {
      _length = newPos;
    }
  }

  // ─── Utils ──────────────────────────────────────────────────────

  /// Resets the writer to an empty state, clearing any previously written bytes.
  ///
  /// After calling, both [position] and [getBytesWritten] will return 0.
  ///
  /// #### Example
  /// ```dart
  /// final w = BincodeWriter();
  /// w.writeU8(1);
  /// w.writeU8(2);
  /// print(w.getBytesWritten()); // 2
  /// w.reset();
  /// print(w.getBytesWritten()); // 0
  /// print(w.position);         // 0
  /// ```
  void reset() {
    _pos = 0;
    _length = 0;
  }

  /// Resets the writer and initializes it with a new buffer of the provided size.
  /// The position and the bytes written are reset, and the writer is ready to write new data.
  void resetWith(int newCapacity) {
    // Create a new buffer of the specified size
    _bytes = Uint8List(newCapacity);
    _capacity = newCapacity;

    // Reset internal positions
    _pos = 0;
    _length = 0;
    _updateView();
  }

  /// Helps track how full the buffer is (especially if you pre-allocated).
  int get remainingCapacity => _capacity - _pos;

  /// Returns the number of bytes written so far (the high‑water mark).
  ///
  /// This tells you exactly how many valid bytes are in the internal buffer.
  ///
  /// #### Example
  /// ```dart
  /// final w = BincodeWriter();
  /// w.writeU16(0xABCD);
  /// print(w.getBytesWritten()); // 2
  /// ```
  int getBytesWritten() => _length;

  /// Returns a `Uint8List` view over the valid bytes written so far.
  ///
  /// This is a slice of the internal buffer from 0 up to [getBytesWritten].
  /// Further writes to the writer will not modify this returned list.
  ///
  /// #### Example
  /// ```dart
  /// final w = BincodeWriter();
  /// w.writeU8(42);
  /// final bytes = w.toBytes(); // [42]
  /// ```
  @override
  Uint8List toBytes() => Uint8List.view(_bytes.buffer, 0, _length);

  /// Returns a `ByteBuffer` containing only the valid data written so far.
  ///
  /// Equivalent to `toBytes().buffer`.
  ///
  /// #### Example
  /// ```dart
  /// final w = BincodeWriter();
  /// w.writeU32(0x12345678);
  /// final buf = w.buffer;
  /// // `buf` now holds exactly 4 bytes: 0x78,0x56,0x34,0x12
  /// ```
  ByteBuffer get buffer => toBytes().buffer;

// ─── Positioning Helpers ────────────────────────────────────────────────────

  /// Moves the write cursor forward by [offset] bytes.
  ///
  /// #### Solves:
  /// Allows you to skip ahead (e.g. reserve space for a header) without writing.
  ///
  /// #### Example:
  /// ```dart
  /// writer.seek(4);      // leave 4 bytes blank for a length prefix
  /// // ... write payload ...
  /// writer.position = 0; // come back and fill in the prefix
  /// ```
  @override
  void seek(int offset) {
    final newPos = _pos + offset;
    if (newPos < 0 || newPos > _capacity) {
      throw RangeError(
          'Seeking by $offset from $_pos results in invalid position $newPos for capacity $_capacity');
    }
    _pos = newPos;
  }

  /// Returns the current write cursor position (0‑based).
  ///
  /// #### Solves:
  /// Let’s you inspect how many bytes have already been written.
  ///
  /// #### Example:
  /// ```dart
  /// print('At byte index ${writer.position}');
  /// ```
  @override
  int get position => _pos;

  /// Sets the write cursor to [v].
  ///
  /// #### Solves:
  /// Jump around in the buffer, e.g. to backpatch headers or overwrite fields.
  ///
  /// #### Example:
  /// ```dart
  /// writer.position = 2;  // next write will go at byte index 2
  /// ```
  @override
  set position(int v) {
    if (v < 0 || v > _capacity) {
      throw RangeError(
          'Position $v is out of bounds for buffer capacity $_capacity');
    }
    _pos = v;
  }

  /// Reset the write cursor to the start (does **not** clear existing bytes).
  ///
  /// #### Solves:
  /// Quickly start a fresh serialization pass (paired with `reset()`).
  ///
  /// #### Example:
  /// ```dart
  /// writer.rewind();
  /// writer.writeU8(0x01); // overwrites the first byte
  /// ```
  @override
  void rewind() => _pos = 0;

  /// Moves the write cursor to the end of the current data (`length`).
  ///
  /// #### Solves:
  /// Continue appending after a back‑patch or inspection.
  ///
  /// #### Example:
  /// ```dart
  /// writer.rewind();
  /// writer.writeU8(0x00);      // overwrite first byte
  /// writer.skipToEnd();
  /// writer.writeU8(0xFF);      // re‑append at the end
  /// ```
  @override
  void skipToEnd() => _pos = _length;

  /// Sets the write cursor to an absolute byte offset [offset].
  ///
  /// #### Solves:
  /// Same as `position = offset`, but reads more naturally.
  ///
  /// #### Example:
  /// ```dart
  /// writer.seekTo(headerStart);
  /// ```
  @override
  void seekTo(int offset) => _pos = offset;

// ─── One‑Shot Encoding Helpers ───────────────────────────────────────────────

  /// Reset, serialize [value], and return exactly the bytes written.
  ///
  /// #### Solves:
  /// Encapsulates “reset + encode + toBytes” into one simple call.
  ///
  /// #### Example:
  /// ```dart
  /// final bytes = BincodeWriter.encodeToBytes(myStruct);
  /// ```
  Uint8List encodeToBytes(BincodeCodable value) {
    reset();
    value.encode(this);
    return toBytes();
  }

  /// “Dry‑run” encode of [value] to measure its final byte length, without copies.
  ///
  /// #### Solves:
  /// Let’s you pre‑allocate or reserve the exact needed buffer size first.
  ///
  /// #### Example:
  /// ```dart
  /// final size = writer.measure(myStruct);
  /// writer.reserve(size);
  /// writer.encodeToBytes(myStruct);
  /// ```
  int measure(BincodeCodable value) {
    reset();
    value.encode(this);
    return getBytesWritten();
  }

  /// Static shortcut for `BincodeWriter(initialCapacity).encodeToBytes(value)`.
  ///
  /// #### Solves:
  /// One‑liner encoding when you don’t need to reuse a writer.
  ///
  /// #### Example:
  /// ```dart
  /// final bytes = BincodeWriter.encode(myStruct);
  /// ```
  static Uint8List encode(BincodeCodable value, {int initialCapacity = 128}) {
    final w = BincodeWriter(initialCapacity: initialCapacity);
    return w.encodeToBytes(value);
  }

  /// Ensure the buffer can hold at least [minCapacity] bytes without resizing.
  ///
  /// #### Solves:
  /// Avoid mid‑serialization growth/copies if you already know your payload size.
  ///
  /// #### Example:
  /// ```dart
  /// final expected = writer.measure(header);
  /// writer.reserve(expected + extraSpace);
  /// writer.encodeToBytes(header);
  /// ```
  void reserve(int minCapacity) {
    if (_capacity < minCapacity) {
      final newCap =
          (minCapacity > _capacity * 2) ? minCapacity : _capacity * 2;
      final newBytes = Uint8List(newCap)..setRange(0, _length, _bytes, 0);
      _bytes = newBytes;
      _capacity = newCap;
      _updateView();
    }
  }

  /// Reset, serialize [value], and stream its raw bytes into an [IOSink].
  ///
  /// #### Solves:
  /// Encoding directly to a file/socket without ever allocating a full `Uint8List`.
  ///
  /// #### Example:
  /// ```dart
  /// await writer.encodeToSink(myStruct, socket);
  /// ```
  Future<void> encodeToSink(BincodeCodable value, IOSink sink) async {
    reset();
    value.encode(this);
    sink.add(Uint8List.view(_bytes.buffer, _bytes.offsetInBytes, _length));
    await sink.flush();
  }

// ─── Formatting & Output Helpers ─────────────────────────────────────────────

  /// Produce a hex‑dump of [bytes], with `-` between each two‑digit hex byte.
  ///
  /// #### Example:
  /// ```dart
  /// final hex = BincodeWriter.toHex(bytes);
  /// print(hex); // "ff-00-1a-2b"
  /// ```
  static String toHex(Uint8List bytes) {
    if (bytes.isEmpty) return '';
    final sb = StringBuffer();
    for (var i = 0; i < bytes.length; i++) {
      sb.write(bytes[i].toRadixString(16).padLeft(2, '0'));
      if (i + 1 < bytes.length) sb.write('-');
    }
    return sb.toString();
  }

  /// Base64‑encode a freshly serialized [value].
  ///
  /// #### Solves:
  /// Embedding binary blobs in text‑only formats (JSON, logs, etc.).
  ///
  /// #### Example:
  /// ```dart
  /// final b64 = BincodeWriter.encodeToBase64(myStruct);
  /// ```
  static String encodeToBase64(BincodeCodable value) {
    final bytes = encode(value);
    return base64.encode(bytes);
  }

  /// Synchronously serialize [value] and write the bytes to disk at [path].
  ///
  /// #### Solves:
  /// One‑line, blocking file‑dump for quick CLI tools or tests.
  ///
  /// #### Example:
  /// ```dart
  /// BincodeWriter.encodeToFileSync(myStruct, 'out.bin');
  /// ```
  static void encodeToFileSync(
    BincodeCodable value,
    String path, {
    int initialCapacity = 128,
  }) {
    final bytes = encode(value, initialCapacity: initialCapacity);
    File(path).writeAsBytesSync(bytes, flush: true);
  }

// ─── Primitive Writes ───────────────────────────────────────────────────────

  /// Writes an unsigned 8‑bit integer [v], matching Rust’s `u8`.
  ///
  /// 1. Ensures there is space for 1 byte.
  /// 2. Masks and stores the low 8 bits of [v] into the buffer.
  /// 3. Advances the write cursor and updates the high‑water mark.
  ///
  /// #### Rust Context Example:
  /// ```rust
  /// #[derive(Serialize)]
  /// struct Packet { id: u8 }
  /// // bincode::serialize(&Packet { id: 255 }) yields one byte [0xFF].
  /// ```
  @override
  @pragma('vm:prefer-inline')
  void writeU8(int v) {
    _ensureCapacity(1);
    final p = _pos;
    _dataView!.setUint8(p, v);
    _track(_pos + 1);
  }

  /// Writes an unsigned 16‑bit integer [v] in little‑endian order (`u16`).
  ///
  /// 1. Ensures space for 2 bytes.
  /// 2. Stores the low byte, then the high byte.
  /// 3. Advances cursor by 2.
  ///
  /// #### Rust Context Example:
  /// ```rust
  /// struct Header { version: u16 }
  /// // serializes to two bytes: low then high.
  /// ```
  @override
  @pragma('vm:prefer-inline')
  void writeU16(int v) {
    _ensureCapacity(2);
    final p = _pos;
    _dataView!.setUint16(p, v, Endian.little);
    _track(p + 2);
  }

  /// Writes an unsigned 32‑bit integer [v] in little‑endian order (`u32`).
  ///
  /// 1. Ensures space for 4 bytes.
  /// 2. Unpacks and stores each of the four bytes from least to most significant.
  /// 3. Advances cursor by 4.
  ///
  /// #### Rust Context Example:
  /// ```rust
  /// struct Record { count: u32 }
  /// ```
  @override
  @pragma('vm:prefer-inline')
  void writeU32(int v) {
    _ensureCapacity(4);
    final p = _pos;
    _dataView!.setUint32(p, v, Endian.little);
    _track(p + 4);
  }

  /// Writes an unsigned 64‑bit integer [v] in little‑endian order (`u64`).
  ///
  /// 1. Ensures space for 8 bytes.
  /// 2. Writes bytes 0 through 7 (LSB first).
  /// 3. Advances cursor by 8.
  ///
  /// #### Rust Context Example:
  /// ```rust
  /// struct Timestamp { nanos: u64 }
  /// ```
  @override
  @pragma('vm:prefer-inline')
  void writeU64(int v) {
    _ensureCapacity(8);
    final p = _pos;
    _dataView!.setUint64(p, v, Endian.little);
    _track(p + 8);
  }

  /// Writes a signed 8‑bit integer [v] (`i8`) by reusing [writeU8].
  ///
  /// Two’s‑complement values are stored identically as `u8`.
  ///
  /// #### Rust Context Example:
  /// ```rust
  /// struct Value { signed: i8 }
  /// ```
  @override
  @pragma('vm:prefer-inline')
  void writeI8(int v) => writeU8(v);

  /// Writes a signed 16‑bit integer [v] (`i16`) in little‑endian format.
  ///
  /// Identical to `u16` representation, but interprets the bits as signed.
  ///
  /// #### Rust Context Example:
  /// ```rust
  /// struct Reading { temp: i16 }
  /// ```
  @override
  @pragma('vm:prefer-inline')
  void writeI16(int v) {
    _ensureCapacity(2);
    final p = _pos;
    _dataView!.setInt16(p, v, Endian.little);
    _track(p + 2);
  }

  /// Writes a signed 32‑bit integer [v] (`i32`) in little‑endian format.
  ///
  /// Same byte‑packing as `u32`, two’s complement interpretation.
  ///
  /// #### Rust Context Example:
  /// ```rust
  /// struct Vec2 { x: i32 }
  /// ```
  @override
  @pragma('vm:prefer-inline')
  void writeI32(int v) {
    _ensureCapacity(4);
    final p = _pos;
    _dataView!.setInt32(p, v, Endian.little);
    _track(p + 4);
  }

  /// Writes a signed 64‑bit integer [v] (`i64`) in little‑endian format.
  ///
  /// Same layout as `u64`, interpreted as signed two’s complement.
  ///
  /// #### Rust Context Example:
  /// ```rust
  /// struct Balance { amount: i64 }
  /// ```
  @override
  @pragma('vm:prefer-inline')
  void writeI64(int v) {
    _ensureCapacity(8);
    final p = _pos;
    _dataView!.setInt64(p, v, Endian.little);
    _track(p + 8);
  }

  /// Writes a 32‑bit float [x] in little‑endian IEEE‑754 format (`f32`).
  ///
  /// 1. Ensures 4‑byte space.
  /// 2. Stores the 32‑bit bitpattern via a shared scratch buffer.
  /// 3. Copies bytes into `_bytes` then advances.
  ///
  /// #### Rust Context Example:
  /// ```rust
  /// struct Point { x: f32 }
  /// ```
  @override
  @pragma('vm:prefer-inline')
  void writeF32(double x) {
    _ensureCapacity(4);
    final p = _pos;
    _dataView!.setFloat32(p, x, Endian.little);
    _track(p + 4);
  }

  /// Writes a 64‑bit float [x] in little‑endian IEEE‑754 format (`f64`).
  ///
  /// Uses  scratch buffer trick to avoid repeated allocations.
  ///
  /// #### Rust Context Example:
  /// ```rust
  /// struct Point { y: f64 }
  /// ```
  @override
  @pragma('vm:prefer-inline')
  void writeF64(double x) {
    _ensureCapacity(8);
    final p = _pos;
    _dataView!.setFloat64(p, x, Endian.little);
    _track(p + 8);
  }

  // ─── Common Writes ────────────────────────────────────────────────────────

  /// Writes a boolean [b] as a single byte: `1` for `true`, `0` for `false`.
  ///
  /// This corresponds to `bool` in Rust.
  ///
  /// #### Rust Context Example:
  /// ```rust
  /// struct State {
  ///     active: bool,
  /// }
  /// ```
  /// **Layout:** `[1]` for `true`, `[0]` for `false`
  @override
  void writeBool(bool b) => writeU8(b ? 1 : 0);

  /// Writes a raw list of bytes [data] directly into the buffer.
  ///
  /// No length prefix is written. This is a low-level method
  /// used by higher-level constructs like `writeString`.
  ///
  /// #### Rust Context Example:
  /// ```rust
  /// struct RawPayload {
  ///     buffer: Vec<u8>, // used with length prefix in Rust
  /// }
  /// ```
  /// **Layout:** `[....]` raw bytes only
  @override
  void writeBytes(List<int> data) {
    final len = data.length;
    _ensureCapacity(len);
    final end = _pos + len;
    if (data is Uint8List) {
      _bytes.setRange(_pos, end, data);
    } else {
      for (int i = 0; i < len; ++i) {
        _bytes[_pos + i] = data[i];
      }
    }
    _track(end);
  }

  /// Writes a UTF‑8 encoded [String] with a `u64` length prefix.
  ///
  /// Encodes Rust's `String` or `&str` by prefixing the byte count
  /// as `u64`, followed by the UTF‑8 bytes of the string.
  ///
  /// #### Rust Context Example:
  /// ```rust
  /// struct Message {
  ///     text: String,
  /// }
  /// ```
  /// **Layout:** `[len: u64][utf8-bytes...]`
  @override
  void writeString(String s) {
    final bytes = utf8.encode(s);
    writeU64(bytes.length);
    writeBytes(bytes);
  }

  /// Writes a fixed-length UTF‑8 string padded with zeros.
  ///
  /// This mimics Rust's with a
  /// constant-length array `[u8; N]` for fixed string fields.
  ///
  /// #### Rust Context Example:
  /// ```rust
  /// struct Header {
  ///     label: [u8; 16], // fixed size
  /// }
  /// ```
  /// **Layout:** `[utf8...][0 0 ...]` (padded to exactly `len` bytes)
  @override
  void writeFixedString(String s, int len) {
    _ensureCapacity(len);
    final bytes = utf8.encode(s);
    final p = _pos;
    final copyLen = bytes.length < len ? bytes.length : len;
    _bytes.setRange(p, p + copyLen, bytes);
    if (copyLen < len) {
      _bytes.fillRange(p + copyLen, p + len, 0);
    }
    _track(p + len);
  }

  /// Writes a single Dart character (a String of length 1) as a Rust `char`.
  ///
  /// Writes the character's Unicode code point (rune) as a little-endian `u32`.
  /// This format is compatible with Bincode 1.x and Bincode 2.x legacy/Fixint mode.
  /// Note: Bincode 2.x default mode uses variable-length UTF-8 for chars.
  /// Throws [ArgumentError] if [char] is not a single character string.
  ///
  /// #### Rust Type: `char`
  /// #### Rust Context Example:
  /// ```rust
  /// #[derive(Serialize)]
  /// struct Message { first_initial: char }
  /// ```
  /// **Layout:** `[u32 rune]` (little-endian)
  @override
  void writeChar(String char) {
    if (char.length != 1) {
      throw ArgumentError(
          'writeChar expects a single character string, got length ${char.length}');
    }
    // (Bincode v1 / legacy style)
    writeU32(char.runes.first);
  }

  /// Writes an enum discriminant (variant index) as a `u32`.
  ///
  /// The caller is responsible for subsequently writing any payload associated
  /// with the variant. This matches the typical Bincode representation (in Fixint/legacy mode)
  /// for C-like enums or the discriminant part of enums with data.
  /// Throws [ArgumentError] if [discriminant] is negative.
  ///
  /// #### Rust Type: Part of `enum` serialization
  /// #### Rust Context Example:
  /// ```rust
  /// enum Status { Running, Stopped, Errored = 255 }
  /// ```
  /// **Layout:** `[u32 discriminant]` (little-endian)
  @override
  void writeEnumDiscriminant(int discriminant) {
    if (discriminant < 0) {
      throw ArgumentError(
          'Enum discriminant cannot be negative: $discriminant');
    }
    //  u32 for enum discriminants in Fixint/legacy mode
    writeU32(discriminant);
  }

  // --- Optional Value Writing Methods ---

  /// Writes an optional boolean [value] according to Bincode's `Option<T>` encoding.
  ///
  /// Writes a 1-byte flag (0 for None/null, 1 for Some/non-null). If Some,
  /// it then writes the boolean value using [writeBool] (as a single byte, 0 or 1).
  /// Corresponds to serializing `Option<bool>` in Rust.
  ///
  /// #### Rust Context Example:
  /// ```rust
  /// #[derive(Serialize)]
  /// struct Flags {
  ///     is_enabled: Option<bool>, // Use for this field
  /// }
  /// ```
  /// **Layout:** `[0]` if `value` is `null`, `[1][0]` (false) or `[1][1]` (true) otherwise.
  @override
  void writeOptionBool(bool? value) {
    writeU8(value != null ? 1 : 0);
    if (value != null) writeBool(value);
  }

  /// Writes an optional unsigned 8-bit integer [value] according to Bincode's `Option<T>` encoding.
  ///
  /// Writes a 1-byte flag (0 for None/null, 1 for Some/non-null). If Some,
  /// it then writes the `u8` value using [writeU8].
  /// Corresponds to serializing `Option<u8>` in Rust.
  ///
  /// #### Rust Context Example:
  /// ```rust
  /// #[derive(Serialize)]
  /// struct Config {
  ///     priority: Option<u8>, // Use for this field
  /// }
  /// ```
  /// **Layout:** `[0]` if `value` is `null`, `[1][u8 byte]` otherwise.
  @override
  void writeOptionU8(int? value) {
    writeU8(value != null ? 1 : 0);
    if (value != null) writeU8(value);
  }

  /// Writes an optional unsigned 16-bit integer [value] according to Bincode's `Option<T>` encoding.
  ///
  /// Writes a 1-byte flag (0 for None/null, 1 for Some/non-null). If Some,
  /// it then writes the `u16` value using [writeU16].
  /// Corresponds to serializing `Option<u16>` in Rust.
  ///
  /// #### Rust Context Example:
  /// ```rust
  /// #[derive(Serialize)]
  /// struct Item {
  ///     item_id: Option<u16>, // Use for this field
  /// }
  /// ```
  /// **Layout:** `[0]` if `value` is `null`, `[1][u16 bytes]` otherwise.
  @override
  void writeOptionU16(int? value) {
    writeU8(value != null ? 1 : 0);
    if (value != null) writeU16(value);
  }

  /// Writes an optional unsigned 32-bit integer [value] according to Bincode's `Option<T>` encoding.
  ///
  /// Writes a 1-byte flag (0 for None/null, 1 for Some/non-null). If Some,
  /// it then writes the `u32` value using [writeU32].
  /// Corresponds to serializing `Option<u32>` in Rust.
  ///
  /// #### Rust Context Example:
  /// ```rust
  /// #[derive(Serialize)]
  /// struct Message {
  ///     sequence_num: Option<u32>, // Use for this field
  /// }
  /// ```
  /// **Layout:** `[0]` if `value` is `null`, `[1][u32 bytes]` otherwise.
  @override
  void writeOptionU32(int? value) {
    writeU8(value != null ? 1 : 0);
    if (value != null) writeU32(value);
  }

  /// Writes an optional unsigned 64-bit integer [value] according to Bincode's `Option<T>` encoding.
  ///
  /// Writes a 1-byte flag (0 for None/null, 1 for Some/non-null). If Some,
  /// it then writes the `u64` value using [writeU64].
  /// Corresponds to serializing `Option<u64>` in Rust.
  ///
  /// #### Rust Context Example:
  /// ```rust
  /// #[derive(Serialize)]
  /// struct Timestamps {
  ///     last_updated_ns: Option<u64>, // Use for this field
  /// }
  /// ```
  /// **Layout:** `[0]` if `value` is `null`, `[1][u64 bytes]` otherwise.
  @override
  void writeOptionU64(int? value) {
    writeU8(value != null ? 1 : 0);
    if (value != null) writeU64(value);
  }

  /// Writes an optional signed 8-bit integer [value] according to Bincode's `Option<T>` encoding.
  ///
  /// Writes a 1-byte flag (0 for None/null, 1 for Some/non-null). If Some,
  /// it then writes the `i8` value using [writeI8].
  /// Corresponds to serializing `Option<i8>` in Rust.
  ///
  /// #### Rust Context Example:
  /// ```rust
  /// #[derive(Serialize)]
  /// struct Adjustment {
  ///     delta: Option<i8>, // Use for this field
  /// }
  /// ```
  /// **Layout:** `[0]` if `value` is `null`, `[1][i8 byte]` otherwise.
  @override
  void writeOptionI8(int? value) {
    writeU8(value != null ? 1 : 0);
    if (value != null) writeI8(value);
  }

  /// Writes an optional signed 16-bit integer [value] according to Bincode's `Option<T>` encoding.
  ///
  /// Writes a 1-byte flag (0 for None/null, 1 for Some/non-null). If Some,
  /// it then writes the `i16` value using [writeI16].
  /// Corresponds to serializing `Option<i16>` in Rust.
  ///
  /// #### Rust Context Example:
  /// ```rust
  /// #[derive(Serialize)]
  /// struct Point {
  ///     z_coord: Option<i16>, // Use for this field
  /// }
  /// ```
  /// **Layout:** `[0]` if `value` is `null`, `[1][i16 bytes]` otherwise.
  @override
  void writeOptionI16(int? value) {
    writeU8(value != null ? 1 : 0);
    if (value != null) writeI16(value);
  }

  /// Writes an optional signed 32-bit integer [value] according to Bincode's `Option<T>` encoding.
  ///
  /// Writes a 1-byte flag (0 for None/null, 1 for Some/non-null). If Some,
  /// it then writes the `i32` value using [writeI32].
  /// Corresponds to serializing `Option<i32>` in Rust.
  ///
  /// #### Rust Context Example:
  /// ```rust
  /// #[derive(Serialize)]
  /// struct Event {
  ///     event_code: Option<i32>, // Use for this field
  /// }
  /// ```
  /// **Layout:** `[0]` if `value` is `null`, `[1][i32 bytes]` otherwise.
  @override
  void writeOptionI32(int? value) {
    writeU8(value != null ? 1 : 0);
    if (value != null) writeI32(value);
  }

  /// Writes an optional signed 64-bit integer [value] according to Bincode's `Option<T>` encoding.
  ///
  /// Writes a 1-byte flag (0 for None/null, 1 for Some/non-null). If Some,
  /// it then writes the `i64` value using [writeI64].
  /// Corresponds to serializing `Option<i64>` in Rust.
  ///
  /// #### Rust Context Example:
  /// ```rust
  /// #[derive(Serialize)]
  /// struct FileInfo {
  ///     size_bytes: Option<i64>, // Use for this field
  /// }
  /// ```
  /// **Layout:** `[0]` if `value` is `null`, `[1][i64 bytes]` otherwise.
  @override
  void writeOptionI64(int? value) {
    writeU8(value != null ? 1 : 0);
    if (value != null) writeI64(value);
  }

  /// Writes an optional 32-bit floating point number [value] according to Bincode's `Option<T>`encoding.
  ///
  /// Writes a 1-byte flag (0 for None/null, 1 for Some/non-null). If Some,
  /// it then writes the `f32` value using [writeF32].
  /// Corresponds to serializing `Option<f32>` in Rust.
  ///
  /// #### Rust Context Example:
  /// ```rust
  /// #[derive(Serialize)]
  /// struct Measurement {
  ///     temperature: Option<f32>, // Use for this field
  /// }
  /// ```
  /// **Layout:** `[0]` if `value` is `null`, `[1][f32 bytes]` otherwise.
  @override
  void writeOptionF32(double? value) {
    writeU8(value != null ? 1 : 0);
    if (value != null) writeF32(value);
  }

  /// Writes an optional 64-bit floating point number [value] according to Bincode's `Option<T>` encoding.
  ///
  /// Writes a 1-byte flag (0 for None/null, 1 for Some/non-null). If Some,
  /// it then writes the `f64` value using [writeF64].
  /// Corresponds to serializing `Option<f64>` in Rust.
  ///
  /// #### Rust Context Example:
  /// ```rust
  /// #[derive(Serialize)]
  /// struct Coordinates {
  ///     longitude: Option<f64>, // Use for this field
  /// }
  /// ```
  /// **Layout:** `[0]` if `value` is `null`, `[1][f64 bytes]` otherwise.
  @override
  void writeOptionF64(double? value) {
    writeU8(value != null ? 1 : 0);
    if (value != null) writeF64(value);
  }

  /// Writes an optional string [value] according to Bincode's `Option<T>` encoding.
  ///
  /// Writes a 1-byte flag (0 for None/null, 1 for Some/non-null). If Some,
  /// it then writes the string value using [writeString], which includes a
  /// U64 length prefix followed by the string bytes (default UTF-8).
  /// Corresponds to serializing `Option<String>` in Rust.
  ///
  /// #### Rust Context Example:
  /// ```rust
  /// #[derive(Serialize)]
  /// struct UserProfile {
  ///     nickname: Option<String>, // Use for this field
  /// }
  /// ```
  /// **Layout:** `[0]` if `value` is `null`, `[1][u64 length][string bytes...]` otherwise.
  @override
  void writeOptionString(String? value) {
    writeU8(value != null ? 1 : 0);
    if (value != null) writeString(value);
  }

  /// Writes an optional fixed-length string representation [value] to the buffer.
  ///
  /// Writes a 1-byte flag (0 for None/null, 1 for Some/non-null). If Some,
  /// it then writes a fixed number of bytes (`length`) derived from the string
  /// using [writeFixedString], padding with zeros or truncating as necessary.
  ///
  /// #### Rust Context Example (Conceptual):
  /// ```rust
  /// #[derive(Serialize)]
  /// struct Record {
  ///     // If tag is optional and always stored in 8 bytes (UTF-8, padded/truncated)
  ///     optional_tag: Option<[u8; 8]>, // Use for this field
  /// }
  /// ```
  /// **Layout:** `[0]` if `value` is `null`, `[1][N fixed bytes...]` otherwise.
  @override
  void writeOptionFixedString(String? value, int length) {
    writeU8(value != null ? 1 : 0);
    if (value != null) writeFixedString(value, length);
  }

  /// Writes an optional single Dart character [char] as `Option<char>`.
  ///
  /// Writes a tag (u8: 0=None, 1=Some) followed by the char (as u32 rune) if present.
  /// Throws if [char] is non-null and not a single character.
  ///
  /// #### Rust Type: `Option<char>`
  /// #### Rust Context Example:
  /// ```rust
  /// #[derive(Serialize)]
  /// struct MaybeInitial { initial: Option<char> }
  /// ```
  /// **Layout:** `[0]` or `[1][u32 rune]`
  @override
  void writeOptionChar(String? char) {
    writeU8(char != null ? 1 : 0);
    if (char != null) writeChar(char); // Delegates validation to writeChar
  }

  /// Writes an optional [Duration] value using the defined format.
  ///
  /// Writes a tag (u8: 0=None, 1=Some). If Some, writes the Duration as
  /// seconds (i64) + positive nanoseconds (u32).
  ///
  /// #### Rust Type: `Option<chrono::Duration>` (via serde)
  /// #### Rust Context Example:
  /// ```rust
  /// #[derive(Serialize)]
  /// struct Task { estimated_time: Option<chrono::Duration> }
  /// ```
  /// **Layout:** `[0]` or `[1][i64 seconds][u32 positive_nanos]`
  @override
  void writeOptionDuration(Duration? value) {
    writeU8(value != null ? 1 : 0);
    if (value != null) {
      writeDuration(value);
    }
  }

  /// Writes an optional enum discriminant (variant index).
  ///
  /// Writes the `u8` tag (0 for None, 1 for Some). If the tag is `1`,
  /// writes the non-null [discriminant] value as a `u32`.
  /// This corresponds to writing the discriminant part of an `Option<Enum>`.
  ///
  /// #### Rust Type: Part of `Option<Enum>` serialization
  /// #### Rust Context Example:
  /// ```rust
  /// enum MaybeStatus { None, Some(Status) }
  /// ```
  /// **Layout:** `[0]` or `[1][u32 discriminant]`
  @override
  void writeOptionEnumDiscriminant(int? discriminant) {
    writeU8(discriminant != null ? 1 : 0);
    if (discriminant != null) {
      writeEnumDiscriminant(discriminant);
    }
  }

  /// Writes a generic list of [values] as a Bincode sequence.
  ///
  /// Writes a U64 length prefix (number of elements) followed by each element
  /// serialized using the provided [writeElement] callback function.
  /// This is the generic method for lists of any type `T`. For lists of primitive
  /// types, prefer the specialized methods like [writeInt32List] for better performance.
  /// Corresponds to serializing `Vec<T>` in Rust where `T` can be any serializable type.
  ///
  /// **Performance Warning:** This method can be less performant for large lists compared
  /// to specialized methods (e.g., [writeInt32List], [writeFloat64List]) for primitive types,
  /// or using `writeBytes` directly for `Uint8List`, due to Dart loop overhead and
  /// function call indirection for each element.
  ///
  /// #### Rust Context Example:
  ///  Used when serializing `Vec<T>` where T is a custom struct, enum, or complex type.
  /// ```rust
  /// #[derive(Serialize)]
  /// struct Scene {
  ///     // Vec of custom structs, each serialized via callback in Dart
  ///     objects: Vec<SceneObject>, // Use for this field
  /// }
  ///
  /// #[derive(Serialize)]
  /// struct SceneObject { /* id, position, etc. */ }
  /// ```
  /// **Layout:** `[u64 length][element 1 bytes][element 2 bytes]...`
  @override
  void writeList<T>(List<T> values, void Function(T value) writeElement) {
    writeU64(values.length);
    for (final value in values) {
      writeElement(value);
    }
  }

  /// Writes a generic map of key-value pairs [values] as a Bincode sequence.
  ///
  /// Writes a U64 length prefix (number of key-value entries) followed by each entry.
  /// Each entry consists of the key serialized using the [writeKey] callback,
  /// immediately followed by the value serialized using the [writeValue] callback.
  /// This is the generic method for maps with any serializable key/value types `K` and `V`.
  /// Corresponds to serializing map types like `HashMap<K, V>` or `BTreeMap<K, V>` in Rust.
  ///
  /// **Performance Warning:** Similar to [writeList], this can be less performant for large
  /// maps due to iteration and callback overhead for each key and value.
  ///
  /// #### Rust Context Example:
  /// ```rust
  /// use std::collections::HashMap;
  ///
  /// #[derive(Serialize)]
  /// struct GameState {
  ///     // Map from player ID (String) to PlayerData (custom struct)
  ///     players: HashMap<String, PlayerData>, // Use for this field
  /// }
  ///
  /// #[derive(Serialize)]
  /// struct PlayerData { /* score, position, inventory, etc. */ }
  /// ```
  /// **Layout:** `[u64 num_entries][key 1 bytes][value 1 bytes][key 2 bytes][value 2 bytes]...`
  @override
  void writeMap<K, V>(Map<K, V> values, void Function(K key) writeKey,
      void Function(V value) writeValue) {
    writeU64(values.length);
    for (final entry in values.entries) {
      writeKey(entry.key);
      writeValue(entry.value);
    }
  }

  /// Writes a fixed-size array of elements without a length prefix.
  ///
  /// Writes exactly [expectedLength] items sequentially using the [writeElement] callback.
  /// Asserts that `items.length == expectedLength` before writing.
  /// This corresponds to serializing Rust's fixed-size array type `[T; N]`.
  ///
  /// **Performance Warning:** Performance depends on the complexity of [writeElement].
  /// For primitive types, writing a `TypedData` list using `writeBytes` might be faster
  /// if the data is already in that format, but that requires manual length handling.
  ///
  /// #### Rust Type: `[T; N]`
  /// #### Rust Context Example:
  /// ```rust
  /// #[derive(Serialize)]
  /// struct Header { checksum: [u16; 4] } // N = 4, T = u16
  /// ```
  /// **Layout:** `[element 1 bytes][element 2 bytes]...[element N bytes]` (no length prefix)
  @override
  void writeFixedArray<T>(
      List<T> items, int expectedLength, void Function(T value) writeElement) {
    if (items.length != expectedLength) {
      throw ArgumentError(
          'writeFixedArray requires exactly $expectedLength items, but list has ${items.length} items.');
    }
    // No length prefix
    for (final item in items) {
      writeElement(item);
    }
  }

  /// Writes a `Set<T>` as a Bincode sequence (length + elements).
  ///
  /// Writes a U64 length prefix (number of elements) followed by each element
  /// serialized using the provided [writeElement] callback function. The iteration
  /// order depends on the specific `Set` implementation (usually `LinkedHashSet` in Dart).
  /// Corresponds to serializing `HashSet<T>` or `BTreeSet<T>` in Rust (which are
  /// typically serialized identically to `Vec<T>`).
  ///
  /// **Performance Warning:** Similar to [writeList], performance depends on the
  /// number of elements and the complexity of the [writeElement] callback.
  ///
  /// #### Rust Type: `HashSet<T>`, `BTreeSet<T>` (Serialized like `Vec<T>`)
  /// #### Rust Context Example:
  /// ```rust
  /// use std::collections::HashSet;
  /// #[derive(Serialize)]
  /// struct UniqueIds { ids: HashSet<u64> }
  /// ```
  /// **Layout:** `[u64 length][element 1 bytes][element 2 bytes]...`
  @override
  void writeSet<T>(Set<T> items, void Function(T value) writeElement) {
    writeU64(items.length);
    items.forEach(writeElement);
  }

// --- Specialized List Writers ---

  /// Writes a u64 length prefix followed by the list of signed 8‑bit integers [values].
  /// Optimizes for `Int8List` by writing its raw bytes.
  /// Writes a Bincode sequence representation of the list of signed 8-bit integers [values].
  ///
  /// Writes a U64 length prefix followed by the elements. It optimizes performance
  /// by checking if [values] is an [Int8List] and writing its raw byte
  /// representation directly using [writeBytes] if possible. Otherwise, it iterates
  /// and writes each element individually using [writeI8].
  /// Corresponds to serializing `Vec<i8>` in Rust.
  /// The internal write calls respect the `unchecked` flag.
  ///
  /// #### Rust Context Example:
  /// ```rust
  /// #[derive(Serialize)]
  /// struct AudioSample {
  ///     pcm_data_i8: Vec<i8>, // Use for this field
  /// }
  /// ```
  /// **Layout:** `[u64 length][i8 byte][i8 byte]...`
  @override
  void writeInt8List(List<int> values) {
    writeU64(values.length);
    if (values is Int8List) {
      writeBytes(values.buffer
          .asUint8List(values.offsetInBytes, values.lengthInBytes));
    } else {
      values.forEach(writeI8);
    }
  }

  /// Writes a Bincode sequence representation of the list of signed 16-bit integers [values].
  ///
  /// Writes a U64 length prefix followed by the elements. It optimizes performance
  /// by checking if [values] is an [Int16List] and writing its raw byte
  /// representation directly using [writeBytes] if possible. Otherwise, it iterates
  /// and writes each element individually using [writeI16].
  /// Corresponds to serializing `Vec<i16>` in Rust.
  /// The internal write calls respect the `unchecked` flag.
  ///
  /// #### Rust Context Example:
  /// ```rust
  /// #[derive(Serialize)]
  /// struct PointCloud {
  ///     coords_i16: Vec<i16>, // Use for this field (e.g., x1, y1, z1, x2, y2, z2...)
  /// }
  /// ```
  /// **Layout:** `[u64 length][i16 bytes][i16 bytes]...` (Little Endian elements)
  @override
  void writeInt16List(List<int> values) {
    writeU64(values.length);
    if (values is Int16List) {
      writeBytes(values.buffer
          .asUint8List(values.offsetInBytes, values.lengthInBytes));
    } else {
      values.forEach(writeI16);
    }
  }

  /// Writes a Bincode sequence representation of the list of signed 32-bit integers [values].
  ///
  /// Writes a U64 length prefix followed by the elements. It optimizes performance
  /// by checking if [values] is an [Int32List] and writing its raw byte
  /// representation directly using [writeBytes] if possible. Otherwise, it iterates
  /// and writes each element individually using [writeI32].
  /// Corresponds to serializing `Vec<i32>` in Rust.
  /// The internal write calls respect the `unchecked` flag.
  ///
  /// #### Rust Context Example:
  /// ```rust
  /// #[derive(Serialize)]
  /// struct IdList {
  ///     user_ids: Vec<i32>, // Use for this field
  /// }
  /// ```
  /// **Layout:** `[u64 length][i32 bytes][i32 bytes]...` (Little Endian elements)
  @override
  void writeInt32List(List<int> values) {
    writeU64(values.length);
    if (values is Int32List) {
      writeBytes(values.buffer
          .asUint8List(values.offsetInBytes, values.lengthInBytes));
    } else {
      values.forEach(writeI32);
    }
  }

  /// Writes a Bincode sequence representation of the list of signed 64-bit integers [values].
  ///
  /// Writes a U64 length prefix followed by the elements. It optimizes performance
  /// by checking if [values] is an [Int64List] and writing its raw byte
  /// representation directly using [writeBytes] if possible. Otherwise, it iterates
  /// and writes each element individually using [writeI64].
  /// Corresponds to serializing `Vec<i64>` in Rust.
  /// The internal write calls respect the `unchecked` flag.
  ///
  /// #### Rust Context Example:
  /// ```rust
  /// #[derive(Serialize)]
  /// struct EventLog {
  ///     timestamps_ms: Vec<i64>, // Use for this field
  /// }
  /// ```
  /// **Layout:** `[u64 length][i64 bytes][i64 bytes]...` (Little Endian elements)
  @override
  void writeInt64List(List<int> values) {
    writeU64(values.length);
    if (values is Int64List) {
      writeBytes(values.buffer
          .asUint8List(values.offsetInBytes, values.lengthInBytes));
    } else {
      values.forEach(writeI64);
    }
  }

  /// Writes a Bincode sequence representation of the list of unsigned 8-bit integers [values].
  ///
  /// Writes a U64 length prefix followed by the bytes. It optimizes performance
  /// by checking if [values] is a [Uint8List] and writing it directly using [writeBytes]
  /// if possible (most efficient). Otherwise, it iterates and writes each byte
  /// individually using [writeU8].
  /// Corresponds to serializing `Vec<u8>` in Rust.
  /// The internal write calls respect the `unchecked` flag.
  ///
  /// #### Rust Context Example:
  /// ```rust
  /// #[derive(Serialize)]
  /// struct RawData {
  ///     buffer: Vec<u8>, // Use for this field
  /// }
  /// ```
  /// **Layout:** `[u64 length][u8 byte][u8 byte]...`
  @override
  void writeUint8List(List<int> values) {
    writeU64(values.length);
    if (values is Uint8List) {
      // If it's already Uint8List, write its bytes directly (most efficient)
      writeBytes(values);
    } else {
      // Otherwise, iterate and write each element
      values.forEach(writeU8);
    }
  }

  /// Writes a Bincode sequence representation of the list of unsigned 16-bit integers [values].
  ///
  /// Writes a U64 length prefix followed by the elements. It optimizes performance
  /// by checking if [values] is a [Uint16List] and writing its raw byte
  /// representation directly using [writeBytes] if possible. Otherwise, it iterates
  /// and writes each element individually using [writeU16].
  /// Corresponds to serializing `Vec<u16>` in Rust.
  /// The internal write calls respect the `unchecked` flag.
  ///
  /// #### Rust Context Example:
  /// ```rust
  /// #[derive(Serialize)]
  /// struct CharCodes {
  ///     utf16_codes: Vec<u16>, // Use for this field
  /// }
  /// ```
  /// **Layout:** `[u64 length][u16 bytes][u16 bytes]...` (Little Endian elements)
  @override
  void writeUint16List(List<int> values) {
    writeU64(values.length);
    if (values is Uint16List) {
      writeBytes(values.buffer
          .asUint8List(values.offsetInBytes, values.lengthInBytes));
    } else {
      values.forEach(writeU16);
    }
  }

  /// Writes a Bincode sequence representation of the list of unsigned 32-bit integers [values].
  ///
  /// Writes a U64 length prefix followed by the elements. It optimizes performance
  /// by checking if [values] is a [Uint32List] and writing its raw byte
  /// representation directly using [writeBytes] if possible. Otherwise, it iterates
  /// and writes each element individually using [writeU32].
  /// Corresponds to serializing `Vec<u32>` in Rust.
  /// The internal write calls respect the `unchecked` flag.
  ///
  /// #### Rust Context Example:
  /// ```rust
  /// #[derive(Serialize)]
  /// struct ColorPalette {
  ///     rgba_colors: Vec<u32>, // Use for this field
  /// }
  /// ```
  /// **Layout:** `[u64 length][u32 bytes][u32 bytes]...` (Little Endian elements)
  @override
  void writeUint32List(List<int> values) {
    writeU64(values.length);
    if (values is Uint32List) {
      writeBytes(values.buffer
          .asUint8List(values.offsetInBytes, values.lengthInBytes));
    } else {
      values.forEach(writeU32);
    }
  }

  /// Writes a Bincode sequence representation of the list of unsigned 64-bit integers [values].
  ///
  /// Writes a U64 length prefix followed by the elements. It optimizes performance
  /// by checking if [values] is a [Uint64List] and writing its raw byte
  /// representation directly using [writeBytes] if possible. Otherwise, it iterates
  /// and writes each element individually using [writeU64].
  /// Corresponds to serializing `Vec<u64>` in Rust.
  /// The internal write calls respect the `unchecked` flag.
  ///
  /// #### Rust Context Example:
  /// ```rust
  /// #[derive(Serialize)]
  /// struct IdentifierList {
  ///     unique_ids: Vec<u64>, // Use for this field
  /// }
  /// ```
  /// **Layout:** `[u64 length][u64 bytes][u64 bytes]...` (Little Endian elements)
  @override
  void writeUint64List(List<int> values) {
    writeU64(values.length);
    if (values is Uint64List) {
      writeBytes(values.buffer
          .asUint8List(values.offsetInBytes, values.lengthInBytes));
    } else {
      values.forEach(writeU64);
    }
  }

  /// Writes a Bincode sequence representation of the list of 32-bit floats [values].
  ///
  /// Writes a U64 length prefix followed by the elements. It optimizes performance
  /// by checking if [values] is a [Float32List] and writing its raw byte
  /// representation directly using [writeBytes] if possible. Otherwise, it iterates
  /// and writes each element individually using [writeF32].
  /// Corresponds to serializing `Vec<f32>` in Rust.
  /// The internal write calls respect the `unchecked` flag.
  ///
  /// #### Rust Context Example:
  /// ```rust
  /// #[derive(Serialize)]
  /// struct SignalData {
  ///     samples_f32: Vec<f32>, // Use for this field
  /// }
  /// ```
  /// **Layout:** `[u64 length][f32 bytes][f32 bytes]...` (IEEE 754, Little Endian elements)
  @override
  void writeFloat32List(List<double> values) {
    writeU64(values.length);
    if (values is Float32List) {
      writeBytes(values.buffer
          .asUint8List(values.offsetInBytes, values.lengthInBytes));
    } else {
      values.forEach(writeF32);
    }
  }

  /// Writes a Bincode sequence representation of the list of 64-bit floats [values].
  ///
  /// Writes a U64 length prefix followed by the elements. It optimizes performance
  /// by checking if [values] is a [Float64List] and writing its raw byte
  /// representation directly using [writeBytes] if possible. Otherwise, it iterates
  /// and writes each element individually using [writeF64].
  /// Corresponds to serializing `Vec<f64>` in Rust.
  /// The internal write calls respect the `unchecked` flag.
  ///
  /// #### Rust Context Example:
  /// ```rust
  /// #[derive(Serialize)]
  /// struct GraphPoints {
  ///     y_values: Vec<f64>, // Use for this field
  /// }
  /// ```
  /// **Layout:** `[u64 length][f64 bytes][f64 bytes]...` (IEEE 754, Little Endian elements)
  @override
  void writeFloat64List(List<double> values) {
    writeU64(values.length);
    if (values is Float64List) {
      writeBytes(values.buffer
          .asUint8List(values.offsetInBytes, values.lengthInBytes));
    } else {
      values.forEach(writeF64);
    }
  }

  // --- Nested Object Writing Methods ---

  /// Writes a nested value as a length-prefixed inline payload.
  ///
  /// This mimics how Bincode encodes nested dynamically-sized types in Rust:
  /// it writes a `u64` prefix (denoting byte count), followed by the actual serialized content.
  ///
  /// Instead of allocating a temporary buffer, it reserves 8 bytes first,
  /// serializes the object directly into the same buffer, and patches
  /// the length afterward.
  ///
  /// #### Rust Context Example:
  /// ```rust
  /// #[derive(Serialize)]
  /// struct Wrapper {
  ///     child: NestedType,
  /// }
  /// ```
  /// In Rust, `child` is serialized as `[len: u64][NestedType bytes...]` inline.
  ///
  /// **Layout:** `[len: u64][encoded nested bytes...]`
  @override
  void writeNestedValueForCollection(BincodeCodable value) {
    final lenPos = _pos;
    _ensureCapacity(8);
    _pos += 8;
    final startPayloadPos = _pos;
    value.encode(this);
    final endPayloadPos = _pos;
    final nestedLen = endPayloadPos - startPayloadPos;
    _dataView!.setUint64(lenPos, nestedLen, Endian.little);
  }

  /// Writes a nested value directly into the buffer with **no length prefix**.
  ///
  /// This method serializes the value inline as raw bytes, using its fixed layout.
  /// It does **not** include a `u64` length, tag, or any metadata. The value must
  /// have a consistent, fixed-size binary representation.
  ///
  /// This is used for structs where the layout is known at compile time,
  /// and size can be determined without a prefix.
  ///
  /// #### Rust Bincode Context:
  /// ```rust
  /// #[derive(Serialize)]
  /// struct Vec3 {
  ///     x: f32,
  ///     y: f32,
  ///     z: f32,
  /// }
  /// ```
  ///
  /// **Layout:** Raw struct bytes only. Does **not** include length or option tag.
  @override
  void writeNestedValueForFixed(BincodeEncodable value) {
    value.encode(this);
  }

  /// Writes an `Option<T>` where `T` is a dynamically-sized nested type.
  ///
  /// Encodes as:
  /// - `0u8` if `value == null`
  /// - `1u8` followed by `[len: u64][nested bytes...]` if present
  ///
  /// #### Rust Context Example:
  /// ```rust
  /// #[derive(Serialize)]
  /// struct Wrapper {
  ///     child: Option<NestedType>,
  /// }
  /// ```
  /// This is equivalent to Bincode's encoding of `Option<T>` where `T: Encode`.
  ///
  /// **Layout:** `[0]` if null, or `[1][len: u64][encoded value]`
  @override
  void writeOptionNestedValueForCollection(BincodeCodable? value) {
    writeU8(value != null ? 1 : 0);
    if (value != null) {
      writeNestedValueForCollection(value);
    }
  }

  /// Writes an `Option<T>` where `T` is a fixed-size nested type.
  ///
  /// Encodes as:
  /// - `0u8` if `value == null`
  /// - `1u8` followed by the raw fixed-size bytes
  ///
  /// #### Rust Context Example:
  /// ```rust
  /// #[derive(Serialize)]
  /// struct Wrapper {
  ///     config: Option<FixedStruct>,
  /// }
  /// ```
  /// This follows Bincode’s layout for optional fixed-size nested types.
  ///
  /// **Layout:** `[0]` if null, or `[1][encoded struct bytes]`
  @override
  void writeOptionNestedValueForFixed(BincodeEncodable? value) {
    writeU8(value != null ? 1 : 0);
    if (value != null) {
      writeNestedValueForFixed(value);
    }
  }

  /// Writes a fixed-size object using `value.encode()`, validating it consumed exactly [knownSize] bytes.
  ///
  /// Bypasses length prefixing, similar to [writeNestedValueForFixed].
  /// Use when the encoded size is known and needs verification during write.
  /// Throws [BincodeException] if the actual bytes written by `value.encode()` do not match [knownSize].
  /// Caller MUST ensure [knownSize] is correct. Requires `T` to implement [BincodeEncodable].
  ///
  /// Example: `writer.writeNestedObjectWithKnownSize(myStruct, MyStruct.bincodeSize);`
  @override
  void writeNestedObjectWithKnownSize(BincodeEncodable value, int knownSize) {
    if (knownSize < 0) {
      throw BincodeException("Known size cannot be negative: $knownSize");
    }

    final startPos = _pos;
    value.encode(this);
    final endPos = _pos;
    final bytesWritten = endPos - startPos;

    if (bytesWritten != knownSize) {
      throw BincodeException(
          'Encode for ${value.runtimeType} with knownSize=$knownSize wrote incorrect bytes. Actual: $bytesWritten.');
    }
  }

  /// Writes an optional fixed-size object using `value.encode()`, validating size if present.
  ///
  /// Writes a `u8` tag (0 for None, 1 for Some). If 1 (Some), calls
  /// [writeNestedObjectWithKnownSize] to write the object and validate its size.
  /// Caller MUST ensure [knownSize] is correct for the type of [value].
  /// Requires `T` to implement [BincodeEncodable].
  ///
  /// Example: `writer.writeOptionNestedObjectWithKnownSize(optionalStruct, MyStruct.bincodeSize);`
  @override
  void writeOptionNestedObjectWithKnownSize(
      BincodeEncodable? value, int knownSize) {
    writeU8(value != null ? 1 : 0);
    if (value != null) {
      writeNestedObjectWithKnownSize(value, knownSize);
    }
  }

  /// Writes a [Duration] value using a defined format compatible with Rust's `chrono::Duration`.
  ///
  /// Serializes as two components:
  /// 1. Seconds as a little-endian signed 64-bit integer (`i64`).
  /// 2. Nanoseconds within the second as a little-endian unsigned 32-bit integer (`u32`),
  ///    always positive and less than 1,000,000,000.
  /// This handles positive and negative durations correctly.
  ///
  /// #### Rust Type: `chrono::Duration` (via serde)
  /// #### Rust Context Example:
  /// ```rust
  /// #[derive(Serialize)]
  /// struct Timer { elapsed: chrono::Duration }
  /// ```
  /// **Layout:** `[i64 seconds][u32 positive_nanos]` (little-endian)
  @override
  void writeDuration(Duration value) {
    int totalNanos = value.inMicroseconds * 1000;
    const nanosPerSecond = 1000000000;
    int seconds = totalNanos ~/ nanosPerSecond;
    int nanos = totalNanos.remainder(nanosPerSecond);

    if (nanos < 0) {
      nanos += nanosPerSecond;
      seconds -= 1;
    }

    writeI64(seconds);
    writeU32(nanos);
  }
}
