//! AST-to-Dart source rendering.
//!
//! This module implements [`Emitter<Dart>`](super::super::Emitter) for
//! each node type in the format AST, turning abstract type descriptions into
//! idiomatic Dart code.
//!
//! # Emitter implementations
//!
//! | AST node | Dart output |
//! |---|---|
//! | [`Module`] | `import` statements, type aliases, feature helpers |
//! | [`Container`] | `export class` or `export abstract class` + variant subclasses |
//! | [`Named<Format>`](Named) | `public` property declaration |
//! | [`Format`] | Inline type expression (`number`, `string`, `Array<T>`, …) |
//! | [`Doc`] | `///` doc comments |
//! | `(Named<VariantFormat>, …)` | Enum variant subclass extending the abstract base |
//!
//! # Dart type mapping
//!
//! The [`Format`] emitter maps Rust/reflection types to Dart equivalents
//! via type aliases — for example `I32` → `number` (via `type int32 = number`),
//! `Str` → `string`, `Seq(T)` → `T[]` (via `type Seq<T> = T[]`),
//! `Option(T)` → `Optional<T>` (i.e. `T | null`), `Map(K,V)` → `Map<K, V>`,
//! tuples → `[A, B]` (via `Tuple<[…]>`), fixed-size arrays → `ListTuple<[T]>`.
//!
//! # Encoding-dependent output
//!
//! The [`Dart`] language tag carries the active [`Encoding`] and a list
//! of [`EmitterPlugin`]s. All encoding-specific behaviour — serialize /
//! deserialize methods and feature helper snippets — is delegated to those
//! plugins. With no plugins (`Encoding::None`), only plain type declarations
//! are emitted.
//!
//! # Plugins
//!
//! - [`BincodePlugin`](crate::generation::bincode::BincodePlugin) supplies
//!   `serialize` / `deserialize` methods and the Bincode feature helpers.
//! - [`JsonPlugin`](crate::generation::json::JsonPlugin) supplies the same
//!   interface for JSON (the Dart Serializer/Deserializer API is
//!   identical for both encodings).
//! - With no plugins (`Encoding::None`), only plain type declarations are
//!   emitted.

use std::{
    collections::BTreeMap,
    io::{Result, Write},
    sync::Arc,
};

use heck::ToUpperCamelCase;

use crate::{
    generation::{
        CodeGeneratorConfig, Container, Emitter, Encoding, PackageLocation,
        bincode::BincodePlugin,
        indent::{IndentWrite, Newlines},
        json::JsonPlugin,
        module::Module,
        plugin::{EmitContext, EmitterPlugin, VariantInfo},
    },
    reflection::format::{ContainerFormat, Doc, Format, Named, VariantFormat},
};

/// Language tag for Dart code generation.
///
/// Carries the active [`Encoding`] (None / Bincode / Json) and the plugin
/// list built from it. Emitter implementations consult the plugins at each
/// extension point; the encoding field is retained for the module-level
/// `Serializer`/`Deserializer` import check.
#[derive(Debug, Clone)]
pub struct Dart {
    pub(crate) plugins: Vec<Arc<dyn EmitterPlugin<Self>>>,
}

impl Dart {
    /// Create a Dart language tag, building the appropriate plugins for
    /// the encoding specified in `config`.
    ///
    /// - [`Encoding::Bincode`] → includes `BincodePlugin`
    /// - [`Encoding::Json`] → includes `JsonPlugin` (generates `toJson` / `fromJson`
    ///   matching serde's default externally tagged JSON format)
    /// - [`Encoding::None`] → no plugins
    #[must_use]
    pub fn new(config: &CodeGeneratorConfig, _registry: &crate::Registry) -> Self {
        let plugins: Vec<Arc<dyn EmitterPlugin<Self>>> = match config.encoding {
            Encoding::Bincode => vec![Arc::new(BincodePlugin)],
            Encoding::Json => vec![Arc::new(JsonPlugin)],
            Encoding::None => vec![],
        };
        Self { plugins }
    }

    /// Access the plugin list.
    #[must_use]
    pub fn plugins(&self) -> &[Arc<dyn EmitterPlugin<Self>>] {
        &self.plugins
    }
}

impl Module {
    // English: dart_serde_import_path() removed — Dart doesn't import a Serializer/Deserializer
    // runtime from an external path; bincode runtime comes from `package:d_bincode/d_bincode.dart`
    // (declared as a pub.dev dep in the installer-generated pubspec.yaml), JSON uses built-in
    // dart:convert. Plugin module_helpers handles the imports.
    // 中文:dart_serde_import_path() 删除 —— Dart 不从外部路径 import Serializer/Deserializer
    // 运行时;bincode 运行时来自 `package:d_bincode/d_bincode.dart`(通过 installer 生成的
    // pubspec.yaml 声明为 pub.dev 依赖),JSON 用内置 dart:convert。Plugin 的 module_helpers
    // 负责相应 import。

    fn dart_namespace_import_path(&self, namespace: &str) -> String {
        self.config().external_packages.get(namespace).map_or_else(
            // English: Sibling modules live in the same `lib/` directory
            // (`lib/<root>.dart` and `lib/<namespace>.dart`), so the import
            // path is just the bare namespace name — no `../` prefix.
            // 中文:同级模块都在 `lib/` 目录下(`lib/<root>.dart` 和
            // `lib/<namespace>.dart`),所以 import 路径就是裸的命名空间名,
            // 不需要 `../` 前缀。
            || namespace.to_string(),
            |path| match &path.location {
                PackageLocation::Path(_) => {
                    let name = &path.for_namespace;
                    path.module_name
                        .as_ref()
                        .map_or_else(|| name.clone(), |mod_name| format!("{name}/{mod_name}"))
                }
                PackageLocation::Url(_) => path.for_namespace.clone(),
            },
        )
    }
}

impl Emitter<Dart> for Module {
    fn write<W: IndentWrite>(&self, w: &mut W, lang: &Dart) -> Result<()> {
        let CodeGeneratorConfig {
            referenced_namespaces,
            used_format_types,
            ..
        } = self.config();

        // English: If any field uses Bytes (Uint8List), we need dart:typed_data
        // 中文:如果有任何字段用到 Bytes,需要 import dart:typed_data
        if used_format_types.iter().any(|t| t == "bytes") {
            writeln!(w, "import 'dart:typed_data';")?;
        }

        // English: Namespace imports — Dart `import 'foo.dart' as Foo;`
        // 中文:跨命名空间 import —— Dart 用 `import 'foo.dart' as Foo;`
        let mut import_paths: BTreeMap<String, String> = BTreeMap::new();
        for namespace in referenced_namespaces {
            let import_path = self.dart_namespace_import_path(namespace);
            import_paths.insert(namespace.to_upper_camel_case(), import_path);
        }
        for (namespace, path) in import_paths {
            writeln!(w, "import '{path}.dart' as {namespace};")?;
        }

        // English: No TYPE_ALIASES — Dart has native int/bool/double/List/Map/T? etc.
        // 中文:不需要 TYPE_ALIASES —— Dart 原生有 int/bool/double/List/Map/T? 等

        // Plugin module helpers (bincode/json runtime imports, feature helpers).
        for plugin in lang.plugins() {
            plugin.module_helpers(w, self.config())?;
        }

        Ok(())
    }
}

impl Emitter<Dart> for Doc {
    fn write<W: IndentWrite>(&self, w: &mut W, _lang: &Dart) -> Result<()> {
        for comment in self.comments() {
            writeln!(w, "/// {comment}")?;
        }

        Ok(())
    }
}

/// Field layout strategy for Dart constructor emission.
///
/// English: Controls whether the generated constructor uses positional or named
/// parameters. Dart allows both, but different Rust container shapes map to
/// different idiomatic choices.
///
/// Marked `pub` so the bincode/json plugins (which generate decode code that
/// invokes the constructor) can pick the matching argument style.
///
/// 中文:控制生成的 Dart 构造器用位置参数还是命名参数。Dart 两种都支持,但
/// 不同的 Rust 容器形状对应不同的 Dart 习惯用法。标记为 `pub` 是为了让
/// bincode/json plugin(生成调用构造器的 decode 代码)能选择匹配的参数风格。
#[derive(Copy, Clone, PartialEq, Eq)]
pub enum FieldLayout {
    /// English: No fields — emits `const Foo();`
    /// 中文:无字段 —— 生成 `const Foo();`
    Unit,
    /// English: Positional parameters — emits `const Foo(this.a, this.b);`
    /// Used for Rust NewType and Tuple structs/variants where field names are
    /// synthetic (`value`, `field0`, `field1`, ...).
    /// 中文:位置参数 —— 生成 `const Foo(this.a, this.b);`
    /// 用于 Rust NewType 和 Tuple 结构体/变体,字段名是合成的。
    Positional,
    /// English: Named parameters — emits `const Foo({required this.a, required this.b});`
    /// Used for Rust Struct variants with real field names.
    /// 中文:命名参数 —— 生成 `const Foo({required this.a, required this.b});`
    /// 用于带真实字段名的 Rust Struct 变体。
    Named,
}

impl FieldLayout {
    /// English: Derive `FieldLayout` from a top-level [`ContainerFormat`].
    /// Useful for plugins that need to know how the container's constructor
    /// will be called.
    /// 中文:从顶层 [`ContainerFormat`] 推导 `FieldLayout`,供需要知道容器
    /// 构造器调用风格的 plugin 使用。
    #[must_use]
    pub fn from_container(fmt: &ContainerFormat) -> Self {
        match fmt {
            ContainerFormat::UnitStruct(_) => Self::Unit,
            ContainerFormat::NewTypeStruct(..) | ContainerFormat::TupleStruct(..) => {
                Self::Positional
            }
            ContainerFormat::Struct(..) => Self::Named,
            // English: Enums don't have a "container" constructor; variant
            // subclasses do, and they should call FieldLayout::from_variant.
            // 中文:enum 本身没有"容器"构造器;变体子类有,应该调用 from_variant。
            ContainerFormat::Enum(..) => Self::Unit,
        }
    }

    /// English: Derive `FieldLayout` from a [`VariantFormat`] (enum variant).
    /// 中文:从 [`VariantFormat`](enum 变体)推导 `FieldLayout`。
    #[must_use]
    pub fn from_variant(fmt: &VariantFormat) -> Self {
        match fmt {
            VariantFormat::Unit => Self::Unit,
            VariantFormat::NewType(_) | VariantFormat::Tuple(_) => Self::Positional,
            VariantFormat::Struct(_) => Self::Named,
            VariantFormat::Variable(_) => panic!("unexpected Variable variant"),
        }
    }
}

impl Emitter<Dart> for Container<'_> {
    fn write<W: IndentWrite>(&self, w: &mut W, lang: &Dart) -> Result<()> {
        let Container {
            name: qualified_name,
            format,
            ..
        } = self;
        let name = &qualified_name.name;

        match format {
            ContainerFormat::UnitStruct(doc) => {
                let ctx = EmitContext::top_level(self);
                output_struct_or_variant(w, &ctx, name, &[], FieldLayout::Unit, doc, lang)
            }
            ContainerFormat::NewTypeStruct(format, doc) => {
                let fields = vec![Named::new(format.as_ref(), "value".to_string())];
                let ctx = EmitContext::top_level(self);
                output_struct_or_variant(w, &ctx, name, &fields, FieldLayout::Positional, doc, lang)
            }
            ContainerFormat::TupleStruct(formats, doc) => {
                let fields: Vec<_> = formats
                    .iter()
                    .enumerate()
                    .map(|(i, f)| Named::new(f, format!("field{i}")))
                    .collect();
                let ctx = EmitContext::top_level(self);
                output_struct_or_variant(w, &ctx, name, &fields, FieldLayout::Positional, doc, lang)
            }
            ContainerFormat::Struct(fields, doc) => {
                let ctx = EmitContext::top_level(self);
                output_struct_or_variant(w, &ctx, name, fields, FieldLayout::Named, doc, lang)
            }
            ContainerFormat::Enum(variants, doc) => {
                output_enum_container(w, self, name, variants, doc, lang)
            }
        }
    }
}

impl Emitter<Dart> for Format {
    fn write<W: IndentWrite>(&self, w: &mut W, lang: &Dart) -> Result<()> {
        match self {
            // English: Qualified type names map to bare Dart class names (namespace
            // handled via `import ... as` prefixes at the Module level)
            // 中文:限定类型名映射到裸 Dart 类名(命名空间通过 Module 级别的 import 前缀处理)
            Self::TypeName(type_) => {
                write!(
                    w,
                    "{}",
                    type_.format(ToUpperCamelCase::to_upper_camel_case, ".")
                )
            }
            // English: Rust `()` maps to Dart `Null` — both are unit types with exactly one
            // value (Rust's `()` and Dart's `null`). `final Null unit;` is semantically exact:
            // the field can only hold `null`, preserving the "no information" meaning of `()`.
            // 中文:Rust 的 `()` 映射到 Dart 的 `Null` —— 两者都是"单值类型"(Rust 的 `()`
            // 和 Dart 的 `null` 各自只有一个值)。`final Null unit;` 语义精确:字段只能持有
            // `null`,保留了 `()` 的"无信息"含义。
            Self::Unit => write!(w, "Null"),
            Self::Bool => write!(w, "bool"),
            // English: Dart int is 64-bit signed; Rust i8..i64 fit cleanly, u8..u32 also fit
            //          u64 may overflow Dart int above 2^63 — accepted limitation in v1
            // 中文:Dart 的 int 是 64-bit 有符号;Rust i8..i64 完整容纳,u8..u32 也完整;
            //       u64 超过 2^63 会溢出——第一版接受这个限制
            Self::I8 | Self::I16 | Self::I32 | Self::I64 => write!(w, "int"),
            Self::U8 | Self::U16 | Self::U32 | Self::U64 => write!(w, "int"),
            // English: i128/u128 require BigInt in Dart (int is only 64-bit)
            // 中文:i128/u128 在 Dart 中需要 BigInt(int 只有 64-bit)
            Self::I128 | Self::U128 => write!(w, "BigInt"),
            Self::F32 | Self::F64 => write!(w, "double"),
            // English: Dart has no char type; use String with single-character convention
            // 中文:Dart 没有 char 类型,用单字符 String 代替
            Self::Char => write!(w, "String"),
            Self::Str => write!(w, "String"),
            // English: Bytes map to Uint8List from dart:typed_data
            // 中文:字节数组映射到 dart:typed_data 的 Uint8List
            Self::Bytes => write!(w, "Uint8List"),

            // English: Dart 3 native nullable syntax (T?).
            // Nested `Option<Option<T>>` cannot be represented in Dart's null
            // semantics: Rust has 3 distinct values (None / Some(None) / Some(Some(v)))
            // but Dart's `T?` only has 2 (null / T). We panic with a clear error
            // directing users to use a custom wrapper type instead.
            // 中文:Dart 3 原生可空语法 (T?)。嵌套 `Option<Option<T>>` 在 Dart
            // 可空语义下无法表达:Rust 有 3 种值,Dart `T?` 只有 2 种。此时 panic
            // 并给出清晰错误,建议用户改用自定义包装类型。
            Self::Option(format) => {
                if matches!(format.as_ref(), Self::Option(_)) {
                    panic!(
                        "Dart backend: nested Option<Option<T>> is not supported \
                         — Rust has 3 distinct values but Dart's `T?` only has 2. \
                         Use a custom wrapper struct if you need to distinguish \
                         `None` from `Some(None)`."
                    );
                }
                format.write(w, lang)?;
                write!(w, "?")
            }
            // English: Vec<T>/Set<T> both map to List<T> for now; Dart's Set<T> has different
            // hashing/ordering semantics that don't round-trip cleanly with bincode/serde
            // 中文:Vec<T>/Set<T> 都映射为 List<T>;Dart 的 Set<T> 哈希/排序语义不同,
            // 和 bincode/serde 往返会丢信息
            Self::Seq(format) | Self::Set(format) => {
                write!(w, "List<")?;
                format.write(w, lang)?;
                write!(w, ">")
            }
            Self::Map { key, value } => {
                write!(w, "Map<")?;
                key.write(w, lang)?;
                write!(w, ", ")?;
                value.write(w, lang)?;
                write!(w, ">")
            }
            // English: Tuples map to List<dynamic>. Dart 3 records exist but lack stable
            // JSON/bincode interop; List<dynamic> is JSON-friendly and works with d_bincode.
            // 中文:tuple 映射为 List<dynamic>。Dart 3 的 records 存在但没有稳定的 JSON/bincode
            // 互操作;List<dynamic> 对 JSON 和 d_bincode 都友好。
            Self::Tuple(_formats) => write!(w, "List<dynamic>"),
            // English: Fixed-size arrays also map to List<T>; the size constraint is
            // documented but not enforced at the type level
            // 中文:固定大小数组也映射为 List<T>;大小约束写在文档里,不在类型层面强制
            Self::TupleArray { content, .. } => {
                write!(w, "List<")?;
                content.write(w, lang)?;
                write!(w, ">")
            }
            Self::Variable(_) => panic!("unexpected value"),
        }
    }
}

impl Emitter<Dart> for Named<Format> {
    fn write<W: IndentWrite>(&self, w: &mut W, lang: &Dart) -> Result<()> {
        // English: Dart field declaration — `final <Type> <name>` (type first, not last like TS)
        // 中文:Dart 字段声明 —— `final <类型> <字段名>`(类型在前,和 TS 相反)
        write!(w, "final ")?;
        self.value.write(w, lang)?;
        write!(w, " {}", sanitize_dart_ident(&self.name))
    }
}

/// Sanitize a field name against Dart reserved words and built-in identifiers.
///
/// English: Dart has a set of reserved words (`class`, `final`, `const`, `is`,
/// `as`, `in`, `switch`, etc.) and built-in identifiers that cannot be used as
/// variable names. If the Rust source type has a field with a name colliding
/// with one of these, we suffix it with `_` to make it a valid Dart identifier.
/// This is a minimal conservative list — Dart's full keyword list is longer
/// but most keywords are not plausible Rust field names.
///
/// Marked `pub(crate)` so the bincode/json plugins can emit matching field
/// references when generating encode/decode method bodies.
///
/// 中文:Dart 有一组保留字(`class`、`final`、`const`、`is`、`as`、`in`、
/// `switch` 等)和内置标识符,不能直接用作变量名。如果 Rust 源类型有字段名
/// 和这些冲突,后缀加 `_` 使其成为合法 Dart 标识符。标记为 `pub(crate)`
/// 以便 bincode/json plugin 在生成 encode/decode 方法体时发出匹配的字段引用。
pub(crate) fn sanitize_dart_ident(name: &str) -> String {
    // Conservative list of Dart reserved words and built-ins that are plausible
    // as Rust field names. Dart identifiers can contain `$` and start with `_`,
    // but an `_` prefix makes the field library-private — not what we want. So
    // we suffix with `_` instead.
    const DART_RESERVED: &[&str] = &[
        "abstract", "as", "assert", "async", "await", "break", "case", "catch",
        "class", "const", "continue", "covariant", "default", "deferred", "do",
        "dynamic", "else", "enum", "export", "extends", "extension", "external",
        "factory", "false", "final", "finally", "for", "function", "get", "hide",
        "if", "implements", "import", "in", "interface", "is", "late", "library",
        "mixin", "new", "null", "of", "on", "operator", "part", "rethrow", "return",
        "sealed", "set", "show", "static", "super", "switch", "sync", "this",
        "throw", "true", "try", "typedef", "var", "void", "while", "with", "yield",
    ];
    if DART_RESERVED.contains(&name) {
        format!("{name}_")
    } else {
        name.to_string()
    }
}

// English: quote_type() removed — it was used to stringify type expressions for
// TypeScript-style `constructor (public a: T, public b: U)` parameter lists.
// Dart's `final <Type> <name>;` field declarations are written directly via
// `Named<Format>::write`, so no intermediate stringification is needed.
// 中文:quote_type() 删除 —— 原本用于把类型表达式序列化成字符串供 TS 风格的
// `constructor (public a: T, public b: U)` 参数列表用。Dart 的 `final <Type> <name>;`
// 字段声明直接通过 `Named<Format>::write` 写入,不需要中间字符串化步骤。

/// Generate a Dart `final class` body.
///
/// English: For variants, emits `final class {Base}Variant{Name} extends {Base}`.
/// For top-level containers, emits `final class {Name}`. Field declarations go
/// first, followed by a `const` constructor whose parameter style is chosen
/// based on [`FieldLayout`]:
/// - `Unit` → `const Foo();`
/// - `Positional` → `const Foo(this.a, this.b);` (for NewType/Tuple)
/// - `Named` → `const Foo({required this.a, required this.b});` (for Struct)
///
/// Variant subclasses extending a sealed base class omit `: super()` since the
/// base class has a const no-arg constructor and Dart's implicit super call works.
///
/// 中文:为 variant 生成 `final class {Base}Variant{Name} extends {Base}`;
/// 为顶层容器生成 `final class {Name}`。字段声明在前,`const` 构造器在后,
/// 构造器参数风格由 [`FieldLayout`] 决定。
#[allow(clippy::too_many_arguments)]
fn output_struct_or_variant<W: IndentWrite>(
    w: &mut W,
    ctx: &EmitContext<'_>,
    name: &str,
    fields: &[Named<Format>],
    layout: FieldLayout,
    doc: &Doc,
    lang: &Dart,
) -> Result<()> {
    let variant_base = ctx.variant.as_ref().map(|v| v.parent_name);

    writeln!(w)?;
    doc.write(w, lang)?;

    // English: Class header — variant subclasses extend their parent sealed class
    // 中文:类头 —— variant 子类继承父 sealed class
    if let Some(base) = variant_base {
        write!(w, "final class {base}Variant{name} extends {base} ")?;
    } else {
        write!(w, "final class {name} ")?;
    }
    let mut w = w.block(Newlines::BOTH)?;

    // English: Field declarations — `final <Type> <name>;` on its own line
    // 中文:字段声明 —— 每个一行 `final <类型> <字段名>;`
    for field in fields {
        field.write(&mut w, lang)?;
        writeln!(w, ";")?;
    }
    // Blank line between fields and constructor for readability
    if !fields.is_empty() {
        writeln!(w)?;
    }

    // English: Const constructor — parameter style depends on FieldLayout
    // 中文:const 构造器 —— 参数风格取决于 FieldLayout
    let class_name: String = if let Some(base) = variant_base {
        format!("{base}Variant{name}")
    } else {
        name.to_string()
    };

    // English: Constructor parameters reference sanitized field names so they
    // match the field declarations above (which also go through sanitize_dart_ident).
    // 中文:构造器参数引用 sanitize 后的字段名,以和上面的字段声明保持一致
    // (字段声明也通过 sanitize_dart_ident 处理)。
    match layout {
        FieldLayout::Unit => {
            writeln!(w, "const {class_name}();")?;
        }
        FieldLayout::Positional => {
            let params: Vec<String> = fields
                .iter()
                .map(|f| format!("this.{}", sanitize_dart_ident(&f.name)))
                .collect();
            writeln!(w, "const {class_name}({});", params.join(", "))?;
        }
        FieldLayout::Named => {
            let params: Vec<String> = fields
                .iter()
                .map(|f| format!("required this.{}", sanitize_dart_ident(&f.name)))
                .collect();
            writeln!(w, "const {class_name}({{{}}});", params.join(", "))?;
        }
    }

    // Plugin type bodies (serialize / deserialize methods — added in Step 5b/5c).
    for plugin in lang.plugins() {
        plugin.type_body(&mut w as &mut dyn IndentWrite, ctx)?;
    }

    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn output_variant<W: IndentWrite>(
    w: &mut W,
    parent: &Container<'_>,
    base: &str,
    index: u32,
    name: &str,
    variant: &VariantFormat,
    doc: &Doc,
    lang: &Dart,
) -> Result<()> {
    // English: Build fields + pick FieldLayout from the variant shape
    // 中文:根据 variant 形状构建字段列表 + 选择 FieldLayout
    let (fields, layout): (Vec<Named<Format>>, FieldLayout) = match variant {
        VariantFormat::Unit => (Vec::new(), FieldLayout::Unit),
        VariantFormat::NewType(format) => (
            vec![Named::new(format.as_ref(), "value".to_string())],
            FieldLayout::Positional,
        ),
        VariantFormat::Tuple(formats) => (
            formats
                .iter()
                .enumerate()
                .map(|(i, f)| Named::new(f, format!("field{i}")))
                .collect(),
            FieldLayout::Positional,
        ),
        VariantFormat::Struct(fields) => (fields.clone(), FieldLayout::Named),
        VariantFormat::Variable(_) => panic!("incorrect value"),
    };

    let variant_info = VariantInfo {
        name,
        index: index as usize,
        format: variant,
        fields: &fields,
        parent_name: base,
    };
    let ctx = EmitContext::for_variant(parent, variant_info);
    output_struct_or_variant(w, &ctx, name, &fields, layout, doc, lang)
}

/// Generate a Dart enum container.
///
/// English: If all variants are unit, emit a native Dart `enum`. Otherwise,
/// emit a `sealed class` with `final class FooVariantX extends Foo` subclasses.
/// Dart 3's sealed class gives exhaustive switch coverage equivalent to
/// Rust's match.
///
/// 中文:如果所有 variant 都是 unit,生成 Dart 原生 `enum`;否则生成
/// `sealed class` + 子类。Dart 3 的 sealed class 提供穷尽性 switch 检查,
/// 等价于 Rust 的 match。
fn output_enum_container<W: IndentWrite>(
    w: &mut W,
    container: &Container<'_>,
    name: &str,
    variants: &BTreeMap<u32, Named<VariantFormat>>,
    doc: &Doc,
    lang: &Dart,
) -> Result<()> {
    // English: Detect if all variants are unit — if so, use Dart's native enum
    // 中文:检测是否所有 variant 都是 unit —— 是则用 Dart 原生 enum
    let all_unit = variants
        .values()
        .all(|v| matches!(v.value, VariantFormat::Unit));

    if all_unit {
        writeln!(w)?;
        doc.write(w, lang)?;
        write!(w, "enum {name} ")?;
        let mut w = w.block(Newlines::BOTH)?;

        // English: Dart enum values — one per line, comma-separated, semicolon after last.
        // Variant names are preserved in PascalCase (Rust convention) for simpler
        // bincode/json mapping; Dart's idiomatic lowerCamelCase could be done in a
        // post-processing pass later.
        // 中文:Dart enum 值,每行一个,逗号分隔,最后一个加分号。变体名保持
        // PascalCase(和 Rust 一致),便于 bincode/json 映射;Dart 习惯的
        // lowerCamelCase 可以作为后处理通道
        let variant_list: Vec<&str> = variants.values().map(|v| v.name.as_str()).collect();
        writeln!(w, "{};", variant_list.join(", "))?;

        // Plugin type bodies (bincodeEncode/Decode, toJson/fromJson — added later).
        let ctx = EmitContext::top_level(container);
        for plugin in lang.plugins() {
            plugin.type_body(&mut w as &mut dyn IndentWrite, &ctx)?;
        }

        return Ok(());
    }

    // English: Mixed or payload variants — use sealed class + final subclasses
    // 中文:混合或带 payload 的 variant —— 用 sealed class + final 子类
    writeln!(w)?;
    doc.write(w, lang)?;
    write!(w, "sealed class {name} ")?;
    {
        let mut w = w.block(Newlines::BOTH)?;
        // English: Const no-arg base constructor for variant subclasses to call
        // 中文:无参 const 基类构造器,供 variant 子类调用
        writeln!(w, "const {name}();")?;

        // Plugin type bodies (abstract serialize + static deserialize switch).
        let ctx = EmitContext::top_level(container);
        for plugin in lang.plugins() {
            plugin.type_body(&mut w as &mut dyn IndentWrite, &ctx)?;
        }
    }
    for (index, variant) in variants {
        output_variant(
            w,
            container,
            name,
            *index,
            &variant.name,
            &variant.value,
            &variant.doc,
            lang,
        )?;
    }

    Ok(())
}

// English: TYPE_ALIASES constant and format_type_aliases() helper removed —
// Dart doesn't need type aliases because it has native `int` / `bool` / `double` /
// `String` / `List<T>` / `Map<K,V>` / `T?` / `Uint8List`. The TypeScript backend
// uses aliases like `type int32 = number` for readability, but in Dart we emit
// the native types directly.
// 中文:TYPE_ALIASES 常量和 format_type_aliases() 辅助函数都删除 —— Dart 有
// 原生的 `int`/`bool`/`double`/`String`/`List<T>`/`Map<K,V>`/`T?`/`Uint8List`,
// 不需要像 TypeScript 那样写 `type int32 = number` 之类的别名来提升可读性。

// English: Three test modules cover the three encoding modes:
// - tests.rs: Encoding::None (pure types, no serialization methods) — Step 3
// - tests_bincode.rs: Encoding::Bincode (DartBincodePlugin output) — Step 5b
// - tests_json.rs: Encoding::Json (DartJsonPlugin output) — Step 5c
// 中文:三个测试模块对应三种编码模式。
#[cfg(test)]
#[path = "tests.rs"]
mod tests;
#[cfg(test)]
mod tests_bincode;
#[cfg(test)]
mod tests_json;
