//! A product placed along an alignment resolves to a world transform.
//!
//! Before this, product_world_transform refused every IFC4x3 linearly
//! placed product by type, because the IfcLocalPlacement walk cannot reach
//! one. ADR 0003 amended 2026-09-15 permits the ifc-alignment dependency.

#![cfg(feature = "lowering")]

use ifc_geometry::units::UnitScale;
use ifc_geometry::{product_world_transform, GeometryError};
use ifc_model::{Entity, EntityId, Model, Value};

/// Build a model with a sign placed 1240 m along an alignment curve.
///
/// `cached` controls whether the authoring tool wrote the shortcut
/// `CartesianPosition` alongside the linear expression.
fn model_with_linear_placement(cached: bool) -> (Model, EntityId) {
    let mut model = Model::default();
    // The basis curve the distance is measured along.
    model.insert(
        EntityId(1),
        Entity::new("IFCPOLYLINE", vec![Value::List(vec![])]),
    );
    // IfcPointByDistanceExpression: 1240 m along, 2 m right, 3 m up.
    model.insert(
        EntityId(2),
        Entity::new(
            "IFCPOINTBYDISTANCEEXPRESSION",
            vec![
                Value::Real(1240.0),
                Value::Real(2.0),
                Value::Real(3.0),
                Value::Null,
                Value::Ref(EntityId(1)),
            ],
        ),
    );
    // IfcAxis2PlacementLinear wrapping it.
    model.insert(
        EntityId(3),
        Entity::new(
            "IFCAXIS2PLACEMENTLINEAR",
            vec![Value::Ref(EntityId(2)), Value::Null, Value::Null],
        ),
    );
    // The cached explicit position most authoring tools also write.
    model.insert(
        EntityId(4),
        Entity::new(
            "IFCCARTESIANPOINT",
            vec![Value::List(vec![
                Value::Real(100.0),
                Value::Real(200.0),
                Value::Real(5.0),
            ])],
        ),
    );
    model.insert(
        EntityId(5),
        Entity::new(
            "IFCAXIS2PLACEMENT3D",
            vec![Value::Ref(EntityId(4)), Value::Null, Value::Null],
        ),
    );
    let cartesian = if cached {
        Value::Ref(EntityId(5))
    } else {
        Value::Null
    };
    model.insert(
        EntityId(6),
        Entity::new(
            "IFCLINEARPLACEMENT",
            vec![Value::Null, Value::Ref(EntityId(3)), cartesian],
        ),
    );
    // A sign, placed by that linear placement.
    let mut sign = vec![Value::Null; 7];
    sign[5] = Value::Ref(EntityId(6));
    model.insert(EntityId(7), Entity::new("IFCSIGN", sign));
    (model, EntityId(7))
}

#[test]
fn a_linearly_placed_product_resolves_through_the_cached_position() {
    let (model, sign) = model_with_linear_placement(true);
    let world = product_world_transform(&model, &UnitScale::default(), sign)
        .expect("a linear placement with a cached position must resolve");
    assert_eq!(world.origin, [100.0, 200.0, 5.0]);
}

/// Without the cached position the frame must be derived from the basis
/// curve, which is not implemented here. That must be a refusal naming the
/// gap, never a silent identity: a sign at the origin is a wrong answer,
/// not a missing one.
#[test]
fn without_a_cached_position_the_gap_is_named_not_guessed() {
    let (model, sign) = model_with_linear_placement(false);
    let error = product_world_transform(&model, &UnitScale::default(), sign)
        .expect_err("deriving a frame from the basis curve is not implemented");
    let GeometryError::Unsupported { detail, .. } = error else {
        panic!("expected a typed Unsupported refusal, got {error:?}");
    };
    assert!(
        detail.contains("CartesianPosition"),
        "detail must name the gap: {detail}"
    );
}

/// A malformed linear expression is refused even when a cached position
/// exists. A shortcut does not make a broken file correct, and trusting it
/// blindly would let a dangling BasisCurve through unnoticed.
#[test]
fn a_malformed_expression_is_refused_despite_the_shortcut() {
    let (mut model, sign) = model_with_linear_placement(true);
    // Point the expression at a BasisCurve that does not exist.
    let mut broken = model
        .get(EntityId(2))
        .expect("expression")
        .attributes
        .clone();
    broken[4] = Value::Ref(EntityId(999));
    model.insert(
        EntityId(2),
        Entity::new("IFCPOINTBYDISTANCEEXPRESSION", broken),
    );
    let result = product_world_transform(&model, &UnitScale::default(), sign);
    assert!(
        result.is_err(),
        "a dangling BasisCurve must not pass because a shortcut exists"
    );
}
