//! `EmitterPlugin<Dart>` implementation for the [`BincodePlugin`].
//!
//! Generates `bincodeEncode` and `bincodeDecode` methods on Dart classes and
//! enums, producing byte output that is **byte-level compatible** with Rust's
//! `bincode` crate in `fixint_encoding` mode — exactly what Crux's
//! `crux_core::bridge::BincodeFfiFormat` uses.
//!
//! # Wire format reference
//!
//! This plugin's output has been verified against a real-Rust/real-Dart
//! round-trip suite in `verification/dart-bincode-compat/`. See that directory's
//! README for the full wire-format cheat sheet. The key facts:
//!
//! - Numeric types use their natural little-endian width (`i32` = 4 bytes, etc.)
//! - `String`: `[u64 byte_len][utf8 bytes]`
//! - `Vec<T>` / `Set<T>`: `[u64 elem_count][T, T, ...]`
//! - `Option<T>::None` = `[u8: 0]`, `Option<T>::Some(v)` = `[u8: 1] + bincode(v)`
//! - `enum` variant index is **u32** (4 bytes) — never u8
//! - `struct` has no separators or length prefix — fields are concatenated in
//!   declaration order
//!
//! # Dart side runtime
//!
//! Generated code calls into `d_bincode`'s `BincodeWriter` / `BincodeReader`
//! API (`writeString`, `writeU32`, `readBool`, etc.). The caller is expected
//! to have `d_bincode` vendored into their project (e.g. under
//! `lib/vendor/d_bincode/`).
//!
//! # Differences from the TypeScript bincode plugin
//!
//! 1. **No feature helper snippets** (`serializeArray`, `serializeOption`, …).
//!    Dart's `for` loops and `if null` checks are concise enough to inline,
//!    and d_bincode provides the primitive write/read methods directly.
//! 2. **No `load()` on variant subclasses** — the sealed base class's
//!    `bincodeDecode` reads the variant index and inlines the payload read
//!    via a Dart 3 `switch` expression. Variant subclasses only emit
//!    `bincodeEncode` (no decode at all).
//! 3. **All-unit enums get `index`-based encode/decode** using Dart's native
//!    `enum` index/values API instead of a sealed class hierarchy.
//!
//! # English / 中文
//!
//! English: See module-level docs above.
//! 中文:见模块顶部英文文档。核心:为 Dart class 和 enum 生成 bincodeEncode /
//! bincodeDecode,产出和 Rust bincode 1.x fixint 模式完全字节对齐的字节流。

use std::collections::BTreeMap;
use std::io;

use crate::generation::{
    CodeGeneratorConfig,
    dart::{Dart, FieldLayout, sanitize_dart_ident},
    indent::{IndentWrite, Newlines, with_block},
    plugin::{EmitContext, EmitterPlugin, VariantInfo},
};
use crate::reflection::format::{ContainerFormat, Format, Named, VariantFormat};

use super::BincodePlugin;

// ---------------------------------------------------------------------------
// EmitterPlugin implementation
// ---------------------------------------------------------------------------

impl EmitterPlugin<Dart> for BincodePlugin {
    fn module_helpers(
        &self,
        w: &mut dyn IndentWrite,
        _config: &CodeGeneratorConfig,
    ) -> io::Result<()> {
        // English: Every generated class references `BincodeWriter` /
        // `BincodeReader` from the `d_bincode` package. We emit a single
        // top-level import so downstream `dart analyze` can resolve them.
        // Per-field encode/decode code remains inline inside each class
        // (d_bincode's API is direct enough that no helper functions needed).
        // 中文:所有生成的 class 都引用 d_bincode 包的 `BincodeWriter` /
        // `BincodeReader`,这里统一 emit 一行顶层 import,下游 `dart analyze`
        // 才能解析它们。每个字段的 encode/decode 代码仍在 class 内部内联
        // (d_bincode 的 API 足够直接,不需要辅助函数)。
        writeln!(w, "import 'package:d_bincode/d_bincode.dart';")?;
        Ok(())
    }

    fn has_type_body(&self, _ctx: &EmitContext) -> bool {
        true
    }

    fn type_body(&self, w: &mut dyn IndentWrite, ctx: &EmitContext) -> io::Result<()> {
        // English: Three dispatch cases:
        //   1. Variant subclass (ctx.variant is Some) → emit encode only
        //   2. Enum with all-unit variants → native enum methods
        //   3. Enum with payload variants → sealed base class methods
        //   4. Struct → encode + decode on the class
        // 中文:4 种分派:
        //   1. 变体子类(ctx.variant 为 Some)→ 只生成 encode
        //   2. 全 unit 变体的 enum → 原生 enum 方法
        //   3. 带 payload 的 enum → sealed 基类方法
        //   4. struct → class 上的 encode + decode
        if let Some(variant) = &ctx.variant {
            return write_variant_encode(w, variant);
        }
        match ctx.container.format {
            ContainerFormat::Enum(variants, _) => {
                let all_unit = variants
                    .values()
                    .all(|v| matches!(v.value, VariantFormat::Unit));
                if all_unit {
                    write_native_enum_body(w, ctx.name(), variants)
                } else {
                    write_sealed_enum_base_body(w, ctx.name(), variants)
                }
            }
            _ => {
                let layout = FieldLayout::from_container(ctx.container.format);
                write_struct_type_body(w, ctx.name(), &ctx.fields(), layout)
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Struct / non-enum container bodies
// ---------------------------------------------------------------------------

/// Emit `bincodeEncode` and `static bincodeDecode` for a plain struct
/// (`UnitStruct`, `NewTypeStruct`, `TupleStruct`, or `Struct`).
fn write_struct_type_body(
    w: &mut dyn IndentWrite,
    name: &str,
    fields: &[Named<Format>],
    layout: FieldLayout,
) -> io::Result<()> {
    // ─── bincodeEncode ────────────────────────────────────────────────────
    writeln!(w)?;
    write!(w, "void bincodeEncode(BincodeWriter w) ")?;
    with_block(w, Newlines::BOTH, |w| {
        for field in fields {
            let ident = sanitize_dart_ident(&field.name);
            write_encode(w, &ident, &field.value)?;
        }
        Ok(())
    })?;

    // ─── static bincodeDecode ─────────────────────────────────────────────
    writeln!(w)?;
    write!(w, "static {name} bincodeDecode(BincodeReader r) ")?;
    with_block(w, Newlines::BOTH, |w| {
        for field in fields {
            let ident = sanitize_dart_ident(&field.name);
            write_decode_as_local(w, &ident, &field.value)?;
        }
        write_construct_call(w, name, fields, layout)
    })?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Variant subclass body (encode only)
// ---------------------------------------------------------------------------

/// Emit only `bincodeEncode` for an enum variant subclass. The corresponding
/// decode is inlined in the sealed base class's `bincodeDecode` dispatch
/// (see [`write_sealed_enum_base_body`]), so variant subclasses don't need
/// their own decode method.
fn write_variant_encode(
    w: &mut dyn IndentWrite,
    variant: &VariantInfo<'_>,
) -> io::Result<()> {
    let index = variant.index;
    let fields = variant.fields;

    writeln!(w)?;
    writeln!(w, "@override")?;
    write!(w, "void bincodeEncode(BincodeWriter w) ")?;
    with_block(w, Newlines::BOTH, |w| {
        // English: Variant index is u32 (4 bytes), not u8 — see wire format
        // cheat sheet in verification/dart-bincode-compat/README.md.
        // 中文:变体索引是 u32(4 字节),不是 u8。
        writeln!(w, "w.writeU32({index});")?;
        for field in fields {
            let ident = sanitize_dart_ident(&field.name);
            write_encode(w, &format!("this.{ident}"), &field.value)?;
        }
        Ok(())
    })?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Enum body (sealed base class OR native Dart enum)
// ---------------------------------------------------------------------------

/// Emit the body for a native Dart `enum` (used when all variants are `Unit`).
fn write_native_enum_body(
    w: &mut dyn IndentWrite,
    name: &str,
    _variants: &BTreeMap<u32, Named<VariantFormat>>,
) -> io::Result<()> {
    // ─── bincodeEncode ────────────────────────────────────────────────────
    writeln!(w)?;
    write!(w, "void bincodeEncode(BincodeWriter w) ")?;
    with_block(w, Newlines::BOTH, |w| {
        // English: Dart's native enum has a built-in `index` property — we use
        // that as the variant index directly.
        // 中文:Dart 原生 enum 有内置 `index` 属性,直接作为变体索引。
        writeln!(w, "w.writeU32(index);")
    })?;

    // ─── static bincodeDecode ─────────────────────────────────────────────
    writeln!(w)?;
    write!(w, "static {name} bincodeDecode(BincodeReader r) ")?;
    with_block(w, Newlines::BOTH, |w| {
        writeln!(w, "final idx = r.readU32();")?;
        writeln!(w, "if (idx < 0 || idx >= values.length) {{")?;
        writeln!(
            w,
            "    throw StateError('Unknown {name} variant index: $idx');"
        )?;
        writeln!(w, "}}")?;
        writeln!(w, "return values[idx];")
    })?;
    Ok(())
}

/// Emit the body for a sealed base class of an enum with payload variants.
///
/// Emits:
/// - Abstract `bincodeEncode` signature (implementations live on variant subclasses)
/// - Concrete `static bincodeDecode` that reads the u32 variant index and
///   dispatches via a `switch` expression to construct the appropriate
///   variant subclass, inlining the payload reads.
fn write_sealed_enum_base_body(
    w: &mut dyn IndentWrite,
    name: &str,
    variants: &BTreeMap<u32, Named<VariantFormat>>,
) -> io::Result<()> {
    // ─── abstract bincodeEncode ───────────────────────────────────────────
    writeln!(w)?;
    writeln!(w, "void bincodeEncode(BincodeWriter w);")?;

    // ─── static bincodeDecode (dispatcher) ────────────────────────────────
    writeln!(w)?;
    write!(w, "static {name} bincodeDecode(BincodeReader r) ")?;
    with_block(w, Newlines::BOTH, |w| {
        writeln!(w, "final variant = r.readU32();")?;
        write!(w, "switch (variant) ")?;
        with_block(w, Newlines::BOTH, |w| {
            for (index, named_variant) in variants {
                let vname = &named_variant.name;
                let variant_class = format!("{name}Variant{vname}");

                // English: Walk the variant's fields to generate local decode
                // statements, then call the variant subclass constructor.
                // 中文:遍历变体字段生成局部 decode 语句,然后调用变体子类构造器。
                match &named_variant.value {
                    VariantFormat::Unit => {
                        writeln!(w, "case {index}: return const {variant_class}();")?;
                    }
                    VariantFormat::NewType(inner) => {
                        let fields =
                            vec![Named::new(inner.as_ref(), "value".to_string())];
                        write_case_block(w, *index, &variant_class, &fields, FieldLayout::Positional)?;
                    }
                    VariantFormat::Tuple(inners) => {
                        let fields: Vec<Named<Format>> = inners
                            .iter()
                            .enumerate()
                            .map(|(i, f)| Named::new(f, format!("field{i}")))
                            .collect();
                        write_case_block(w, *index, &variant_class, &fields, FieldLayout::Positional)?;
                    }
                    VariantFormat::Struct(fs) => {
                        write_case_block(w, *index, &variant_class, fs, FieldLayout::Named)?;
                    }
                    VariantFormat::Variable(_) => panic!("unexpected Variable variant"),
                }
            }
            writeln!(
                w,
                "default: throw StateError('Unknown {name} variant: $variant');"
            )
        })
    })?;
    Ok(())
}

/// Emit one `case N: { ... }` block inside the sealed enum dispatcher.
fn write_case_block(
    w: &mut dyn IndentWrite,
    index: u32,
    variant_class: &str,
    fields: &[Named<Format>],
    layout: FieldLayout,
) -> io::Result<()> {
    write!(w, "case {index}: ")?;
    with_block(w, Newlines::BOTH, |w| {
        for field in fields {
            let ident = sanitize_dart_ident(&field.name);
            write_decode_as_local(w, &ident, &field.value)?;
        }
        write_return_construct(w, variant_class, fields, layout)
    })?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Encode helpers
// ---------------------------------------------------------------------------

/// Emit a single encode statement that writes `value_expr` as a value of type
/// `format` to the `w` [`BincodeWriter`] (recursive for composite types).
fn write_encode(
    w: &mut dyn IndentWrite,
    value_expr: &str,
    format: &Format,
) -> io::Result<()> {
    match format {
        Format::TypeName(_) => writeln!(w, "{value_expr}.bincodeEncode(w);"),
        // English: Rust `()` → Dart `Null`. Bincode's unit type has ZERO bytes
        // (it carries no information), so no write needed.
        // 中文:Rust `()` → Dart `Null`。bincode 的 unit 类型不占字节,无需写入。
        Format::Unit => writeln!(w, "// unit: no bytes"),
        Format::Bool => writeln!(w, "w.writeBool({value_expr});"),
        Format::I8 => writeln!(w, "w.writeI8({value_expr});"),
        Format::I16 => writeln!(w, "w.writeI16({value_expr});"),
        Format::I32 => writeln!(w, "w.writeI32({value_expr});"),
        Format::I64 => writeln!(w, "w.writeI64({value_expr});"),
        // English: Rust i128/u128 → Dart BigInt (see emitter Format::I128 mapping)
        // 中文:Rust i128/u128 → Dart BigInt(见 emitter 的 Format::I128 映射)
        Format::I128 => writeln!(w, "w.writeI128({value_expr});"),
        Format::U8 => writeln!(w, "w.writeU8({value_expr});"),
        Format::U16 => writeln!(w, "w.writeU16({value_expr});"),
        Format::U32 => writeln!(w, "w.writeU32({value_expr});"),
        Format::U64 => writeln!(w, "w.writeU64({value_expr});"),
        Format::U128 => writeln!(w, "w.writeU128({value_expr});"),
        Format::F32 => writeln!(w, "w.writeF32({value_expr});"),
        Format::F64 => writeln!(w, "w.writeF64({value_expr});"),
        // English: Rust `char` → Dart `String` (single-char). d_bincode has a
        // dedicated writeChar for 4-byte u32 rune encoding.
        // 中文:Rust `char` → Dart `String`(单字符)。d_bincode 有专门的
        // writeChar,用 4 字节 u32 rune 编码。
        Format::Char => writeln!(w, "w.writeChar({value_expr});"),
        Format::Str => writeln!(w, "w.writeString({value_expr});"),
        Format::Bytes => {
            // English: Bytes wire format = [u64 len][raw bytes]
            // 中文:Bytes wire format = [u64 长度][原始字节]
            writeln!(w, "w.writeU64({value_expr}.length);")?;
            writeln!(w, "w.writeBytes({value_expr});")
        }
        Format::Option(inner) => {
            // English: Option wire format: [u8: 0] for None, [u8: 1] + bincode(v) for Some.
            // 中文:Option 线格式:None = [u8: 0];Some = [u8: 1] + bincode(v)。
            writeln!(w, "if ({value_expr} != null) {{")?;
            writeln!(w, "    w.writeU8(1);")?;
            // English: Dart null-safety forces us to null-assert inside the if
            // 中文:Dart 空安全强制在 if 里加 null-assert
            let inner_expr = format!("{value_expr}!");
            write_encode_indented(w, 1, &inner_expr, inner)?;
            writeln!(w, "}} else {{")?;
            writeln!(w, "    w.writeU8(0);")?;
            writeln!(w, "}}")
        }
        Format::Seq(inner) | Format::Set(inner) => {
            // English: Vec/Set wire format: [u64 len] + each element inline
            // 中文:Vec/Set 线格式:[u64 长度] + 每个元素内联
            writeln!(w, "w.writeU64({value_expr}.length);")?;
            writeln!(w, "for (final _item in {value_expr}) {{")?;
            write_encode_indented(w, 1, "_item", inner)?;
            writeln!(w, "}}")
        }
        Format::Map { key, value } => {
            // English: Map wire format: [u64 len] + each (key, value) pair inline
            // 中文:Map 线格式:[u64 长度] + 每个 (key, value) 对内联
            writeln!(w, "w.writeU64({value_expr}.length);")?;
            writeln!(
                w,
                "for (final _entry in {value_expr}.entries) {{"
            )?;
            write_encode_indented(w, 1, "_entry.key", key)?;
            write_encode_indented(w, 1, "_entry.value", value)?;
            writeln!(w, "}}")
        }
        Format::Tuple(formats) => {
            // English: Tuple is encoded as `List<dynamic>` at the Dart type level,
            // but the wire format is just the elements concatenated in order
            // (no length prefix, no separators).
            // 中文:tuple 在 Dart 类型层面是 List<dynamic>,线格式是元素顺序拼接
            //(无长度前缀,无分隔符)。
            for (i, fmt) in formats.iter().enumerate() {
                let elem_expr = format!("({value_expr}[{i}])");
                write_encode(w, &elem_expr, fmt)?;
            }
            Ok(())
        }
        Format::TupleArray { content, size } => {
            // English: TupleArray is a fixed-size array — wire format has no
            // length prefix (size is known statically), just the elements.
            // 中文:TupleArray 是固定大小数组,线格式无长度前缀(大小静态已知),
            // 只有元素本身。
            writeln!(w, "// TupleArray: fixed size {size}, no length prefix")?;
            writeln!(
                w,
                "for (var _i = 0; _i < {size}; _i++) {{ final _item = {value_expr}[_i];"
            )?;
            write_encode_indented(w, 1, "_item", content)?;
            writeln!(w, "}}")
        }
        Format::Variable(_) => panic!("unexpected Variable in write_encode"),
    }
}

/// Helper for nested encode calls that need an indent shift (e.g. inside
/// `if (... != null) {{ ... }}` blocks). Indent is handled by the writer so
/// this is just a thin wrapper for clarity — [`IndentWrite`] handles the rest.
fn write_encode_indented(
    w: &mut dyn IndentWrite,
    _levels: usize,
    value_expr: &str,
    format: &Format,
) -> io::Result<()> {
    // English: `IndentWrite` auto-indents newlines at the current indent level.
    // We pass through; the levels arg is reserved for future manual indenting
    // if the helper grows.
    // 中文:`IndentWrite` 会在当前缩进层自动处理换行。levels 参数保留供未来
    // 手工调整缩进使用。
    write_encode(w, value_expr, format)
}

// ---------------------------------------------------------------------------
// Decode helpers
// ---------------------------------------------------------------------------

/// Emit decode code that stores the decoded value into a `final` local named
/// `local_name`. Used when generating struct bodies where we decode each field
/// into a local, then pass them to the constructor.
fn write_decode_as_local(
    w: &mut dyn IndentWrite,
    local_name: &str,
    format: &Format,
) -> io::Result<()> {
    // English: Simple primitive / named types → single-expression `final x = ...;`
    // 中文:原生类型 / 命名类型 → 单表达式 `final x = ...;`
    if is_simple_decode(format) {
        let expr = simple_decode_expr(format);
        return writeln!(w, "final {local_name} = {expr};");
    }
    // English: Composite types — may need multi-statement blocks
    // 中文:复合类型 —— 可能需要多语句块
    match format {
        Format::Option(inner) => {
            // English: Decode as `T? name`. Read tag; if 1, decode inner.
            // 中文:解码为 `T? name`。读 tag;为 1 时解码内部。
            writeln!(w, "final _{local_name}_tag = r.readU8();")?;
            // Use dynamic local until resolved for simplicity; Dart infers type.
            writeln!(w, "final {local_name} = _{local_name}_tag == 1")?;
            writeln!(w, "    ? {}", simple_or_inline_decode_expr(inner))?;
            writeln!(w, "    : null;")
        }
        Format::Seq(inner) | Format::Set(inner) => {
            writeln!(w, "final _{local_name}_len = r.readU64();")?;
            writeln!(
                w,
                "final {local_name} = List.generate(_{local_name}_len, (_) => {});",
                simple_or_inline_decode_expr(inner)
            )
        }
        Format::Map { key, value } => {
            writeln!(w, "final _{local_name}_len = r.readU64();")?;
            writeln!(w, "final {local_name} = {{")?;
            writeln!(
                w,
                "    for (var _i = 0; _i < _{local_name}_len; _i++)"
            )?;
            writeln!(
                w,
                "        {}: {},",
                simple_or_inline_decode_expr(key),
                simple_or_inline_decode_expr(value)
            )?;
            writeln!(w, "}};")
        }
        Format::Tuple(formats) => {
            // English: Decode each element into its own temp, then build a
            // List<dynamic> literal.
            // 中文:每个元素解码到临时变量,然后构造 List<dynamic> 字面量。
            let temps: Vec<String> = (0..formats.len())
                .map(|i| format!("_{local_name}_t{i}"))
                .collect();
            for (i, fmt) in formats.iter().enumerate() {
                writeln!(w, "final {} = {};", temps[i], simple_or_inline_decode_expr(fmt))?;
            }
            writeln!(w, "final {local_name} = <dynamic>[{}];", temps.join(", "))
        }
        Format::TupleArray { content, size } => {
            writeln!(
                w,
                "final {local_name} = List.generate({size}, (_) => {});",
                simple_or_inline_decode_expr(content)
            )
        }
        Format::Variable(_) => panic!("unexpected Variable in write_decode_as_local"),
        // English: All "simple" formats (primitives + TypeName) are handled
        // by the early-return `if is_simple_decode(format)` above. This arm
        // is unreachable but required for Rust's exhaustiveness checker.
        // 中文:所有"简单"类型(primitive + TypeName)都在顶上的
        // `if is_simple_decode(format)` 早退分支处理。此分支不可达,
        // 只是满足 Rust 穷尽性检查。
        _ => unreachable!("simple types handled by early-return above"),
    }
}

/// Returns `true` iff the format can be decoded as a single expression
/// (no block statements needed). These are primitives and named types.
const fn is_simple_decode(format: &Format) -> bool {
    matches!(
        format,
        Format::TypeName(_)
            | Format::Unit
            | Format::Bool
            | Format::I8
            | Format::I16
            | Format::I32
            | Format::I64
            | Format::I128
            | Format::U8
            | Format::U16
            | Format::U32
            | Format::U64
            | Format::U128
            | Format::F32
            | Format::F64
            | Format::Char
            | Format::Str
            | Format::Bytes
    )
}

/// Returns a Dart expression that decodes a primitive or named type from `r`.
fn simple_decode_expr(format: &Format) -> String {
    match format {
        Format::TypeName(qn) => {
            let name = qn.format(
                heck::ToUpperCamelCase::to_upper_camel_case,
                ".",
            );
            format!("{name}.bincodeDecode(r)")
        }
        Format::Unit => "null".to_string(), // Rust `()` → Dart `null` of type Null
        Format::Bool => "r.readBool()".to_string(),
        Format::I8 => "r.readI8()".to_string(),
        Format::I16 => "r.readI16()".to_string(),
        Format::I32 => "r.readI32()".to_string(),
        Format::I64 => "r.readI64()".to_string(),
        Format::I128 => "r.readI128()".to_string(),
        Format::U8 => "r.readU8()".to_string(),
        Format::U16 => "r.readU16()".to_string(),
        Format::U32 => "r.readU32()".to_string(),
        Format::U64 => "r.readU64()".to_string(),
        Format::U128 => "r.readU128()".to_string(),
        Format::F32 => "r.readF32()".to_string(),
        Format::F64 => "r.readF64()".to_string(),
        Format::Char => "r.readChar()".to_string(),
        Format::Str => "r.readString()".to_string(),
        Format::Bytes => {
            // English: Bytes = [u64 len] + raw bytes. d_bincode's readBytes
            // takes a length argument, so we wrap in an IIFE.
            // 中文:Bytes = [u64 长度] + 原始字节。d_bincode 的 readBytes
            // 接受长度参数,用 IIFE 包装。
            "(() { final _l = r.readU64(); return r.readBytes(_l); })()"
                .to_string()
        }
        _ => panic!("simple_decode_expr called with composite format"),
    }
}

/// Returns a Dart expression that decodes a value of `format`, suitable for
/// use inline in another expression (e.g. inside `List.generate` or `? :`).
/// For simple types this is `simple_decode_expr`. For composite types it's
/// a self-contained IIFE.
fn simple_or_inline_decode_expr(format: &Format) -> String {
    if is_simple_decode(format) {
        return simple_decode_expr(format);
    }
    // English: Wrap composite decode in an IIFE so it can be used in an
    // expression position. This is slightly less efficient than the block
    // form but much cleaner to embed.
    // 中文:用 IIFE 包装复合 decode,使其能用于表达式位置。比块形式略低效
    // 但嵌入更干净。
    match format {
        Format::Option(inner) => {
            format!(
                "(() {{ final _t = r.readU8(); return _t == 1 ? {} : null; }})()",
                simple_or_inline_decode_expr(inner)
            )
        }
        Format::Seq(inner) | Format::Set(inner) => {
            format!(
                "(() {{ final _l = r.readU64(); return List.generate(_l, (_) => {}); }})()",
                simple_or_inline_decode_expr(inner)
            )
        }
        Format::Map { key, value } => {
            format!(
                "(() {{ final _l = r.readU64(); return {{ for (var _i = 0; _i < _l; _i++) {}: {} }}; }})()",
                simple_or_inline_decode_expr(key),
                simple_or_inline_decode_expr(value)
            )
        }
        Format::Tuple(formats) => {
            let elems: Vec<String> =
                formats.iter().map(simple_or_inline_decode_expr).collect();
            format!("<dynamic>[{}]", elems.join(", "))
        }
        Format::TupleArray { content, size } => format!(
            "List.generate({size}, (_) => {})",
            simple_or_inline_decode_expr(content)
        ),
        _ => panic!("simple_or_inline_decode_expr: unexpected format"),
    }
}

// ---------------------------------------------------------------------------
// Construction helpers
// ---------------------------------------------------------------------------

/// Write `return Foo(args);` (or the equivalent for named parameters).
/// Used at the end of `bincodeDecode` to construct the class from locals.
fn write_construct_call(
    w: &mut dyn IndentWrite,
    class_name: &str,
    fields: &[Named<Format>],
    layout: FieldLayout,
) -> io::Result<()> {
    write_return_construct(w, class_name, fields, layout)
}

/// Write `return Foo(args);` with parameter style determined by `layout`.
fn write_return_construct(
    w: &mut dyn IndentWrite,
    class_name: &str,
    fields: &[Named<Format>],
    layout: FieldLayout,
) -> io::Result<()> {
    match layout {
        FieldLayout::Unit => writeln!(w, "return const {class_name}();"),
        FieldLayout::Positional => {
            let args: Vec<String> = fields
                .iter()
                .map(|f| sanitize_dart_ident(&f.name))
                .collect();
            writeln!(w, "return {class_name}({});", args.join(", "))
        }
        FieldLayout::Named => {
            let args: Vec<String> = fields
                .iter()
                .map(|f| {
                    let ident = sanitize_dart_ident(&f.name);
                    format!("{ident}: {ident}")
                })
                .collect();
            writeln!(w, "return {class_name}({});", args.join(", "))
        }
    }
}
