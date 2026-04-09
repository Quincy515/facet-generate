//! `EmitterPlugin<Dart>` implementation for the [`JsonPlugin`].
//!
//! Generates `toJson` and `fromJson` methods on Dart classes and enums,
//! producing/consuming JSON that matches **serde's default externally tagged
//! format** — so the output round-trips with `serde_json` on the Rust side
//! (or with Crux's `crux_core::bridge::JsonFfiFormat`).
//!
//! # Wire format reference
//!
//! For a plain Rust struct:
//! ```rust
//! struct Foo { name: String, count: u32 }
//! ```
//! JSON:
//! ```json
//! { "name": "alice", "count": 42 }
//! ```
//!
//! For a plain Rust enum (externally tagged by default in serde):
//! ```rust
//! enum Event {
//!     None,                       // unit variant
//!     SetName(String),            // newtype variant
//!     Move { x: i32, y: i32 },    // struct variant
//!     Point(i32, i32),            // tuple variant
//! }
//! ```
//! JSON:
//! ```json
//! "None"                          // unit → bare string
//! { "SetName": "alice" }          // newtype → wrapped value
//! { "Move": { "x": 10, "y": -20 } } // struct variant → wrapped object
//! { "Point": [10, -20] }          // tuple variant → wrapped array
//! ```
//!
//! For `Option<T>`:
//! - `None` → `null`
//! - `Some(v)` → `v` (no wrapper)
//!
//! For `Vec<T>` / `Set<T>`:
//! - JSON array `[a, b, c]`
//!
//! For `Map<String, V>`:
//! - JSON object `{ "k1": v1, "k2": v2 }`
//! - **Map keys must be `String`** for now — non-string keys would require
//!   stringification per serde's behavior (panics if encountered)
//!
//! # Dart side runtime
//!
//! The generated methods produce/consume `Map<String, dynamic>` (and `Object`
//! for sealed enums whose JSON form can be either a `String` or a `Map`).
//! Users feed the result to `jsonEncode` from `dart:convert` to get a JSON
//! string, or feed `jsonDecode` output back into `fromJson`.
//!
//! # Differences from the bincode Dart plugin
//!
//! 1. **Method names**: `toJson` / `fromJson` instead of `bincodeEncode` /
//!    `bincodeDecode`
//! 2. **Variant tag**: variant **name** (string) instead of variant **index** (u32)
//! 3. **Option**: relies on JSON `null` instead of an explicit `[u8 tag]`
//! 4. **No length prefixes**: JSON arrays and objects are self-delimiting
//! 5. **Sealed enum decode** dispatches on the JSON shape — bare string for
//!    unit variants, single-key map for variants with payload — and uses
//!    Dart 3 pattern matching
//!
//! # English / 中文
//!
//! English: See above. Implements EmitterPlugin<Dart> for JsonPlugin to
//! generate toJson/fromJson methods matching serde's default JSON format.
//! 中文:实现 EmitterPlugin<Dart> for JsonPlugin,生成 toJson/fromJson 方法,
//! 匹配 serde 默认的 JSON 格式(externally tagged enum,Option 用 null,
//! Vec 用 array,Map 用 object)。

use std::collections::BTreeMap;
use std::io;

use crate::generation::{
    CodeGeneratorConfig,
    dart::{Dart, FieldLayout, sanitize_dart_ident},
    indent::{IndentWrite, Newlines, with_block},
    plugin::{EmitContext, EmitterPlugin, VariantInfo},
};
use crate::reflection::format::{ContainerFormat, Format, Named, VariantFormat};

use super::JsonPlugin;

// ---------------------------------------------------------------------------
// EmitterPlugin implementation
// ---------------------------------------------------------------------------

impl EmitterPlugin<Dart> for JsonPlugin {
    fn module_helpers(
        &self,
        _w: &mut dyn IndentWrite,
        _config: &CodeGeneratorConfig,
    ) -> io::Result<()> {
        // English: No top-level helpers needed. JSON encoding for the Dart
        // backend uses inline `Map<String, dynamic>` literals and `dart:core`
        // built-ins (List/Map/null) — no runtime library required.
        // 中文:不需要顶层辅助函数。Dart 后端的 JSON 编码用 inline
        // `Map<String, dynamic>` 字面量和 `dart:core` 内置类型,无需运行时库。
        Ok(())
    }

    fn has_type_body(&self, _ctx: &EmitContext) -> bool {
        true
    }

    fn type_body(&self, w: &mut dyn IndentWrite, ctx: &EmitContext) -> io::Result<()> {
        // English: Same dispatch shape as bincode/dart.rs:
        //   1. Variant subclass → emit toJson only (decode is in base)
        //   2. Native enum (all unit variants) → string-based encode/decode
        //   3. Sealed enum (mixed/payload) → abstract toJson + base fromJson dispatch
        //   4. Struct → toJson + fromJson on the class
        // 中文:分派形状和 bincode/dart.rs 一致(4 种)。
        if let Some(variant) = &ctx.variant {
            return write_variant_to_json(w, variant);
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
// Struct body
// ---------------------------------------------------------------------------

/// Emit `toJson` and `factory fromJson` for a plain struct.
fn write_struct_type_body(
    w: &mut dyn IndentWrite,
    name: &str,
    fields: &[Named<Format>],
    layout: FieldLayout,
) -> io::Result<()> {
    // ─── toJson ───────────────────────────────────────────────────────────
    writeln!(w)?;
    write!(w, "Map<String, dynamic> toJson() ")?;
    with_block(w, Newlines::BOTH, |w| {
        writeln!(w, "return {{")?;
        for field in fields {
            // English: JSON wire-format key uses the *original* Rust field name,
            // not the sanitized Dart identifier. This is what serde does.
            // The Dart-side reference uses the sanitized name.
            // 中文:JSON 线格式 key 用 *原始* Rust 字段名,不用 sanitize 后的
            // Dart 标识符 —— 这和 serde 一致。Dart 侧的引用用 sanitize 名。
            let json_key = &field.name;
            let dart_ident = sanitize_dart_ident(&field.name);
            let encode_expr = json_encode_expr(&field.value, &dart_ident);
            writeln!(w, "    '{json_key}': {encode_expr},")?;
        }
        writeln!(w, "}};")
    })?;

    // ─── factory fromJson ─────────────────────────────────────────────────
    writeln!(w)?;
    write!(
        w,
        "factory {name}.fromJson(Map<String, dynamic> json) "
    )?;
    with_block(w, Newlines::BOTH, |w| {
        write_construct_from_json(w, name, fields, layout, "json")
    })?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Variant subclass body (toJson only)
// ---------------------------------------------------------------------------

/// Emit only `toJson` for an enum variant subclass. The corresponding
/// fromJson is inlined in the sealed base class's `fromJson` dispatcher
/// (see [`write_sealed_enum_base_body`]).
fn write_variant_to_json(
    w: &mut dyn IndentWrite,
    variant: &VariantInfo<'_>,
) -> io::Result<()> {
    let variant_name = variant.name;
    let fields = variant.fields;
    let layout = FieldLayout::from_variant(variant.format);

    writeln!(w)?;
    writeln!(w, "@override")?;
    write!(w, "Object toJson() ")?;
    with_block(w, Newlines::BOTH, |w| {
        match layout {
            FieldLayout::Unit => {
                // English: Unit variant → bare string (serde externally tagged convention)
                // 中文:Unit 变体 → 裸字符串(serde externally tagged 约定)
                writeln!(w, "return '{variant_name}';")
            }
            FieldLayout::Positional => {
                if fields.len() == 1 {
                    // English: NewType variant → wrap the single value directly
                    // 中文:NewType 变体 → 直接包装单个值
                    let field = &fields[0];
                    let dart_ident = sanitize_dart_ident(&field.name);
                    let encoded = json_encode_expr(&field.value, &dart_ident);
                    writeln!(w, "return {{'{variant_name}': {encoded}}};")
                } else {
                    // English: Tuple variant → wrap as JSON array
                    // 中文:Tuple 变体 → 包装为 JSON 数组
                    write!(w, "return {{'{variant_name}': [")?;
                    let parts: Vec<String> = fields
                        .iter()
                        .map(|f| {
                            let ident = sanitize_dart_ident(&f.name);
                            json_encode_expr(&f.value, &ident)
                        })
                        .collect();
                    write!(w, "{}", parts.join(", "))?;
                    writeln!(w, "]}};")
                }
            }
            FieldLayout::Named => {
                // English: Struct variant → wrap as JSON object with field names
                // 中文:Struct 变体 → 包装为带字段名的 JSON 对象
                writeln!(w, "return {{'{variant_name}': {{")?;
                for field in fields {
                    let json_key = &field.name;
                    let dart_ident = sanitize_dart_ident(&field.name);
                    let encoded = json_encode_expr(&field.value, &dart_ident);
                    writeln!(w, "    '{json_key}': {encoded},")?;
                }
                writeln!(w, "}}}};")
            }
        }
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
    // ─── toJson ───────────────────────────────────────────────────────────
    writeln!(w)?;
    write!(w, "String toJson() ")?;
    with_block(w, Newlines::BOTH, |w| {
        // English: Dart enum's built-in `name` property returns the variant
        // identifier as a String — exactly what serde externally tagged uses
        // for unit variants.
        // 中文:Dart enum 的内置 `name` 属性返回变体标识符字符串,正好是
        // serde externally tagged 对 unit 变体的格式。
        writeln!(w, "return name;")
    })?;

    // ─── static fromJson ──────────────────────────────────────────────────
    writeln!(w)?;
    write!(w, "static {name} fromJson(String s) ")?;
    with_block(w, Newlines::BOTH, |w| {
        writeln!(w, "for (final v in values) {{")?;
        writeln!(w, "    if (v.name == s) return v;")?;
        writeln!(w, "}}")?;
        writeln!(w, "throw StateError('Unknown {name} variant: $s');")
    })?;
    Ok(())
}

/// Emit the body for a sealed base class of an enum with payload variants.
fn write_sealed_enum_base_body(
    w: &mut dyn IndentWrite,
    name: &str,
    variants: &BTreeMap<u32, Named<VariantFormat>>,
) -> io::Result<()> {
    // ─── abstract toJson ──────────────────────────────────────────────────
    writeln!(w)?;
    // English: Returns `Object` because unit variants emit a `String` while
    // payload variants emit a `Map<String, dynamic>` — `Object` is the
    // common supertype.
    // 中文:返回 `Object`,因为 unit 变体返回 `String`、带 payload 的变体
    // 返回 `Map`,`Object` 是它们的公共父类。
    writeln!(w, "Object toJson();")?;

    // ─── factory fromJson ─────────────────────────────────────────────────
    writeln!(w)?;
    write!(w, "factory {name}.fromJson(Object json) ")?;
    with_block(w, Newlines::BOTH, |w| {
        // Has any unit variants?
        let has_unit = variants
            .values()
            .any(|v| matches!(v.value, VariantFormat::Unit));
        // Has any payload variants?
        let has_payload = variants
            .values()
            .any(|v| !matches!(v.value, VariantFormat::Unit));

        if has_unit {
            // English: Unit variants are bare strings in JSON
            // 中文:Unit 变体在 JSON 里是裸字符串
            writeln!(w, "if (json is String) {{")?;
            writeln!(w, "    switch (json) {{")?;
            for named_variant in variants.values() {
                if matches!(named_variant.value, VariantFormat::Unit) {
                    let vname = &named_variant.name;
                    let variant_class = format!("{name}Variant{vname}");
                    writeln!(
                        w,
                        "        case '{vname}': return const {variant_class}();"
                    )?;
                }
            }
            writeln!(
                w,
                "        default: throw StateError('Unknown {name} unit variant: $json');"
            )?;
            writeln!(w, "    }}")?;
            writeln!(w, "}}")?;
        }

        if has_payload {
            // English: Payload variants are single-key maps in JSON.
            // 中文:带 payload 的变体在 JSON 里是单 key 的 map。
            writeln!(w, "if (json is Map<String, dynamic>) {{")?;
            writeln!(w, "    final entry = json.entries.single;")?;
            writeln!(w, "    switch (entry.key) {{")?;
            for named_variant in variants.values() {
                if matches!(named_variant.value, VariantFormat::Unit) {
                    continue;
                }
                let vname = &named_variant.name;
                let variant_class = format!("{name}Variant{vname}");
                write_payload_case_block(w, vname, &variant_class, &named_variant.value)?;
            }
            writeln!(
                w,
                "        default: throw StateError('Unknown {name} variant: ${{entry.key}}');"
            )?;
            writeln!(w, "    }}")?;
            writeln!(w, "}}")?;
        }

        writeln!(w, "throw FormatException('Unexpected {name} JSON: $json');")
    })?;
    Ok(())
}

/// Emit one `case 'VariantName': { ... }` block inside the sealed enum
/// `fromJson` payload dispatcher. Handles NewType / Tuple / Struct variants.
fn write_payload_case_block(
    w: &mut dyn IndentWrite,
    variant_name: &str,
    variant_class: &str,
    variant_format: &VariantFormat,
) -> io::Result<()> {
    writeln!(w, "        case '{variant_name}': {{")?;
    match variant_format {
        VariantFormat::Unit => unreachable!("unit variants handled separately"),
        VariantFormat::NewType(inner) => {
            // English: NewType payload is a single value directly under the variant key
            // 中文:NewType 的 payload 是变体 key 下直接的单个值
            let decode = json_decode_expr(inner, "entry.value");
            writeln!(w, "            final value = {decode};")?;
            writeln!(w, "            return {variant_class}(value);")?;
        }
        VariantFormat::Tuple(inners) => {
            // English: Tuple payload is a JSON array under the variant key
            // 中文:Tuple 的 payload 是变体 key 下的 JSON 数组
            writeln!(
                w,
                "            final inner = entry.value as List<dynamic>;"
            )?;
            for (i, inner) in inners.iter().enumerate() {
                let elem_expr = format!("inner[{i}]");
                let decode = json_decode_expr(inner, &elem_expr);
                writeln!(w, "            final field{i} = {decode};")?;
            }
            let args: Vec<String> = (0..inners.len()).map(|i| format!("field{i}")).collect();
            writeln!(
                w,
                "            return {variant_class}({});",
                args.join(", ")
            )?;
        }
        VariantFormat::Struct(fields) => {
            // English: Struct payload is a JSON object under the variant key
            // 中文:Struct 的 payload 是变体 key 下的 JSON 对象
            writeln!(
                w,
                "            final inner = entry.value as Map<String, dynamic>;"
            )?;
            for field in fields {
                let json_key = &field.name;
                let dart_ident = sanitize_dart_ident(&field.name);
                let access = format!("inner['{json_key}']");
                let decode = json_decode_expr(&field.value, &access);
                writeln!(w, "            final {dart_ident} = {decode};")?;
            }
            let args: Vec<String> = fields
                .iter()
                .map(|f| {
                    let ident = sanitize_dart_ident(&f.name);
                    format!("{ident}: {ident}")
                })
                .collect();
            writeln!(
                w,
                "            return {variant_class}({});",
                args.join(", ")
            )?;
        }
        VariantFormat::Variable(_) => panic!("unexpected Variable variant"),
    }
    writeln!(w, "        }}")?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Encode expression helper (toJson side)
// ---------------------------------------------------------------------------

/// Returns a Dart expression that converts `value_expr` of type `format` into
/// a JSON-encodable value (suitable for putting into a `Map<String, dynamic>`
/// or `List<dynamic>` literal).
fn json_encode_expr(format: &Format, value_expr: &str) -> String {
    match format {
        // Primitives — JSON-encodable as-is
        Format::Bool
        | Format::I8
        | Format::I16
        | Format::I32
        | Format::I64
        | Format::U8
        | Format::U16
        | Format::U32
        | Format::U64
        | Format::F32
        | Format::F64
        | Format::Char
        | Format::Str => value_expr.to_string(),
        // English: BigInt for i128/u128 — JSON has no native BigInt, encode
        // as string and parse back on decode
        // 中文:i128/u128 用 BigInt —— JSON 没有原生 BigInt,编码为字符串
        Format::I128 | Format::U128 => format!("{value_expr}.toString()"),
        // English: Bytes (Uint8List) → base64 string is the cleanest JSON
        // representation, but serde's default for Vec<u8> is a JSON array of
        // numbers. We follow serde and emit a List<int>.
        // 中文:Bytes (Uint8List) → serde 默认是数字 JSON 数组,我们跟它一致。
        Format::Bytes => format!("{value_expr}.toList()"),
        // English: Unit → null (Rust () serializes as null in serde_json)
        // 中文:Unit → null
        Format::Unit => "null".to_string(),
        // English: TypeName → recursively call .toJson() on the nested object
        // 中文:TypeName → 递归调用 .toJson()
        Format::TypeName(_) => format!("{value_expr}.toJson()"),
        // English: Option<T> → null if null, else encode T (Dart's null
        // propagation handles it). For complex T we use ?. operator.
        // 中文:Option<T> → null 时为 null;否则递归编码 T。
        Format::Option(inner) => {
            let inner_encoded = json_encode_expr(inner, value_expr);
            if inner_encoded == *value_expr {
                // Primitive inner — value can be passed through directly
                value_expr.to_string()
            } else {
                // Composite inner — need null-safe call
                // English: Use a conditional rather than ?.something because
                // some encode patterns (like .toList()) need a non-null receiver.
                // 中文:用条件表达式而非 ?.,因为某些 encode 模式需要非空接收器。
                let non_null = format!("{value_expr}!");
                let non_null_encoded = json_encode_expr(inner, &non_null);
                format!("{value_expr} == null ? null : {non_null_encoded}")
            }
        }
        // English: Vec<T>/Set<T> → if T is JSON-friendly, pass through; else
        // .map((e) => encode(e)).toList()
        // 中文:Vec<T>/Set<T> → T 是 JSON 友好类型时直接传;否则 .map(...).toList()
        Format::Seq(inner) | Format::Set(inner) => {
            if is_json_passthrough(inner) {
                value_expr.to_string()
            } else {
                let inner_encoded = json_encode_expr(inner, "e");
                format!("{value_expr}.map((e) => {inner_encoded}).toList()")
            }
        }
        // English: Map<K, V> → if K is String AND V is JSON-friendly, pass
        // through. Otherwise we need to map values.
        // 中文:Map<K, V> → K 是 String 且 V 友好时直接传;否则需要 map values。
        Format::Map { key, value } => {
            // For now require String keys (serde behavior matches)
            if !matches!(key.as_ref(), Format::Str) {
                panic!(
                    "JSON Dart plugin: only Map<String, V> is supported \
                     (non-string keys would require stringification)"
                );
            }
            if is_json_passthrough(value) {
                value_expr.to_string()
            } else {
                let v_encoded = json_encode_expr(value, "v");
                format!("{value_expr}.map((k, v) => MapEntry(k, {v_encoded}))")
            }
        }
        // English: Tuple → JSON array, encode each element
        // 中文:Tuple → JSON 数组,逐个编码元素
        Format::Tuple(formats) => {
            let parts: Vec<String> = formats
                .iter()
                .enumerate()
                .map(|(i, f)| json_encode_expr(f, &format!("({value_expr})[{i}]")))
                .collect();
            format!("<dynamic>[{}]", parts.join(", "))
        }
        Format::TupleArray { content, .. } => {
            if is_json_passthrough(content) {
                value_expr.to_string()
            } else {
                let inner_encoded = json_encode_expr(content, "e");
                format!("{value_expr}.map((e) => {inner_encoded}).toList()")
            }
        }
        Format::Variable(_) => panic!("unexpected Variable in json_encode_expr"),
    }
}

/// Returns `true` if a value of `format` is already a valid JSON value
/// without any encoding transformation (e.g. `int`, `String`, `bool`, `List<int>`).
fn is_json_passthrough(format: &Format) -> bool {
    match format {
        Format::Bool
        | Format::I8
        | Format::I16
        | Format::I32
        | Format::I64
        | Format::U8
        | Format::U16
        | Format::U32
        | Format::U64
        | Format::F32
        | Format::F64
        | Format::Char
        | Format::Str
        | Format::Bytes => true,
        // BigInt needs stringification
        Format::I128 | Format::U128 => false,
        // Unit always serializes to null — passthrough since `null` is already valid
        Format::Unit => false, // requires explicit "null" expression, not the value itself
        // TypeName needs recursive .toJson()
        Format::TypeName(_) => false,
        // Option of passthrough is also passthrough (Dart null propagates)
        Format::Option(inner) => is_json_passthrough(inner),
        // Vec/Set/Map of passthrough is also passthrough
        Format::Seq(inner) | Format::Set(inner) => is_json_passthrough(inner),
        Format::Map { value, .. } => is_json_passthrough(value),
        // Tuple/TupleArray need explicit construction
        Format::Tuple(_) | Format::TupleArray { .. } => false,
        Format::Variable(_) => false,
    }
}

// ---------------------------------------------------------------------------
// Decode expression helper (fromJson side)
// ---------------------------------------------------------------------------

/// Returns a Dart expression that decodes `json_expr` (a `dynamic` reference
/// to a JSON value, e.g. `json['name']` or `entry.value`) into a value of
/// type `format`.
fn json_decode_expr(format: &Format, json_expr: &str) -> String {
    match format {
        // Primitives — cast directly from dynamic
        Format::Bool => format!("{json_expr} as bool"),
        Format::I8
        | Format::I16
        | Format::I32
        | Format::I64
        | Format::U8
        | Format::U16
        | Format::U32
        | Format::U64 => format!("({json_expr} as num).toInt()"),
        Format::F32 | Format::F64 => format!("({json_expr} as num).toDouble()"),
        Format::Char | Format::Str => format!("{json_expr} as String"),
        // BigInt encoded as string
        Format::I128 | Format::U128 => format!("BigInt.parse({json_expr} as String)"),
        // Bytes encoded as List<int>
        Format::Bytes => format!(
            "Uint8List.fromList(({json_expr} as List<dynamic>).cast<int>())"
        ),
        // Unit → always null
        Format::Unit => "null".to_string(),
        // TypeName → recursive fromJson
        Format::TypeName(qn) => {
            let name = qn.format(
                heck::ToUpperCamelCase::to_upper_camel_case,
                ".",
            );
            format!("{name}.fromJson({json_expr} as Map<String, dynamic>)")
        }
        // Option<T> → null check, else recurse
        Format::Option(inner) => {
            let inner_decoded = json_decode_expr(inner, json_expr);
            // Check if the inner decode itself starts with `null` (Unit case);
            // for normal Option<T>, we wrap with a conditional.
            format!("{json_expr} == null ? null : ({inner_decoded})")
        }
        // Vec<T> / Set<T> → cast to List<dynamic>, map elements
        Format::Seq(inner) | Format::Set(inner) => {
            let elem = json_decode_expr(inner, "_e");
            format!(
                "({json_expr} as List<dynamic>).map((_e) => {elem}).toList()"
            )
        }
        // Map<K, V> → assume K is String, map values
        Format::Map { key, value } => {
            if !matches!(key.as_ref(), Format::Str) {
                panic!(
                    "JSON Dart plugin: only Map<String, V> is supported \
                     (non-string keys would require parsing)"
                );
            }
            let v_decoded = json_decode_expr(value, "_v");
            format!(
                "({json_expr} as Map<String, dynamic>).map((k, _v) => MapEntry(k, {v_decoded}))"
            )
        }
        // Tuple → cast to List<dynamic>, decode each element by index
        Format::Tuple(formats) => {
            let elems: Vec<String> = formats
                .iter()
                .enumerate()
                .map(|(i, f)| {
                    let elem_expr = format!("(_t[{i}])");
                    json_decode_expr(f, &elem_expr)
                })
                .collect();
            format!(
                "(() {{ final _t = {json_expr} as List<dynamic>; return <dynamic>[{}]; }})()",
                elems.join(", ")
            )
        }
        Format::TupleArray { content, .. } => {
            let elem = json_decode_expr(content, "_e");
            format!(
                "({json_expr} as List<dynamic>).map((_e) => {elem}).toList()"
            )
        }
        Format::Variable(_) => panic!("unexpected Variable in json_decode_expr"),
    }
}

// ---------------------------------------------------------------------------
// Construction helper
// ---------------------------------------------------------------------------

/// Write `return Foo(...)` from JSON map field accesses.
fn write_construct_from_json(
    w: &mut dyn IndentWrite,
    class_name: &str,
    fields: &[Named<Format>],
    layout: FieldLayout,
    json_var: &str,
) -> io::Result<()> {
    match layout {
        FieldLayout::Unit => {
            writeln!(w, "return const {class_name}();")
        }
        FieldLayout::Positional => {
            // Decode each field into a local, then call positionally
            for field in fields {
                let json_key = &field.name;
                let dart_ident = sanitize_dart_ident(&field.name);
                let access = format!("{json_var}['{json_key}']");
                let decode = json_decode_expr(&field.value, &access);
                writeln!(w, "final {dart_ident} = {decode};")?;
            }
            let args: Vec<String> = fields
                .iter()
                .map(|f| sanitize_dart_ident(&f.name))
                .collect();
            writeln!(w, "return {class_name}({});", args.join(", "))
        }
        FieldLayout::Named => {
            for field in fields {
                let json_key = &field.name;
                let dart_ident = sanitize_dart_ident(&field.name);
                let access = format!("{json_var}['{json_key}']");
                let decode = json_decode_expr(&field.value, &access);
                writeln!(w, "final {dart_ident} = {decode};")?;
            }
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
