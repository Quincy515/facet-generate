# d_bincode

[![Pub Version](https://img.shields.io/pub/v/d_bincode)](https://pub.dev/packages/d_bincode) [![License](https://img.shields.io/badge/license-MIT-blue.svg)](/LICENSE) 

A Dart implementation of the [Bincode](https://github.com/bincode-org/bincode) binary serialization format. Designed for efficient, compact, and cross-language data serialization, particularly suitable for Inter-Process Communication (IPC) (my last use case) and anything else where performance and size matter.

IMPORTANT:
Currently only fixed int encoding/decoding supported in 2.x of rust bincode.
Because fixed int is faster than variable encoding (In future variable encoding will be supported
via `BincodeVariableWriter` or `BincodeVariableReader` splitted to reduce conditional check overheat.)
```rust
let config = config::standard().with_fixed_int_encoding(); // <<< this is key in 2.x
```

Dart implementation of the Bincode binary format. Manual BincodeCodable implementation is currently required, providing control but demanding more setup. You can use the Rust to Dart Bincode [Converter](https://github.com/Binurie/rust_to_dart_bincode_converter) for simple Structs. It creates an dart class with BincodeCodable implemented, toString and empty constructor.

## Features

- **Core Bincode Types:** Supports encoding and decoding of:
    * Integers (u8, u16, u32, u64, i8, i16, i32, i64)
    * Floats (f32, f64)
    * Booleans (bool)
    * UTF-8 Strings (dynamic length)
    * Fixed-length Strings (with truncation/padding)
- **Collections:**
    * Lists (`List<T>`) / Vectors (`Vec<T>`)
    * Maps (`Map<K, V>`)
    * Efficient typed lists (`Uint8List`, `Int8List`, `Uint16List`, etc., `Float32List`, `Float64List`)
- **Options:** Handles nullable types (`T?`) using Bincode's `Option<T>` representation.
- **Custom Types:** Easily serialize custom classes by implementing the `BincodeCodable` interface.
- **Nested Structures:** Supports arbitrarily nested complex objects, including combinations of lists, maps, and custom types.
- **Utilities:**
    * `BincodeWriter` for encoding data with buffer management (`reserve`, `reset`).
    * `BincodeReader` for decoding data with cursor control (`seek`, `rewind`, `skip`, `align`, `peek`).
    * Helper functions for common tasks (`encodeToBytes`, `decode`, `isValidBincode`, `measure`, `toHex`, `encodeToBase64`, `stripNulls`).
    * Support for `IOSink` and file operations (`encodeToSink`, `encodeToFileSync`).
- **Alignment:** Handles data alignment considerations.
- **Robustness:** Includes checks for available bytes and validation capabilities.
* **[BitMask]**: Utility for managing 8 boolean flags packed into a single byte.
* **[BincodeWriterPool]**: Optimizes high-frequency serialization by reusing `BincodeWriter` instances.

## Installation

Add `d_bincode` to your `pubspec.yaml`:

```yaml
dependencies:
  d_bincode: ^3.2.0
````

Or install using the command line:

```bash
dart pub add d_bincode
```

## Usage

### 1\. Define Your Data Structure

Implement the `BincodeCodable` interface for any custom class you want to both serialize/deserialize (<->).

Implement the `BincodeDecodable` interface for any custom class you want to deserialize (<-).

Implement the `BincodeEncodable` interface for any custom class you want to serialize (->).

```dart
import 'package:d_bincode/d_bincode.dart';

class ExampleData implements BincodeCodable {
  int id;
  double value;
  String label;

  ExampleData(this.id, this.value, this.label);

  ExampleData.empty()
      : id = 0,
        value = 0.0,
        label = '';

  // Encode fields in order
  @override
  void encode(BincodeWriter writer) {
    writer
      ..writeU32(id)       // u32
      ..writeF32(value)    // f32
      ..writeFixedString(label, 8); //fixed 8-byte string (padded/truncated)
  }

  // Decode fields in the same order
  @override
  void decode(BincodeReader reader) {
    id = reader.readU32();
    value = reader.readF32();
    // cleans null padding
    label = reader.readCleanFixedString(8);
  }

  @override
  String toString() => 'ExampleData(id: $id, value: $value, label: "$label")';
}
```

### 2\. Encoding (Serialization)

Use `BincodeWriter` to encode your object into bytes.

```dart
final data = ExampleData(123, 1.5, 'Test');

// Method 1: Using an instance
final writer = BincodeWriter();
data.encode(writer); // Call the encode method you defined
final Uint8List encodedBytes = writer.toBytes();
print('Encoded: $encodedBytes'); // Output: e.g., [123, 0, 0, 0, 0, 0, 192, 63, 84, 101, 115, 116, 0, 0, 0, 0]

// Method 2: Using the static helper
final Uint8List encodedBytesStatic = BincodeWriter.encode(data);
print('Static Encoded: $encodedBytesStatic');

// Method 3: Using the convenience method on writer
final Uint8List encodedBytesConvenience = BincodeWriter().encodeToBytes(data);
print('Convenience Encoded: $encodedBytesConvenience');
```

### 3\. Decoding (Deserialization)

Use `BincodeReader` to decode bytes back into your object.

```dart
//  'encodedBytes' holds the bytes of a Struct like ExampleData

// Method 1: Using an instance
final reader = BincodeReader(encodedBytes);
final decodedData = ExampleData.empty(); // Create an empty instance
decodedData.decode(reader); // Call the decode method
print('Decoded: $decodedData'); // Output: ExampleData(id: 123, value: 1.5, label: "Test")

// Method 2: Using the static helper (for fixed-size or known types)
// Note: Use decodeFixed for simple, fixed-layout objects if applicable.
// Use decode for objects containing dynamic data like strings/lists.
try {
  // For simple fixed-layout data:
  // final decodedFixed = BincodeReader.decodeFixed(encodedBytes, ExampleData.empty());
  // print('Static Decoded Fixed: $decodedFixed');

  // For data containing dynamic parts (like strings, lists, maps):
  final decodedDynamic = BincodeReader.decode(encodedBytes, ExampleData.empty());
  print('Static Decoded Dynamic: $decodedDynamic');
} catch (e) {
  print('Decoding failed: $e');
}
```

### Supported Types and Methods

| Type                       | Writer Method(s)                                             | Reader Method(s)                                                       | Notes                                               |
| :------------------------- | :----------------------------------------------------------- | :--------------------------------------------------------------------- | :-------------------------------------------------- |
| `int` (u8)                 | `writeU8`                                                    | `readU8`                                                               | 8-bit unsigned integer                            |
| `int` (u16)                | `writeU16`                                                   | `readU16`                                                              | 16-bit unsigned integer (Little Endian)           |
| `int` (u32)                | `writeU32`                                                   | `readU32`                                                              | 32-bit unsigned integer (Little Endian)           |
| `int` (u64)                | `writeU64`                                                   | `readU64`                                                              | 64-bit unsigned integer (Little Endian)           |
| `int` (i8)                 | `writeI8`                                                    | `readI8`                                                               | 8-bit signed integer                              |
| `int` (i16)                | `writeI16`                                                   | `readI16`                                                              | 16-bit signed integer (Little Endian)             |
| `int` (i32)                | `writeI32`                                                   | `readI32`                                                              | 32-bit signed integer (Little Endian)             |
| `int` (i64)                | `writeI64`                                                   | `readI64`                                                              | 64-bit signed integer (Little Endian)             |
| `double` (f32)             | `writeF32`                                                   | `readF32`                                                              | 32-bit float (IEEE 754, Little Endian)            |
| `double` (f64)             | `writeF64`                                                   | `readF64`                                                              | 64-bit float (IEEE 754, Little Endian)            |
| `bool`                     | `writeBool`                                                  | `readBool`                                                             | `1` for true, `0` for false                       |
| `String` (char)            | `writeChar(char)`                                            | `readChar()`                                                           | Single Unicode char (rune as u32 - Bincode v1/legacy) |
| `String`                   | `writeString`                                                | `readString`                                                           | UTF-8, length-prefixed (u64)                      |
| `String` (fixed)           | `writeFixedString(value, len)`                               | `readFixedString(len)`, `readCleanFixedString(len)`                    | Fixed byte length, padded/truncated               |
| `T?` (Option\<T\>)           | `writeOption<Type>`, `writeOptionBool`, etc.                  | `readOption<Type>`, `readOptionBool`, etc.                             | `0` tag for None, `1` tag + value for Some        |
| `String?` (optional char)  | `writeOptionChar(char)`                                      | `readOptionChar()`                                                     | `Option<char>`: tag (u8) + char (u32) if Some     |
| `List<T>`                  | `writeList(list, elementWriter)`                             | `readList(elementReader)`                                              | `Vec<T>`: Length-prefixed (u64) + elements        |
| `List<T>` (fixed array)    | `writeFixedArray(list, len, elementWriter)`                  | `readFixedArray(len, elementReader)`                                   | `[T; N]`: Fixed size, no length prefix            |
| `Set<T>`                   | `writeSet(set, elementWriter)`                               | `readSet(elementReader)`                                               | Like `Vec<T>`: length-prefixed (u64) + elements   |
| `Map<K, V>`                | `writeMap(map, keyWriter, valueWriter)`                      | `readMap(keyReader, valueReader)`                                      | Length-prefixed (u64) + key/value pairs           |
| `Uint8List`                | `writeUint8List`, `writeBytes`                               | `readUint8List`, `readBytes(len)`, `readRawBytes(len)`                 | Length-prefixed (u64) or raw bytes                |
| Integer `TypedData`        | `writeInt8List`, `writeUint16List`, `writeInt32List`, etc.   | `readInt8List`, `readUint16List`, `readInt32List`, etc.                | `Vec<Int>`: Length-prefixed (u64) + typed data    |
| Float `TypedData`          | `writeFloat32List`, `writeFloat64List`                       | `readFloat32List`, `readFloat64List`                                   | `Vec<Float>`: Length-prefixed (u64) + typed data  |
| `BincodeCodable` (Known)| `writeNestedObjectWithKnownSize(v, sz)` | `readNestedObjectWithKnownSize(i, sz)` | Fixed layout, no prefix. `KnownSize`has better performance when size is known. |
| `BincodeCodable?` (Known)|`writeOptionNestedObjectWithKnownSize(v, sz)` | `readOptionNestedObjectWithKnownSize(c, sz)`** | `Option<Fixed>`: tag + fixed layout.`KnownSize`has better performance when size is known. |
| `BincodeCodable` (Nested)  | `writeNestedValueForFixed`, `writeNestedValueForCollection`  | `readNestedObjectForFixed`, `readNestedObjectForCollection`            | Handles nested serializable objects               |
| `BincodeCodable?` (Nested) | `writeOptionNestedValueForFixed`, `writeOptionNestedValueForCollection` | `readOptionNestedObjectForFixed`, `readOptionNestedObjectForCollection` | Handles optional nested serializable objects    |
| `int` (enum discriminant)  | `writeEnumDiscriminant(discriminant)`                        | `readEnumDiscriminant()`                                               | Represents enum variant index (u32 - legacy mode) |
| `Duration`                 | `writeDuration(duration)`                                    | `readDuration()`                                                       | Custom: seconds (i64) + nanos (u32)               |
| `Duration?`                | `writeOptionDuration(duration)`                              | `readOptionDuration()`                                                 | `Option<Duration>`: tag (u8) + duration if Some |

### Working with Nested Objects

When encoding nested `BincodeCodable` objects, use the `writeNested...` methods. Choose the appropriate variant:

  * `...ForFixed`: Use when the nested object is part of a structure where its size contribution is implicitly handled (like fields within a class).
  * `...ForCollection`: Use when the nested object is part of a dynamic collection (like `List` or `Map`) where its size needs to be explicitly included in the serialization stream.
  * `...WithKnownSize`: Use for fixed-size objects when the size is known upfront. For reading (`read...WithKnownSize`), this **avoids the overhead of encoding a temporary object just to determine the size**, improving performance compared to `readNestedObjectForFixed`.

<!-- end list -->

```dart
class Inner implements BincodeCodable {
  int code;
  Inner(this.code);
  Inner.empty() : code = 0;
  @override void encode(BincodeWriter w) => w.writeU32(code);
  @override void decode(BincodeReader r) => code = r.readU32();
}

class Outer implements BincodeCodable {
  Inner innerFixed;       // This is like a fixed struct field
  List<Inner> innerList;  // This requires collection handling

  Outer(this.innerFixed, this.innerList);
  Outer.empty() : innerFixed = Inner.empty(), innerList = [];

  @override
  void encode(BincodeWriter w) {
    w.writeNestedValueForFixed(innerFixed); // Fixed context
    w.writeList<Inner>(innerList, (item) {
      w.writeNestedValueForCollection(item); // Collection context
    });
  }

  @override
  void decode(BincodeReader r) {
    innerFixed = r.readNestedObjectForFixed(Inner.empty()); // Fixed context
    innerList = r.readList<Inner>(() {
      return r.readNestedObjectForCollection(Inner.empty()); // Collection context
    });
  }
}
```

### Writer Utilities

  * `writer.toBytes()`: Get the encoded `Uint8List`.
  * `writer.getBytesWritten()`: Get the number of bytes written so far.
  * `writer.measure(codable)`: Calculate the encoded size without actually writing.
  * `writer.reserve(bytes)`: Ensure capacity without losing existing data.
  * `writer.reset()`: Clear the writer for reuse.
  * `BincodeWriter.encode(codable)`: Static method for direct encoding.
  * `BincodeWriter.encodeToBase64(codable)`: Encode directly to Base64 string.
  * `BincodeWriter.toHex(bytes)`: Convert bytes to hex string.
  * `writer.encodeToSink(codable, sink)`: Asynchronously write to an `IOSink`.
  * `BincodeWriter.encodeToFileSync(codable, path)`: Synchronously write to a file.

### Reader Utilities

  * `reader.position`: Get the current read position.
  * `reader.seek(offset)` / `reader.seekTo(position)` / `reader.rewind()` / `reader.skipToEnd()`: Navigate the buffer.
  * `reader.hasBytes(count)`: Check if enough bytes remain.
  * `reader.isAligned(bytes)` / `reader.align(bytes)`: Check and enforce alignment.
  * `reader.skipU8()`, `reader.skipU16()`, etc.: Skip over data types.
  * `reader.peekSession(() => ...)`: Read data without advancing the position.
  * `reader.skipOption(() => ...)`: Skip over an optional value (present or absent).
  * `BincodeReader.decode(bytes, emptyInstance)`: Static method for decoding dynamically sized objects.
  * `BincodeReader.decodeFixed(bytes, emptyInstance)`: Static method for decoding fixed-layout objects.
  * `BincodeReader.isValidBincode(bytes, emptyInstance)`: Validate if bytes can be decoded.
  * `BincodeReader.fromBuffer(buffer)`: Create reader from a `ByteBuffer`.
  * `BincodeReader.stripNulls(string)`: Remove trailing null characters.
  * `BincodeReader.peekLength(bytes)`: Read the initial u64 length prefix.

  ### Performance Utilities

* **[BincodeWriterPool]**: For applications performing frequent serialization (e.g. IPC etc.), using a pool can reduce object allocation overhead and GC pressure by reusing `BincodeWriter` instances and their buffers.
    ```dart
    final pool = BincodeWriterPool(); // Create a shared pool
    final bytes = pool.use((writer) { // Use a writer from the pool
      writer.writeI32(123);
      return writer.toBytes();
    });
    ```
* **[BitMask]**: A helper class to efficiently manage up to 8 boolean flags stored within a single byte. Useful for status fields, options, etc.
    ```dart
    final mask = BitMask();
    mask.setBit(0, true); // Set first flag
    mask.setBit(7, true); // Set last flag
    writer.writeU8(mask.value); // Write the byte (value: 129)

    final readMask = BitMask(reader.readU8());
    print(readMask.getBit(0)); // true
    print(readMask.getBit(1)); // false
    ```

## Benchmarks

**Disclaimer:** The following benchmark results are provided for illustrative purposes only. Actual performance may vary significantly depending on the specific data structures being serialized, the hardware used, the Dart VM version, overall system load, and other factors. These figures do not constitute a guarantee of performance in your specific application. Always benchmark within your own use case.

### MyData Class Example

Results are from code/encode the example MyData with Bincode vs Json vs Protobuf via benchmark_harness package.

Json was generated via json_serializable.

Protobuf via protoc cmd generate command.

Bincode from d_bincode like in the example.

```text

Data Setup & Initial Serialization:
  Bincode Initial Encode took: 0.01 ms
  JSON Initial Encode took:    0.06 ms
  Protobuf Initial Encode took:0.10 ms (includes Dart->Proto conversion)

Serialized Size:
  Bincode:  670.0 bytes
  JSON:     1679.0 bytes
  Protobuf: 582.0 bytes
  Size Ratios (vs Bincode): JSON: 2.51x | Protobuf: 0.87x
  Size Ratio (JSON / Protobuf): 2.88x
----------------------------------------------------------------------
Running Benchmarks (Average time per operation)...

Benchmark Results:
======================================================================
 ENCODING (Object -> Bytes)
----------------------------------------------------------------------
  Bincode:  14.53 µs     / op  |  Ops/sec:      68820
  JSON:     235.24 µs    / op  |  Ops/sec:       4251
  Protobuf: 59.92 µs     / op  |  Ops/sec:      16688
----------------------------------------------------------------------
  Ratios (vs Bincode): JSON: 16.19x | Protobuf: 4.12x
  Ratio (JSON / Protobuf): 3.93x
----------------------------------------------------------------------
 DECODING (Bytes -> Object)
----------------------------------------------------------------------
  Bincode:  20.13 µs     / op  |  Ops/sec:      49667
  JSON:     180.22 µs    / op  |  Ops/sec:       5549
  Protobuf: 59.99 µs     / op  |  Ops/sec:      16670
----------------------------------------------------------------------
  Ratios (vs Bincode): JSON: 8.95x | Protobuf: 2.98x
  Ratio (JSON / Protobuf): 3.00x
======================================================================

```

### Complex Nested Data Test

Results from serializing/deserializing a complex, multi-level nested object 5 million times:

```text
======================================
 Serialization Benchmark (×5,000,000)
======================================

>>> Bincode
Serialize                Total: 6311.90ms   Avg:    1.26µs
Deserialize              Total: 9677.11ms   Avg:    1.94µs
  Size: 518 bytes

>>> JSON
Round‑trip               Total: 144801.50ms   Avg:   28.96µs
  Size: 1,420 bytes

>>> Speedups & Savings
  Serialize speedup:     22.94×
  Deserialize speedup:   14.96×
  Size ratio (JSON/B):   2.74×
  Saved bytes:           902 (63.5% smaller)
--------------------------------------
```

### Simple Data Test (Single u32)

Results from serializing/deserializing a single 32-bit unsigned integer 100 million times:

```text
==========================================
 Minimal Payload Benchmark (×100,000,000) 
==========================================

Bincode Encode     | Total:   225.7ms | Avg:   0.002µs
Bincode Decode     | Total:  1064.8ms | Avg:   0.011µs
JSON round‑trip    | Total: 51050.1ms | Avg:   0.511µs
--------------------------------------------------
Round‑trips        | Total:  1290.5ms | Avg:   0.013µs

Speed‑ups vs JSON:
  • Encode faster:      226.20×
  • Decode faster:      47.94×
  • Round‑trip faster:  39.56×

Size Comparison:
  • Bincode: 4 bytes
  • JSON:    15 bytes
  • JSON is 3.75× larger
  • Bincode saves 73.3%

Total Saved Over All Items:
  • Bytes: 1,100,000,000
  • KB:    1074218.75
  • MB:    1049.04
```

**Note:** As stated in the disclaimer, these specific values depend heavily on the testing environment and the nature of the data.

## Additional Information

Originally created for a private project, this package wasn't built with every use case in mind. If you want to extend it for your own needs, fork or open a pull request.

## License

[MIT License](https://www.google.com/search?q=/LICENSE)
