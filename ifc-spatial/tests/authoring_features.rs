//! Feature relationships: voids, fills, projections, and positioning.
//!
//! The pair worth testing is void/fill. They describe one physical
//! arrangement -- a door in a wall -- from opposite ends, and the
//! schema inverts which end is `Relating`. Writing one with the
//! other's direction produces a file that parses and means the
//! opposite thing.

use ifc_model::{Model, Transaction, Value};
use ifc_spatial::authoring::{
    adhere_to_element, assign_to_resource, associate_profile_def, fill_element, position_products,
    project_element, void_element,
};

const GUID_A: &str = "0EI0MSHbX9gg8Fxwar7lb8";
const GUID_B: &str = "1EI0MSHbX9gg8Fxwar7lb8";

fn ids(model: &Model, tx: &mut Transaction, n: usize) -> Vec<ifc_model::EntityId> {
    let _ = model;
    (0..n)
        .map(|_| tx.create(ifc_model::Entity::new("IFCWALL", vec![Value::Null; 9])))
        .collect()
}

/// A wall is voided by an opening: the wall is the relating end.
#[test]
fn a_void_puts_the_element_first() {
    let model = Model::new();
    let mut tx = Transaction::new(&model);
    let e = ids(&model, &mut tx, 2);
    let (wall, opening) = (e[0], e[1]);

    let id = void_element(&mut tx, GUID_A, wall, opening).expect("a well formed void");

    let mut model = model;
    tx.commit(&mut model).expect("commit");
    let rel = model.get(id).expect("staged");
    assert_eq!(rel.type_name.as_ref(), "IFCRELVOIDSELEMENT");
    assert_eq!(rel.attributes[4], Value::Ref(wall), "element relating");
    assert_eq!(rel.attributes[5], Value::Ref(opening), "opening related");
}

/// A door fills an opening: the opening is the relating end, the
/// inverse of a void. This is the assertion that catches a copied
/// implementation.
#[test]
fn a_fill_inverts_the_void_direction() {
    let model = Model::new();
    let mut tx = Transaction::new(&model);
    let e = ids(&model, &mut tx, 2);
    let (opening, door) = (e[0], e[1]);

    let id = fill_element(&mut tx, GUID_A, opening, door).expect("a well formed fill");

    let mut model = model;
    tx.commit(&mut model).expect("commit");
    let rel = model.get(id).expect("staged");
    assert_eq!(rel.type_name.as_ref(), "IFCRELFILLSELEMENT");
    assert_eq!(rel.attributes[4], Value::Ref(opening), "opening relating");
    assert_eq!(rel.attributes[5], Value::Ref(door), "door related");
}

/// The related end is a reference, not a one-element list.
#[test]
fn a_single_related_end_is_not_a_list() {
    let model = Model::new();
    let mut tx = Transaction::new(&model);
    let e = ids(&model, &mut tx, 2);

    let id = project_element(&mut tx, GUID_A, e[0], e[1]).expect("a well formed projection");

    let mut model = model;
    tx.commit(&mut model).expect("commit");
    let rel = model.get(id).expect("staged");
    assert!(
        matches!(rel.attributes[5], Value::Ref(_)),
        "a single feature is a Ref, not a List"
    );
}

/// An element cannot void, fill, or project from itself.
#[test]
fn an_element_cannot_relate_to_itself() {
    let model = Model::new();
    let mut tx = Transaction::new(&model);
    let e = ids(&model, &mut tx, 1);

    assert!(void_element(&mut tx, GUID_A, e[0], e[0]).is_err());
    assert!(fill_element(&mut tx, GUID_A, e[0], e[0]).is_err());
    assert!(project_element(&mut tx, GUID_A, e[0], e[0]).is_err());
}

/// `NoSelfReference` on IfcRelPositions: the positioning element may
/// not appear among the products it positions.
#[test]
fn positioning_refuses_self_reference() {
    let model = Model::new();
    let mut tx = Transaction::new(&model);
    let e = ids(&model, &mut tx, 2);

    assert!(
        position_products(&mut tx, GUID_A, e[0], &[e[1], e[0]]).is_err(),
        "NoSelfReference"
    );
    assert!(
        position_products(&mut tx, GUID_B, e[0], &[]).is_err(),
        "an empty product set positions nothing"
    );
    assert!(position_products(&mut tx, GUID_A, e[0], &[e[1]]).is_ok());
}

/// The assigns and associates families put RelatedObjects at 4, so the
/// relating end lands at 6 and 5 respectively -- not the 4/5 pair the
/// decompose family uses.
#[test]
fn the_assign_families_keep_their_own_slot_order() {
    let model = Model::new();
    let mut tx = Transaction::new(&model);
    let e = ids(&model, &mut tx, 3);
    let (resource, profile) = (e[0], e[1]);

    let a = assign_to_resource(&mut tx, GUID_A, resource, &[e[2]]).expect("assign");
    let b = associate_profile_def(&mut tx, GUID_B, profile, &[e[2]]).expect("associate");

    let mut model = model;
    tx.commit(&mut model).expect("commit");

    let ra = model.get(a).expect("staged");
    assert_eq!(ra.attributes[6], Value::Ref(resource), "resource at 6");
    assert!(matches!(ra.attributes[4], Value::List(_)), "objects at 4");

    let rb = model.get(b).expect("staged");
    assert_eq!(rb.attributes[5], Value::Ref(profile), "profile at 5");
    assert!(matches!(rb.attributes[4], Value::List(_)), "objects at 4");
}

/// Surface features adhere as a set, and an empty set is refused.
#[test]
fn adhesion_takes_a_non_empty_feature_set() {
    let model = Model::new();
    let mut tx = Transaction::new(&model);
    let e = ids(&model, &mut tx, 2);

    assert!(adhere_to_element(&mut tx, GUID_A, e[0], &[]).is_err());
    let id = adhere_to_element(&mut tx, GUID_A, e[0], &[e[1]]).expect("adhere");

    let mut model = model;
    tx.commit(&mut model).expect("commit");
    let rel = model.get(id).expect("staged");
    assert!(matches!(rel.attributes[5], Value::List(_)));
}

/// Each feature relationship writes its own type name.
///
/// Void, fill, and project share a slot layout and a helper, so a
/// copied constant produces a file that parses and asserts the wrong
/// relationship. The type name is the only thing distinguishing them.
#[test]
fn each_feature_relationship_keeps_its_own_type_name() {
    let model = Model::new();
    let mut tx = Transaction::new(&model);
    let e = ids(&model, &mut tx, 2);

    let v = void_element(&mut tx, GUID_A, e[0], e[1]).expect("void");
    let f = fill_element(&mut tx, GUID_B, e[0], e[1]).expect("fill");
    let p = project_element(&mut tx, GUID_A, e[0], e[1]).expect("project");

    let mut model = model;
    tx.commit(&mut model).expect("commit");

    assert_eq!(
        model.get(v).unwrap().type_name.as_ref(),
        "IFCRELVOIDSELEMENT"
    );
    assert_eq!(
        model.get(f).unwrap().type_name.as_ref(),
        "IFCRELFILLSELEMENT"
    );
    assert_eq!(
        model.get(p).unwrap().type_name.as_ref(),
        "IFCRELPROJECTSELEMENT"
    );
}
