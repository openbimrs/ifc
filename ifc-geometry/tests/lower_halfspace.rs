#![cfg(feature = "lowering")]
//! Requires the `lowering` feature: this suite exercises the neutral DAG.
//! Half-space lowering against the committed corpus.
//!
//! # Why the flyaway fixture
//!
//! `issue_1155_halfspace_flyaway.ifc` is an upstream regression case: a
//! clipping result whose second operand is an infinite half space. Before this
//! family lowered, the whole boolean failed as unsupported, so the clip was
//! dropped and the wall rendered unclipped. Lowering the operand is what makes
//! the enclosing boolean resolvable at all, which is the property asserted
//! here.

use axiolid_model::GeometryNode;
use ifc_geometry::lower::{lower_representation_item, LoweringSession, Tolerance};
use ifc_geometry::transform::Transform;
use ifc_geometry::units;
use ifc_model::{Codec, Model};
use ifc_step::StepCodec;
use std::path::PathBuf;

fn model_of(relative: &str) -> Model {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../test/fixtures")
        .join(relative);
    StepCodec
        .read_path(&path)
        .unwrap_or_else(|e| panic!("fixture {} must parse: {e:?}", path.display()))
}

/// The half space in the flyaway fixture lowers to a bounded plane.
#[test]
fn the_flyaway_half_space_lowers_to_a_plane() {
    let model = model_of("ifclite-geometry/issue_1155_halfspace_flyaway.ifc");
    let scale = units::resolve(&model);
    let id = *model
        .ids_of_type("IFCHALFSPACESOLID")
        .first()
        .expect("the fixture contains a half space");

    let mut session = LoweringSession::new(&model, &scale, Tolerance::building_scale());
    let node = lower_representation_item(&mut session, id, Transform::identity())
        .expect("the half space must lower through the dispatcher");
    let lowered = session.finish(node).expect("session finishes");

    let half_space = match lowered.graph.get(lowered.root).expect("root") {
        GeometryNode::HalfSpace(hs) => *hs,
        other => panic!("expected a HalfSpace node, got {other:?}"),
    };

    let normal = half_space.boundary.normal.to_array();
    let length = (normal[0].powi(2) + normal[1].powi(2) + normal[2].powi(2)).sqrt();
    assert!(
        (length - 1.0).abs() < 1e-9,
        "the boundary normal must be unit length, got {length}"
    );
    assert!(
        half_space
            .boundary
            .origin
            .to_array()
            .iter()
            .all(|v| v.is_finite()),
        "the boundary origin must be finite"
    );
}

/// The enclosing clipping result now resolves end to end.
///
/// This is the actual user-visible fix: the boolean was previously
/// unsupported purely because its cutting tool could not be lowered.
#[test]
fn the_enclosing_clipping_result_resolves_now_that_its_operand_lowers() {
    let model = model_of("ifclite-geometry/issue_1155_halfspace_flyaway.ifc");
    let scale = units::resolve(&model);
    let id = *model
        .ids_of_type("IFCBOOLEANCLIPPINGRESULT")
        .first()
        .expect("the fixture contains a clipping result");

    let mut session = LoweringSession::new(&model, &scale, Tolerance::building_scale());
    let node = lower_representation_item(&mut session, id, Transform::identity())
        .expect("the clipping result must lower");
    let lowered = session.finish(node).expect("session finishes");

    match lowered.graph.get(lowered.root).expect("root") {
        GeometryNode::SolidOperation(_) => {}
        other => panic!("expected a SolidOperation at the root, got {other:?}"),
    }

    // The half space must be present as a dependency, not silently dropped.
    let has_half_space = lowered
        .graph
        .iter()
        .any(|(_, node)| matches!(node, GeometryNode::HalfSpace(_)));
    assert!(
        has_half_space,
        "the lowered clipping tree must contain the half-space operand"
    );
}

/// Every half space in the corpus lowers through the dispatcher.
#[test]
fn every_corpus_half_space_lowers() {
    let mut checked = 0usize;
    for name in [
        "ifclite-geometry/issue_1155_halfspace_flyaway.ifc",
        "ifclite-geometry/bath_csg_solid.ifc",
    ] {
        let model = model_of(name);
        let scale = units::resolve(&model);
        for id in model.ids_of_type("IFCHALFSPACESOLID") {
            let mut session = LoweringSession::new(&model, &scale, Tolerance::building_scale());
            let node = lower_representation_item(&mut session, *id, Transform::identity())
                .expect("corpus half spaces must lower");
            let lowered = session.finish(node).expect("finishes");
            assert!(
                matches!(
                    lowered.graph.get(lowered.root),
                    Some(GeometryNode::HalfSpace(_))
                ),
                "{name} must lower its half space to a HalfSpace node"
            );
            checked += 1;
        }
    }
    assert!(checked >= 1, "expected corpus half spaces, saw {checked}");
}
