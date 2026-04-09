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

import 'dart:collection';

import '../d_bincode.dart';

/// Manages reusable [BincodeWriter] instances to optimize frequent serialization.
///
/// **Core Functionality:**
/// This pool optimizes performance by:
/// 1.  **Reusing Objects:** It maintains a queue (`Queue`) of idle `BincodeWriter`
///     instances. Instead of creating a new writer for every task, it tries
///     to provide one from the pool ([acquire]).
/// 2.  **Managing Size:** It controls the number of writer instances, creating
///     new ones on demand up to a configured maximum ([maxSize]). It can be
///     pre-warmed with an initial number of instances ([initialSize]).
///
/// This reduces object allocation overhead and minimizes garbage collection pressure.
///
/// ### Instantiation and Usage Patterns
///
/// `BincodeWriterPool` is a regular Dart class, meaning you can create as many
/// independent instances as needed within your application, limited only by
/// available memory and management complexity.
///
/// While often a single, shared pool instance is sufficient for an application, creating
/// **multiple pools** can be beneficial in specific scenarios:
///
/// - **Isolating Workloads:** If different parts of your application have vastly
///   different serialization patterns (e.g., one part serializes frequent small
///   messages, another handles infrequent large ones), separate pools can be
///   tuned ([initialSize], [maxSize]) independently for optimal performance
///   in each context.
/// - **Concurrency with Isolates:** In multi-isolate applications, each Dart isolate
///   can manage its own dedicated `BincodeWriterPool`. This avoids potential
///   contention or the need for complex synchronization mechanisms that might
///   arise if isolates shared direct access to a single pool's internal queue.
/// - **Testing:** Using distinct pool instances for different test suites or
///   scenarios ensures better test isolation.
///
/// Consider that managing multiple pools increases configuration overhead slightly.
/// Evaluate whether the benefits of isolation or specific tuning outweigh the
/// simplicity of a single shared pool for your use case.
///
/// **Recommended Usage:**
/// ```dart
/// // Typically, create one pool shared across relevant parts of your app:
/// final sharedPool = BincodeWriterPool(initialSize: 4, maxSize: 16);
///
/// // Use the pool via `use` or `useAsync`:
/// Uint8List data = sharedPool.use((writer) => writer..writeI32(123)..toBytes());
///
/// await sharedPool.useAsync((writer) async {
///   writer.writeString("hello async");
///   // ... process data ...
/// });
/// ```
class BincodeWriterPool {
  final Queue<BincodeWriter> _pool;
  final int _maxSize;
  int _createdInstances = 0;

  /// Creates a writer pool with optional initial and maximum sizes.
  ///
  /// Writers are created with an initial buffer capacity of 128 bytes.
  ///
  /// - [initialSize]: Pre-allocated writers (default: 4). Must be >= 0 and <= [maxSize].
  /// - [maxSize]: Max writers managed (default: 16). Must be > 0.
  ///
  /// Throws [ArgumentError] if size constraints are violated.
  BincodeWriterPool({int initialSize = 4, int maxSize = 16})
      : _pool = Queue<BincodeWriter>(),
        _maxSize = maxSize {
    if (initialSize < 0) {
      throw ArgumentError.value(
          initialSize, 'initialSize', 'cannot be negative');
    }
    if (maxSize <= 0) {
      throw ArgumentError.value(maxSize, 'maxSize', 'must be positive');
    }
    if (initialSize > maxSize) {
      throw ArgumentError(
          'initialSize ($initialSize) cannot be greater than maxSize ($maxSize)');
    }

    for (int i = 0; i < initialSize; i++) {
      _pool.add(BincodeWriter(initialCapacity: 128));
      _createdInstances++;
    }
  }

  /// Acquires a writer from the pool or creates one if capacity allows.
  ///
  /// **Warning:** Requires manual release via [release]. Prefer [use] or [useAsync].
  /// Throws [StateError] if pool is empty and max size is reached.
  BincodeWriter acquire() {
    if (_pool.isNotEmpty) {
      return _pool.removeFirst();
    }
    if (_createdInstances < _maxSize) {
      _createdInstances++;
      return BincodeWriter(initialCapacity: 128);
    }
    throw StateError(
        'BincodeWriterPool reached maximum size ($_maxSize) and no writers are available.');
  }

  /// Resets and returns a writer to the pool if space is available.
  ///
  /// **Warning:** Must be paired with [acquire]. Prefer [use] or [useAsync].
  /// If the pool is full, the writer is discarded.
  void release(BincodeWriter writer) {
    if (_pool.length < _maxSize) {
      writer.reset();
      _pool.addLast(writer);
    } else {
      _createdInstances--;
    }
  }

  /// Executes a synchronous [action] with a managed [BincodeWriter].
  ///
  /// Automatically acquires, provides a reset writer to [action], and releases it.
  /// Returns the result of [action].
  R use<R>(R Function(BincodeWriter writer) action) {
    final writer = acquire();
    try {
      return action(writer);
    } finally {
      release(writer);
    }
  }

  /// Executes an asynchronous [action] with a managed [BincodeWriter].
  ///
  /// Automatically acquires, provides a reset writer to [action], awaits completion,
  /// and releases the writer. Returns the [Future] result of [action].
  Future<R> useAsync<R>(Future<R> Function(BincodeWriter writer) action) async {
    final writer = acquire();
    try {
      return await action(writer);
    } finally {
      release(writer);
    }
  }

  /// Number of idle writers currently in the pool.
  int get available => _pool.length;

  /// Total writers created (pooled or in-use). Decreases if writers are discarded.
  int get createdCount => _createdInstances;

  /// The maximum number of writers this pool can manage.
  int get maximumSize => _maxSize;
}
