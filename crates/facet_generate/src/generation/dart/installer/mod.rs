//! Project scaffolding — writes a ready-to-build Dart package to disk.
//!
//! The [`Installer`] is the final stage of the Dart generation pipeline.
//! While [`DartCodeGenerator`] produces the *contents* of a single source file,
//! the installer is responsible for the surrounding project structure:
//!
//! 1. **Per-module source files** — splits the registry by namespace (via
//!    [`module::split`]) and calls [`DartCodeGenerator`] once per namespace,
//!    writing each to its own `lib/<namespace>.dart` file. Dart's package
//!    layout requires source files to live under `lib/` so they can be
//!    referenced via `import 'package:<package_name>/<module>.dart'`.
//!
//! 2. **`pubspec.yaml`** — generates a Dart pubspec manifest with dependencies
//!    (external packages as `path:` references for local paths or as
//!    versioned hosted entries for URL locations).
//!
//! # No runtime files
//!
//! Unlike the TypeScript installer (which copies `serde` and `bincode`
//! runtime sources alongside the generated code), the Dart installer
//! **does not install any runtime files**. The Dart bincode runtime
//! (`d_bincode`) is expected to be vendored into the consuming project
//! separately — see `verification/dart-bincode-compat/README.md` and
//! the Obsidian decision note §13.5 for the reasoning.
//!
//! # English / 中文
//!
//! English: see above.
//! 中文:见英文文档。Dart installer 只负责产出 lib/*.dart + pubspec.yaml,
//! **不**安装任何运行时文件。bincode 运行时 d_bincode 由消费方项目自行 vendor。

use std::{
    fs::{File, create_dir_all},
    io::Write as _,
    path::{Path, PathBuf},
};

use crate::{
    Registry,
    generation::{
        CodeGeneratorConfig, Encoding, Error, ExternalPackage, ExternalPackages, PackageLocation,
        SourceInstaller, dart::DartCodeGenerator, module,
    },
};

/// Installer for generated source files in Dart.
///
/// # Examples
///
/// ```rust,ignore
/// use facet_generate::generation::dart;
///
/// let output_dir = std::path::PathBuf::from("output");
/// let installer = dart::Installer::new("my_package", &output_dir);
/// ```
pub struct Installer {
    package_name: String,
    install_dir: PathBuf,
    external_packages: ExternalPackages,
    encoding: Encoding,
}

impl Installer {
    /// Create a new installer for the given package name and output directory.
    ///
    /// Use the builder methods [`encoding`](Self::encoding) and
    /// [`external_packages`](Self::external_packages) to configure, then call
    /// [`generate`](Self::generate) to produce the output.
    #[must_use]
    pub fn new(package_name: &str, install_dir: impl AsRef<Path>) -> Self {
        Self {
            package_name: package_name.to_string(),
            install_dir: install_dir.as_ref().to_path_buf(),
            external_packages: ExternalPackages::new(),
            encoding: Encoding::default(),
        }
    }

    /// Set the encoding for serialization/deserialization.
    ///
    /// Unlike other language installers, the Dart installer does not install
    /// any runtime files based on the encoding — it only forwards the encoding
    /// to the code generator (which decides whether to emit bincode/JSON
    /// methods on the generated classes). Runtime libraries (`d_bincode`,
    /// `dart:convert`) are the consuming project's responsibility.
    #[must_use]
    pub const fn encoding(mut self, encoding: Encoding) -> Self {
        self.encoding = encoding;
        self
    }

    /// Set external packages to reference.
    #[must_use]
    pub fn external_packages(mut self, packages: &[ExternalPackage]) -> Self {
        self.external_packages = packages
            .iter()
            .map(|d| (d.for_namespace.clone(), d.clone()))
            .collect();
        self
    }

    /// Generate all code for the given registry.
    ///
    /// This method:
    /// 1. Splits the registry by namespace and writes each module to
    ///    `lib/<namespace>.dart`
    /// 2. Writes `pubspec.yaml` at the install root
    ///
    /// **No runtime files are installed** — see the module-level docs.
    ///
    /// # Errors
    ///
    /// Returns an error if any file operation or code generation step fails.
    pub fn generate(mut self, registry: &Registry) -> Result<(), Error> {
        // Split by namespace and install each module
        for (m, module_registry) in module::split(&self.package_name, registry) {
            let config = m.config().clone().with_encoding(self.encoding);
            self.install_module(&config, &module_registry)?;
        }

        // Write the package manifest
        let package_name = self.package_name.clone();
        self.install_manifest(&package_name)?;

        Ok(())
    }

    /// Produce the contents of a `pubspec.yaml` manifest as a YAML string.
    ///
    /// English: Hand-written YAML to avoid pulling in a YAML serde dependency
    /// for what is structurally a very simple file. The format follows Dart's
    /// pubspec specification:
    /// ```yaml
    /// name: my_package
    /// description: ...
    /// version: 0.1.0
    /// environment:
    ///   sdk: ^3.5.0
    /// dependencies:
    ///   shared_types:
    ///     path: ../shared_types
    ///   http: ^1.0.0
    /// ```
    ///
    /// 中文:手写 YAML 以避免引入 YAML serde 依赖(这个文件结构非常简单)。
    /// 格式遵循 Dart pubspec 规范。
    #[must_use]
    pub fn make_manifest(&self, package_name: &str) -> String {
        let mut out = String::new();
        out.push_str(&format!("name: {package_name}\n"));
        out.push_str("description: Generated Dart types from facet-generate.\n");
        out.push_str("version: 0.1.0\n");
        out.push_str("publish_to: 'none'\n");
        out.push('\n');
        out.push_str("environment:\n");
        out.push_str("  sdk: ^3.5.0\n");

        // English: When bincode encoding is active, the generated classes
        // reference `BincodeWriter` / `BincodeReader` from the `d_bincode`
        // package. We emit a path dependency pointing at a sibling `d_bincode`
        // directory under the install root; consumers vendor d_bincode there.
        // 中文:bincode 编码激活时,生成的类会引用 d_bincode 包的 BincodeWriter /
        // BincodeReader。我们在 pubspec 里声明一个 path 依赖,指向安装根目录下的
        // 同级 `d_bincode/` 目录,消费方在那里 vendor d_bincode。
        let need_d_bincode = self.encoding == Encoding::Bincode;

        if !self.external_packages.is_empty() || need_d_bincode {
            out.push('\n');
            out.push_str("dependencies:\n");
            if need_d_bincode {
                out.push_str("  d_bincode:\n");
                out.push_str("    path: ./d_bincode\n");
            }

            // English: Sort by namespace name for stable output (BTreeMap iteration is
            // already sorted, but ExternalPackages is a HashMap-like — be safe).
            // 中文:按 namespace 名排序输出,保证稳定性。
            let mut sorted: Vec<&ExternalPackage> = self.external_packages.values().collect();
            sorted.sort_by(|a, b| a.for_namespace.cmp(&b.for_namespace));

            for ext in sorted {
                let pkg_name = &ext.for_namespace;
                match &ext.location {
                    PackageLocation::Path(path) => {
                        // English: Path dependency uses YAML nested mapping
                        // 中文:路径依赖用嵌套 mapping
                        out.push_str(&format!("  {pkg_name}:\n"));
                        out.push_str(&format!("    path: {path}\n"));
                    }
                    PackageLocation::Url(_url) => {
                        // English: Treat URL packages as hosted (pub.dev) with their
                        // version constraint. Dart's pubspec format for hosted packages
                        // with a simple version constraint uses inline syntax:
                        //   <pkg>: <version>
                        // 中文:URL 包视为 pub.dev 上的 hosted 包,用版本约束
                        let version = ext
                            .version
                            .clone()
                            .unwrap_or_else(|| "any".to_string());
                        out.push_str(&format!("  {pkg_name}: {version}\n"));
                    }
                }
            }
        }

        out
    }
}

impl SourceInstaller for Installer {
    /// Generate a single `.dart` source file for one namespace.
    ///
    /// The file is written as `lib/<namespace>.dart` under the install
    /// directory. Namespaces that correspond to external packages are skipped
    /// — their types are imported rather than generated.
    fn install_module(
        &mut self,
        config: &CodeGeneratorConfig,
        registry: &Registry,
    ) -> Result<(), Error> {
        let skip_module = self.external_packages.contains_key(config.module_name());
        if skip_module {
            return Ok(());
        }
        // English: Dart packages require source files under `lib/` for them
        // to be importable via `package:<name>/<module>.dart`. Create it.
        // 中文:Dart 包要求源码在 `lib/` 下,才能通过 `package:` import 引用。
        let lib_dir = self.install_dir.join("lib");
        create_dir_all(&lib_dir)?;
        let module_name = config.module_name();
        let file_name = lib_dir.join(format!("{module_name}.dart"));
        let mut file = File::create(file_name)?;

        // Update config with external packages from installer
        let mut updated_config = config.clone();
        updated_config.external_packages = self.external_packages.clone();

        let generator = DartCodeGenerator::new(&updated_config);
        generator.output(&mut file, registry)?;

        Ok(())
    }

    /// **No-op for Dart.** The Dart bincode runtime (`d_bincode`) is vendored
    /// into the consuming project separately, not installed alongside generated
    /// code. See module-level docs for the rationale.
    fn install_serde_runtime(&mut self) -> Result<(), Error> {
        // English: intentional no-op — Dart users vendor d_bincode themselves
        // 中文:故意空实现 —— Dart 用户自己 vendor d_bincode
        Ok(())
    }

    /// **No-op for Dart.** Same reasoning as `install_serde_runtime`.
    fn install_bincode_runtime(&self) -> Result<(), Error> {
        // English: intentional no-op
        // 中文:故意空实现
        Ok(())
    }

    /// Write `pubspec.yaml` to the output directory.
    fn install_manifest(&self, package_name: &str) -> std::result::Result<(), Error> {
        let manifest = self.make_manifest(package_name);

        let manifest_path = self.install_dir.join("pubspec.yaml");
        let mut file = File::create(manifest_path)?;
        file.write_all(manifest.as_bytes())?;

        Ok(())
    }
}

#[cfg(test)]
mod tests;
