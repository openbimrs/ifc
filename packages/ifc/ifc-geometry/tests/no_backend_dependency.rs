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

/// Geometry access is an explicit allowlist, not an accident.
///
/// The original rule was "only `ifc-geometry` may touch geometry." That held
/// when it was the single seam, but `ifc-alignment` (IFC4x3 linear referencing)
/// and `ifc-georef` (map conversion) legitimately need curve and transform
/// types of their own — alignment geometry is deliberately NOT part of the
/// building-shape pipeline, so folding it into `ifc-geometry` would force every
/// building-only consumer to compile clothoid evaluation.
///
/// So the invariant is now: geometry-touching IFC crates are **declared here**.
/// Adding a geometry dependency to any other crate fails this test, which
/// forces the question "does this really belong in the IFC layer?" to be
/// answered deliberately rather than by whoever edited a manifest last.
const MAY_USE_GEOMETRY: &[&str] = &["ifc-geometry", "ifc-alignment", "ifc-georef"];

#[test]
fn geometry_access_is_limited_to_the_allowlist() {
    for entry in std::fs::read_dir(ifc_dir()).expect("ifc dir must exist") {
        let path = entry.unwrap().path();
        let name = path.file_name().unwrap().to_string_lossy().to_string();
        let manifest = path.join("Cargo.toml");
        if !manifest.exists() || MAY_USE_GEOMETRY.contains(&name.as_str()) {
            continue;
        }
        let body = uncommented(&std::fs::read_to_string(&manifest).unwrap());
        assert!(
            !body.contains("geom-"),
            "packages/ifc/{name} depends on geometry but is not in \
             MAY_USE_GEOMETRY.\n\
             The IFC layer stays pure so a consumer doing property, quantity, \
             cost or classification work never compiles the geometry stack. \
             If this crate genuinely needs geometry, add it to the allowlist \
             with a comment saying why."
        );
    }
}

/// The allowlist itself must stay honest: every crate named in it has to exist.
/// A stale entry would silently permit geometry in a crate that was renamed.
#[test]
fn allowlist_names_only_real_crates() {
    for allowed in MAY_USE_GEOMETRY {
        assert!(
            ifc_dir().join(allowed).join("Cargo.toml").exists(),
            "MAY_USE_GEOMETRY names `{allowed}`, which is not a crate under packages/ifc/"
        );
    }
}
