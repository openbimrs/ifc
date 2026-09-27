//! CSG primitives and half spaces: authored, then read back.
//!
//! Primitives share one slot layout -- `Position` at 0, dimensions from
//! 1 upward -- so the risk is not arity but *order*: a cone's `Height`
//! and `BottomRadius` are both lengths, and swapping them still parses.
//! Every test below asserts the values land in the slots the crate's own
//! readers look in.

use ifc_geometry::authoring::{
    axis2_placement_3d, block, bounding_box, boxed_half_space, cartesian_point, cone, csg_solid,
    cylinder, half_space, polygonal_bounded_half_space, polyline, rectangular_pyramid, sphere,
};
use ifc_geometry::solid::csg::{CsgPrimitive3D, CsgSolid};
use ifc_geometry::solid::halfspace::{BoxedHalfSpace, HalfSpaceSolid, PolygonalBoundedHalfSpace};
use ifc_model::{EntityId, Model, Transaction, Value};

/// A placement at the origin, which every primitive needs.
fn origin(tx: &mut Transaction) -> EntityId {
    let point = cartesian_point(tx, &[0.0, 0.0, 0.0]).expect("point");
    axis2_placement_3d(tx, point, None, None)
}

/// Read a real out of an absolute slot.
fn real_at(model: &Model, id: EntityId, index: usize) -> f64 {
    match &model.get(id).expect("entity").attributes[index] {
        Value::Real(v) => *v,
        other => panic!("slot {index} is not a real: {other:?}"),
    }
}

/// Every primitive writes its dimensions in schema order from slot 1.
///
/// The cone case is the one that matters: `Height` precedes
/// `BottomRadius`, and the two are distinct here so a swap is visible.
#[test]
fn primitives_write_their_dimensions_in_schema_order() {
    let model = Model::default();
    let mut tx = Transaction::new(&model);
    let at = origin(&mut tx);

    let solids = [
        (
            "IFCBLOCK",
            block(&mut tx, at, 3.0, 5.0, 7.0).expect("block"),
            vec![3.0, 5.0, 7.0],
        ),
        (
            "IFCSPHERE",
            sphere(&mut tx, at, 2.5).expect("sphere"),
            vec![2.5],
        ),
        (
            "IFCRIGHTCIRCULARCYLINDER",
            cylinder(&mut tx, at, 4.0, 1.5).expect("cylinder"),
            vec![4.0, 1.5],
        ),
        (
            "IFCRIGHTCIRCULARCONE",
            cone(&mut tx, at, 9.0, 2.0).expect("cone"),
            vec![9.0, 2.0],
        ),
        (
            "IFCRECTANGULARPYRAMID",
            rectangular_pyramid(&mut tx, at, 6.0, 8.0, 10.0).expect("pyramid"),
            vec![6.0, 8.0, 10.0],
        ),
    ];

    let mut model = model;
    tx.commit(&mut model).expect("commit");

    for (type_name, id, dims) in solids {
        let entity = model.get(id).expect("primitive present");
        assert_eq!(entity.type_name.as_ref(), type_name);
        assert_eq!(entity.attributes.len(), 1 + dims.len(), "{type_name} arity");

        // Position is read through the crate's own view, not by index.
        let view = CsgPrimitive3D::new(id, entity);
        assert_eq!(view.position().expect("position"), at, "{type_name}");

        for (offset, expected) in dims.iter().enumerate() {
            assert_eq!(
                real_at(&model, id, 1 + offset),
                *expected,
                "{type_name} dimension {offset}"
            );
        }
    }
}

/// The three half spaces round-trip through their views.
///
/// `AgreementFlag` is the attribute worth pinning: it is a bare boolean
/// that decides which side of the surface is solid, so writing the wrong
/// one produces a valid file denoting the complement of what was meant.
#[test]
fn half_spaces_round_trip_through_their_views() {
    let model = Model::default();
    let mut tx = Transaction::new(&model);
    let at = origin(&mut tx);
    let corner = cartesian_point(&mut tx, &[0.0, 0.0, 0.0]).expect("corner");
    let enclosure = bounding_box(&mut tx, corner, 10.0, 10.0, 10.0).expect("box");

    // A plane stands in for the base surface; the writer takes it as a
    // reference and does not evaluate it.
    let plane = tx.create(ifc_model::Entity::new("IFCPLANE", vec![Value::Ref(at)]));

    let plain = half_space(&mut tx, plane, true);
    let boxed = boxed_half_space(&mut tx, plane, false, enclosure);

    let a = cartesian_point(&mut tx, &[0.0, 0.0]).expect("a");
    let b = cartesian_point(&mut tx, &[4.0, 0.0]).expect("b");
    let c = cartesian_point(&mut tx, &[4.0, 4.0]).expect("c");
    let boundary = polyline(&mut tx, &[a, b, c, a]).expect("boundary");
    let bounded = polygonal_bounded_half_space(&mut tx, plane, true, at, boundary);

    let mut model = model;
    tx.commit(&mut model).expect("commit");

    let view = HalfSpaceSolid::new(plain, model.get(plain).expect("plain"));
    assert_eq!(view.base_surface().expect("base"), plane);
    assert!(view.agreement_flag().expect("flag"));

    let entity = model.get(boxed).expect("boxed");
    let view = BoxedHalfSpace::new(boxed, entity);
    assert!(
        !view.base().agreement_flag().expect("flag"),
        "false must survive"
    );
    assert_eq!(view.enclosure().expect("enclosure"), enclosure);

    let entity = model.get(bounded).expect("bounded");
    let view = PolygonalBoundedHalfSpace::new(bounded, entity);
    assert_eq!(view.position().expect("position"), at);
    assert_eq!(view.polygonal_boundary().expect("boundary"), boundary);
}

/// A CSG solid carries its tree root unresolved.
#[test]
fn a_csg_solid_carries_its_tree_root() {
    let model = Model::default();
    let mut tx = Transaction::new(&model);
    let at = origin(&mut tx);
    let primitive = block(&mut tx, at, 1.0, 2.0, 3.0).expect("block");
    let solid = csg_solid(&mut tx, primitive);

    let mut model = model;
    tx.commit(&mut model).expect("commit");

    let view = CsgSolid::new(solid, model.get(solid).expect("solid"));
    assert_eq!(view.tree_root_expression().expect("root"), primitive);
}

/// Primitives with a non-positive dimension are refused.
///
/// Every dimension on every primitive is `IfcPositiveLengthMeasure`, so
/// unlike the profile fillets there is no legal zero here.
#[test]
fn degenerate_primitives_are_refused() {
    let model = Model::default();
    let mut tx = Transaction::new(&model);
    let at = origin(&mut tx);

    assert!(block(&mut tx, at, 0.0, 1.0, 1.0).is_err(), "zero edge");
    assert!(block(&mut tx, at, 1.0, -1.0, 1.0).is_err(), "negative edge");
    assert!(sphere(&mut tx, at, 0.0).is_err(), "zero radius");
    assert!(cylinder(&mut tx, at, 1.0, 0.0).is_err(), "zero radius");
    assert!(cone(&mut tx, at, 0.0, 1.0).is_err(), "zero height");
    assert!(
        rectangular_pyramid(&mut tx, at, 1.0, 1.0, f64::NAN).is_err(),
        "NaN height"
    );
    assert!(
        sphere(&mut tx, at, f64::INFINITY).is_err(),
        "infinite radius"
    );

    let corner = cartesian_point(&mut tx, &[0.0, 0.0, 0.0]).expect("corner");
    assert!(
        bounding_box(&mut tx, corner, 1.0, 0.0, 1.0).is_err(),
        "a flat bounding box is not expressible"
    );
}
