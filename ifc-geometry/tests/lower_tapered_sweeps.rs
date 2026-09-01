#![cfg(feature = "lowering")]
//! Requires the `lowering` feature: this suite exercises the neutral DAG.
//! Tapered and variable-section sweeps, plus the sectioned spine.

use axiolid_model::{GeometryNode, SolidOperation};
use ifc_geometry::lower::{lower_representation_item, LoweringSession};
use ifc_geometry::transform::Transform;
use ifc_geometry::units;
use ifc_model::{Codec, EntityId, Model};
use ifc_step::StepCodec;
use std::path::PathBuf;

fn model(name: &str) -> Model {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../test/fixtures/synthetic-surfaces/")
        .join(name);
    StepCodec
        .read_path(&path)
        .unwrap_or_else(|e| panic!("fixture {name} must parse: {e:?}"))
}

fn only(model: &Model, kind: &str) -> EntityId {
    let ids = model.ids_of_type(kind);
    assert_eq!(ids.len(), 1, "{kind}: expected exactly one instance");
    ids[0]
}

fn lower(model: &Model, id: EntityId) -> ifc_geometry::lower::LoweredGeometry {
    let scale = units::resolve(model);
    let mut session = LoweringSession::new(model, &scale);
    let node = lower_representation_item(&mut session, id, Transform::identity())
        .unwrap_or_else(|e| panic!("{id:?} must lower: {e}"));
    session.finish(node).expect("session finishes")
}

/// Find the single solid operation of interest in a lowered graph.
fn operation(lowered: &ifc_geometry::lower::LoweredGeometry) -> SolidOperation {
    lowered
        .graph
        .iter()
        .find_map(|(_, n)| match n {
            GeometryNode::SolidOperation(op) => Some(op.clone()),
            _ => None,
        })
        .expect("a solid operation reaches the graph")
}

/// A taper keeps BOTH profiles: start and end must be different nodes.
///
/// Reusing `SweptArea` for both ends produces a prism that builds, renders,
/// and silently discards the taper. The fixture's profiles are 400x300 mm and
/// 200x150 mm, so the two profile nodes cannot legitimately coincide.
#[test]
fn a_tapered_extrusion_keeps_a_distinct_end_profile() {
    let model = model("synthetic_tapered_sweeps.ifc");
    let lowered = lower(&model, only(&model, "IFCEXTRUDEDAREASOLIDTAPERED"));
    match operation(&lowered) {
        SolidOperation::TaperedExtrusion {
            start_profile,
            end_profile,
            depth,
            ..
        } => {
            assert_ne!(
                start_profile, end_profile,
                "start and end profiles must be distinct nodes"
            );
            assert!(
                (depth - 2.500).abs() < 1e-9,
                "2500 mm -> 2.5 m, got {depth}"
            );
        }
        other => panic!("expected a TaperedExtrusion, got {other:?}"),
    }
}

/// The revolution angle is converted from the file's declared unit.
///
/// The fixture declares DEGREE and authors 90. Treating that as radians is
/// roughly fourteen full turns, which still builds a solid.
#[test]
fn a_tapered_revolution_converts_its_angle_from_degrees() {
    let model = model("synthetic_tapered_sweeps.ifc");
    let lowered = lower(&model, only(&model, "IFCREVOLVEDAREASOLIDTAPERED"));
    match operation(&lowered) {
        SolidOperation::TaperedRevolution {
            start_profile,
            end_profile,
            angle,
            axis_origin,
            ..
        } => {
            assert_ne!(start_profile, end_profile, "profiles must be distinct");
            let expected = std::f64::consts::FRAC_PI_2;
            assert!(
                (angle - expected).abs() < 1e-9,
                "90 degrees -> {expected} rad, got {angle}"
            );
            // The axis origin is a length and converts to metres.
            assert!(
                (axis_origin.to_array()[0] - 0.900).abs() < 1e-9,
                "axis origin 900 mm -> 0.9 m, got {:?}",
                axis_origin.to_array()
            );
        }
        other => panic!("expected a TaperedRevolution, got {other:?}"),
    }
}

/// The non-tapered revolution uses the same declared angle and axis contracts.
#[test]
fn a_regular_revolution_converts_angle_and_axis_units() {
    let model = model("synthetic_tapered_sweeps.ifc");
    let lowered = lower(&model, only(&model, "IFCREVOLVEDAREASOLID"));
    match operation(&lowered) {
        SolidOperation::Revolution {
            angle,
            axis_origin,
            axis_direction,
            ..
        } => {
            let expected = std::f64::consts::FRAC_PI_4;
            assert!((angle - expected).abs() < 1e-9, "45 degrees -> pi/4");
            assert!((axis_origin.to_array()[0] - 0.9).abs() < 1e-9);
            assert_eq!(axis_direction.to_array(), [0.0, 1.0, 0.0]);
        }
        other => panic!("expected a Revolution, got {other:?}"),
    }
}

/// The fixed reference direction survives lowering.
///
/// This is what separates a fixed-reference sweep from an ordinary directrix
/// sweep: the section holds a constant orientation instead of rotating with
/// the curve. The fixture uses +Z, not the +X a lowerer would default to, so
/// dropping or hardcoding the reference is observable.
#[test]
fn a_fixed_reference_sweep_preserves_its_reference_direction() {
    let model = model("synthetic_tapered_sweeps.ifc");
    let lowered = lower(&model, only(&model, "IFCFIXEDREFERENCESWEPTAREASOLID"));
    match operation(&lowered) {
        SolidOperation::FixedReferenceSweep {
            reference_direction,
            parameter_range,
            ..
        } => {
            assert_eq!(
                reference_direction.to_array(),
                [0.0, 0.0, 1.0],
                "the fixture's FixedReference is +Z"
            );
            let (start, end) = parameter_range.expect("both trim params are present");
            assert!(
                start.abs() < 1e-9 && (end - 2.0).abs() < 1e-9,
                "polyline params are lengths in file units, got {start}..{end}"
            );
        }
        other => panic!("expected a FixedReferenceSweep, got {other:?}"),
    }
}

/// Both polygonal disks lower, and the fillet radius survives in metres.
///
/// The fixture holds a sharp disk and one with `FilletRadius=90` mm. Sharp
/// corners are `None`; the filleted one must arrive as 0.09 m. A lowerer that
/// dropped the radius would still produce a valid solid, just one whose bends
/// are wrong, so this asserts the value rather than mere success.
#[test]
fn a_polygonal_disk_carries_its_fillet_radius() {
    let model = model("synthetic_tapered_sweeps.ifc");
    let ids = model.ids_of_type("IFCSWEPTDISKSOLIDPOLYGONAL");
    assert_eq!(ids.len(), 2, "the fixture has a sharp and a filleted disk");

    let scale = units::resolve(&model);
    let mut fillets = Vec::new();
    for &id in ids {
        let mut session = LoweringSession::new(&model, &scale);
        let node = lower_representation_item(&mut session, id, Transform::identity())
            .expect("both polygonal disks lower now that the kernel has a fillet");
        let lowered = session.finish(node).expect("finishes");
        match lowered.graph.get(lowered.root).expect("root") {
            GeometryNode::SolidOperation(SolidOperation::SweptDisk { fillet_radius, .. }) => {
                fillets.push(*fillet_radius)
            }
            other => panic!("expected a SweptDisk, got {other:?}"),
        }
    }
    fillets.sort_by(|a, b| a.partial_cmp(b).unwrap());

    assert_eq!(fillets[0], None, "the sharp disk has no fillet");
    let radius = fillets[1].expect("the filleted disk keeps its radius");
    assert!(
        (radius - 0.09).abs() < 1e-9,
        "90 mm -> 0.09 m, got {radius}"
    );
}

/// The spine keeps every section, paired with its own placement.
///
/// The fixture has three DIFFERENT profiles at three DISTINCT stations, so a
/// lowerer that keeps only the first section, or collapses the placements onto
/// one transform, is observable. A plain zip over mismatched lists would
/// silently truncate; the reader rejects that instead.
#[test]
fn a_sectioned_spine_keeps_every_section_at_its_own_station() {
    let model = model("synthetic_sectioned_spine.ifc");
    let lowered = lower(&model, only(&model, "IFCSECTIONEDSPINE"));
    match operation(&lowered) {
        SolidOperation::SectionedSpine { sections, .. } => {
            assert_eq!(sections.len(), 3, "all three cross sections survive");

            let profiles: std::collections::BTreeSet<_> =
                sections.iter().map(|s| s.profile).collect();
            assert_eq!(
                profiles.len(),
                3,
                "the three profiles are distinct nodes, not one reused"
            );

            // Stations are 0, 1200 and 1200/900 mm; in metres the x offsets
            // must be 0, 1.2, 1.2 and the last must differ in y.
            let origins: Vec<[f64; 3]> = sections
                .iter()
                .map(|s| s.placement.translation.to_array())
                .collect();
            assert!(
                (origins[0][0]).abs() < 1e-9
                    && (origins[1][0] - 1.200).abs() < 1e-9
                    && (origins[2][1] - 0.900).abs() < 1e-9,
                "section origins must convert to metres and stay distinct, got {origins:?}"
            );
        }
        other => panic!("expected a SectionedSpine, got {other:?}"),
    }
}
