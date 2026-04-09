//! JSON plugin for Dart — generates `toJson` / `fromJson` methods that
//! match serde's default wire format (externally tagged by default,
//! or adjacently/internally tagged when `#[facet(tag = ..., content = ...)]`
//! attributes are present on the Rust source type).
//!
//! English: Stub only. Full implementation follows Step 3 of the Dart
//! backend roadmap (see Obsidian note §9.4 for the target code shape
//! and §13.5 for the encoding strategy).
//!
//! 中文:当前仅为占位 stub。完整实现见路线 3 的 Step 3(详见 Obsidian
//! 笔记《Crux + Flutter 集成决策》§9.4 目标代码形态,§13.5 编码策略)。
//!
//! Target architecture: mirrors the structure of `bincode/dart.rs`
//! (walk type tree, dispatch per field type) but emits code that builds
//! `Map<String, dynamic>` and calls the Dart constructor from JSON maps,
//! rather than calling into the `d_bincode` API.
//!
//! 目标架构:和 `bincode/dart.rs` 结构同构(遍历类型树,按字段类型分派),
//! 但输出 `Map<String, dynamic>` 构建代码 + 从 JSON map 调用 Dart 构造器,
//! 而不是调用 d_bincode API。
