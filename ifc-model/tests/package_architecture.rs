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
/// The one crate allowed to reach an execution provider (ADR 0004).
const COMPILE_HOST: &str = "ifc-geometry";

/// Execution providers: they compute, rather than represent.
///
/// Listed so the boundary check can tell "geometry vocabulary" from "geometry
/// engine". Adding a crate here widens what `ifc-geometry` may opt into; it
/// does not widen who may depend on it.
const EXECUTION_PROVIDER: &[&str] = &[
    "axiolid-contracts",
    "axiolid-mesh-boolean-boolmesh",
    "axiolid-mesh-compile",
    "axiolid-mesh-compile-contract",
];

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

fn ifc_root(metadata: &Metadata) -> std::path::PathBuf {
    let workspace_root = metadata.workspace_root.as_std_path();
    if workspace_root.join("ifc-model/Cargo.toml").is_file() {
        workspace_root.to_path_buf()
    } else {
        workspace_root.join("packages/ifc")
    }
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
    let ifc_root = ifc_root(&metadata);
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

/// Is `dependency` optional AND reachable only through a non-default feature?
///
/// Walks the feature graph from `default` rather than trusting the name: a
/// feature called `compile` that some default feature happens to enable would
/// still ship the provider to every consumer, which is the thing ADR 0004
/// forbids.
fn optional_behind_compile(package: &Package, dependency: &str) -> bool {
    let is_optional = package
        .dependencies
        .iter()
        .any(|d| d.name == dependency && d.optional);
    if !is_optional {
        return false;
    }

    let mut reachable = BTreeSet::new();
    let mut stack = vec!["default".to_string()];
    while let Some(feature) = stack.pop() {
        if !reachable.insert(feature.clone()) {
            continue;
        }
        for entry in package.features.get(&feature).into_iter().flatten() {
            if !entry.starts_with("dep:") {
                stack.push(entry.clone());
            }
        }
    }

    let enabled_by_default = reachable.iter().any(|feature| {
        package
            .features
            .get(feature)
            .into_iter()
            .flatten()
            .any(|entry| entry == &format!("dep:{dependency}"))
    });

    !enabled_by_default
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
                if EXECUTION_PROVIDER.contains(&dependency.as_str()) {
                    // ADR 0004 admits execution behind one opt-in feature in
                    // one crate. Optional alone is not enough: an optional dep
                    // enabled by a DEFAULT feature still ships to everyone, so
                    // the feature table is what gets checked.
                    if krate != COMPILE_HOST {
                        violations.push(format!(
                            "{krate} depends on execution provider {dependency}; only {COMPILE_HOST} may, behind its opt-in `compile` feature"
                        ));
                    } else if !optional_behind_compile(package, &dependency) {
                        violations.push(format!(
                            "{COMPILE_HOST} must reach {dependency} only as an optional dependency enabled by the non-default `compile` feature"
                        ));
                    }
                    continue;
                }
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

#[test]
fn step_and_express_syntax_live_below_ifc() {
    let metadata = metadata();
    let root = ifc_root(&metadata);
    let packages = ifc_packages();

    for consumer in ["ifc-step", "ifc-schema"] {
        let dependencies = production_dependencies(packages.get(consumer).unwrap());
        assert!(
            dependencies.contains("openbim-step"),
            "{consumer} must consume the generic openbim-step substrate"
        );
    }

    for extracted in ["lexer.rs", "escape.rs", "header.rs", "partition.rs"] {
        assert!(
            !root.join("ifc-step/src").join(extracted).exists(),
            "generic STEP source remains inside IFC: ifc-step/src/{extracted}"
        );
    }

    let express_adapter = std::fs::read_to_string(root.join("ifc-schema/src/express.rs"))
        .expect("ifc-schema EXPRESS adapter");
    assert!(
        express_adapter.contains("openbim_step::express"),
        "ifc-schema must delegate generic EXPRESS parsing to openbim-step"
    );
    assert!(
        !express_adapter.contains("struct Parser"),
        "generic EXPRESS parser implementation remains inside IFC"
    );
}
