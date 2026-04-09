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

/// Base exception used for all bincode-related errors.
base class BincodeException implements Exception {
  /// A descriptive error message.
  final String message;

  /// An optional underlying error or additional context.
  final dynamic cause;

  /// Creates a new BincodeException with a message and optional cause.
  BincodeException(this.message, [this.cause]);

  @override
  String toString() {
    return cause == null
        ? 'BincodeException: $message'
        : 'BincodeException: $message\nCaused by: $cause';
  }
}

/// Thrown when an invalid boolean value is encountered during decoding.
///
/// According to the specification, boolean values should be encoded as
/// a single byte of value 0 (false) or 1 (true). Any other value triggers this error.
base class InvalidBooleanValueException extends BincodeException {
  /// The value that was read which is not a valid boolean.
  final int value;

  InvalidBooleanValueException(this.value)
      : super('Invalid boolean value encountered: $value. Expected 0 or 1.');
}

/// Thrown when an invalid option tag is encountered.
/// Option fields should be encoded with a single byte: 0 (None) or 1 (Some).
base class InvalidOptionTagException extends BincodeException {
  final int tag;
  InvalidOptionTagException(this.tag)
      : super(
            'Invalid option tag encountered: $tag. Expected 0 (None) or 1 (Some).');
}
