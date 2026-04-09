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

/// A Dart implementation of the Bincode binary serialization format, designed
/// for efficient, compact, and optionally schema-less data exchange.
///
/// This library provides tools for encoding Dart objects into Bincode byte
/// sequences and decoding those bytes back into Dart objects. It aims for
/// high performance and compatibility with the Bincode specification found
/// across other languages (like Rust's `bincode` crate).
///
/// ## Core Components & Utilities:
/// - [BincodeWriter]: Serializes Dart types into Bincode bytes. Offers methods
///   for various primitive types, strings, collections, optionals, and nested objects.
/// - [BincodeReader]: Deserializes Bincode bytes back into Dart types, providing
///   corresponding read methods for all supported types.
/// - [BincodeCodable], [BincodeEncodable], [BincodeDecodable]: Interfaces for
///   making custom classes serializable and deserializable.
/// - [BincodeWriterPool]: Optimizes performance in high-frequency serialization
///   scenarios by managing reusable [BincodeWriter] instances.
/// - [BitMask]: A utility class for easily managing up to 8 boolean flags packed
///   into a single byte (`u8`), useful for status fields or options.
///
/// ## Basic Usage
///
/// To serialize custom objects, implement the [BincodeCodable] interface
/// on your class, defining the `encode` and `decode` methods. Then use
/// [BincodeWriter] to serialize instances and [BincodeReader] to deserialize
/// the resulting bytes.
///
/// ```dart
/// import 'package:d_bincode/d_bincode.dart';
///
/// // 1. Implement BincodeCodable
/// class Point implements BincodeCodable {
///   double x, y;
///
///   Point(this.x, this.y);
///   Point.empty() : x = 0, y = 0; // Needed for common decoding patterns
///
///   @override
///   void encode(BincodeWriter w) => w..writeF64(x)..writeF64(y);
///
///   @override
///   void decode(BincodeReader r) {
///     x = r.readF64();
///     y = r.readF64();
///   }
///
///   @override String toString() => 'Point($x, $y)';
/// }
///
/// void main() {
///   // 2. Encode
///   final writer = BincodeWriter();
///   final originalPoint = Point(1.0, -2.5);
///   originalPoint.encode(writer);
///   final bytes = writer.toBytes();
///   print('Encoded: $bytes');
///
///   // 3. Decode
///   final decodedPoint = Point.empty(); // Create instance
///   decodedPoint.decode(BincodeReader(bytes)); // Populate from bytes
///   print('Decoded: $decodedPoint'); // Output: Point(1.0, -2.5)
/// }
/// ```
///
/// See individual classes ([BincodeWriter], [BincodeReader], [BitMask], etc.)
/// and interface documentation ([BincodeCodable]) for detailed information on
/// supported types, configuration options, and advanced usage patterns.
library;

export 'src/bitmask.dart';
export 'src/codable.dart';
export 'src/pool.dart';
export 'src/reader.dart';
export 'src/writer.dart';
