//! Architecture test: the geometry stack is layered, and the layering is real.
//!
//! `packages/ifc/ifc-geometry/tests/no_backend_dependency.rs` guards the
//! IFC-to-geometry seam from the IFC side. It says nothing about the geometry
//! side, so until this file existed the internal layering of
//! `packages/geometry/` was documented in `AGENTS.md` and enforced by nobody.
//!
//! Two invariants live here.
//!
//! # 1. Geometry never depends on IFC (the direction of the seam)
//!
//! The whole premise is that the geometry stack is format-agnostic and could be
//! consumed by a STEP-CAD or CityGML front end. One `ifc-model` dependency in
//! any crate under `packages/geometry/` silently ends that, and it would end it
//! without breaking a build, because the workspace makes the crate reachable.
//!
//! # 2. Data flows up the tiers, never down
//!
//! ```text
//!   L0  math / data      geom-core                      (no siblings at all)
//!    ^
//!   L1  representation   geom-mesh, geom-profile, geom-curve,
//!                        geom-surface, geom-topology, geom-primitive,
//!                        geom-model
//!    ^
//!   L2  algorithms       geom-sweep, geom-tessellate, geom-spatial,
//!                        geom-measure, geom-heal, geom-kernel
//!    ^
//!   L3  implementations  geom-backend-cpu, geom-backend-gpu
//!    ^
//!   L4  facade           geom
//! ```
//!
//! A crate may depend on a lower tier or on its own tier (`geom-surface` needs
//! `geom-curve` for trimming; both are representation). It may never depend on
//! a higher one. The rule that matters most is the L1 one: representation types
//! must stay usable without dragging in an algorithm crate, because that is what
//! lets a foreign kernel accept our `TriMesh` without accepting our kernel.
//!
//! `geom-core` is special-cased to zero geometry dependencies. It is the shared
//! vocabulary; the moment it depends on a sibling, the siblings stop being
//! siblings and the backends stop being swappable for one another.

use std::collections::BTreeSet;
use std::path::PathBuf;

/// Tier of each crate under `packages/geometry/`, low to high.
///
/// This list is the specification. A crate missing from it fails
/// [`every_geometry_crate_has_a_declared_tier`], which forces the question
/// "which layer is this?" to be answered when the crate is created rather than
/// inferred by whoever reads the dependency graph later.
const TIERS: &[(&str, u8)] = &[
    // L0 -- data and math only. No algorithms, no backends.
    ("geom-core", 0),
    // L1 -- geometric representation. Types, evaluation, no solving.
    ("geom-mesh", 1),
    ("geom-profile", 1),
    ("geom-curve", 1),
    ("geom-surface", 1),
    ("geom-topology", 1),
    ("geom-primitive", 1),
    // `geom-model` is the format-neutral item graph every front end lowers into
    // and every kernel consumes. It sits at the top of L1 because it is a
    // representation that composes the other representations -- it solves
    // nothing itself.
    ("geom-model", 1),
    // L2 -- algorithms over those representations, plus the backend contract.
    ("geom-sweep", 2),
    ("geom-tessellate", 2),
    ("geom-spatial", 2),
    ("geom-measure", 2),
    ("geom-heal", 2),
    ("geom-kernel", 2),
    // L3 -- concrete runtime implementations.
    ("geom-backend-cpu", 3),
    ("geom-backend-gpu", 3),
    // L4 -- opt-in facade over lower layers.
    ("geom", 4),
];

fn geometry_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..")
}

fn ifc_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../ifc")
}

/// Crate directory names actually present under `packages/geometry/`.
fn geometry_crates() -> BTreeSet<String> {
    let mut found = BTreeSet::new();
    for entry in std::fs::read_dir(geometry_dir()).expect("packages/geometry must exist") {
        let path = entry.unwrap().path();
        if path.join("Cargo.toml").exists() {
            found.insert(path.file_name().unwrap().to_string_lossy().to_string());
        }
    }
    found
}

/// Crate directory names present under `packages/ifc/`, i.e. everything this
/// layer is forbidden to know about. Read from disk rather than hardcoded so a
/// newly added IFC crate is covered the day it appears.
fn ifc_crates() -> BTreeSet<String> {
    let mut found = BTreeSet::new();
    for entry in std::fs::read_dir(ifc_dir()).expect("packages/ifc must exist") {
        let path = entry.unwrap().path();
        if path.join("Cargo.toml").exists() {
            found.insert(path.file_name().unwrap().to_string_lossy().to_string());
        }
    }
    found
}

/// Every dependency name declared in a manifest, from any dependency table.
///
/// `dev-dependencies` count deliberately: a test that reaches up a tier proves
/// the layering is not real in practice, whatever the release graph says.
/// Comments are stripped so a `# never depend on ifc-model` note is not a hit.
fn declared_dependencies(manifest: &str) -> BTreeSet<String> {
    let mut deps = BTreeSet::new();
    let mut in_deps = false;
    for raw in manifest.lines() {
        let line = raw.split('#').next().unwrap_or("").trim();
        if line.starts_with('[') {
            // `[dependencies]`, `[dev-dependencies]`, `[target.'..'.dependencies]`
            in_deps = line.trim_end_matches(']').ends_with("dependencies");
            continue;
        }
        if !in_deps || line.is_empty() {
            continue;
        }
        // `geom-mesh.workspace = true` and `geom-kernel = { .. }` both start
        // with the crate name; take the token before `=`, `.` or whitespace.
        let name: String = line
            .chars()
            .take_while(|c| c.is_alphanumeric() || *c == '-' || *c == '_')
            .collect();
        if !name.is_empty() {
            deps.insert(name);
        }
    }
    deps
}

fn manifest_of(crate_name: &str) -> String {
    let path = geometry_dir().join(crate_name).join("Cargo.toml");
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

fn tier_of(crate_name: &str) -> Option<u8> {
    TIERS
        .iter()
        .find(|(n, _)| *n == crate_name)
        .map(|(_, t)| *t)
}

/// The tier table must describe the directory, not a remembered version of it.
/// Without this, adding a crate would silently opt it out of every rule below.
#[test]
fn every_geometry_crate_has_a_declared_tier() {
    let on_disk = geometry_crates();
    let declared: BTreeSet<String> = TIERS.iter().map(|(n, _)| n.to_string()).collect();

    let untiered: Vec<&String> = on_disk.difference(&declared).collect();
    assert!(
        untiered.is_empty(),
        "packages/geometry crates with no tier in TIERS: {untiered:?}\n\
         Add each to TIERS in this file with the layer it belongs to:\n\
         \t0 = math/data, 1 = representation, 2 = algorithms/contract,\n\
         \t3 = backend implementation, 4 = facade.\n\
         A crate outside the table is exempt from the layering rules, which is \
         how a layered design quietly stops being one."
    );

    let missing: Vec<&String> = declared.difference(&on_disk).collect();
    assert!(
        missing.is_empty(),
        "TIERS names crates that do not exist: {missing:?}\n\
         A stale entry lets a renamed crate escape the layering checks."
    );
}

/// Invariant 1: the seam points one way.
#[test]
fn geometry_never_depends_on_ifc() {
    let forbidden = ifc_crates();
    assert!(
        forbidden.len() >= 10,
        "expected to discover the packages/ifc crates, saw {}",
        forbidden.len()
    );

    for name in geometry_crates() {
        for dep in declared_dependencies(&manifest_of(&name)) {
            assert!(
                !forbidden.contains(&dep),
                "packages/geometry/{name} depends on `{dep}`, an IFC crate.\n\
                 The geometry stack is format-agnostic on purpose: it is what a \
                 STEP-CAD or CityGML front end would reuse, and what makes the \
                 kernel extractable into its own workspace. IFC semantics belong \
                 in packages/ifc/ifc-geometry, which translates INTO these types."
            );
        }
    }
}

/// Invariant 2: dependencies point down the stack or sideways, never up.
#[test]
fn geometry_dependencies_never_point_up_a_tier() {
    for name in geometry_crates() {
        let Some(tier) = tier_of(&name) else { continue };
        for dep in declared_dependencies(&manifest_of(&name)) {
            let Some(dep_tier) = tier_of(&dep) else {
                continue;
            };
            assert!(
                dep_tier <= tier,
                "packages/geometry/{name} (tier {tier}) depends on `{dep}` \
                 (tier {dep_tier}).\n\
                 Geometry is layered math -> representation -> algorithms. A \
                 representation crate that depends on an algorithm crate forces \
                 every consumer of the TYPES to compile the SOLVERS, which is \
                 exactly the coupling that makes a kernel unswappable."
            );
        }
    }
}

/// `geom-core` is the shared vocabulary. It gets no geometry dependencies at
/// all, not even same-tier ones, because there is no lower tier to escape to.
#[test]
fn geom_core_has_no_geometry_dependencies() {
    for dep in declared_dependencies(&manifest_of("geom-core")) {
        assert!(
            !dep.starts_with("geom-"),
            "geom-core depends on `{dep}`.\n\
             geom-core is the root: every other crate and every backend names \
             its types. If it depends on a sibling, that sibling is dragged into \
             everything and the backends stop being true alternatives."
        );
    }
}
