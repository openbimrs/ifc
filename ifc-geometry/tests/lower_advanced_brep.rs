#![cfg(feature = "lowering")]
//! Requires the `lowering` feature: this suite exercises the neutral DAG.
//! Advanced B-rep lowering on the generated fixture.
//!
//! # Why a generated fixture
//!
//! No licensed corpus we surveyed carries an `IfcAdvancedBrep`. The fixture is
//! a half-cylinder plug: two planar caps and one cylindrical lateral face,
//! which is the smallest shape that forces every advanced-specific decision --
//! shared edges via `IfcOrientedEdge`, a curved support surface on a face, a
//! `SameSense` flag that is actually false, and edge curves that are a mix of
//! circles and lines.

use axiolid_model::GeometryNode;
use axiolid_topology::Orientation;
use ifc_geometry::lower::{lower_representation_item, LoweringSession, Tolerance};
use ifc_geometry::transform::Transform;
use ifc_geometry::units;
use ifc_model::{Codec, Model};
use ifc_step::StepCodec;
use std::path::PathBuf;

const FIXTURE: &str = "../test/fixtures/synthetic-surfaces/synthetic_advanced_brep.ifc";

fn model() -> Model {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(FIXTURE);
    StepCodec
        .read_path(&path)
        .unwrap_or_else(|e| panic!("fixture must parse: {e:?}"))
}

fn brep(model: &Model) -> axiolid_topology::BRep<axiolid_model::NodeId> {
    let scale = units::resolve(model);
    let ids = model.ids_of_type("IFCADVANCEDBREP");
    assert_eq!(ids.len(), 1, "one advanced brep in the fixture");
    let mut session = LoweringSession::new(model, &scale, Tolerance::building_scale());
    let node = lower_representation_item(&mut session, ids[0], Transform::identity())
        .unwrap_or_else(|e| panic!("the advanced brep must lower: {e}"));
    let lowered = session.finish(node).expect("session finishes");
    match lowered.graph.get(lowered.root).expect("root node") {
        GeometryNode::BRep(b) => b.clone(),
        other => panic!("expected a BRep, got {other:?}"),
    }
}

/// Eight oriented edges over four shared edges: the manifold is preserved.
///
/// This is the whole point of interning. The fixture's three faces reference
/// `IfcOrientedEdge` eight times, but only four distinct `IfcEdgeCurve`
/// records exist. Emitting one edge per use would give eight, and no edge
/// would be shared -- silently turning a closed solid into loose facets.
#[test]
fn oriented_edge_reuse_collapses_onto_shared_edges() {
    let model = model();
    assert_eq!(
        model.ids_of_type("IFCORIENTEDEDGE").len(),
        8,
        "the fixture reuses edges; if this changes the test below is moot"
    );
    let brep = brep(&model);
    assert_eq!(
        brep.edges().len(),
        4,
        "four IfcEdgeCurve records must intern to four edges, not eight uses"
    );
    assert_eq!(
        brep.vertices().len(),
        4,
        "four IfcVertexPoint records must intern to four vertices"
    );
    assert_eq!(brep.faces().len(), 3, "two caps and one lateral face");
    assert_eq!(brep.solids().len(), 1, "one closed solid");
}

/// A curved face carries a surface handle; the kernel needs it to evaluate.
///
/// A planar `IfcFace` legitimately has `surface: None` because its loop fixes
/// the plane. An `IfcAdvancedFace` does not: without the handle a cylinder is
/// indistinguishable from a flat polygon through the same rim points.
#[test]
fn every_advanced_face_keeps_its_support_surface() {
    let brep = brep(&model());
    assert!(
        brep.faces().iter().all(|f| f.surface.is_some()),
        "every advanced face names a support surface"
    );
}

/// `SameSense=.F.` must invert the face, not be silently dropped.
///
/// The fixture's bottom cap is authored with `SameSense=.F.`, so exactly one
/// of the three faces must come back reversed. A lowerer that ignores the
/// flag produces an inside-out face: the shape still renders, and a volume
/// or boolean query gets the wrong sign.
#[test]
fn a_false_same_sense_flag_reverses_exactly_that_face() {
    let brep = brep(&model());
    let reversed = brep
        .faces()
        .iter()
        .filter(|f| f.orientation == Orientation::Reversed)
        .count();
    assert_eq!(
        reversed, 1,
        "the bottom cap is authored SameSense=.F. and must be the only reversed face"
    );
}

/// Curved edges keep their support curve; straight seams keep theirs too.
///
/// Every edge in this fixture is an `IfcEdgeCurve`, so all four must carry a
/// handle. An edge whose curve is dropped degenerates to a straight segment
/// between its endpoints -- which for the two semicircular rims silently
/// replaces an arc with a chord.
#[test]
fn every_edge_curve_keeps_its_support_curve() {
    let model = model();
    assert_eq!(
        model.ids_of_type("IFCEDGECURVE").len(),
        4,
        "all four edges are curve-backed in this fixture"
    );
    let brep = brep(&model);
    assert!(
        brep.edges().iter().all(|e| e.curve.is_some()),
        "an edge that loses its curve becomes a chord"
    );
}

/// The shared edge is traversed in opposite directions by its two faces.
///
/// A closed manifold requires it: if both uses ran the same way the surface
/// would not close. The lateral face and a cap meet at each vertical seam, so
/// at least one loop must contain a reversed edge use.
#[test]
fn shared_edges_are_walked_in_both_directions() {
    let brep = brep(&model());
    let reversed_uses: usize = brep
        .loops()
        .iter()
        .map(|l| {
            l.edges
                .iter()
                .filter(|u| u.orientation == Orientation::Reversed)
                .count()
        })
        .sum();
    assert!(
        reversed_uses > 0,
        "a closed solid walks shared edges both ways; found none reversed"
    );
}

/// The lateral face's surface is the cylinder, in metres.
#[test]
fn the_lateral_face_surface_is_the_cylinder_in_metres() {
    let model = model();
    assert_eq!(
        model.ids_of_type("IFCCYLINDRICALSURFACE").len(),
        1,
        "one cylindrical surface backs the lateral face"
    );
    let scale = units::resolve(&model);
    let ids = model.ids_of_type("IFCADVANCEDBREP");
    let mut session = LoweringSession::new(&model, &scale, Tolerance::building_scale());
    let node =
        lower_representation_item(&mut session, ids[0], Transform::identity()).expect("lowers");
    let lowered = session.finish(node).expect("finishes");
    let found = lowered.graph.iter().any(|(_, n)| {
        matches!(n, GeometryNode::Surface(axiolid_surface::Surface::Cylinder(c))
            if (c.radius - 0.12).abs() < 1e-9)
    });
    assert!(found, "the 120 mm cylinder must reach the graph as 0.12 m");
}

/// Both sense flags compose: the edge-curve's and the oriented-edge's.
///
/// Derived from the fixture, not observed from output. Stored edge senses
/// follow `IfcEdgeCurve.SameSense`: #30 #31 #36 forward, #41 reversed. Each
/// `IfcOrientedEdge` then flips its base when its own flag is `.F.`:
///
/// | use | base edge | base sense | own flag | result   |
/// |-----|-----------|------------|----------|----------|
/// | #42 | #30       | forward    | .T.      | forward  |
/// | #43 | #41       | reversed   | .T.      | REVERSED |
/// | #44 | #31       | forward    | .F.      | REVERSED |
/// | #45 | #36       | forward    | .F.      | REVERSED |
/// | #51 | #30       | forward    | .T.      | forward  |
/// | #52 | #30       | forward    | .F.      | REVERSED |
/// | #54 | #31       | forward    | .T.      | forward  |
/// | #55 | #31       | forward    | .F.      | REVERSED |
///
/// Five reversed of eight. Dropping either flag changes that count, which is
/// why one assertion covers both and neither can hide the other.
#[test]
fn both_sense_flags_compose_across_shared_edges() {
    let model = model();
    let brep = brep(&model);

    assert_eq!(
        brep.edges().iter().filter(|e| e.curve.is_some()).count(),
        4,
        "every edge in this fixture carries a support curve"
    );

    let reversed_uses = brep
        .loops()
        .iter()
        .flat_map(|l| l.edges.iter())
        .filter(|u| u.orientation == Orientation::Reversed)
        .count();
    let total_uses = brep.loops().iter().map(|l| l.edges.len()).sum::<usize>();

    assert_eq!(total_uses, 8, "eight oriented-edge uses across three loops");
    assert_eq!(
        reversed_uses, 5,
        "five uses are reversed once both flags compose"
    );
}
