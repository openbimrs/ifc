//! Unit tests for placing a revolved surface.

use axiolid_curve::Curve3;
use axiolid_model::{GeometryNode, SurfaceRelation};
use ifc_model::{EntityId, Model, Value};

use super::lower_revolution;
use crate::lower::session::LoweringSession;
use crate::solid::testkit::{entity, n, r};
use crate::transform::Transform;
use crate::units::UnitScale;

/// A STEP list of reals, the shape coordinates and ratios take.
fn reals(values: &[f64]) -> Value {
    Value::List(values.iter().map(|v| n(*v)).collect())
}

/// `IfcSweptSurface.Position` places a revolved surface (#547).
///
/// A conforming generatrix is a 2D profile (`IfcArbitraryOpenProfileDef.WR12`),
/// so without `Position` it lies in XY and the surface lands in the wrong
/// place, with no error. The axis is placed with it, as the extrusion
/// direction is: both are stated in the coordinate system `Position` sets up.
#[test]
fn a_revolved_surface_is_placed_through_its_position() {
    let mut model = Model::default();
    // The 2D generatrix, (1, 0) to (2, 1).
    model.insert(
        EntityId(1),
        entity("IFCCARTESIANPOINT", vec![reals(&[1.0, 0.0])]),
    );
    model.insert(
        EntityId(2),
        entity("IFCCARTESIANPOINT", vec![reals(&[2.0, 1.0])]),
    );
    model.insert(
        EntityId(3),
        entity("IFCPOLYLINE", vec![Value::List(vec![r(1), r(2)])]),
    );
    // CURVE profile: ProfileType, ProfileName, Curve.
    model.insert(
        EntityId(4),
        entity(
            "IFCARBITRARYOPENPROFILEDEF",
            vec![Value::Enum("CURVE".into()), Value::Null, r(3)],
        ),
    );
    // Position: local x is world +X, local y is world +Z, origin (10, 0, 5).
    model.insert(
        EntityId(5),
        entity("IFCCARTESIANPOINT", vec![reals(&[10.0, 0.0, 5.0])]),
    );
    model.insert(
        EntityId(6),
        entity("IFCDIRECTION", vec![reals(&[0.0, -1.0, 0.0])]),
    );
    model.insert(
        EntityId(7),
        entity("IFCDIRECTION", vec![reals(&[1.0, 0.0, 0.0])]),
    );
    model.insert(
        EntityId(8),
        entity("IFCAXIS2PLACEMENT3D", vec![r(5), r(6), r(7)]),
    );
    // The axis, in the same local system: through the origin along local y.
    model.insert(
        EntityId(9),
        entity("IFCCARTESIANPOINT", vec![reals(&[0.0, 0.0, 0.0])]),
    );
    model.insert(
        EntityId(10),
        entity("IFCDIRECTION", vec![reals(&[0.0, 1.0, 0.0])]),
    );
    model.insert(EntityId(11), entity("IFCAXIS1PLACEMENT", vec![r(9), r(10)]));
    // SweptCurve, Position, AxisPosition
    model.insert(
        EntityId(12),
        entity("IFCSURFACEOFREVOLUTION", vec![r(4), r(8), r(11)]),
    );

    let scale = UnitScale::SI;
    let mut session = LoweringSession::new(&model, &scale);
    let node = lower_revolution(&mut session, EntityId(12), Transform::identity())
        .expect("the revolution must lower");
    let lowered = session.finish(node).expect("finishes");

    let Some(GeometryNode::SurfaceRelation(SurfaceRelation::Revolution {
        swept_curve,
        axis_origin,
        axis_direction,
    })) = lowered.graph.get(lowered.root)
    else {
        panic!("expected a revolution");
    };
    let Some(GeometryNode::Curve3(Curve3::Polyline(line))) = lowered.graph.get(*swept_curve) else {
        panic!("expected a polyline generatrix");
    };
    let points: Vec<[f64; 3]> = line.points.iter().map(|p| p.to_array()).collect();
    assert_eq!(points, vec![[11.0, 0.0, 5.0], [12.0, 0.0, 6.0]]);
    assert_eq!(axis_origin.to_array(), [10.0, 0.0, 5.0]);
    assert_eq!(axis_direction.to_array(), [0.0, 0.0, 1.0]);
}
