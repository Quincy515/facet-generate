// English: Extended byte-level compatibility smoke between Crux's BincodeFfiFormat
// (bincode 1.3 + fixint_encoding) and vendored d_bincode. Covers the common Rust
// shape library: struct, Option<T>, Vec<T>, and enum variants (unit / newtype / struct).
// 中文:扩展版字节级兼容 smoke test,覆盖 struct、Option<T>、Vec<T>、enum variant
// (unit / newtype / struct)等常见 Rust 形状。

import 'dart:typed_data';
import 'package:d_bincode/d_bincode.dart';

// ─── Fixtures matching Rust side ─────────────────────────────────────────

class ViewModel {
  final String text;
  final bool confirmed;
  final String platform;
  const ViewModel(
      {required this.text, required this.confirmed, required this.platform});

  void bincodeEncode(BincodeWriter w) {
    w.writeString(text);
    w.writeBool(confirmed);
    w.writeString(platform);
  }

  static ViewModel bincodeDecode(BincodeReader r) => ViewModel(
        text: r.readString(),
        confirmed: r.readBool(),
        platform: r.readString(),
      );

  @override
  String toString() =>
      'ViewModel(text: $text, confirmed: $confirmed, platform: $platform)';
}

class WithOptions {
  final String? maybeName;
  final int? maybeCount;
  final bool? definitelyNone;
  const WithOptions(
      {required this.maybeName,
      required this.maybeCount,
      required this.definitelyNone});

  // English: Manual Option encoding matches Rust serde:
  //   None → [0]
  //   Some(v) → [1] + bincode(v)
  // 中文:手工 Option 编码,对齐 Rust serde 的协议
  void bincodeEncode(BincodeWriter w) {
    if (maybeName != null) {
      w.writeU8(1);
      w.writeString(maybeName!);
    } else {
      w.writeU8(0);
    }
    if (maybeCount != null) {
      w.writeU8(1);
      w.writeU32(maybeCount!);
    } else {
      w.writeU8(0);
    }
    if (definitelyNone != null) {
      w.writeU8(1);
      w.writeBool(definitelyNone!);
    } else {
      w.writeU8(0);
    }
  }

  static WithOptions bincodeDecode(BincodeReader r) {
    final hasName = r.readU8() == 1;
    final name = hasName ? r.readString() : null;
    final hasCount = r.readU8() == 1;
    final count = hasCount ? r.readU32() : null;
    final hasDef = r.readU8() == 1;
    final def = hasDef ? r.readBool() : null;
    return WithOptions(
        maybeName: name, maybeCount: count, definitelyNone: def);
  }

  @override
  String toString() =>
      'WithOptions(maybeName: $maybeName, maybeCount: $maybeCount, definitelyNone: $definitelyNone)';
}

class WithVec {
  final List<String> items;
  final List<int> numbers;
  const WithVec({required this.items, required this.numbers});

  void bincodeEncode(BincodeWriter w) {
    w.writeU64(items.length);
    for (final s in items) {
      w.writeString(s);
    }
    w.writeU64(numbers.length);
    for (final n in numbers) {
      w.writeI32(n);
    }
  }

  static WithVec bincodeDecode(BincodeReader r) {
    final itemsLen = r.readU64();
    final items = <String>[];
    for (var i = 0; i < itemsLen; i++) {
      items.add(r.readString());
    }
    final numbersLen = r.readU64();
    final numbers = <int>[];
    for (var i = 0; i < numbersLen; i++) {
      numbers.add(r.readI32());
    }
    return WithVec(items: items, numbers: numbers);
  }

  @override
  String toString() => 'WithVec(items: $items, numbers: $numbers)';
}

// English: sealed class Event = Rust `enum Event { None, Increment, SetName(String), Move { x, y } }`
// variant index is u32 in bincode 1.x fixint mode.
// 中文:sealed class Event 对应 Rust enum,variant 索引在 bincode 1.x fixint 模式下是 u32。
sealed class Event {
  const Event();

  void bincodeEncode(BincodeWriter w);

  static Event bincodeDecode(BincodeReader r) {
    final variant = r.readU32();
    return switch (variant) {
      0 => const EventNone(),
      1 => const EventIncrement(),
      2 => EventSetName(r.readString()),
      3 => EventMove(r.readI32(), r.readI32()),
      _ => throw StateError('Unknown Event variant: $variant'),
    };
  }
}

final class EventNone extends Event {
  const EventNone();
  @override
  void bincodeEncode(BincodeWriter w) => w.writeU32(0);
  @override
  String toString() => 'Event::None';
}

final class EventIncrement extends Event {
  const EventIncrement();
  @override
  void bincodeEncode(BincodeWriter w) => w.writeU32(1);
  @override
  String toString() => 'Event::Increment';
}

final class EventSetName extends Event {
  final String value;
  const EventSetName(this.value);
  @override
  void bincodeEncode(BincodeWriter w) {
    w.writeU32(2);
    w.writeString(value);
  }

  @override
  String toString() => 'Event::SetName($value)';
}

final class EventMove extends Event {
  final int x;
  final int y;
  const EventMove(this.x, this.y);
  @override
  void bincodeEncode(BincodeWriter w) {
    w.writeU32(3);
    w.writeI32(x);
    w.writeI32(y);
  }

  @override
  String toString() => 'Event::Move(x: $x, y: $y)';
}

// ─── Test helper ─────────────────────────────────────────────────────────

int _passCount = 0;
int _failCount = 0;

/// English: Run one round-trip check. Rust bytes → Dart decode → assert →
/// Dart encode → compare to Rust bytes.
/// 中文:跑一次 round-trip 检查:Rust 字节 → Dart 解码 → 断言 → Dart 编码 →
/// 和 Rust 字节比对。
void check<T>({
  required String label,
  required List<int> rustBytes,
  required T Function(BincodeReader r) decode,
  required void Function(T value, BincodeWriter w) encode,
  required T expected,
  required bool Function(T a, T b) equals,
}) {
  print('── $label ──');
  final bytes = Uint8List.fromList(rustBytes);
  print('  Rust bytes (${bytes.length}): ${_hex(bytes)}');

  // Dart decode
  final decoded = decode(BincodeReader(bytes));
  print('  Decoded: $decoded');
  if (!equals(decoded, expected)) {
    print('  ❌ FAIL: decoded value does not match expected');
    _failCount++;
    return;
  }

  // Dart encode → byte-level diff
  final w = BincodeWriter();
  encode(decoded, w);
  final dartBytes = w.toBytes();
  if (dartBytes.length != bytes.length ||
      !_bytesEqual(dartBytes, bytes)) {
    print('  ❌ FAIL: Dart re-encode does not match Rust bytes');
    print('     Dart (${dartBytes.length}): ${_hex(dartBytes)}');
    _failCount++;
    return;
  }

  print('  ✅ PASS');
  _passCount++;
}

String _hex(Uint8List bytes) =>
    bytes.map((b) => b.toRadixString(16).padLeft(2, '0')).join('');

bool _bytesEqual(Uint8List a, Uint8List b) {
  if (a.length != b.length) return false;
  for (var i = 0; i < a.length; i++) {
    if (a[i] != b[i]) return false;
  }
  return true;
}

// ─── Main ────────────────────────────────────────────────────────────────

void main() {
  print('════════════════════════════════════════════════════');
  print('  Bincode byte-level compatibility smoke (extended)');
  print('  crux_core::bridge::BincodeFfiFormat');
  print('  ↔ d_bincode (bincode 1.x fixint mode)');
  print('════════════════════════════════════════════════════\n');

  // Case 1: Simple struct
  check<ViewModel>(
    label: 'Case 1: ViewModel struct',
    rustBytes: [
      5, 0, 0, 0, 0, 0, 0, 0, 104, 101, 108, 108, 111, //
      1, //
      3, 0, 0, 0, 0, 0, 0, 0, 105, 79, 83,
    ],
    decode: ViewModel.bincodeDecode,
    encode: (v, w) => v.bincodeEncode(w),
    expected: const ViewModel(
        text: 'hello', confirmed: true, platform: 'iOS'),
    equals: (a, b) =>
        a.text == b.text &&
        a.confirmed == b.confirmed &&
        a.platform == b.platform,
  );

  // Case 2: Option Some/Some/None
  check<WithOptions>(
    label: 'Case 2: WithOptions (Some("alice"), Some(42), None)',
    rustBytes: [
      1, 5, 0, 0, 0, 0, 0, 0, 0, 97, 108, 105, 99, 101, //
      1, 42, 0, 0, 0, //
      0,
    ],
    decode: WithOptions.bincodeDecode,
    encode: (v, w) => v.bincodeEncode(w),
    expected: const WithOptions(
        maybeName: 'alice', maybeCount: 42, definitelyNone: null),
    equals: (a, b) =>
        a.maybeName == b.maybeName &&
        a.maybeCount == b.maybeCount &&
        a.definitelyNone == b.definitelyNone,
  );

  // Case 3: Option all None
  check<WithOptions>(
    label: 'Case 3: WithOptions (all None)',
    rustBytes: [0, 0, 0],
    decode: WithOptions.bincodeDecode,
    encode: (v, w) => v.bincodeEncode(w),
    expected: const WithOptions(
        maybeName: null, maybeCount: null, definitelyNone: null),
    equals: (a, b) =>
        a.maybeName == b.maybeName &&
        a.maybeCount == b.maybeCount &&
        a.definitelyNone == b.definitelyNone,
  );

  // Case 4: Vec<String> and Vec<i32>
  check<WithVec>(
    label: 'Case 4: Vec<String> + Vec<i32>',
    rustBytes: [
      3, 0, 0, 0, 0, 0, 0, 0, // Vec<String> len
      1, 0, 0, 0, 0, 0, 0, 0, 97, //
      2, 0, 0, 0, 0, 0, 0, 0, 98, 99, //
      3, 0, 0, 0, 0, 0, 0, 0, 100, 101, 102, //
      3, 0, 0, 0, 0, 0, 0, 0, // Vec<i32> len
      1, 0, 0, 0, //
      254, 255, 255, 255, //
      3, 0, 0, 0,
    ],
    decode: WithVec.bincodeDecode,
    encode: (v, w) => v.bincodeEncode(w),
    expected: const WithVec(items: ['a', 'bc', 'def'], numbers: [1, -2, 3]),
    equals: (a, b) =>
        a.items.length == b.items.length &&
        List.generate(a.items.length, (i) => a.items[i] == b.items[i])
            .every((x) => x) &&
        a.numbers.length == b.numbers.length &&
        List.generate(a.numbers.length, (i) => a.numbers[i] == b.numbers[i])
            .every((x) => x),
  );

  // Case 5: Event::None
  check<Event>(
    label: 'Case 5: Event::None (unit variant, index 0 as u32)',
    rustBytes: [0, 0, 0, 0],
    decode: Event.bincodeDecode,
    encode: (v, w) => v.bincodeEncode(w),
    expected: const EventNone(),
    equals: (a, b) => a.toString() == b.toString(),
  );

  // Case 6: Event::Increment
  check<Event>(
    label: 'Case 6: Event::Increment (unit variant, index 1 as u32)',
    rustBytes: [1, 0, 0, 0],
    decode: Event.bincodeDecode,
    encode: (v, w) => v.bincodeEncode(w),
    expected: const EventIncrement(),
    equals: (a, b) => a.toString() == b.toString(),
  );

  // Case 7: Event::SetName("bob")
  check<Event>(
    label: 'Case 7: Event::SetName("bob") (newtype variant)',
    rustBytes: [2, 0, 0, 0, 3, 0, 0, 0, 0, 0, 0, 0, 98, 111, 98],
    decode: Event.bincodeDecode,
    encode: (v, w) => v.bincodeEncode(w),
    expected: const EventSetName('bob'),
    equals: (a, b) => a.toString() == b.toString(),
  );

  // Case 8: Event::Move { x: 10, y: -20 }
  check<Event>(
    label: 'Case 8: Event::Move { x: 10, y: -20 } (struct variant)',
    rustBytes: [3, 0, 0, 0, 10, 0, 0, 0, 236, 255, 255, 255],
    decode: Event.bincodeDecode,
    encode: (v, w) => v.bincodeEncode(w),
    expected: const EventMove(10, -20),
    equals: (a, b) => a.toString() == b.toString(),
  );

  print('\n════════════════════════════════════════════════════');
  if (_failCount == 0) {
    print('  ALL $_passCount TESTS PASSED ✅');
    print('  Byte-level compatibility across struct/Option/Vec/enum');
    print('  between Crux BincodeFfiFormat and d_bincode: CONFIRMED');
  } else {
    print('  $_passCount passed, $_failCount FAILED ❌');
  }
  print('════════════════════════════════════════════════════');
}
