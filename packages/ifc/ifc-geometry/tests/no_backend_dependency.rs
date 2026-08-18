//! Architecture test: the swap boundary is real, not aspirational.
//!
//! The project requirement is that the geometry kernel can be replaced by a
//! better one without touching the IFC layer. That holds only if no crate under
//! `packages/ifc/` binds to a geometry **implementation** — only to the
//! `geom-kernel` contract and the `geom-core`/`geom-mesh` data types.
//!
//! Since the backends became *features* of `geom-kernel` rather than separate
//! crates, the enforceable invariant is precise: any `packages/ifc/` crate
//! depending on `geom-kernel` must do so with `default-features = false`, which
//! compiles the traits and none of the implementations.
//!
//! A comment cannot enforce that; a dependency added in a hurry would silently
//! weld the workspace to one implementation. This test reads the manifests and
//! fails the build if it happens.

use std::path::{Path, PathBuf};

/// The crate that may carry geometry in `packages/ifc/`. Everything else must
/// be free of it entirely.
const GEOMETRY_BRIDGE: &str = "ifc-geometry";

fn ifc_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../packages/ifc")
}

/// Strip comments so a `# NOTE: never enable default features` line is not a hit.
fn uncommented(manifest: &str) -> String {
    manifest
        .lines()
        .map(|l| l.split('#').next().unwrap_or(""))
        .collect::<Vec<_>>()
        .join("\n")
}

/// A `geom-kernel` dependency must explicitly opt out of default features,
/// because the defaults are the scalar and SIMD *implementations*.
fn assert_contract_only(manifest_path: &Path, body: &str) {
    for line in body.lines() {
        let line = line.trim();
        if !line.starts_with("geom-kernel") {
            continue;
        }
        assert!(
            line.contains("default-features = false"),
            "{} depends on geom-kernel WITH default features.\n\
             Defaults pull in the scalar/simd backend implementations. Crates \
             under packages/ifc/ must take the contract only:\n\
             \tgeom-kernel = {{ workspace = true, default-features = false }}\n\
             Binding to an implementation breaks the requirement that the \
             geometry kernel is swappable.",
            manifest_path.display()
        );
    }
}

#[test]
fn ifc_crates_take_the_kernel_contract_without_implementations() {
    let mut checked = 0;
    for entry in std::fs::read_dir(ifc_dir()).expect("packages/ifc must exist") {
        let manifest = entry.unwrap().path().join("Cargo.toml");
        if manifest.exists() {
            let raw = std::fs::read_to_string(&manifest).unwrap();
            assert_contract_only(&manifest, &uncommented(&raw));
            checked += 1;
        }
    }
    assert!(
        checked >= 7,
        "expected to check every packages/ifc crate, saw {checked}"
    );
}

/// Only `ifc-geometry` may touch geometry at all. If another IFC crate grows a
/// geometry dependency, the "pure IFC logic" separation has been lost — and with
/// it the property that a consumer doing property/quantity work compiles no
/// geometry code.
#[test]
fn only_ifc_geometry_touches_geometry() {
    let mut saw_bridge = false;
    for entry in std::fs::read_dir(ifc_dir()).expect("packages/ifc must exist") {
        let path = entry.unwrap().path();
        let name = path.file_name().unwrap().to_string_lossy().to_string();
        let manifest = path.join("Cargo.toml");
        if !manifest.exists() {
            continue;
        }
        if name == GEOMETRY_BRIDGE {
            saw_bridge = true;
            continue;
        }
        let body = uncommented(&std::fs::read_to_string(&manifest).unwrap());
        assert!(
            !body.contains("geom-"),
            "packages/ifc/{name} depends on geometry. Only {GEOMETRY_BRIDGE} may — \
             the rest of the IFC layer is pure IFC logic so consumers doing \
             property/quantity work never compile the geometry stack."
        );
    }
    assert!(
        saw_bridge,
        "{GEOMETRY_BRIDGE} not found — this test's premise has moved"
    );
}
