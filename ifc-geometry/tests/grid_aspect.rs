//! Authoring shape aspects and grids.

use ifc_geometry::authoring::{grid, shape_aspect};
use ifc_model::{Entity, EntityId, Model, Transaction, Value};

const GUID: &str = "1jQ2A$rnvCJhUvFV5RxFtz";

fn axis(tx: &mut Transaction) -> EntityId {
    tx.create(Entity::new("IFCGRIDAXIS", vec![Value::Null; 3]))
}

/// ProductDefinitional is a logical, so absent means UNKNOWN.
///
/// Collapsing it to false would assert that the aspect does not
/// define the product shape, which is a different claim from saying
/// nothing.
#[test]
fn an_unset_product_definitional_is_unknown_not_false() {
    let mut model = Model::default();
    let mut tx = Transaction::new(&model);
    let representation = tx.create(Entity::new("IFCSHAPEREPRESENTATION", vec![Value::Null; 4]));

    let unknown = shape_aspect(&mut tx, &[representation], Some("Web"), None, None, None)
        .expect("shape aspect");
    let stated = shape_aspect(&mut tx, &[representation], None, None, Some(false), None)
        .expect("shape aspect");
    tx.commit(&mut model).expect("commit");

    let staged = model.get(unknown).expect("staged");
    assert_eq!(staged.type_name.as_ref(), "IFCSHAPEASPECT");
    assert_eq!(staged.attributes.len(), 5);
    assert_eq!(staged.attributes[3], Value::LogicalUnknown);
    assert_eq!(
        model.get(stated).expect("staged").attributes[3],
        Value::Bool(false),
        "an explicit false is not the same as unset",
    );
}

/// An aspect representing nothing is refused.
#[test]
fn an_empty_shape_aspect_is_refused() {
    let model = Model::default();
    let mut tx = Transaction::new(&model);
    assert!(
        shape_aspect(&mut tx, &[], None, None, None, None).is_err(),
        "an empty LIST [1:?] was accepted",
    );
}

/// A grid stages its eleven slots with the W axes optional.
#[test]
fn a_grid_stages() {
    let mut model = Model::default();
    let mut tx = Transaction::new(&model);
    let (u1, u2) = (axis(&mut tx), axis(&mut tx));
    let v1 = axis(&mut tx);

    let id = grid(
        &mut tx,
        GUID,
        None,
        (&[u1, u2], &[v1], &[]),
        Some("RECTANGULAR"),
    )
    .expect("grid");
    tx.commit(&mut model).expect("commit");

    let staged = model.get(id).expect("staged");
    assert_eq!(staged.type_name.as_ref(), "IFCGRID");
    assert_eq!(staged.attributes.len(), 11);
    assert_eq!(
        staged.attributes[7],
        Value::List(vec![Value::Ref(u1), Value::Ref(u2)]),
    );
    assert_eq!(staged.attributes[8], Value::List(vec![Value::Ref(v1)]));
    assert_eq!(staged.attributes[9], Value::Null, "WAxes stays absent");
    assert_eq!(staged.attributes[10], Value::Enum("RECTANGULAR".into()));
}

/// UNIQUE: an axis may not appear twice in one list.
///
/// A repeated axis makes the grid ambiguous about which intersection
/// a gridline names, so it is refused rather than deduplicated.
#[test]
fn a_repeated_grid_axis_is_refused() {
    let model = Model::default();
    let mut tx = Transaction::new(&model);
    let u = axis(&mut tx);
    let v = axis(&mut tx);

    assert!(
        grid(&mut tx, GUID, None, (&[u, u], &[v], &[]), None).is_err(),
        "a repeated U axis was accepted",
    );
    assert!(
        grid(&mut tx, GUID, None, (&[u], &[v], &[v, v]), None).is_err(),
        "a repeated W axis was accepted",
    );
    // The same axis in different directions is legal: UNIQUE binds
    // within a list, not across them.
    grid(&mut tx, GUID, None, (&[u], &[u], &[]), None)
        .expect("an axis may appear in both directions");
}

/// A grid needs at least one axis in each of U and V.
#[test]
fn a_grid_without_axes_is_refused() {
    let model = Model::default();
    let mut tx = Transaction::new(&model);
    let a = axis(&mut tx);

    assert!(
        grid(&mut tx, GUID, None, (&[], &[a], &[]), None).is_err(),
        "a grid with no U axes was accepted",
    );
    assert!(
        grid(&mut tx, GUID, None, (&[a], &[], &[]), None).is_err(),
        "a grid with no V axes was accepted",
    );
}
