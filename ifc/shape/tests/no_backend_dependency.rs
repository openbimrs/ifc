//! Architecture test: the swap boundary is real, not aspirational.
//!
//! The project requirement is that the `geom` package can be replaced by a
//! better one without touching `ifc`. That only holds if no crate under `ifc/`
//! depends on a concrete geometry **backend** — only on the `geom-kernel`
//! contract and `geom-core` data types.
//!
//! A comment cannot enforce that; a dependency added in a hurry would silently
//! weld the workspace to one implementation. This test reads the manifests and
//! fails the build if it happens.

use std::path::{Path, PathBuf};

/// Crates that implement geometry. No `ifc/` crate may depend on these.
const BACKEND_CRATES: &[&str] = &["geom-cpu", "geom-simd", "geom-gpu", "geom-dispatch"];

fn ifc_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("ifc")
}

/// Strip comments so a `# NOTE: geom-cpu must not appear` line is not a hit.
fn uncommented(manifest: &str) -> String {
    manifest
        .lines()
        .map(|l| l.split('#').next().unwrap_or(""))
        .collect::<Vec<_>>()
        .join("\n")
}

fn assert_no_backend_dep(manifest_path: &Path) {
    let raw = std::fs::read_to_string(manifest_path)
        .unwrap_or_else(|e| panic!("cannot read {}: {e}", manifest_path.display()));
    let body = uncommented(&raw);
    for backend in BACKEND_CRATES {
        assert!(
            !body.contains(backend),
            "{} depends on backend crate `{}`.\n\
             Crates under ifc/ must depend only on geom-kernel (traits) and \
             geom-core (data). Depending on a backend breaks the requirement \
             that the geom package is swappable.",
            manifest_path.display(),
            backend
        );
    }
}

#[test]
fn no_ifc_crate_depends_on_a_geometry_backend() {
    let mut checked = 0;
    for entry in std::fs::read_dir(ifc_dir()).expect("ifc/ dir must exist") {
        let manifest = entry.unwrap().path().join("Cargo.toml");
        if manifest.exists() {
            assert_no_backend_dep(&manifest);
            checked += 1;
        }
    }
    assert!(
        checked >= 4,
        "expected to check all ifc/ crates, saw {checked}"
    );
}

/// Only `ifc-shape` may touch geometry at all. If another ifc crate grows a
/// geometry dependency, the "pure IFC logic" separation has been lost.
#[test]
fn only_ifc_shape_touches_geometry() {
    for entry in std::fs::read_dir(ifc_dir()).expect("ifc/ dir must exist") {
        let path = entry.unwrap().path();
        let name = path.file_name().unwrap().to_string_lossy().to_string();
        let manifest = path.join("Cargo.toml");
        if !manifest.exists() || name == "shape" {
            continue;
        }
        let body = uncommented(&std::fs::read_to_string(&manifest).unwrap());
        assert!(
            !body.contains("geom-"),
            "ifc/{name} depends on geometry. Only ifc-shape may — the rest of \
             ifc/ is pure IFC logic so consumers doing property/quantity work \
             never compile the geometry stack."
        );
    }
}
