## 1.0.0

- Initial version.

## 1.0.1

- Added nested objects/classes support - writeNested, writeOptionalNested, readNestedObject, readOptionalNestedObject.
- `toBincode()` / `fromBincode()` API inherit from BincodeEncodable / BincodeDecodable.
- Added error handling and validation.
- Removed `Debuggable` and `Fluent` APIs.
- Added benchmarks (speed & size) vs JSON example.
- Improved documentation and fixed lints for pub.dev compliance.

## 2.0.0

### Breaking Changes

- Removed `length` parameter from `read*List` methods (now reads length prefix). Update calls by removing the argument.
- Renamed `loadFromBytes` to `fromBincode`.

### Added

- Optional `unsafe`/`unchecked` flags for `BincodeReader`/`BincodeWriter` to bypass checks for performance.
- New `readNestedObjectForFixed` and `readOptionNestedObjectForFixed` methods for fixed-size object reading.
- `readRawBytes(int length)` method.

### Deprecated

- `readNestedObject` / `readOptionNestedObject`. Use `ForCollection` or `ForFixed` variants instead.

### Removed

- Internal implementation classes (`Builder`, `Wrapper`, `Exception`) from public API.

## 3.0.0

### Breaking Changes

- BincodeEncodable & BincodeDecodable use now encode(BincodeReader reader) and decode(BincodeWriter writer) - from and toBincode are removed.

### Added

- More static methods for BincodeWriter and BincodeReader
- Improved performance by alot by removing Wrapper overheat and overall impls of the methods
- More methods for BincodeWriter and BincodeReader for manual buffer modifications

### Removed

- ByteDataWrapper removed to increase performance
- String Encode and Decode Method's and Enums removed - only utf8/ASCII Supported
- Less Exceptions and checks
- euc dependency got removed

## 3.0.1

### Patch

- Fixed formating and describtion in pubspec.

## 3.1.0

### Added

- Support for serializing/deserializing Dart `String` (single character) as Rust `char` (via `u32` rune - Bincode v1/legacy compatible) using `writeChar`/`readChar` and `writeOptionChar`/`readOptionChar`.
- Support for fixed-size arrays (Rust `[T; N]`) via `writeFixedArray`/`readFixedArray`, which serialize elements sequentially without a length prefix.
- Support for `Set<T>` via `writeSet`/`readSet`, serializing like `Vec<T>` (u64 length prefix + elements).
- Support for Rust-style enum discriminants (variant index) via `writeEnumDiscriminant`/`readEnumDiscriminant` (using `u32`) and `writeOptionEnumDiscriminant`/`readOptionEnumDiscriminant`.
- Support for `Duration` via `writeDuration`/`readDuration` and `writeOptionDuration`/`readOptionDuration`, using a format compatible with Rust's `chrono::Duration` (i64 seconds + u32 nanos).

## 3.2.0

### Added

-   **`BitMask` Utility:** Added class for managing 8 boolean flags packed into a single byte (`u8`). (Note: Bincode sends/receives the underlying `u8`; `BitMask` helps interpret it.)
-   **`BincodeWriterPool`:** Added class for reusing `BincodeWriter` instances to optimize performance and work with isolates etc.
-   **Known Size Methods:** Added `read/write...WithKnownSize` and `read/writeOption...WithKnownSize` methods for explicit size handling and validation of fixed-size objects. Improves Read performance.

### Deprecated

-   `BincodeWriter.measureFixedSize`: Superseded by internal reader caching and `...WithKnownSize` methods. Slated for future removal.

### Changed

-   Updated library documentation and README to cover `BitMask`, `Pool`, and `KnownSize` methods/conventions.