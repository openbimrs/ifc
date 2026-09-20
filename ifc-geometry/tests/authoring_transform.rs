//! Transformation operators, maps and mapped items.
//!
//! `Scale2` lives at slot 4 on the 2D non-uniform operator and slot 5
//! on the 3D one, because `Axis3` comes between. A writer that used
//! one layout for both would put a real where a direction belongs.

use ifc_geometry::authoring::{
    axis2_placement_3d, cartesian_point, direction, mapped_item, representation_map,
    topology_representation, transformation_operator_2d, transformation_operator_2d_non_uniform,
    transformation_operator_3d, transformation_operator_3d_non_uniform, vertex_point, Transform,
};
use ifc_geometry::resource::operator::CartesianTransformationOperator;
use ifc_model::{Model, Transaction, Value};

/// Each non-uniform operator writes `Scale2` where its own schema says.
#[test]
fn scale2_sits_at_a_different_slot_in_each_branch() {
    let model = Model::default();
    let mut tx = Transaction::new(&model);
    let origin2d = cartesian_point(&mut tx, &[0.0, 0.0]).expect("2d origin");
    let origin3d = cartesian_point(&mut tx, &[0.0, 0.0, 0.0]).expect("3d origin");
    let z = direction(&mut tx, &[0.0, 0.0, 1.0]).expect("z");

    let flat = transformation_operator_2d_non_uniform(
        &mut tx,
        origin2d,
        Transform {
            scale: Some(2.0),
            ..Transform::default()
        },
        Some(3.0),
    )
    .expect("2d non uniform");

    let solid = transformation_operator_3d_non_uniform(
        &mut tx,
        origin3d,
        Transform {
            scale: Some(2.0),
            ..Transform::default()
        },
        Some(z),
        Some(3.0),
        Some(4.0),
    )
    .expect("3d non uniform");

    let mut model = model;
    tx.commit(&mut model).expect("commit");

    // 2D: no Axis3, so Scale2 follows Scale directly.
    let entity = model.get(flat).expect("2d");
    assert_eq!(entity.attributes.len(), 5);
    assert_eq!(entity.attributes[3], Value::Real(2.0), "Scale");
    assert_eq!(entity.attributes[4], Value::Real(3.0), "Scale2 at slot 4");

    // 3D: Axis3 occupies slot 4, pushing the extra scales to 5 and 6.
    let entity = model.get(solid).expect("3d");
    assert_eq!(entity.attributes.len(), 7);
    assert_eq!(entity.attributes[3], Value::Real(2.0), "Scale");
    assert_eq!(entity.attributes[4], Value::Ref(z), "Axis3 at slot 4");
    assert_eq!(entity.attributes[5], Value::Real(3.0), "Scale2 at slot 5");
    assert_eq!(entity.attributes[6], Value::Real(4.0), "Scale3 at slot 6");

    // The reader agrees about which scale is which.
    let view = CartesianTransformationOperator::new(solid, model.get(solid).expect("3d"));
    assert_eq!(view.scale().expect("scale"), 2.0);
    // The origin resolves to coordinates, so compare those.
    assert_eq!(view.local_origin(&model).expect("origin"), [0.0, 0.0, 0.0]);
}

/// An absent scale means one; an explicit zero collapses the geometry.
///
/// `Scl := NVL(Scale, 1.0)` with `Scl > 0.0`, so `None` is legal and
/// zero is not. Conflating the two would reject every operator that
/// simply omits its scale, which is most of them.
#[test]
fn an_absent_scale_is_one_and_zero_is_refused() {
    let model = Model::default();
    let mut tx = Transaction::new(&model);
    let at = cartesian_point(&mut tx, &[0.0, 0.0]).expect("origin");

    let default = transformation_operator_2d(&mut tx, at, Transform::default())
        .expect("an operator may omit its scale");

    assert!(
        transformation_operator_2d(
            &mut tx,
            at,
            Transform {
                scale: Some(0.0),
                ..Transform::default()
            }
        )
        .is_err(),
        "ScaleGreaterZero: a zero scale collapses everything to a point"
    );
    assert!(
        transformation_operator_2d(
            &mut tx,
            at,
            Transform {
                scale: Some(-1.0),
                ..Transform::default()
            }
        )
        .is_err(),
        "a negative scale is a reflection the schema does not allow here"
    );
    assert!(
        transformation_operator_2d_non_uniform(&mut tx, at, Transform::default(), Some(0.0))
            .is_err(),
        "Scale2 is held to the same rule"
    );
    assert!(
        transformation_operator_3d_non_uniform(
            &mut tx,
            at,
            Transform::default(),
            None,
            Some(1.0),
            Some(-2.0),
        )
        .is_err(),
        "Scale3 too"
    );

    let mut model = model;
    tx.commit(&mut model).expect("commit");

    let entity = model.get(default).expect("default");
    assert_eq!(
        entity.attributes[3],
        Value::Null,
        "an omitted scale is `$`, and the schema derives it to 1.0"
    );
    let view = CartesianTransformationOperator::new(default, entity);
    assert_eq!(
        view.scale().expect("scale"),
        1.0,
        "the reader applies the same default"
    );
}

/// A map is authored once and placed by each mapped item.
#[test]
fn a_representation_map_is_shared_by_its_mapped_items() {
    let model = Model::default();
    let mut tx = Transaction::new(&model);
    let point = cartesian_point(&mut tx, &[0.0, 0.0, 0.0]).expect("point");
    let placement = axis2_placement_3d(&mut tx, point, None, None);
    let vertex = vertex_point(&mut tx, point);
    let context = cartesian_point(&mut tx, &[0.0, 0.0, 0.0]).expect("stand-in context");

    let shape = topology_representation(&mut tx, context, Some("Body"), Some("Brep"), &[vertex])
        .expect("representation");
    assert!(
        topology_representation(&mut tx, context, None, None, &[]).is_err(),
        "SET [1:?] needs an item"
    );

    let map = representation_map(&mut tx, placement, shape);
    let here =
        transformation_operator_3d(&mut tx, point, Transform::default(), None).expect("operator");
    let there = transformation_operator_3d(
        &mut tx,
        point,
        Transform {
            scale: Some(2.0),
            ..Transform::default()
        },
        None,
    )
    .expect("operator");
    let first = mapped_item(&mut tx, map, here);
    let second = mapped_item(&mut tx, map, there);

    let mut model = model;
    tx.commit(&mut model).expect("commit");

    let entity = model.get(map).expect("map");
    assert_eq!(entity.attributes[0], Value::Ref(placement));
    assert_eq!(entity.attributes[1], Value::Ref(shape));

    // Both items point at the same source and differ only in target.
    let a = model.get(first).expect("first");
    let b = model.get(second).expect("second");
    assert_eq!(a.attributes[0], b.attributes[0], "one shared source map");
    assert_eq!(a.attributes[0], Value::Ref(map));
    assert_ne!(a.attributes[1], b.attributes[1], "placed differently");

    let entity = model.get(shape).expect("shape");
    assert_eq!(entity.attributes[3], Value::List(vec![Value::Ref(vertex)]));
}

/// Axis1 and Axis2 are the local X and Y, and are not interchangeable.
///
/// A transposition swaps the frame's handedness while leaving the
/// record structurally perfect. Both axes have to be authored, and
/// distinct, for the test to see it.
#[test]
fn the_two_axes_keep_their_order() {
    let model = Model::default();
    let mut tx = Transaction::new(&model);
    let at = cartesian_point(&mut tx, &[0.0, 0.0, 0.0]).expect("origin");
    let x = direction(&mut tx, &[1.0, 0.0, 0.0]).expect("x");
    let y = direction(&mut tx, &[0.0, 1.0, 0.0]).expect("y");

    let id = transformation_operator_3d(
        &mut tx,
        at,
        Transform {
            axis1: Some(x),
            axis2: Some(y),
            scale: None,
        },
        None,
    )
    .expect("operator");

    let mut model = model;
    tx.commit(&mut model).expect("commit");

    let entity = model.get(id).expect("operator");
    assert_eq!(entity.attributes[0], Value::Ref(x), "Axis1 is the local X");
    assert_eq!(entity.attributes[1], Value::Ref(y), "Axis2 is the local Y");
    assert_ne!(
        entity.attributes[0], entity.attributes[1],
        "a transposition would swap the frame's handedness"
    );
}
