//! M4: the corpus gate. Every fixture must go through the whole pipeline.
//!
//! This is the test that would have caught the `Profile::Derived` gap: the
//! synthetic graphs in `geom-compile` all passed while every real file failed,
//! because real lowering wraps profiles in their 2D placement.

use ifc_cli_support::{compile_model, compile_products, fixtures, signed_volume};
use ifc_model::Codec;

#[path = "../src/mesh.rs"]
mod mesh_impl;

mod ifc_cli_support {
    pub use super::mesh_impl::{compile_model, compile_products, signed_volume};
    use std::path::PathBuf;

    /// Every committed geometry fixture, sorted for stable reporting.
    pub fn fixtures() -> Vec<PathBuf> {
        let root = PathBuf::from(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../test/fixtures/ifclite-geometry"
        ));
        let mut files: Vec<_> = std::fs::read_dir(&root)
            .expect("fixture directory")
            .filter_map(|entry| entry.ok().map(|entry| entry.path()))
            .filter(|path| path.extension().is_some_and(|ext| ext == "ifc"))
            .collect();
        files.sort();
        files
    }
}

/// No fixture may produce a *compile* failure.
///
/// Lowering gaps are expected and tracked separately: those are unimplemented
/// IFC families, which fail honestly upstream. A compile failure means an item
/// the IFC layer successfully lowered met a provider that could not handle it,
/// which is the gap this milestone exists to close.
#[test]
fn every_lowered_item_in_the_corpus_compiles() {
    let files = fixtures();
    assert!(files.len() >= 11, "expected the committed corpus");

    let mut offenders = Vec::new();
    let mut total_meshed = 0usize;
    for path in &files {
        let Ok(model) = ifc_step::StepCodec.read_path(path) else {
            continue;
        };
        let summary = compile_model(&model, false);
        total_meshed += summary.meshed;
        if summary.not_compiled > 0 {
            offenders.push(format!(
                "{}: {} items lowered but not compiled",
                path.file_name().expect("named").to_string_lossy(),
                summary.not_compiled
            ));
        }
    }
    assert!(offenders.is_empty(), "compile gaps: {offenders:#?}");
    assert!(
        total_meshed >= 40,
        "corpus produced only {total_meshed} meshes; a regression in coverage"
    );
}

/// Signed volume via the divergence theorem.
fn volume(mesh: &geom_mesh::TriMesh) -> f64 {
    mesh.indices
        .chunks_exact(3)
        .map(|t| {
            let (a, b, c) = (
                mesh.positions[t[0] as usize],
                mesh.positions[t[1] as usize],
                mesh.positions[t[2] as usize],
            );
            a.dot(b.cross(c))
        })
        .sum::<f64>()
        / 6.0
}

/// The headline gate: a real IFC wall with three overlapping openings must
/// produce the volume an independent Monte-Carlo integration predicts.
///
/// 2.0807 comes from a 4M-sample integration recorded in ADR 0014, computed
/// without reference to this implementation. The wall is the largest solid in
/// the file, so it is identified by volume rather than by entity id, which
/// would couple the test to the fixture's numbering.
#[test]
fn the_wall_fixture_matches_the_monte_carlo_volume() {
    let path = fixtures()
        .into_iter()
        .find(|p| {
            p.file_name()
                .is_some_and(|n| n == "issue_2019_wall_two_overlapping_openings.ifc")
        })
        .expect("wall fixture");
    let model = ifc_step::StepCodec.read_path(&path).expect("read");
    let summary = compile_model(&model, false);
    assert_eq!(summary.not_compiled, 0);
    assert_eq!(summary.meshed, 4, "wall plus three openings");

    // Every solid must be outward-oriented; a negative volume is the
    // inside-out failure mode that still looks like valid geometry.
    for mesh in &summary.meshes {
        assert!(volume(mesh) > 0.0, "a compiled solid is inside-out");
    }

    // Representation items are the UNCUT solids: the wall's own body is
    // 4 x 0.2 x 3 = 2.4. Voids are a product-level relationship, so the net
    // solid only exists after `IfcRelVoidsElement` is applied.
    let largest = summary
        .meshes
        .iter()
        .map(volume)
        .fold(f64::NEG_INFINITY, f64::max);
    assert!(
        (largest - 2.4).abs() < 1e-9,
        "uncut wall body should be 2.4, got {largest}"
    );

    let products = mesh_impl::compile_products(&model);
    let wall = products
        .iter()
        .find(|p| p.type_name == "IFCWALL")
        .expect("the wall product");
    assert_eq!(wall.voids_applied, 3, "three openings must be cut");
    assert!(
        model.get(wall.id).is_some(),
        "the reported product id must resolve to a record"
    );

    // ADR 0014 records 2.0807 from an independent 4M-sample Monte-Carlo
    // integration, computed without reference to this implementation.
    let net = volume(&wall.mesh);
    assert!(
        (net - 2.0807).abs() < 5e-3,
        "net wall volume {net} disagrees with the Monte-Carlo oracle 2.0807"
    );
}

/// Every produced solid must be edge-manifold and outward-oriented.
///
/// Manifold-in / manifold-out is the ADR 0003 contract. Checking it across the
/// whole corpus is what makes the claim measured rather than asserted: a
/// single-fixture check would not have caught the `Profile::Derived` gap.
#[test]
fn every_produced_solid_is_manifold_and_outward() {
    use std::collections::HashMap;

    let mut checked = 0usize;
    let mut open_surfaces = 0usize;
    for path in fixtures() {
        let Ok(model) = ifc_step::StepCodec.read_path(&path) else {
            continue;
        };
        let name = path
            .file_name()
            .expect("named")
            .to_string_lossy()
            .to_string();
        let summary = compile_model(&model, false);
        for (index, mesh) in summary.meshes.iter().enumerate() {
            // An open surface is not a solid. A faceted brep may declare
            // IFCCLOSEDSHELL yet leave boundary edges; the closure assertions
            // below only describe solids, so classify first and check what
            // actually applies. Skipping silently would let a real regression
            // hide, so an open mesh is still required to be edge-consistent:
            // no directed edge may repeat.
            let open_boundary = {
                let mut directed: HashMap<(u32, u32), i32> = HashMap::new();
                for t in mesh.indices.chunks_exact(3) {
                    for &(a, b) in &[(t[0], t[1]), (t[1], t[2]), (t[2], t[0])] {
                        *directed.entry((a, b)).or_default() += 1;
                    }
                }
                directed
                    .iter()
                    .any(|(&(a, b), _)| directed.get(&(b, a)).copied().unwrap_or(0) == 0)
            };
            if open_boundary {
                let mut directed: HashMap<(u32, u32), i32> = HashMap::new();
                for t in mesh.indices.chunks_exact(3) {
                    for &(a, b) in &[(t[0], t[1]), (t[1], t[2]), (t[2], t[0])] {
                        *directed.entry((a, b)).or_default() += 1;
                    }
                }
                for (&(a, b), &count) in &directed {
                    assert_eq!(
                        count, 1,
                        "{name}[{index}]: directed edge {a}->{b} repeats in an open surface"
                    );
                }
                open_surfaces += 1;
                continue;
            }

            // Directed-edge parity: each directed edge exactly once, and each
            // undirected edge with exactly one opposing half. This catches a
            // flipped cap that the volume integral cannot see when the cap
            // lies in a plane through the origin.
            let mut directed: HashMap<(u32, u32), i32> = HashMap::new();
            for t in mesh.indices.chunks_exact(3) {
                for &(a, b) in &[(t[0], t[1]), (t[1], t[2]), (t[2], t[0])] {
                    *directed.entry((a, b)).or_default() += 1;
                }
            }
            for (&(a, b), &count) in &directed {
                assert_eq!(count, 1, "{name}[{index}]: directed edge {a}->{b} repeats");
                assert_eq!(
                    directed.get(&(b, a)).copied().unwrap_or(0),
                    1,
                    "{name}[{index}]: edge {a}-{b} has no opposing half"
                );
            }
            assert!(volume(mesh) > 0.0, "{name}[{index}]: solid is inside-out");
            checked += 1;
        }
    }
    assert!(checked >= 40, "expected the corpus to produce solids");
    // Pin the split so a future change cannot quietly reclassify closed solids
    // as open surfaces to dodge the manifold assertions above.
    assert_eq!(
        open_surfaces, 12,
        "expected exactly the synthetic open-shell fixture bodies"
    );
}

/// The wall fixture must match the IfcOpenShell reference volume.
///
/// This number was produced by an independent implementation (IfcOpenShell
/// 0.8.5) and cross-checked against a Monte-Carlo integration. It is pinned
/// here so the placement bug that made it 42.107 instead of 32.419 cannot
/// return silently: without `ObjectPlacement` every opening lands at the
/// origin and barely intersects the wall it is meant to cut.
#[test]
fn the_wall_fixture_matches_the_ifcopenshell_reference() {
    let path = fixtures()
        .into_iter()
        .find(|p| p.ends_with("issue_098_wall_W.ifc"))
        .expect("the wall fixture is committed");
    let model = ifc_step::StepCodec.read_path(&path).expect("read");
    let products = compile_products(&model);
    let wall = products
        .iter()
        .find(|p| p.type_name == "IFCWALLSTANDARDCASE")
        .expect("the wall is present");
    let volume = signed_volume(&wall.mesh);
    // IfcOpenShell 0.8.5, same corpus, independently computed.
    const REFERENCE: f64 = 32.419_067_748_586_31;
    let relative = (volume - REFERENCE).abs() / REFERENCE.abs();
    assert!(
        relative < 1e-6,
        "wall volume {volume} disagrees with the IfcOpenShell reference {REFERENCE} (rel {relative:.2e})"
    );
    assert_eq!(wall.voids_applied, 7, "all seven openings must be cut");
}
