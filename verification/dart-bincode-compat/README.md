# Dart ↔ Bincode Byte-Level Compatibility Verification

**Status**: ✅ 8/8 test cases passing (2026-04-09, Step 5a exit criteria met)

## 这是什么 / What this is

一个**一次性的字节级兼容 smoke test**,证明 `facet_generate` 的 Dart 后端将要
依赖的 `d_bincode` Dart 库,和 `crux_core::bridge::BincodeFfiFormat`(即
`bincode = "=1.3"` + `.with_fixint_encoding().allow_trailing_bytes()`)
在 wire format 上**完全字节级对齐**。

A one-shot byte-level compatibility smoke test proving that the `d_bincode`
Dart library (which the Dart backend of `facet_generate` will rely on) is
byte-for-byte compatible with `crux_core::bridge::BincodeFfiFormat`
(which pins `bincode = "=1.3"` with
`.with_fixint_encoding().allow_trailing_bytes()`).

## 为什么重要 / Why it matters

这是 **Step 5 的 exit criteria**(见 Obsidian 笔记
《Crux + Flutter 集成决策》§13.5)。如果字节级不对齐,Step 5b 写的
`DartBincodePlugin` 生成的代码会因为一个字节错位就全部失败,前功尽弃。
必须在**写 plugin 前**先证明 wire format 对齐。

This is the **exit criterion for Step 5** of the Dart backend work. If the
wire format is not byte-aligned, the `DartBincodePlugin` written in Step 5b
would silently produce code that fails on a single misaligned byte, wasting
all subsequent implementation work. We MUST prove byte alignment before
writing the plugin.

## 结构 / Layout

```
verification/dart-bincode-compat/
├── README.md               ← this file
├── rust/
│   ├── Cargo.toml          ← bincode = "=1.3", serde
│   └── src/main.rs         ← emits 8 fixtures to stdout as decimal arrays
└── dart/
    ├── pubspec.yaml        ← path dependency on ./d_bincode
    ├── bin/smoke.dart      ← reads 8 fixtures, round-trips, byte-diffs
    └── d_bincode/          ← vendored d_bincode 3.2.0 (self-contained)
```

## 跑一次 / How to run

```bash
# Rust side: generate fixture bytes
cd rust
cargo run

# Dart side: consume bytes, assert byte-level equality
cd ../dart
dart pub get
dart run bin/smoke.dart
```

两边输出的字节应完全一致,Dart 侧会打印 `ALL 8 TESTS PASSED ✅`。

## 测试覆盖的 8 个 case / 8 cases covered

| # | Rust shape | Bytes | Key fact |
|---|---|---:|---|
| 1 | `struct { text: String, confirmed: bool, platform: String }` | 25 | String = `[u64 len][utf8]`,bool = `[u8 0/1]` |
| 2 | `Option<String> + Option<u32> + Option<bool>`(Some/Some/None) | 20 | Some tag `[1]` + 值,None tag `[0]` |
| 3 | `Option` 全 None | 3 | 三个 `[0]` tag |
| 4 | `Vec<String> + Vec<i32>`(含负数) | 58 | `Vec` = `[u64 len][elem elem ...]`,i32 小端 |
| 5 | `Event::None`(unit variant 0) | 4 | **variant index 是 u32(4 bytes)**,不是 u8 |
| 6 | `Event::Increment`(unit variant 1) | 4 | variant index = `[1, 0, 0, 0]` |
| 7 | `Event::SetName("bob")`(newtype) | 15 | `[u32 index] + [u64 len][utf8]` |
| 8 | `Event::Move { x: 10, y: -20 }`(struct) | 12 | `[u32 index] + i32 + i32` |

## Wire format cheat sheet(供 Step 5b 写 `DartBincodePlugin` 参考)

```
Rust type                   bincode 1.3 fixint wire format
─────────────────────────────────────────────────────────────────
bool                        [u8: 0 or 1]
i8/u8                       1 byte
i16/u16                     2 bytes LE
i32/u32                     4 bytes LE
i64/u64                     8 bytes LE
f32                         4 bytes LE (IEEE 754)
f64                         8 bytes LE (IEEE 754)
String                      [u64 byte_len][utf8 bytes]
Vec<T>                      [u64 elem_count][T, T, ...]
Option<T>::None             [u8 = 0]
Option<T>::Some(v)          [u8 = 1] + bincode(v)
enum variant (任意)          [u32 variant_index] + bincode(payload)
struct (任意)                [field0] + [field1] + ... (无分隔,无长度)
```

**注意点 / Critical notes**:

1. **Enum variant index 是 u32(4 字节)**,不是 u8——即使类型只有几个
   variant。DartBincodePlugin 的 encode 代码**必须**用 `w.writeU32(index)`
   而不是 `w.writeU8(index)`,否则第 256 个 variant 开始字节错位,debug 灾难。
2. **Struct 字段按声明顺序编码**,没有名字、没有长度前缀。**字段顺序必须严格
   按 `facet::Field` 的 index**,不能按字母序。
3. **String 长度是 u64(8 字节)**,即使字符串很短。这是 fixint 模式的特征。
4. d_bincode 默认构造 `BincodeWriter()` 就是 fixint 模式,**无需任何参数配置**。

## 维护约定 / Maintenance

如果将来 Crux 升级 bincode 版本(例如从 1.x 到 2.x 默认模式),wire format
会变。届时这个 smoke test 会开始失败,需要同步升级 vendor 的 d_bincode
或切换编码模式。**这个 test 存在的主要价值就是这种"版本升级保护"**。

If Crux upgrades its bincode version in the future (e.g. from 1.x to 2.x
default mode), the wire format will change. This smoke test will start
failing, alerting us to synchronize the vendored `d_bincode` or switch
encoding modes. **Version-upgrade protection is the main value of this
test.**

## 下一步 / What follows

Step 5b 会在 `crates/facet_generate/src/generation/bincode/dart.rs` 重写
`DartBincodePlugin`,生成的 Dart 代码调用 `d_bincode` 的 API(`writeString`、
`writeU32`、`readBool` 等)。届时可以回来引用本目录的 wire format cheat sheet。

Step 5b will rewrite `DartBincodePlugin` in
`crates/facet_generate/src/generation/bincode/dart.rs` to emit Dart code
that calls `d_bincode`'s API (`writeString`, `writeU32`, `readBool`, etc.).
At that point you can come back here for the wire format cheat sheet.
