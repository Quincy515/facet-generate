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

import 'dart:typed_data';

import '../d_bincode.dart';

/// Abstract interface for writing bincode‐formatted data.
abstract class BincodeWriterBuilder {
  /// Gets the current write cursor position in bytes.
  int get position;

  /// Sets the write cursor to the given [value] (in bytes).
  ///
  /// Throws a [RangeError] if the [value] is outside the valid range.
  set position(int value);

  // -- Cursor / Positioning Methods --

  /// Moves the cursor by an [offset] relative to its current position.
  ///
  /// If [offset] is negative, the cursor moves backward.
  void seek(int offset);

  /// Moves the cursor to the specified absolute position [absolutePosition].
  void seekTo(int absolutePosition);

  /// Resets the cursor to the beginning (position 0).
  void rewind();

  /// Moves the cursor to the end of the written data.
  void skipToEnd();

  // -- Base Write Methods --

  /// Writes an unsigned 8‑bit integer [value] to the output.
  void writeU8(int value);

  /// Writes an unsigned 16‑bit integer [value] to the output.
  void writeU16(int value);

  /// Writes an unsigned 32‑bit integer [value] to the output.
  void writeU32(int value);

  /// Writes an unsigned 64‑bit integer [value] to the output.
  void writeU64(int value);

  /// Writes a signed 8‑bit integer [value] to the output.
  void writeI8(int value);

  /// Writes a signed 16‑bit integer [value] to the output.
  void writeI16(int value);

  /// Writes a signed 32‑bit integer [value] to the output.
  void writeI32(int value);

  /// Writes a signed 64‑bit integer [value] to the output.
  void writeI64(int value);

  /// Writes a 32‑bit floating‑point [value] to the output.
  void writeF32(double value);

  /// Writes a 64‑bit floating‑point [value] to the output.
  void writeF64(double value);

  /// Writes a boolean [value] to the output.
  ///
  /// Booleans are represented as 0 (false) or 1 (true).
  void writeBool(bool value);

  // -- String & Bytes Write Methods --

  /// Writes a list of bytes [bytes] directly to the output (no length prefix).
  void writeBytes(List<int> bytes);

  /// Writes a length‑prefixed string [s] using utf8 encoding.
  ///
  /// The method first writes the length of the encoded string as an unsigned 64‑bit integer,
  /// followed by the encoded bytes.
  void writeString(String s);

  /// Writes a fixed‑length string [value], padding with zeros if necessary.
  ///
  /// If [value] is shorter than [length] bytes (in UTF-8), the output is padded with zeros.
  /// If it is longer, it is truncated.
  void writeFixedString(String value, int length);

  /// Writes a single Dart character (a String of length 1) as a Rust `char`.
  ///
  /// Bincode v1 encoding: writes the character's Unicode code point (rune) as a `u32`.
  /// Throws if [char] is not a single character.
  void writeChar(String char);

  // -- Optional Write Methods --

  /// Writes an optional boolean [value].
  ///
  /// Writes a tag (1 for Some, 0 for None) followed by the boolean (if present).
  void writeOptionBool(bool? value);

  /// Writes an optional unsigned 8‑bit integer [value].
  void writeOptionU8(int? value);

  /// Writes an optional unsigned 16‑bit integer [value].
  void writeOptionU16(int? value);

  /// Writes an optional unsigned 32‑bit integer [value].
  void writeOptionU32(int? value);

  /// Writes an optional unsigned 64‑bit integer [value].
  void writeOptionU64(int? value);

  /// Writes an optional signed 8‑bit integer [value].
  void writeOptionI8(int? value);

  /// Writes an optional signed 16‑bit integer [value].
  void writeOptionI16(int? value);

  /// Writes an optional signed 32‑bit integer [value].
  void writeOptionI32(int? value);

  /// Writes an optional signed 64‑bit integer [value].
  void writeOptionI64(int? value);

  /// Writes an optional 32‑bit floating‑point [value].
  void writeOptionF32(double? value);

  /// Writes an optional 64‑bit floating‑point [value].
  void writeOptionF64(double? value);

  /// Writes an optional string [value].
  ///
  /// Writes a tag (1 for Some, 0 for None) followed by the length-prefixed string (if present).
  void writeOptionString(String? value);

  /// Writes an optional fixed‑length string [value] of given [length].
  /// Writes a tag (1 for Some, 0 for None) followed by the fixed-length string (if present).
  void writeOptionFixedString(String? value, int length);

  /// Writes an optional single character [char] as `Option<char>`.
  /// Writes a tag (1 for Some, 0 for None) followed by the char (as u32) if present.
  /// Throws if [char] is non-null and not a single character.
  void writeOptionChar(String? char);

  // -- Collection Write Methods --

  /// Writes a list [values] by first writing its length (as a u64) followed by each element.
  ///
  /// The [writeElement] callback is used to serialize each individual element.
  /// Use this for `Vec<T>`.
  void writeList<T>(List<T> values, void Function(T value) writeElement);

  /// Writes a fixed-size array [items] by writing exactly [expectedLength] elements consecutively.
  /// **No length prefix is written.** Asserts that `items.length == expectedLength`.
  ///
  /// The [writeElement] callback is used to serialize each individual element.
  /// Use this for `[T; N]`.
  void writeFixedArray<T>(
      List<T> items, int expectedLength, void Function(T value) writeElement);

  /// Writes a map [values] by first writing its length (as a u64) followed by each key/value pair.
  ///
  /// The [writeKey] and [writeValue] callbacks are used to serialize the keys and values.
  /// Use this for `HashMap<K, V>` or `BTreeMap<K, V>`.
  void writeMap<K, V>(Map<K, V> values, void Function(K key) writeKey,
      void Function(V value) writeValue);

  /// Writes a set [items] by first writing its length (as a u64) followed by each element.
  ///
  /// The [writeElement] callback is used to serialize each individual element.
  /// Order is determined by the set's iteration order.
  /// Use this for `HashSet<T>` or `BTreeSet<T>`.
  void writeSet<T>(Set<T> items, void Function(T value) writeElement);

  // -- Primitive List Helper Methods --

  /// Writes a list of signed 8‑bit integers [values]. `Vec<i8>`
  void writeInt8List(List<int> values);

  /// Writes a list of signed 16‑bit integers [values]. `Vec<i16>`
  void writeInt16List(List<int> values);

  /// Writes a list of signed 32‑bit integers [values]. `Vec<i32>`
  void writeInt32List(List<int> values);

  /// Writes a list of signed 64‑bit integers [values]. `Vec<i64>`
  void writeInt64List(List<int> values);

  /// Writes a list of unsigned 8‑bit integers [values]. `Vec<u8>`
  void writeUint8List(List<int> values);

  /// Writes a list of unsigned 16‑bit integers [values]. `Vec<u16>`
  void writeUint16List(List<int> values);

  /// Writes a list of unsigned 32‑bit integers [values]. `Vec<u32>`
  void writeUint32List(List<int> values);

  /// Writes a list of unsigned 64‑bit integers [values]. `Vec<u64>`
  void writeUint64List(List<int> values);

  /// Writes a list of 32‑bit floating‑point values [values]. `Vec<f32>`
  void writeFloat32List(List<double> values);

  /// Writes a list of 64‑bit floating‑point values [values]. `Vec<f64>`
  void writeFloat64List(List<double> values);

  // -- Nested Object & Enum Write Methods --

  /// Writes a nested [value] assuming it has a variable size encoding.
  /// Prepends the encoded byte length of the value as a u64.
  /// Requires [value] to implement `BincodeCodable`.
  void writeNestedValueForCollection(BincodeCodable value);

  /// Writes a nested [value] assuming it has a fixed size encoding.
  /// Writes the encoded bytes directly without a length prefix.
  /// Requires [value] to implement `BincodeEncodable`.
  void writeNestedValueForFixed(BincodeEncodable value);

  /// Writes an optional nested [value] assuming variable size encoding (`Option<T>`).
  /// Writes tag + (u64 length + value bytes) if present.
  void writeOptionNestedValueForCollection(BincodeCodable? value);

  /// Writes an optional nested [value] assuming fixed size encoding (`Option<T>`).
  /// Writes tag + (value bytes) if present.
  void writeOptionNestedValueForFixed(BincodeEncodable? value);

  /// Writes a fixed-size object using `value.encode()`, validating it consumed exactly [knownSize] bytes.
  ///
  /// Bypasses length prefixing, similar to [writeNestedValueForFixed].
  /// Use when the encoded size is known and needs verification during write.
  /// Throws [BincodeException] if the actual bytes written by `value.encode()` do not match [knownSize].
  /// Caller MUST ensure [knownSize] is correct. Requires `T` to implement [BincodeEncodable].
  ///
  /// Example: `writer.writeNestedObjectWithKnownSize(myStruct, MyStruct.bincodeSize);`
  void writeNestedObjectWithKnownSize(BincodeEncodable value, int knownSize);

  /// Writes an optional fixed-size object using `value.encode()`, validating size if present.
  ///
  /// Writes a `u8` tag (0 for None, 1 for Some). If 1 (Some), calls
  /// [writeNestedObjectWithKnownSize] to write the object and validate its size.
  /// Caller MUST ensure [knownSize] is correct for the type of [value].
  /// Requires `T` to implement [BincodeEncodable].
  ///
  /// Example: `writer.writeOptionNestedObjectWithKnownSize(optionalStruct, MyStruct.bincodeSize);`
  void writeOptionNestedObjectWithKnownSize(
      BincodeEncodable? value, int knownSize);

  /// Writes an enum discriminant (variant index) as a `u32`.
  /// The caller is responsible for subsequently writing any payload associated with the variant.
  void writeEnumDiscriminant(int discriminant);

  /// Writes a [Duration] value using a defined format.
  /// Format: seconds (i64) + nanoseconds within second (u32).
  void writeDuration(Duration value);

  /// Writes an optional [Duration] value using the defined format.
  /// Format: tag (u8: 0=None, 1=Some) + (i64 seconds + u32 nanoseconds if Some).
  void writeOptionDuration(Duration? value);

  /// Writes an optional enum discriminant.
  /// Writes a tag (u8: 0=None, 1=Some) followed by the u32 discriminant if Some.
  void writeOptionEnumDiscriminant(int? discriminant);

  // -- Output Methods --

  /// Returns the written portion of the buffer as a [Uint8List].
  Uint8List toBytes();
}

/// Abstract interface for reading bincode‐formatted data.
///
/// Implementations of this interface are responsible for reading primitive
/// types, collections, and nested decodable objects from a binary buffer.
/// The reader supports operations such as seeking, rewinding, and accessing the underlying bytes.
abstract class BincodeReaderBuilder {
  /// Gets the current read cursor position (in bytes).
  int get position;

  /// Sets the read cursor position to [value].
  ///
  /// Throws a [RangeError] if [value] is outside the valid buffer range.
  set position(int value);

  /// Gets the number of bytes remaining to be read from the current position.
  int get remainingBytes;

  // -- Cursor / Positioning Methods --

  /// Moves the cursor by an [offset] relative to its current position.
  ///
  /// If [offset] is negative, the cursor moves backward.
  void seek(int offset);

  /// Moves the cursor to the given absolute position [absolutePosition].
  void seekTo(int absolutePosition);

  /// Rewinds the cursor to the beginning (position 0).
  void rewind();

  /// Moves the cursor to the end of the readable portion of the buffer.
  void skipToEnd();

  // -- Base Read Methods --

  /// Reads an unsigned 8‑bit integer from the current position.
  ///
  /// Advances the cursor by 1 byte.
  int readU8();

  /// Reads an unsigned 16‑bit integer from the current position.
  ///
  /// Advances the cursor by 2 bytes.
  int readU16();

  /// Reads an unsigned 32‑bit integer from the current position.
  ///
  /// Advances the cursor by 4 bytes.
  int readU32();

  /// Reads an unsigned 64‑bit integer from the current position.
  ///
  /// Advances the cursor by 8 bytes.
  int readU64();

  /// Reads a signed 8‑bit integer from the current position.
  int readI8();

  /// Reads a signed 16‑bit integer from the current position.
  int readI16();

  /// Reads a signed 32‑bit integer from the current position.
  int readI32();

  /// Reads a signed 64‑bit integer from the current position.
  int readI64();

  /// Reads a 32‑bit floating‑point number from the current position.
  double readF32();

  /// Reads a 64‑bit floating‑point number from the current position.
  double readF64();

  /// Reads a boolean value stored as a single byte (0 for false, non-zero usually 1 for true).
  bool readBool();

  // -- String & Bytes Read Methods --

  /// Reads exactly [count] bytes from the current position and returns them as a `List<int>`.
  /// Advances the cursor by [count] bytes.
  List<int> readBytes(int count);

  /// Reads exactly [count] bytes from the current position and returns them as a `Uint8List`.
  /// Advances the cursor by [count] bytes. This is often more efficient than `readBytes`.
  Uint8List readRawBytes(int count);

  /// Reads a length‑prefixed string (length as u64) using utf8 decoding.
  String readString();

  /// Reads a fixed‑length string of [length] bytes from the current position using utf8.
  /// Advances the cursor by [length] bytes. Result may contain trailing padding.
  String readFixedString(int length);

  /// Reads a fixed‑length string of [length] bytes and trims any trailing NUL (zero) bytes.
  /// Advances the cursor by [length] bytes.
  String readCleanFixedString(int length);

  /// Reads a Rust `char` (encoded as u32 for Bincode v1) and returns it as a single-character Dart String.
  /// Advances the cursor by 4 bytes.
  String readChar();

  // -- Optional Read Methods --

  /// Reads an optional boolean value.
  /// Returns `null` if the option tag is 0.
  bool? readOptionBool();

  /// Reads an optional unsigned 8‑bit integer.
  int? readOptionU8();

  /// Reads an optional unsigned 16‑bit integer.
  int? readOptionU16();

  /// Reads an optional unsigned 32‑bit integer.
  int? readOptionU32();

  /// Reads an optional unsigned 64‑bit integer.
  int? readOptionU64();

  /// Reads an optional signed 8‑bit integer.
  int? readOptionI8();

  /// Reads an optional signed 16‑bit integer.
  int? readOptionI16();

  /// Reads an optional signed 32‑bit integer.
  int? readOptionI32();

  /// Reads an optional signed 64‑bit integer.
  int? readOptionI64();

  /// Reads an optional 32‑bit floating‑point number.
  double? readOptionF32();

  /// Reads an optional 64‑bit floating‑point number.
  double? readOptionF64();

  /// Reads an optional length-prefixed string.
  /// Returns `null` if the option tag indicates the value is not present.
  String? readOptionString();

  /// Reads an optional fixed‑length string of [length] bytes.
  /// Returns the raw fixed‑length string or `null` if not present.
  String? readOptionFixedString(int length);

  /// Reads an optional fixed‑length string of [length] bytes, trimming trailing NUL bytes.
  /// Returns the cleaned string or `null` if not present.
  String? readCleanOptionFixedString(int length);

  /// Reads an optional Rust `char` (encoded as u32 for Bincode v1).
  /// Returns a single-character Dart String or `null` if the option tag is 0.
  String? readOptionChar();

  /// Reads an optional enum discriminant.
  /// Reads the tag (u8). If 1, reads the `u32` discriminant. If 0, returns null.
  /// Throws on invalid tag.
  int? readOptionEnumDiscriminant();

  // -- Collection Read Methods --

  /// Reads a list of elements prefixed by a u64 length.
  /// [readElement] is a callback used to read each element. Used for `Vec<T>`.
  List<T> readList<T>(T Function() readElement);

  /// Reads a fixed-size array by reading exactly [length] elements consecutively.
  /// Assumes no length prefix was written. Used for `[T; N]`.
  /// [readElement] is a callback used to read each element.
  List<T> readFixedArray<T>(int length, T Function() readElement);

  /// Reads a map prefixed by a u64 length (number of key/value pairs).
  /// [readKey] and [readValue] are callbacks used to read each pair. Used for `HashMap<K, V>`.
  Map<K, V> readMap<K, V>(K Function() readKey, V Function() readValue);

  /// Reads a set prefixed by a u64 length (number of elements).
  /// [readElement] is a callback used to read each element. Used for `HashSet<T>`.
  /// Returns a `Set<T>`.
  Set<T> readSet<T>(T Function() readElement);

  // -- Primitive List Read Helper Methods --

  /// Reads a sequence of signed 8-bit integers (`Vec<i8>`) prefixed by a u64 length.
  List<int> readInt8List();

  /// Reads a sequence of signed 16-bit integers (`Vec<i16>`) prefixed by a u64 length.
  List<int> readInt16List();

  /// Reads a sequence of signed 32-bit integers (`Vec<i32>`) prefixed by a u64 length.
  List<int> readInt32List();

  /// Reads a sequence of signed 64-bit integers (`Vec<i64>`) prefixed by a u64 length.
  List<int> readInt64List();

  /// Reads a sequence of unsigned 8-bit integers (`Vec<u8>`) prefixed by a u64 length.
  List<int>
      readUint8List(); // Often returns Uint8List in concrete implementation

  /// Reads a sequence of unsigned 16-bit integers (`Vec<u16>`) prefixed by a u64 length.
  List<int> readUint16List();

  /// Reads a sequence of unsigned 32-bit integers (`Vec<u32>`) prefixed by a u64 length.
  List<int> readUint32List();

  /// Reads a sequence of unsigned 64-bit integers (`Vec<u64>`) prefixed by a u64 length.
  List<int> readUint64List();

  /// Reads a sequence of 32-bit floats (`Vec<f32>`) prefixed by a u64 length.
  List<double> readFloat32List();

  /// Reads a sequence of 64-bit floats (`Vec<f64>`) prefixed by a u64 length.
  List<double> readFloat64List();

  // -- Nested Object & Enum Reading --

  /// Reads a nested object that was written with a length prefix (variable size).
  /// Reads the u64 length, then reads that many bytes, creating a sub-reader
  /// or view, and calls `instance.decode()` with it. Modifies the passed [instance].
  /// Requires [instance] to implement `BincodeDecodable`.
  T readNestedObjectForCollection<T extends BincodeDecodable>(T instance);

  /// Reads a nested object that was written without a length prefix (fixed size).
  /// The byte size is determined by encoding a temporary default instance of T.
  /// Reads the required bytes and calls `instance.decode()` on a sub-reader/view.
  /// Modifies the passed [instance].
  /// Requires [instance] to implement `BincodeCodable` (needs both encode/decode).
  T readNestedObjectForFixed<T extends BincodeCodable>(T instance);

  /// Reads a fixed-size nested object using an explicitly provided [knownSize].
  ///
  /// Bypasses internal size calculation. Reads exactly [knownSize] bytes and calls
  /// `instance.decode()`. Validates that decode consumed exactly [knownSize].
  /// Modifies the passed [instance]. Caller MUST ensure [knownSize] is correct.
  /// Requires `T` to implement [BincodeDecodable].
  ///
  /// Example: `reader.readNestedObjectWithKnownSize(MyStruct(), MyStruct.bincodeSize);`
  T readNestedObjectWithKnownSize<T extends BincodeDecodable>(
      T instance, int knownSize);

  /// Reads an optional fixed-size nested object using an explicitly provided [knownSize].
  ///
  /// Reads a `u8` tag. If 1 (Some), creates an instance via [creator] and decodes
  /// using [readNestedObjectWithKnownSize] with the provided [knownSize].
  /// If 0 (None), returns null. Caller MUST ensure [knownSize] is correct for type `T`.
  /// Requires `T` to implement [BincodeDecodable].
  ///
  /// Example: `reader.readOptionNestedObjectWithKnownSize(() => MyStruct(), MyStruct.bincodeSize);`
  T? readOptionNestedObjectWithKnownSize<T extends BincodeDecodable>(
      T Function() creator, int knownSize);

  /// Reads an optional nested object written with a length prefix.
  /// Reads the tag (u8). If 1, creates an instance using [creator],
  /// reads the length (u64), reads the nested object into the instance using
  /// `readNestedObjectForCollection`'s logic, and returns the instance.
  /// If tag is 0, returns null.
  T? readOptionNestedObjectForCollection<T extends BincodeDecodable>(
      T Function() creator);

  /// Reads an optional nested object written without a length prefix (fixed size).
  /// Reads the tag (u8). If 1, creates an instance using [creator],
  /// reads the nested object into the instance using `readNestedObjectForFixed`'s logic,
  /// and returns the instance. If tag is 0, returns null.
  /// Requires T to implement `BincodeCodable`.
  T? readOptionNestedObjectForFixed<T extends BincodeCodable>(
      T Function() creator);

  /// Reads an enum discriminant (variant index) encoded as a `u32`.
  /// The caller is responsible for subsequently reading any payload based on the discriminant.
  int readEnumDiscriminant();

  /// Reads a [Duration] value using the defined format:
  /// seconds (i64) + nanoseconds within second (u32).
  Duration readDuration();

  /// Reads an optional [Duration] value using the defined format.
  Duration? readOptionDuration();
}
