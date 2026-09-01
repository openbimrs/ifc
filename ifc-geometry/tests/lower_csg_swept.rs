#![cfg(feature = "lowering")]
//! Requires the `lowering` feature: this suite exercises the neutral DAG.
//! CSG solids, swept disks, and their curve directrices on real fixtures.
//!
//! # Why these three fixtures
//!
//! `bath_csg_solid.ifc` is a CSG wrapper around a boolean difference of a
//! block and an extrusion -- the shape a bathtub void actually takes in an
//! export. `issue_1985_scaled_kinds.ifc` carries the same swept disk twice in
//! two different length units, which is the case a hardcoded factor passes and
//! a real unit resolution catches. `swept_disk_composite_arc_crankbar.ifc` has
//! a composite directrix of trimmed lines and arcs, which is the only corpus
//! file that exercises conic trim parameters.

use axiolid_curve::Curve3;
use axiolid_model::{CurveRelation, GeometryNode, SolidOperation};
use ifc_geometry::lower::{lower_representation_item, LoweringSession};
use ifc_geometry::transform::Transform;
use ifc_geometry::units;
use ifc_model::{Codec, Model};
use ifc_step::StepCodec;
use std::path::PathBuf;

fn fixture(name: &str) -> Model {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../test/fixtures")
        .join(name);
    StepCodec
        .read_path(&path)
        .unwrap_or_else(|e| panic!("fixture {} must parse: {e:?}", path.display()))
}

/// The CSG wrapper resolves to the boolean it wraps, not to an extra level.
#[test]
fn the_bath_csg_solid_lowers_to_its_boolean_tree() {
    let model = fixture("ifclite-geometry/bath_csg_solid.ifc");
    let scale = units::resolve(&model);
    let ids = model.ids_of_type("IFCCSGSOLID");
    assert_eq!(ids.len(), 1, "the fixture carries exactly one CSG solid");

    let mut session = LoweringSession::new(&model, &scale);
    let node = lower_representation_item(&mut session, ids[0], Transform::identity())
        .expect("the CSG solid must lower");
    let lowered = session.finish(node).expect("finishes");

    match lowered.graph.get(lowered.root).expect("root") {
        GeometryNode::SolidOperation(SolidOperation::Boolean { .. }) => {}
        other => panic!("the wrapper must resolve to its boolean root, got {other:?}"),
    }
}

/// Every swept disk in the corpus lowers, in whatever unit it was authored.
#[test]
fn every_corpus_swept_disk_lowers_with_resolved_units() {
    let mut checked = 0usize;
    for name in [
        "ifclite-geometry/issue_1985_scaled_kinds.ifc",
        "ifclite-geometry/swept_disk_composite_arc_crankbar.ifc",
    ] {
        let model = fixture(name);
        let scale = units::resolve(&model);
        for id in model.ids_of_type("IFCSWEPTDISKSOLID") {
            let mut session = LoweringSession::new(&model, &scale);
            let node = lower_representation_item(&mut session, *id, Transform::identity())
                .unwrap_or_else(|e| panic!("{name} #{id:?} must lower: {e}"));
            let lowered = session.finish(node).expect("finishes");

            match lowered.graph.get(lowered.root).expect("root") {
                GeometryNode::SolidOperation(SolidOperation::SweptDisk {
                    radius,
                    inner_radius,
                    ..
                }) => {
                    assert!(
                        *radius > 0.0 && radius.is_finite(),
                        "{name}: radius must be positive and finite, got {radius}"
                    );
                    // Every corpus disk is authored at building scale; a
                    // missed unit conversion leaves a metre-model radius in
                    // the tens, which no real pipe has.
                    assert!(
                        *radius < 1.0,
                        "{name}: radius {radius} m suggests an unconverted millimetre value"
                    );
                    if let Some(inner) = inner_radius {
                        assert!(inner < radius, "{name}: bore must be inside the wall");
                    }
                    checked += 1;
                }
                other => panic!("{name}: expected a SweptDisk, got {other:?}"),
            }
        }
    }
    assert_eq!(checked, 3, "the corpus carries three swept disks");
}

/// The crankbar directrix is a composite of trimmed lines and arcs.
///
/// This is the assertion that would catch a conic trim scaled as a length:
/// the arcs are authored in millimetres with radian parameters, so a single
/// length factor turns a 0.08 rad arc into 8e-5 rad.
#[test]
fn the_crankbar_directrix_lowers_as_a_composite_of_trimmed_curves() {
    let model = fixture("ifclite-geometry/swept_disk_composite_arc_crankbar.ifc");
    let scale = units::resolve(&model);
    let ids = model.ids_of_type("IFCSWEPTDISKSOLID");
    assert!(!ids.is_empty(), "the fixture carries a swept disk");

    let mut session = LoweringSession::new(&model, &scale);
    let node = lower_representation_item(&mut session, ids[0], Transform::identity())
        .expect("the crankbar must lower");
    let lowered = session.finish(node).expect("finishes");

    let directrix = match lowered.graph.get(lowered.root).expect("root") {
        GeometryNode::SolidOperation(SolidOperation::SweptDisk { directrix, .. }) => *directrix,
        other => panic!("expected a SweptDisk, got {other:?}"),
    };

    let segments = match lowered.graph.get(directrix).expect("directrix node") {
        GeometryNode::CurveRelation(CurveRelation::Composite { segments }) => segments.clone(),
        other => panic!("expected a composite directrix, got {other:?}"),
    };
    assert!(
        segments.len() >= 5,
        "the crankbar path has several segments"
    );

    let mut arcs = 0usize;
    for segment in &segments {
        if let GeometryNode::CurveRelation(CurveRelation::Trimmed {
            basis, start, end, ..
        }) = lowered.graph.get(segment.curve).expect("segment curve")
        {
            if let GeometryNode::Curve3(Curve3::Circle(circle)) =
                lowered.graph.get(*basis).expect("basis curve")
            {
                arcs += 1;
                assert!(
                    circle.radius > 0.0 && circle.radius < 1.0,
                    "arc radius {} m suggests an unconverted value",
                    circle.radius
                );
                // Trim parameters on a conic are angles: a full circle is
                // 2*pi, so any authored arc parameter stays well under 10.
                for selector in start.iter().chain(end.iter()) {
                    if let axiolid_model::TrimSelector::Parameter(value) = selector {
                        assert!(
                            value.abs() < 10.0,
                            "conic trim parameter {value} is not an angle; \
                             it was probably scaled as a length"
                        );
                    }
                }
            }
        }
    }
    assert!(
        arcs >= 2,
        "the crankbar path includes trimmed arcs, saw {arcs}"
    );
}
