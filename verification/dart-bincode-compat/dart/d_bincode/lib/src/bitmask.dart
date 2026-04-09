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

/// A utility class to manage 8 individual boolean flags packed within a single byte (u8).
///
/// This class helps set, get, and interpret the bits of an 8-bit integer value,
/// commonly used for sending multiple boolean states efficiently.
///
/// Use this class to prepare a byte value representing flags before writing it
/// with `BincodeWriter.writeU8(myMask.value)`, or to interpret a byte read
/// using `final mask = BitMask(BincodeReader.readU8())`.
class BitMask {
  /// The internal 8-bit integer value holding the packed flags.
  int _value;

  /// Creates a BitMask instance.
  ///
  /// - [initialValue]: Optional initial byte value (0-255). Defaults to 0.
  ///   Throws [ArgumentError] if the value is outside the valid u8 range.
  BitMask([int initialValue = 0]) : _value = initialValue {
    if (initialValue < 0 || initialValue > 255) {
      throw ArgumentError(
          'Initial value must be between 0 and 255, got $initialValue');
    }
  }

  /// Creates a BitMask instance directly from a list of boolean flags.
  ///
  /// The flag at index `i` in the list corresponds to bit `i` (LSB at index 0).
  /// Throws [ArgumentError] if the list has more than 8 elements.
  static BitMask fromList(List<bool> flags) {
    if (flags.length > 8) {
      throw ArgumentError(
          'Cannot create a BitMask from more than 8 boolean flags, got ${flags.length}');
    }
    int byteValue = 0;
    for (int i = 0; i < flags.length; i++) {
      if (flags[i]) {
        byteValue |= (1 << i);
      }
    }
    return BitMask(byteValue);
  }

  /// Creates a BitMask instance from named boolean flags for bits 0 through 7.
  ///
  /// Unspecified bits default to `false` (0). Bit 0 is the least significant bit (LSB).
  static BitMask fromNamed({
    bool bit0 = false, // LSB
    bool bit1 = false,
    bool bit2 = false,
    bool bit3 = false,
    bool bit4 = false,
    bool bit5 = false,
    bool bit6 = false,
    bool bit7 = false, // MSB
  }) {
    int byteValue = 0;
    if (bit0) byteValue |= 1;
    if (bit1) byteValue |= 2;
    if (bit2) byteValue |= 4;
    if (bit3) byteValue |= 8;
    if (bit4) byteValue |= 16;
    if (bit5) byteValue |= 32;
    if (bit6) byteValue |= 64;
    if (bit7) byteValue |= 128;
    return BitMask(byteValue);
  }

  /// Gets the underlying 8-bit integer value representing the packed flags.
  ///
  /// This value (0-255) can be directly written using `BincodeWriter.writeU8()`.
  int get value => _value;

  /// Sets the underlying 8-bit integer value.
  ///
  /// Throws [ArgumentError] if the new value is outside the valid u8 range (0-255).
  set value(int newValue) {
    if (newValue < 0 || newValue > 255) {
      throw ArgumentError('Value must be between 0 and 255, got $newValue');
    }
    _value = newValue;
  }

  /// Sets the state of a specific bit.
  ///
  /// - [index]: The bit index (0-7, where 0 is the LSB).
  /// - [flag]: The boolean value (`true` to set the bit to 1, `false` to clear it to 0).
  /// Throws [RangeError] if the index is out of bounds.
  void setBit(int index, bool flag) {
    if (index < 0 || index > 7) {
      throw RangeError('Bit index must be between 0 and 7, got $index');
    }
    if (flag) {
      _value |= (1 << index);
    } else {
      _value &= ~(1 << index);
    }
  }

  /// Gets the state of a specific bit.
  ///
  /// - [index]: The bit index (0-7, where 0 is the LSB).
  /// Returns `true` if the bit is set (1), `false` otherwise (0).
  /// Throws [RangeError] if the index is out of bounds.
  bool getBit(int index) {
    if (index < 0 || index > 7) {
      throw RangeError('Bit index must be between 0 and 7, got $index');
    }
    return (_value & (1 << index)) != 0;
  }

  /// Returns the state of all 8 bits as a fixed-size list of booleans.
  ///
  /// Index `i` in the list corresponds to bit `i` (LSB at index 0).
  List<bool> toList() {
    return List<bool>.generate(8, (i) => (_value & (1 << i)) != 0,
        growable: false);
  }

  @override
  String toString() {
    return 'BitMask(value: $_value, bits: 0b${_value.toRadixString(2).padLeft(8, '0')})';
  }

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      other is BitMask &&
          runtimeType == other.runtimeType &&
          _value == other._value;

  @override
  int get hashCode => _value.hashCode;
}
