#![cfg(feature = "lowering")]
//! Requires the `lowering` feature: this suite exercises the neutral DAG.
//! Surface lowering against the committed corpus.
//!
//! # Why the duct elbow
//!
//! `issue_1485_duct_elbow_surface_curve_swept.ifc` is the only licensed
//! fixture carrying a non-planar-surface construction: a round elbow whose
//! profile is swept along a trimmed arc lying on an
//! `IfcSurfaceOfLinearExtrusion`. It is a real exporter file, and it now
//! lowers end to end: its open generatrix is unwrapped rather than forced
//! through the closed-contour profile model.

use axiolid_model::{GeometryNode, SurfaceRelation};
use axiolid_surface::Surface;
use ifc_geometry::lower::{
    lower_representation_item, lower_surface_node, LoweringSession, Tolerance,
};
use ifc_geometry::transform::Transform;
use ifc_geometry::units;
use ifc_model::{Codec, Model};
use ifc_step::StepCodec;
use std::path::PathBuf;

const DUCT: &str = "ifclite-geometry/issue_1485_duct_elbow_surface_curve_swept.ifc";

fn fixture(name: &str) -> Model {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../test/fixtures")
        .join(name);
    StepCodec
        .read_path(&path)
        .unwrap_or_else(|e| panic!("fixture {} must parse: {e:?}", path.display()))
}

/// The duct elbow's reference surface lowers now that generatrices unwrap.
///
/// This file was the reason `IfcSurfaceCurveSweptAreaSolid` stayed unclaimed:
/// its `SweptCurve` arrives wrapped in an `IfcArbitraryOpenProfileDef`, and
/// lowering that as a profile demands a closed contour. For a swept SURFACE
/// the wrapper is a generatrix, not an area, so it is unwrapped to the curve
/// it names -- no fabricated face, no closed-contour requirement.
#[test]
fn the_duct_elbow_reference_surface_lowers_through_its_open_generatrix() {
    let model = fixture(DUCT);
    let scale = units::resolve(&model);
    let ids = model.ids_of_type("IFCSURFACEOFLINEAREXTRUSION");
    assert_eq!(ids.len(), 1, "the fixture carries one extruded surface");

    let mut session = LoweringSession::new(&model, &scale, Tolerance::building_scale());
    let node = lower_surface_node(&mut session, ids[0], Transform::identity())
        .expect("the reference surface must lower");
    let lowered = session.finish(node).expect("session finishes");
    assert!(
        matches!(
            lowered.graph.get(lowered.root),
            Some(GeometryNode::SurfaceRelation(
                SurfaceRelation::LinearExtrusion { .. }
            ))
        ),
        "expected a linear extrusion at the root"
    );
}

/// The whole swept solid lowers end to end, not just its reference surface.
#[test]
fn the_duct_elbow_swept_solid_lowers_end_to_end() {
    let model = fixture(DUCT);
    let scale = units::resolve(&model);
    let ids = model.ids_of_type("IFCSURFACECURVESWEPTAREASOLID");
    assert_eq!(ids.len(), 1, "the fixture carries one surface-curve sweep");

    let mut session = LoweringSession::new(&model, &scale, Tolerance::building_scale());
    lower_representation_item(&mut session, ids[0], Transform::identity())
        .expect("the sweep must lower now that its generatrix resolves");
}

/// Every plane in the committed corpus lowers to an orthonormal frame.
///
/// Planes are the one surface family real files already carried, so this
/// keeps coverage on sourced data rather than only on generated fixtures.
#[test]
fn every_corpus_plane_lowers_to_an_orthonormal_frame() {
    let mut seen = 0usize;
    for name in [
        "ifclite-geometry/issue_1155_halfspace_flyaway.ifc",
        "synthetic-surfaces/synthetic_curve_bounded_plane.ifc",
    ] {
        let model = fixture(name);
        let scale = units::resolve(&model);
        for id in model.ids_of_type("IFCPLANE") {
            let mut session = LoweringSession::new(&model, &scale, Tolerance::building_scale());
            let node = lower_surface_node(&mut session, *id, Transform::identity())
                .expect("a plane must lower");
            let lowered = session.finish(node).expect("session finishes");
            let Some(GeometryNode::Surface(Surface::Plane(plane))) =
                lowered.graph.get(lowered.root)
            else {
                panic!("expected a plane node");
            };
            for axis in [plane.frame.x, plane.frame.y, plane.frame.z] {
                let a = axis.to_array();
                let len = (a[0] * a[0] + a[1] * a[1] + a[2] * a[2]).sqrt();
                assert!((len - 1.0).abs() < 1e-9, "axis must be unit, got {len}");
            }
            seen += 1;
        }
    }
    assert!(seen >= 2, "expected planes in both fixtures, saw {seen}");
}
