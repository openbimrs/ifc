//! Unit tests for CSG solids, CSG primitives, and swept-disk solids.

use axiolid_model::{GeometryNode, SolidOperation};
use axiolid_primitive::Primitive;
use ifc_model::{EntityId, Model, Value};

use super::{lower_csg_primitive_node, lower_swept_disk_node};
use crate::lower::session::LoweringSession;
use crate::lower::Tolerance;
use crate::solid::testkit::{entity, n, r};
use crate::transform::Transform;
use crate::units::UnitScale;

fn millimetres() -> UnitScale {
    UnitScale {
        length_to_metres: 0.001,
        angle_to_radians: 1.0,
    }
}

fn point(x: f64, y: f64, z: f64) -> ifc_model::Entity {
    entity(
        "IFCCARTESIANPOINT",
        vec![Value::List(vec![n(x), n(y), n(z)])],
    )
}

/// A block placed away from the origin, in millimetres.
fn placed_block() -> Model {
    let mut model = Model::new();
    model.insert(EntityId(1), point(1000.0, 2000.0, 0.0));
    model.insert(
        EntityId(2),
        entity("IFCAXIS2PLACEMENT3D", vec![r(1), Value::Null, Value::Null]),
    );
    model.insert(
        EntityId(3),
        entity("IFCBLOCK", vec![r(2), n(1800.0), n(600.0), n(200.0)]),
    );
    model
}

/// Block extents are lengths and convert; the placement is NOT folded in.
///
/// Folding the origin into the extents would move the corner and silently
/// resize the block. The kernel primitive is local by contract.
#[test]
fn a_block_keeps_local_extents_and_carries_its_placement_separately() {
    let model = placed_block();
    let scale = millimetres();
    let mut session = LoweringSession::new(&model, &scale, Tolerance::building_scale());
    let node = lower_csg_primitive_node(&mut session, EntityId(3), Transform::identity())
        .expect("the block must lower");
    let lowered = session.finish(node).expect("finishes");

    let instance = match lowered.graph.get(lowered.root).expect("root") {
        GeometryNode::Instance(instance) => *instance,
        other => panic!("expected an Instance, got {other:?}"),
    };
    match lowered.graph.get(instance.source).expect("source") {
        GeometryNode::Primitive(Primitive::Block { x, y, z }) => {
            assert!((x - 1.8).abs() < 1e-12, "1800 mm -> 1.8 m, got {x}");
            assert!((y - 0.6).abs() < 1e-12, "600 mm -> 0.6 m, got {y}");
            assert!((z - 0.2).abs() < 1e-12, "200 mm -> 0.2 m, got {z}");
        }
        other => panic!("expected a Block primitive, got {other:?}"),
    }
    let translation = instance.transform.translation.to_array();
    assert!(
        (translation[0] - 1.0).abs() < 1e-12 && (translation[1] - 2.0).abs() < 1e-12,
        "the placement must carry the converted origin, got {translation:?}"
    );
}

/// A swept disk with a polyline directrix keeps radii in metres.
fn swept_disk(inner: Option<f64>, start: Option<f64>, end: Option<f64>) -> Model {
    let mut model = Model::new();
    model.insert(EntityId(1), point(0.0, 0.0, 0.0));
    model.insert(EntityId(2), point(3000.0, 0.0, 0.0));
    model.insert(
        EntityId(3),
        entity("IFCPOLYLINE", vec![Value::List(vec![r(1), r(2)])]),
    );
    model.insert(
        EntityId(4),
        entity(
            "IFCSWEPTDISKSOLID",
            vec![
                r(3),
                n(50.0),
                inner.map(n).unwrap_or(Value::Null),
                start.map(n).unwrap_or(Value::Null),
                end.map(n).unwrap_or(Value::Null),
            ],
        ),
    );
    model
}

/// Radii convert to metres and the inner radius is preserved.
#[test]
fn swept_disk_radii_convert_and_the_pipe_bore_survives() {
    let model = swept_disk(Some(45.0), None, None);
    let scale = millimetres();
    let mut session = LoweringSession::new(&model, &scale, Tolerance::building_scale());
    let node = lower_swept_disk_node(&mut session, EntityId(4), Transform::identity())
        .expect("the swept disk must lower");
    let lowered = session.finish(node).expect("finishes");

    match lowered.graph.get(lowered.root).expect("root") {
        GeometryNode::SolidOperation(SolidOperation::SweptDisk {
            radius,
            inner_radius,
            ..
        }) => {
            assert!(
                (radius - 0.05).abs() < 1e-12,
                "50 mm -> 0.05 m, got {radius}"
            );
            let inner = inner_radius.expect("the bore must survive: dropping it fills the pipe");
            assert!(
                (inner - 0.045).abs() < 1e-12,
                "45 mm -> 0.045 m, got {inner}"
            );
        }
        other => panic!("expected a SweptDisk, got {other:?}"),
    }
}

/// A half-open parameter range is refused rather than silently completed.
///
/// Defaulting the missing end to the curve's extent produces a solid of the
/// wrong length that nothing downstream can distinguish from an intended one.
#[test]
fn a_half_open_parameter_range_is_refused() {
    let model = swept_disk(None, Some(0.0), None);
    let scale = millimetres();
    let mut session = LoweringSession::new(&model, &scale, Tolerance::building_scale());
    let error = lower_swept_disk_node(&mut session, EntityId(4), Transform::identity())
        .expect_err("one-sided trims must not be guessed at");
    assert_eq!(error.entity(), Some(EntityId(4)));
}

/// A polyline directrix parameterises by SEGMENT INDEX, so trims do NOT scale.
///
/// `StartParam`/`EndParam` are `IfcParameterValue`, which is dimensionless, and
/// ISO 10303-42 parameterises a polyline over `[0, n]` with integer values at
/// its vertices. Converting them as lengths turns a millimetre file's `2.0`
/// into `0.002` and collapses the trim onto the curve's start.
///
/// This test previously asserted the opposite. The old expectation was wrong:
/// it required `3000` to become `3.0 m`, but `3000` as a polyline parameter is
/// segment 3000, which a two-point polyline does not have.
#[test]
fn swept_disk_trim_parameters_follow_the_directrix_parameterisation() {
    let model = swept_disk(None, Some(0.0), Some(2.0));
    let scale = millimetres();
    let mut session = LoweringSession::new(&model, &scale, Tolerance::building_scale());
    let node =
        lower_swept_disk_node(&mut session, EntityId(4), Transform::identity()).expect("lowers");
    let lowered = session.finish(node).expect("finishes");

    match lowered.graph.get(lowered.root).expect("root") {
        GeometryNode::SolidOperation(SolidOperation::SweptDisk {
            parameter_range, ..
        }) => {
            let (start, end) = parameter_range.expect("both ends present");
            assert!((start - 0.0).abs() < 1e-12);
            assert!(
                (end - 2.0).abs() < 1e-12,
                "a polyline parameter is an index and stays 2, got {end}"
            );
        }
        other => panic!("expected a SweptDisk, got {other:?}"),
    }
}

/// An inner radius that meets the outer leaves no material: refuse it.
#[test]
fn an_inner_radius_not_smaller_than_the_outer_is_refused() {
    let model = swept_disk(Some(50.0), None, None);
    let scale = millimetres();
    let mut session = LoweringSession::new(&model, &scale, Tolerance::building_scale());
    let error = lower_swept_disk_node(&mut session, EntityId(4), Transform::identity())
        .expect_err("inner >= outer has no volume and must be reported");
    assert_eq!(error.entity(), Some(EntityId(4)));
}

/// The directrix is lowered as a real node, not dropped.
#[test]
fn the_directrix_is_present_in_the_graph_as_a_curve() {
    use axiolid_curve::Curve3;

    let model = swept_disk(None, None, None);
    let scale = millimetres();
    let mut session = LoweringSession::new(&model, &scale, Tolerance::building_scale());
    let node =
        lower_swept_disk_node(&mut session, EntityId(4), Transform::identity()).expect("lowers");
    let lowered = session.finish(node).expect("finishes");

    let directrix = match lowered.graph.get(lowered.root).expect("root") {
        GeometryNode::SolidOperation(SolidOperation::SweptDisk { directrix, .. }) => *directrix,
        other => panic!("expected a SweptDisk, got {other:?}"),
    };
    match lowered
        .graph
        .get(directrix)
        .expect("the directrix node must exist")
    {
        GeometryNode::Curve3(Curve3::Polyline(pl)) => {
            assert_eq!(pl.points.len(), 2, "both directrix points must survive");
        }
        other => panic!("expected a Polyline directrix, got {other:?}"),
    }
}
