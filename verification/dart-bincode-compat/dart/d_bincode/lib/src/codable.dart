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

import 'package:d_bincode/d_bincode.dart';

/// Represents a type that can be both serialized to and deserialized from
/// the Bincode binary format.
///
/// This convenience interface combines [BincodeEncodable] and [BincodeDecodable].
/// Implement it on custom data classes intended for full round-trip
/// serialization and deserialization.
///
/// Implementing classes **must** provide implementations for both [encode] and
/// [decode]. Furthermore, to facilitate deserialization, implementing classes
/// typically need a way to be instantiated *before* [decode] is called,
/// often via an empty constructor or a factory method. See [BincodeDecodable]
/// for more details on the instantiation requirement.
abstract class BincodeCodable implements BincodeEncodable, BincodeDecodable {}

//------------------------------------------------------------------------------

/// Defines the contract for types that can be serialized *into* the Bincode
/// binary format using a [BincodeWriter].
///
/// Classes implementing this should define how their internal state is
/// written as a sequence of bytes according to the Bincode specification rules.
abstract class BincodeEncodable {
  /// Serializes the object's current state into the Bincode format.
  ///
  /// Implementations should write all necessary fields sequentially to the
  /// provided [writer]. The order and types of these writes **must** exactly
  /// match the order and types expected by the corresponding [BincodeDecodable.decode]
  /// method for successful round-trip serialization.
  ///
  /// Example:
  /// ```dart
  /// @override
  /// void encode(BincodeWriter writer) {
  ///   // Write fields in a defined order
  ///   writer.writeU32(id);
  ///   writer.writeString(name);
  ///   writer.writeBool(isActive);
  /// }
  /// ```
  void encode(BincodeWriter writer);
}

//------------------------------------------------------------------------------

/// Defines the contract for types that can be deserialized *from* the Bincode
/// binary format using a [BincodeReader].
///
/// Implementing this allows an object's state to be populated by reading
/// previously serialized Bincode data.
///
/// **Important:** An instance of the implementing class must typically exist
/// *before* this [decode] method can be called to populate its fields.
/// This usually means the class needs:
/// 1. An accessible default/empty constructor (e.g., `MyClass.empty()`).
/// 2. Or, a factory pattern where the decoding logic instantiates the object.
///
/// The common pattern is to create an 'empty' instance and then call `decode` on it:
/// ```dart
/// final myObject = MyClass.empty();
/// myObject.decode(reader);
/// ```
abstract class BincodeDecodable {
  /// Deserializes the object's state by reading from the Bincode format.
  ///
  /// Implementations should read data sequentially from the provided [reader]
  /// to populate the object's fields. The order and types of these reads
  /// **must** exactly match the order and types written by the corresponding
  /// [BincodeEncodable.encode] method. Attempting to read data in a different
  /// order or with mismatched types will lead to exceptions or corrupted data.
  ///
  /// Example (corresponding to `BincodeEncodable.encode` example):
  /// ```dart
  /// @override
  /// void decode(BincodeReader reader) {
  ///   // Read fields in the same order they were written
  ///   id = reader.readU32();
  ///   name = reader.readString();
  ///   isActive = reader.readBool();
  /// }
  /// ```
  void decode(BincodeReader reader);
}
