//! Architecture gates for the IFC/geometry swap boundary.
//!
//! IFC adapters may depend on format-neutral representation and contract crates,
//! but CPU/GPU execution and adapter crates are application choices.

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
            !body.contains("geom-backend-"),
            "{} binds IFC semantics to a geometry execution/adapter crate. Depend on geom-model or \
             geom-kernel contracts and select execution contexts/providers in an app crate.",
            manifest.display()
        );
        checked += 1;
    }
    assert!(
        checked >= 7,
        "expected every packages/ifc crate, saw {checked}"
    );
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
