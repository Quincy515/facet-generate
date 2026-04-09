//! Dart code generation.
//!
//! This module translates a [`Registry`](crate::Registry) of reflected type
//! definitions into idiomatic Dart source code.
//!
//! # Submodules (in pipeline order)
//!
//! 1. **generator** — Top-level orchestrator. [`DartCodeGenerator`](crate::generation::dart::DartCodeGenerator) implements
//!    [`CodeGenerator`](crate::generation::CodeGenerator) to produce a complete Dart source file
//!    from a registry. It resolves qualified type names against the
//!    configuration (external packages, namespaces), and delegates writing to the emitter layer.
//!
//! 2. **emitter** — AST-to-source rendering. Implements
//!    [`Emitter<Dart>`](crate::generation::Emitter) for each AST node type
//!    ([`Module`](crate::generation::module::Module), [`Container`](crate::generation::Container),
//!    `Named<Format>`, `Format`, `Doc`). This is where the Dart language
//!    mapping lives: type aliases, `export class` / `export abstract class` +
//!    variant subclass selection, and `Serializer`/`Deserializer`
//!    interface-based serialize/deserialize method generation. Feature helpers
//!    (`ArrayOfT`, `SetOfT`, etc.) are embedded as `include_bytes!` snippets
//!    and emitted as needed.
//!
//! 3. **installer** — Project scaffolding. [`Installer`](crate::generation::dart::Installer) implements
//!    [`SourceInstaller`](crate::generation::SourceInstaller) to write a ready-to-build
//!    Dart project: it copies serde/bincode runtime sources, splits the
//!    registry by namespace into per-module files, and generates a
//!    `package.json` manifest.

pub use emitter::Dart;
pub use generator::DartCodeGenerator;
pub use installer::Installer;

mod emitter;
mod generator;
mod installer;
