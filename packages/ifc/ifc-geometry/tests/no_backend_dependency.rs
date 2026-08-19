//! Fast local smoke gates for the IFC/geometry swap boundary.
//!
//! IFC adapters may depend on format-neutral representation crates,
//! but CPU/GPU execution and adapter crates are application choices.
//! `ifc-model/tests/package_architecture.rs` is the authoritative,
//! Cargo-metadata-backed boundary gate.

use std::path::PathBuf;

fn ifc_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../packages/ifc")
}

fn uncommented(manifest: &str) -> String {
    manifest
        .lines()
        .map(|line| line.split('#').next().unwrap_or(""))
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn ifc_crates_never_depend_on_geometry_execution_crates() {
    let mut checked = 0;
    for entry in std::fs::read_dir(ifc_dir()).expect("packages/ifc must exist") {
        let manifest = entry.expect("entry").path().join("Cargo.toml");
        if !manifest.exists() {
            continue;
        }
        let body = uncommented(&std::fs::read_to_string(&manifest).expect("manifest readable"));
        assert!(
            !body.contains("geom-backend-") && !body.contains("geom-kernel"),
            "{} binds IFC semantics to a geometry contract/execution crate. Emit neutral geom-model values and select operation providers in an app crate.",
            manifest.display()
        );
        checked += 1;
    }
    assert!(
        checked >= 7,
        "expected every packages/ifc crate, saw {checked}"
    );
}

#[test]
fn active_lowering_does_not_use_legacy_request_vocabulary() {
    fn collect_rs(dir: &std::path::Path, files: &mut Vec<PathBuf>) {
        for entry in std::fs::read_dir(dir).expect("read lower directory") {
            let path = entry.expect("lower entry").path();
            if path.is_dir() {
                collect_rs(&path, files);
            } else if path.extension().and_then(|extension| extension.to_str()) == Some("rs") {
                files.push(path);
            }
        }
    }

    let lower = ifc_dir().join("ifc-geometry/src/lower");
    let mut files = Vec::new();
    collect_rs(&lower, &mut files);
    assert!(!files.is_empty(), "expected active lowering modules");

    let forbidden = [
        "crate::kernel",
        "crate::BooleanOp",
        "crate::CsgShape",
        "crate::Primitive",
        "crate::Profile",
    ];
    let legacy_names = ["BooleanOp", "CsgShape", "Primitive", "Profile"];
    for file in files {
        let source = std::fs::read_to_string(&file).expect("lower module readable");
        let code = source
            .lines()
            .map(|line| line.split("//").next().unwrap_or(""))
            .collect::<Vec<_>>()
            .join("\n");
        for &name in &forbidden {
            assert!(
                !code.contains(name),
                "{} imports legacy pre-DAG value `{name}`; lower directly to geom-model",
                file.display()
            );
        }

        let compact = code
            .chars()
            .filter(|character| !character.is_whitespace())
            .collect::<String>();
        for grouped in compact.split("usecrate::{").skip(1) {
            let imports = grouped.split("};").next().unwrap_or(grouped);
            for item in imports.split(',') {
                let name = item.split("as").next().unwrap_or(item);
                assert!(
                    name != "kernel"
                        && !name.starts_with("kernel::")
                        && !legacy_names.contains(&name),
                    "{} imports legacy pre-DAG value `{name}` through a grouped crate import; lower directly to geom-model",
                    file.display()
                );
            }
        }
    }
}

/// Geometry access is an explicit allowlist, not an accident.
const MAY_USE_GEOMETRY: &[&str] = &["ifc-geometry", "ifc-alignment", "ifc-georef"];

#[test]
fn geometry_access_is_limited_to_the_allowlist() {
    for entry in std::fs::read_dir(ifc_dir()).expect("ifc dir must exist") {
        let path = entry.expect("entry").path();
        let name = path
            .file_name()
            .expect("name")
            .to_string_lossy()
            .to_string();
        let manifest = path.join("Cargo.toml");
        if !manifest.exists() || MAY_USE_GEOMETRY.contains(&name.as_str()) {
            continue;
        }
        let body = uncommented(&std::fs::read_to_string(&manifest).expect("manifest readable"));
        assert!(
            !body.contains("geom-"),
            "packages/ifc/{name} depends on geometry but is not allowlisted. Property, cost, \
             quantity, and classification consumers must not compile geometry accidentally."
        );
    }
}

#[test]
fn allowlist_names_only_real_crates() {
    for allowed in MAY_USE_GEOMETRY {
        assert!(
            ifc_dir().join(allowed).join("Cargo.toml").exists(),
            "MAY_USE_GEOMETRY names missing crate `{allowed}`"
        );
    }
}
