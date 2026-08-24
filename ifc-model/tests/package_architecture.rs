//! Executable dependency boundaries for the IFC package family.
//!
//! These rules are intentionally stricter than "Cargo happens to build". A
//! sibling domain dependency or concrete geometry algorithm may compile today
//! while permanently coupling unrelated capabilities.

use std::collections::{BTreeMap, BTreeSet};

use cargo_metadata::{DependencyKind, Metadata, MetadataCommand, Package};

const GENERIC: &[&str] = &["ifc-model", "ifc-schema"];
const CODECS: &[&str] = &["ifc-step", "ifc-xml"];
const BRIDGES: &[&str] = &["ifc-geometry", "ifc-georef", "ifc-alignment"];
const NEUTRAL_GEOMETRY: &[&str] = &[
    "axiolid-core",
    "axiolid-curve",
    "axiolid-mesh",
    "axiolid-model",
    "axiolid-primitive",
    "axiolid-profile",
    "axiolid-surface",
    "axiolid-topology",
];

fn metadata() -> Metadata {
    MetadataCommand::new()
        .no_deps()
        .exec()
        .expect("cargo metadata must describe the runtime workspace")
}

/// Name of the IFC facade crate. Published as `openbim-ifc` because the short
/// `ifc` name belongs to an unrelated crate on crates.io; its lib target is
/// still `ifc`, so call sites read `use ifc::...`.
const FACADE: &str = "openbim-ifc";

/// The IFC-layer packages, keyed by crate name.
///
/// `packages/` is flat, so membership is selected by NAME, not by parent
/// directory: a directory filter would sweep in the openBIM standard crates,
/// which answer to different rules. The `found 0` assertion at the call site
/// guards against a filter that silently matches nothing.
fn ifc_packages() -> BTreeMap<String, Package> {
    let metadata = metadata();
    let ifc_root = metadata.workspace_root.as_std_path().join("packages/ifc");
    metadata
        .packages
        .into_iter()
        .filter_map(|package| {
            let crate_dir = package.manifest_path.as_std_path().parent()?;
            let under_ifc_group = crate_dir.parent() == Some(ifc_root.as_path());
            let is_ifc_layer = package.name.starts_with("ifc-") || package.name == FACADE;
            (under_ifc_group && is_ifc_layer).then(|| (package.name.to_string(), package))
        })
        .collect()
}

fn production_dependencies(package: &Package) -> BTreeSet<String> {
    package
        .dependencies
        .iter()
        .filter(|dependency| dependency.kind != DependencyKind::Development)
        .map(|dependency| dependency.name.to_string())
        .collect()
}

fn is_ifc_crate(name: &str, known: &BTreeMap<String, Package>) -> bool {
    known.contains_key(name)
}

#[test]
fn dependencies_follow_the_ifc_layers() {
    let packages = ifc_packages();
    assert!(
        packages.len() >= 18,
        "expected the complete IFC package family, found {} crates",
        packages.len()
    );

    let mut violations = Vec::new();
    for (krate, package) in &packages {
        let dependencies = production_dependencies(package);
        for dependency in dependencies {
            if dependency.starts_with("axiolid-") || dependency == "axiolid" {
                if !BRIDGES.contains(&krate.as_str()) {
                    violations.push(format!(
                        "{krate} is semantic/infrastructure code but depends on {dependency}"
                    ));
                } else if !NEUTRAL_GEOMETRY.contains(&dependency.as_str()) {
                    violations.push(format!(
                        "{krate} depends on geometry algorithm/kernel/backend {dependency}; IFC bridges may depend only on neutral representations"
                    ));
                }
                continue;
            }

            if !is_ifc_crate(&dependency, &packages) || krate == FACADE {
                continue;
            }

            let allowed = match krate.as_str() {
                "ifc-model" | "ifc-schema" => false,
                "ifc-step" => dependency == "ifc-model",
                "ifc-xml" | "ifc-validate" => GENERIC.contains(&dependency.as_str()),
                _ => GENERIC.contains(&dependency.as_str()),
            };
            if !allowed {
                violations.push(format!(
                    "{krate} -> {dependency} crosses an IFC layer; compose sibling capabilities in the facade/application instead"
                ));
            }
        }
    }

    assert!(
        violations.is_empty(),
        "IFC dependency boundary violations:\n{}",
        violations.join("\n")
    );
}

#[test]
fn facade_is_the_only_production_aggregator() {
    let packages = ifc_packages();
    let facade = production_dependencies(packages.get(FACADE).expect("ifc facade"));
    for required in ["ifc-model", "ifc-step", "ifc-properties", "ifc-geometry"] {
        assert!(facade.contains(required), "{FACADE} lost {required}");
    }

    for codec in CODECS {
        let dependencies = production_dependencies(packages.get(*codec).unwrap());
        assert!(
            dependencies
                .iter()
                .all(|name| !name.starts_with("ifc-") || GENERIC.contains(&name.as_str())),
            "codec {codec} acquired domain knowledge: {dependencies:?}"
        );
    }
}
