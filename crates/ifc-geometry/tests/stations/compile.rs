//! Lowered stations through the reference mesh compiler (#307), each
//! against a closed form. `axiolid-mesh-compile` 0.3.15 resolves the
//! stations; nothing here or in the library evaluates them.
#![cfg(feature = "compile-reference-backend")]

use axiolid_contracts::ExecutionOptions;
use axiolid_core::Tolerance;
use axiolid_mesh::TriMesh;
use axiolid_mesh_compile_contract::{MeshClosure, MeshCompiler};
use ifc_geometry::compile::{compile_product_mesh, default_backend};
use ifc_model::EntityId;

use super::common::{lower, step};

/// A straight 10 m directrix along +X; a product carrying `item` as body.
fn records(item: &str) -> String {
    format!(
        "#10=IFCPOLYLINE((#11,#12));
#11=IFCCARTESIANPOINT((0.,0.,0.));
#12=IFCCARTESIANPOINT((10.,0.,0.));
#40=IFCRECTANGLEPROFILEDEF(.AREA.,'narrow',#43,2.,1.);
#41=IFCRECTANGLEPROFILEDEF(.AREA.,'wide',#43,4.,1.);
#43=IFCAXIS2PLACEMENT2D(#44,$);
#44=IFCCARTESIANPOINT((0.,0.));
#20=IFCAXIS2PLACEMENTLINEAR(#21,$,$);
#21=IFCPOINTBYDISTANCEEXPRESSION(IFCLENGTHMEASURE(0.),1.,0.5,$,#10);
#22=IFCAXIS2PLACEMENTLINEAR(#23,$,$);
#23=IFCPOINTBYDISTANCEEXPRESSION(IFCLENGTHMEASURE(10.),1.,0.5,$,#10);
#24=IFCAXIS2PLACEMENTLINEAR(#25,$,$);
#25=IFCPOINTBYDISTANCEEXPRESSION(IFCLENGTHMEASURE(0.),$,$,$,#10);
#26=IFCAXIS2PLACEMENTLINEAR(#27,$,$);
#27=IFCPOINTBYDISTANCEEXPRESSION(IFCLENGTHMEASURE(10.),$,$,$,#10);
#50=IFCOPENCROSSPROFILEDEF(.CURVE.,$,.T.,(2.,2.),(0.,0.),('r','c','l'),#52);
#51=IFCOPENCROSSPROFILEDEF(.CURVE.,$,.T.,(3.,3.),(3.141592653589793,3.141592653589793),('l','c','r'),#53);
#52=IFCCARTESIANPOINT((-2.,0.));
#53=IFCCARTESIANPOINT((3.,0.));
#60=IFCBUILDINGELEMENTPROXY('1YvctVUKr0kugbFTf53O9L',$,'Deck',$,$,#61,#62,$,$);
#61=IFCLOCALPLACEMENT($,#9);
#62=IFCPRODUCTDEFINITIONSHAPE($,$,(#63));
#63=IFCSHAPEREPRESENTATION(#2,'Body','AdvancedSweptSolid',(#30));
#30={item};"
    )
}

fn signed_volume(mesh: &TriMesh) -> f64 {
    let base = mesh.positions[0];
    mesh.indices
        .chunks_exact(3)
        .map(|t| {
            let [a, b, c] = [t[0], t[1], t[2]].map(|i| mesh.positions[i as usize] - base);
            a.dot(b.cross(c)) / 6.0
        })
        .sum()
}

fn area(mesh: &TriMesh) -> f64 {
    mesh.indices
        .chunks_exact(3)
        .map(|t| {
            let [a, b, c] = [t[0], t[1], t[2]].map(|i| mesh.positions[i as usize]);
            (b - a).cross(c - a).length() / 2.0
        })
        .sum()
}

fn bounds(mesh: &TriMesh) -> ([f64; 3], [f64; 3]) {
    let mut lo = [f64::INFINITY; 3];
    let mut hi = [f64::NEG_INFINITY; 3];
    for p in &mesh.positions {
        for (axis, value) in [p.x, p.y, p.z].into_iter().enumerate() {
            lo[axis] = lo[axis].min(value);
            hi[axis] = hi[axis].max(value);
        }
    }
    (lo, hi)
}

fn close(actual: [f64; 3], expected: [f64; 3], what: &str) {
    assert!(
        actual
            .iter()
            .zip(expected)
            .all(|(a, e)| (a - e).abs() < 1e-9),
        "{what}: {actual:?} != {expected:?}"
    );
}

/// A rectangle 2 m wide at 0 widening to 4 m at 10, both 1 m to the LEFT
/// and 0.5 m up: profile X runs to the left, profile Y up. The walls are
/// planar trapezoids, so the volume is exact: 1 * (2 + 4) / 2 * 10.
#[test]
fn a_sectioned_solid_meshes_left_and_up_with_its_exact_volume() {
    let model = step(
        &records("IFCSECTIONEDSOLIDHORIZONTAL(#10,(#40,#41),(#20,#22))"),
        false,
    );
    let mesh = compile_product_mesh(&model, EntityId(60), Tolerance::MILLIMETRE)
        .unwrap_or_else(|e| panic!("the deck compiles: {e}"))
        .expect("a body");
    let volume = signed_volume(&mesh);
    assert!((volume - 30.0).abs() < 1e-9, "volume {volume}");
    let (lo, hi) = bounds(&mesh);
    close(lo, [0.0, -1.0, 0.0], "min");
    close(hi, [10.0, 3.0, 1.0], "max");
}

/// Tags pair points across sections whatever the order they are authored
/// in: the second section runs left to right, its tags reversed, and the
/// sheet joins edge to edge. Two planar trapezoids, 2 -> 3 m over 10 m.
#[test]
fn a_reversed_open_section_joins_by_tag() {
    let model = step(
        &records("IFCSECTIONEDSURFACE(#10,(#24,#26),(#50,#51))"),
        false,
    );
    let lowered = lower(&model, 30).expect("lowers");
    let options = ExecutionOptions::new(Tolerance::MILLIMETRE);
    let outcome = default_backend()
        .compile_mesh_reported(&lowered.graph, lowered.root, &options)
        .unwrap_or_else(|e| panic!("the sheet compiles: {e:?}"));
    assert_eq!(outcome.closure, MeshClosure::Surface);
    let expected = 2.0 * 0.5 * (2.0 + 3.0) * 10.0;
    let got = area(&outcome.mesh);
    assert!((got - expected).abs() < 1e-6, "area {got} != {expected}");
    let (lo, hi) = bounds(&outcome.mesh);
    close(lo, [0.0, -3.0, 0.0], "min");
    close(hi, [10.0, 3.0, 0.0], "max");
}

// --- #346: seams, through the reference kernel ---------------------------

use super::seams::{item, seams};

/// Points of `mesh` within `EPS` of `at`.
fn has_vertex(mesh: &TriMesh, at: [f64; 3]) -> bool {
    mesh.positions.iter().any(|p| {
        (p.x - at[0]).abs() < 1e-9 && (p.y - at[1]).abs() < 1e-9 && (p.z - at[2]).abs() < 1e-9
    })
}

/// The kernel resolves a station on a seam from the side the lowering
/// stated: the incoming grade and the incoming leg (8.9.3.48.3).
#[test]
fn the_kernel_resolves_a_seam_station_on_the_incoming_side() {
    let model = seams();
    let norm = 1.0004_f64.sqrt();
    for (name, expected) in [
        (
            "GRADE_BREAK_AT",
            [40.0 - 2.0 * 0.02 / norm, 0.0, 10.8 + 2.0 / norm],
        ),
        (
            "GRADE_BREAK_NEAR",
            [40.0 - 2.0 * 0.02 / norm, 0.0, 10.8 + 2.0 / norm],
        ),
        ("CORNER_AT", [10.5, 1.0, 0.0]),
        ("CORNER_NEAR", [10.5, 1.0, 0.0]),
    ] {
        let lowered = lower(&model, item(&model, name, "Reference")).expect(name);
        let resolved = axiolid_mesh_compile::station::resolve(&lowered.graph, lowered.root)
            .unwrap_or_else(|e| panic!("{name} resolves: {e:?}"));
        close(
            [resolved.point.x, resolved.point.y, resolved.point.z],
            expected,
            name,
        );
    }
}

/// The deck turns the L's corner mitred at half angle (8.8.3.35.1): the
/// mitre plane x + y = 10 holds the section's corners (11, -1) and (9, 1),
/// and the centred 2 x 1 m section over 10 m of centreline holds 20 m3.
#[test]
fn a_sectioned_solid_is_mitred_at_the_corner() {
    let model = seams();
    let lowered = lower(&model, item(&model, "DECK", "Body")).expect("lowers");
    let options = ExecutionOptions::new(Tolerance::MILLIMETRE);
    let outcome = default_backend()
        .compile_mesh_reported(&lowered.graph, lowered.root, &options)
        .unwrap_or_else(|e| panic!("the deck compiles: {e:?}"));
    assert_eq!(outcome.closure, MeshClosure::Solid);
    let mesh = &outcome.mesh;
    let volume = signed_volume(mesh);
    assert!((volume - 20.0).abs() < 1e-9, "volume {volume}");
    let (lo, hi) = bounds(mesh);
    close(lo, [5.0, -1.0, -0.5], "min");
    close(hi, [11.0, 5.0, 0.5], "max");
    for corner in [
        [11.0, -1.0, -0.5],
        [11.0, -1.0, 0.5],
        [9.0, 1.0, -0.5],
        [9.0, 1.0, 0.5],
    ] {
        assert!(has_vertex(mesh, corner), "mitre corner {corner:?}");
    }
}

/// The carriageway crosses the grade break mitred in the vertical plane:
/// two planar strips 7 m wide, 10 sqrt(1.0004) and 10 sqrt(1.0001) m long,
/// meeting on the line (40, y, 10.8).
#[test]
fn a_sectioned_surface_is_mitred_at_the_grade_break() {
    let model = seams();
    let lowered = lower(&model, item(&model, "CARRIAGEWAY", "Body")).expect("lowers");
    let options = ExecutionOptions::new(Tolerance::MILLIMETRE);
    let outcome = default_backend()
        .compile_mesh_reported(&lowered.graph, lowered.root, &options)
        .unwrap_or_else(|e| panic!("the carriageway compiles: {e:?}"));
    assert_eq!(outcome.closure, MeshClosure::Surface);
    let expected = 7.0 * 10.0 * (1.0004_f64.sqrt() + 1.0001_f64.sqrt());
    let got = area(&outcome.mesh);
    assert!((got - expected).abs() < 1e-9, "area {got} != {expected}");
    let (lo, hi) = bounds(&outcome.mesh);
    close(lo, [30.0, -3.5, 10.6], "min");
    close(hi, [50.0, 3.5, 10.8], "max");
    assert!(has_vertex(&outcome.mesh, [40.0, -3.5, 10.8]));
    assert!(has_vertex(&outcome.mesh, [40.0, 3.5, 10.8]));
}

/// The kerb, 1 m left of the L, meets itself at the mitre (9, 1): a disk
/// swept along it stays within (0, 0.9) .. (9.1, 10), where an unmitred
/// offset would reach x = 10.1 or y = -0.1 at the corner.
#[test]
fn an_offset_curve_is_mitred_at_the_corner() {
    let model = seams();
    let lowered = lower(&model, item(&model, "KERB", "Body")).expect("lowers");
    let options = ExecutionOptions::new(Tolerance::MILLIMETRE);
    let outcome = default_backend()
        .compile_mesh_reported(&lowered.graph, lowered.root, &options)
        .unwrap_or_else(|e| panic!("the kerb compiles: {e:?}"));
    let (lo, hi) = bounds(&outcome.mesh);
    assert!(
        (lo[0] - 0.0).abs() < 1e-3 && (lo[1] - 0.9).abs() < 1e-3,
        "min {lo:?}"
    );
    assert!(
        (hi[0] - 9.1).abs() < 1e-3 && (hi[1] - 10.0).abs() < 1e-3,
        "max {hi:?}"
    );
}
