//! Placement resolution tests.
//!
//! These pin the two things a hand-rolled `world_placement` gets wrong:
//! composition order, and applying the unit scale once rather than per link.

use super::*;
use ifc_model::{Entity, Value};

/// `#point = IFCCARTESIANPOINT((x, y, z))` + `#id = IFCAXIS2PLACEMENT3D(#point, $, $)`.
fn axes_at(model: &mut Model, id: u64, point: u64, xyz: [f64; 3]) {
    model.insert(
        EntityId(point),
        Entity::new(
            "IFCCARTESIANPOINT",
            vec![Value::List(xyz.iter().map(|v| Value::Real(*v)).collect())],
        ),
    );
    model.insert(
        EntityId(id),
        Entity::new(
            "IFCAXIS2PLACEMENT3D",
            vec![Value::Ref(EntityId(point)), Value::Null, Value::Null],
        ),
    );
}

/// `#id = IFCLOCALPLACEMENT(parent, #axes)`.
fn local_placement(model: &mut Model, id: u64, parent: Option<u64>, axes: u64) {
    let rel_to = parent.map_or(Value::Null, |p| Value::Ref(EntityId(p)));
    model.insert(
        EntityId(id),
        Entity::new(
            "IFCLOCALPLACEMENT",
            vec![rel_to, Value::Ref(EntityId(axes))],
        ),
    );
}

/// A product with `ObjectPlacement` in slot 5 and no representation.
fn product(model: &mut Model, id: u64, placement: Option<u64>) {
    let mut slots = vec![Value::Null; 7];
    if let Some(p) = placement {
        slots[5] = Value::Ref(EntityId(p));
    }
    model.insert(EntityId(id), Entity::new("IFCWALL", slots));
}

/// A storey at +3, a wall at +2 within it, is at +5 in the world.
///
/// The composition order is the whole point: reversing it puts the wall at
/// the same place only when the offsets happen to commute, which they do not
/// once rotation is involved.
#[test]
fn nested_placement_composes_outermost_last() {
    let mut model = Model::new();
    axes_at(&mut model, 10, 11, [0.0, 0.0, 3.0]); // storey
    axes_at(&mut model, 20, 21, [0.0, 0.0, 2.0]); // wall, relative to storey
    local_placement(&mut model, 30, None, 10);
    local_placement(&mut model, 40, Some(30), 20);
    product(&mut model, 50, Some(40));

    let world = product_world_transform(&model, &UnitScale::default(), EntityId(50)).unwrap();

    assert_eq!(world.origin, [0.0, 0.0, 5.0]);
}

/// The unit scale is applied once to the composed result, not per link.
///
/// A millimetre file two levels deep: 3000 + 2000 mm is 5 m. Converting per
/// link would square the factor and yield 5_000_000.
#[test]
fn unit_scale_applies_once_not_per_link() {
    let mut model = Model::new();
    axes_at(&mut model, 10, 11, [0.0, 0.0, 3000.0]);
    axes_at(&mut model, 20, 21, [0.0, 0.0, 2000.0]);
    local_placement(&mut model, 30, None, 10);
    local_placement(&mut model, 40, Some(30), 20);
    product(&mut model, 50, Some(40));

    let millimetres = UnitScale {
        length_to_metres: 0.001,
        ..UnitScale::default()
    };
    let world = product_world_transform(&model, &millimetres, EntityId(50)).unwrap();

    assert_eq!(world.origin, [0.0, 0.0, 5.0]);
}

/// A product with no ObjectPlacement is model-space, not an error.
#[test]
fn missing_object_placement_is_identity() {
    let mut model = Model::new();
    product(&mut model, 50, None);

    let world = product_world_transform(&model, &UnitScale::default(), EntityId(50)).unwrap();

    assert!(world.is_identity(1e-12));
}

/// A cyclic chain is reported, not hung on.
#[test]
fn cyclic_chain_is_reported() {
    let mut model = Model::new();
    axes_at(&mut model, 10, 11, [1.0, 0.0, 0.0]);
    local_placement(&mut model, 30, Some(40), 10);
    local_placement(&mut model, 40, Some(30), 10);
    product(&mut model, 50, Some(40));

    let err = product_world_transform(&model, &UnitScale::default(), EntityId(50)).unwrap_err();

    assert!(
        matches!(err, GeometryError::CyclicChain { .. }),
        "expected a cycle report, got {err:?}"
    );
}

/// A missing product is an error rather than a silent identity, so a caller
/// cannot mistake "not in the model" for "at the origin".
#[test]
fn missing_product_is_an_error() {
    let model = Model::new();

    let err = product_world_transform(&model, &UnitScale::default(), EntityId(99)).unwrap_err();

    assert!(matches!(err, GeometryError::MissingEntity { .. }));
}

/// The batch form agrees with the single form and keeps per-product errors
/// isolated: one broken chain must not suppress its siblings.
#[test]
fn batch_matches_single_and_isolates_failures() {
    let mut model = Model::new();
    axes_at(&mut model, 10, 11, [0.0, 0.0, 3.0]);
    local_placement(&mut model, 30, None, 10);
    product(&mut model, 50, Some(30));

    // A second product whose chain is cyclic.
    axes_at(&mut model, 60, 61, [1.0, 0.0, 0.0]);
    local_placement(&mut model, 70, Some(80), 60);
    local_placement(&mut model, 80, Some(70), 60);
    product(&mut model, 90, Some(80));

    let units = UnitScale::default();
    let results = products_world_transforms(&model, &units, [EntityId(50), EntityId(90)]);

    assert_eq!(results.len(), 2);
    let good = results[0].1.as_ref().unwrap();
    assert_eq!(
        good.origin,
        product_world_transform(&model, &units, EntityId(50))
            .unwrap()
            .origin
    );
    assert!(
        results[1].1.is_err(),
        "the cyclic product must still report"
    );
}

/// Sharing the resolver across products must not change any answer.
#[test]
fn shared_cache_agrees_with_independent_resolution() {
    let mut model = Model::new();
    axes_at(&mut model, 10, 11, [0.0, 0.0, 3.0]); // shared storey
    local_placement(&mut model, 30, None, 10);
    for (i, offset) in [(0u64, 1.0), (1, 2.0), (2, 3.0)] {
        let base = 100 + i * 10;
        axes_at(&mut model, base, base + 1, [0.0, 0.0, offset]);
        local_placement(&mut model, base + 2, Some(30), base);
        product(&mut model, base + 3, Some(base + 2));
    }

    let units = UnitScale::default();
    let ids = [EntityId(103), EntityId(113), EntityId(123)];
    let batched = products_world_transforms(&model, &units, ids);

    for (id, result) in batched {
        let independent = product_world_transform(&model, &units, id).unwrap();
        assert_eq!(result.unwrap().origin, independent.origin);
    }
}
