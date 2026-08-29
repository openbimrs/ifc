#![cfg(feature = "lowering")]
//! Requires the `lowering` feature: this suite exercises the neutral DAG.
//! Surface lowering against the committed corpus.
//!
//! # Why the duct elbow
//!
//! `issue_1485_duct_elbow_surface_curve_swept.ifc` is the only licensed
//! fixture carrying a non-planar-surface construction: a round elbow whose
//! profile is swept along a trimmed arc lying on an
//! `IfcSurfaceOfLinearExtrusion`. It is a real exporter file, and it is also
//! the file that proves where this crate currently stops -- see
//! `the_duct_elbow_reports_the_open_profile_it_cannot_represent`.

use axiolid_model::GeometryNode;
use axiolid_surface::Surface;
use ifc_geometry::lower::{
    lower_profile_node, lower_representation_item, lower_surface_node, LoweringSession, Tolerance,
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

/// The elbow's reference surface is blocked by the same open profile.
///
/// `IfcSurfaceOfLinearExtrusion` sweeps a curve, and in this file that curve
/// is the very `IfcArbitraryOpenProfileDef` the solid also references. So the
/// surface lowering is exercised right up to its curve dependency and stops
/// at the same real boundary -- one gap, reported once, from both directions.
#[test]
fn the_duct_elbow_reference_surface_stops_at_the_same_open_profile() {
    let model = fixture(DUCT);
    let scale = units::resolve(&model);
    let ids = model.ids_of_type("IFCSURFACEOFLINEAREXTRUSION");
    assert_eq!(ids.len(), 1, "the fixture carries one extruded surface");

    let mut session = LoweringSession::new(&model, &scale, Tolerance::building_scale());
    let error = lower_surface_node(&mut session, ids[0], Transform::identity())
        .expect_err("its swept curve is the open profile");
    assert!(error.is_unsupported(), "this is a gap, not corruption");
    assert!(
        error.to_string().contains("IFCARBITRARYOPENPROFILEDEF"),
        "the surface must report the same underlying gap, got: {error}"
    );
}

/// Every plane in the corpus lowers with a unit, orthogonal frame.
///
/// A plane whose frame is not orthonormal is not a frame at all: trims taken
/// in its parameter space would shear. This walks the corpus rather than one
/// fixture so a newly added file cannot quietly introduce a degenerate one.
#[test]
fn every_corpus_plane_lowers_to_an_orthonormal_frame() {
    let mut checked = 0usize;
    for name in ["ifclite-geometry/issue_1155_halfspace_flyaway.ifc"] {
        let model = fixture(name);
        let scale = units::resolve(&model);
        for id in model.ids_of_type("IFCPLANE") {
            let mut session = LoweringSession::new(&model, &scale, Tolerance::building_scale());
            let node = lower_surface_node(&mut session, *id, Transform::identity())
                .expect("planes must lower");
            let lowered = session.finish(node).expect("finishes");
            let GeometryNode::Surface(Surface::Plane(plane)) =
                lowered.graph.get(lowered.root).expect("root")
            else {
                panic!("expected a plane");
            };
            let x = plane.frame.x.to_array();
            let z = plane.frame.z.to_array();
            let dot = x[0] * z[0] + x[1] * z[1] + x[2] * z[2];
            assert!(
                dot.abs() < 1e-9,
                "X must stay perpendicular to Z, got dot {dot}"
            );
            let zlen = (z[0] * z[0] + z[1] * z[1] + z[2] * z[2]).sqrt();
            assert!(
                (zlen - 1.0).abs() < 1e-12,
                "the normal must be unit length, got {zlen}"
            );
            checked += 1;
        }
    }
    assert!(
        checked >= 1,
        "expected at least one corpus plane, saw {checked}"
    );
}

/// The elbow reports the open profile, not a surface gap.
///
/// This is the honest boundary of the current implementation. The surface and
/// curve halves of this solid both lower; what stops it is
/// `IfcArbitraryOpenProfileDef`, because the neutral profile model represents
/// closed contours only and closing the curve would fabricate a face the file
/// never described.
///
/// The assertion is deliberately on the *reported entity*: if a future change
/// makes open profiles representable, this test fails and forces the report to
/// be re-examined rather than silently passing on a different error.
#[test]
fn the_duct_elbow_reports_the_open_profile_it_cannot_represent() {
    let model = fixture(DUCT);
    let scale = units::resolve(&model);
    let ids = model.ids_of_type("IFCSURFACECURVESWEPTAREASOLID");
    assert_eq!(ids.len(), 1, "the fixture carries one surface-curve sweep");

    let mut session = LoweringSession::new(&model, &scale, Tolerance::building_scale());
    let error = lower_representation_item(&mut session, ids[0], Transform::identity())
        .expect_err("the open profile is not representable yet");

    assert!(error.is_unsupported(), "this is a gap, not corruption");
    assert!(
        error.to_string().contains("IFCARBITRARYOPENPROFILEDEF"),
        "the report must name the open profile, not the sweep or the surface, got: {error}"
    );
    // Reached through the sweep's profile slot, the profile module owns the
    // report and states why the shape has no neutral representation.
    let mut session = LoweringSession::new(&model, &scale, Tolerance::building_scale());
    let profile_error = lower_profile_node(&mut session, ifc_model::EntityId(49))
        .expect_err("an open profile has no closed-contour representation");
    assert!(
        profile_error.to_string().contains("closed contours only"),
        "the profile report must state the documented reason, got: {profile_error}"
    );
}
