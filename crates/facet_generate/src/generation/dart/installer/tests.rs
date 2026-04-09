//! Snapshot tests for the Dart [`Installer`] — **project scaffolding**.
//!
//! These tests verify the `pubspec.yaml` manifest that the installer
//! generates, and the file layout produced by `install_module`. They cover:
//!
//! - Basic manifest structure: package name, version, environment.
//! - External URL dependencies: hosted `pub.dev` packages with version constraints.
//! - External path dependencies: local path dependencies via the
//!   YAML `path:` key.
//! - Multi-module (namespace) scenarios.
//! - Each namespace becomes a separate `lib/<namespace>.dart` file.

use facet::Facet;

use crate as fg;
use crate::{
    generation::{Encoding, ExternalPackage, PackageLocation, SourceInstaller as _, module::split},
    reflect,
};

use super::Installer;

#[test]
fn manifest_with_bincode_pulls_d_bincode_from_pub_dev() {
    // English: When encoding is Bincode, the generated classes need
    // `BincodeWriter` / `BincodeReader` from the `d_bincode` package on
    // pub.dev. The manifest should declare it as a hosted dependency — no
    // vendoring required.
    // 中文:encoding 为 Bincode 时,生成的类需要 pub.dev 上的 d_bincode 包提供
    // BincodeWriter / BincodeReader。manifest 应声明 hosted 依赖,无需 vendor。
    let package_name = "my-package";
    let install_dir = tempfile::tempdir().unwrap();

    let installer =
        Installer::new(package_name, install_dir.path()).encoding(Encoding::Bincode);

    let manifest = installer.make_manifest(package_name);

    insta::assert_snapshot!(manifest, @r"
    name: my-package
    description: Generated Dart types from facet-generate.
    version: 0.1.0
    publish_to: 'none'

    environment:
      sdk: ^3.5.0

    dependencies:
      d_bincode: ^3.2.0
    ");
}

#[test]
fn simple_manifest() {
    let package_name = "my-package";
    let install_dir = tempfile::tempdir().unwrap();

    let installer = Installer::new(package_name, install_dir.path());

    let manifest = installer.make_manifest(package_name);

    insta::assert_snapshot!(manifest, @"
    name: my-package
    description: Generated Dart types from facet-generate.
    version: 0.1.0
    publish_to: 'none'

    environment:
      sdk: ^3.5.0
    ");
}

#[test]
fn manifest_with_dependencies() {
    let package_name = "my-package";
    let install_dir = tempfile::tempdir().unwrap();

    let external_pkgs = vec![
        ExternalPackage {
            for_namespace: "lodash".to_string(),
            location: PackageLocation::Url("https://registry.npmjs.org/lodash".to_string()),
            module_name: None,
            version: Some("^4.17.21".to_string()),
        },
        ExternalPackage {
            for_namespace: "axios".to_string(),
            location: PackageLocation::Url("https://registry.npmjs.org/axios".to_string()),
            module_name: None,
            version: Some("^1.6.0".to_string()),
        },
    ];

    let installer =
        Installer::new(package_name, install_dir.path()).external_packages(&external_pkgs);

    let manifest = installer.make_manifest(package_name);

    insta::assert_snapshot!(manifest, @"
    name: my-package
    description: Generated Dart types from facet-generate.
    version: 0.1.0
    publish_to: 'none'

    environment:
      sdk: ^3.5.0

    dependencies:
      axios: ^1.6.0
      lodash: ^4.17.21
    ");
}

#[test]
fn manifest_with_local_dependencies() {
    let package_name = "my-package";
    let install_dir = tempfile::tempdir().unwrap();

    let external_pkgs = vec![ExternalPackage {
        for_namespace: "shared-types".to_string(),
        location: PackageLocation::Path("../shared-types".to_string()),
        module_name: None,
        version: None,
    }];

    let installer =
        Installer::new(package_name, install_dir.path()).external_packages(&external_pkgs);

    let manifest = installer.make_manifest(package_name);
    insta::assert_snapshot!(manifest, @"
    name: my-package
    description: Generated Dart types from facet-generate.
    version: 0.1.0
    publish_to: 'none'

    environment:
      sdk: ^3.5.0

    dependencies:
      shared-types:
        path: ../shared-types
    ");
}

#[test]
fn manifest_with_mixed_dependencies() {
    let package_name = "my-package";
    let install_dir = tempfile::tempdir().unwrap();

    let external_pkgs = vec![
        ExternalPackage {
            for_namespace: "lodash".to_string(),
            location: PackageLocation::Url("https://registry.npmjs.org/lodash".to_string()),
            module_name: None,
            version: Some("^4.17.21".to_string()),
        },
        ExternalPackage {
            for_namespace: "shared-types".to_string(),
            location: PackageLocation::Path("../shared-types".to_string()),
            module_name: None,

            version: None,
        },
    ];

    let installer =
        Installer::new(package_name, install_dir.path()).external_packages(&external_pkgs);

    let manifest = installer.make_manifest(package_name);
    insta::assert_snapshot!(manifest, @"
    name: my-package
    description: Generated Dart types from facet-generate.
    version: 0.1.0
    publish_to: 'none'

    environment:
      sdk: ^3.5.0

    dependencies:
      lodash: ^4.17.21
      shared-types:
        path: ../shared-types
    ");
}

#[test]
fn manifest_with_serde_module() {
    #[derive(Facet)]
    struct MyStruct {
        id: u32,
        name: String,
    }

    let registry = reflect!(MyStruct).unwrap();

    let package_name = "my-package";
    let install_dir = tempfile::tempdir().unwrap();

    let mut installer = Installer::new(package_name, install_dir.path());

    for (module, registry) in split(package_name, &registry) {
        installer
            .install_module(module.config(), &registry)
            .unwrap();
    }

    installer.install_serde_runtime().unwrap();

    let manifest = installer.make_manifest(package_name);
    insta::assert_snapshot!(manifest, @"
    name: my-package
    description: Generated Dart types from facet-generate.
    version: 0.1.0
    publish_to: 'none'

    environment:
      sdk: ^3.5.0
    ");
}

#[test]
fn manifest_with_namespaces() {
    #[derive(Facet)]
    #[facet(fg::namespace = "another_module")]
    struct Child {
        name: String,
    }

    #[derive(Facet)]
    struct Root {
        child: Child,
    }

    let registry = reflect!(Root).unwrap();

    let package_name = "my-package";
    let install_dir = tempfile::tempdir().unwrap();
    let mut installer = Installer::new(package_name, install_dir.path());

    for (module, registry) in split(package_name, &registry) {
        installer
            .install_module(module.config(), &registry)
            .unwrap();
    }

    let manifest = installer.make_manifest(package_name);
    insta::assert_snapshot!(manifest, @"
    name: my-package
    description: Generated Dart types from facet-generate.
    version: 0.1.0
    publish_to: 'none'

    environment:
      sdk: ^3.5.0
    ");
}

#[test]
fn manifest_with_external_namespace_dependencies() {
    #[derive(Facet)]
    #[facet(fg::namespace = "external_package")]
    struct Child {
        name: String,
    }

    #[derive(Facet)]
    struct Root {
        child: Child,
    }

    let registry = reflect!(Root).unwrap();

    let package_name = "my-package";
    let install_dir = tempfile::tempdir().unwrap();

    let external_pkgs = vec![ExternalPackage {
        for_namespace: "external_package".to_string(),
        location: PackageLocation::Url("https://registry.npmjs.org/external-package".to_string()),
        module_name: None,
        version: Some("^1.0.0".to_string()),
    }];

    let mut installer =
        Installer::new(package_name, install_dir.path()).external_packages(&external_pkgs);

    for (module, registry) in split(package_name, &registry) {
        installer
            .install_module(module.config(), &registry)
            .unwrap();
    }

    let manifest = installer.make_manifest(package_name);
    insta::assert_snapshot!(manifest, @"
    name: my-package
    description: Generated Dart types from facet-generate.
    version: 0.1.0
    publish_to: 'none'

    environment:
      sdk: ^3.5.0

    dependencies:
      external_package: ^1.0.0
    ");
}

#[test]
fn manifest_with_scoped_package() {
    let package_name = "my-package";
    let install_dir = tempfile::tempdir().unwrap();

    let external_pkgs = vec![ExternalPackage {
        for_namespace: "types".to_string(),
        location: PackageLocation::Url("https://registry.npmjs.org/@types/node".to_string()),
        module_name: None,
        version: Some("^20.0.0".to_string()),
    }];

    let installer =
        Installer::new(package_name, install_dir.path()).external_packages(&external_pkgs);

    let manifest = installer.make_manifest(package_name);
    insta::assert_snapshot!(manifest, @"
    name: my-package
    description: Generated Dart types from facet-generate.
    version: 0.1.0
    publish_to: 'none'

    environment:
      sdk: ^3.5.0

    dependencies:
      types: ^20.0.0
    ");
}
