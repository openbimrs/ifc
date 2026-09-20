//! The remaining objectified relationships.
//!
//! The slot-order trap is the thing under test. `IfcRelAggregates`
//! puts the parent at 4 and children at 5; `IfcRelDefinesByObject`
//! reverses that. A writer assuming one uniform layout inverts the
//! relationship and still produces parsable STEP, so these tests
//! assert positions rather than just round-tripping.

use ifc_model::{Entity, Model, Transaction, Value};
use ifc_spatial::authoring::{
    assign_to_actor, assign_to_group_by_factor, assign_to_process, assign_to_product,
    connect_elements, connect_with_realizing_elements, control_flow_element, cover_elements,
    cover_spaces, declare, define_by_object, interfere_elements, serve_buildings,
};

const GUID_A: &str = "0EI0MSHbX9gg8Fxwar7lb8";
const GUID_B: &str = "1EI0MSHbX9gg8Fxwar7lb9";

fn element(tx: &mut Transaction) -> ifc_model::EntityId {
    tx.create(Entity::new("IFCWALL", vec![Value::Null; 8]))
}

/// `IfcRelDefinesByObject` puts the related set first.
///
/// This is the inversion that matters: slot 4 is `RelatedObjects` and
/// slot 5 is `RelatingObject`, the opposite of every aggregate-shaped
/// relationship. Getting it wrong makes the defining object look like
/// one of the things it defines.
#[test]
fn defines_by_object_reverses_the_usual_slot_order() {
    let model = Model::default();
    let mut tx = Transaction::new(&model);

    let defining = element(&mut tx);
    let defined_a = element(&mut tx);
    let defined_b = element(&mut tx);

    let id = define_by_object(&mut tx, GUID_A, defining, &[defined_a, defined_b])
        .expect("a well formed relationship is accepted");

    let mut model = model;
    tx.commit(&mut model).expect("commit");

    let entity = model.get(id).expect("staged");
    assert_eq!(entity.type_name.as_ref(), "IFCRELDEFINESBYOBJECT");
    assert_eq!(
        entity.attributes[5],
        Value::Ref(defining),
        "RelatingObject is slot 5 here, not slot 4"
    );
    assert_eq!(
        entity.attributes[4],
        Value::List(vec![Value::Ref(defined_a), Value::Ref(defined_b)]),
        "RelatedObjects is slot 4"
    );
}

/// The set-valued relationships refuse an empty set and self-membership.
///
/// A relationship with no children is not a relationship; a parent
/// listed among its own children is a cycle the spatial reader will
/// follow.
#[test]
fn set_relationships_refuse_empty_and_self_membership() {
    let model = Model::default();
    let mut tx = Transaction::new(&model);

    let parent = element(&mut tx);
    let child = element(&mut tx);

    assert!(
        cover_elements(&mut tx, GUID_A, parent, &[]).is_err(),
        "an empty covering set is refused"
    );
    assert!(
        cover_spaces(&mut tx, GUID_A, parent, &[parent]).is_err(),
        "a space cannot be its own covering"
    );
    assert!(
        declare(&mut tx, GUID_A, parent, &[parent, child]).is_err(),
        "a context cannot declare itself"
    );
    assert!(
        serve_buildings(&mut tx, GUID_A, parent, &[]).is_err(),
        "a system must serve something"
    );
    assert!(
        control_flow_element(&mut tx, GUID_A, parent, &[parent]).is_err(),
        "a flow element cannot control itself"
    );
    assert!(
        assign_to_actor(&mut tx, "not-a-guid", parent, &[child]).is_err(),
        "a malformed GlobalId is refused"
    );

    assert!(
        assign_to_product(&mut tx, GUID_A, parent, &[child]).is_ok(),
        "a well formed assignment is accepted"
    );
    assert!(
        assign_to_process(&mut tx, GUID_B, parent, &[child]).is_ok(),
        "a well formed assignment is accepted"
    );
}

/// A pair relationship refuses an element connected to itself.
///
/// The connectivity reader walks these as a graph, so a self-edge is
/// a one-element cycle rather than a harmless oddity.
#[test]
fn pair_relationships_refuse_self_connection() {
    let model = Model::default();
    let mut tx = Transaction::new(&model);

    let a = element(&mut tx);
    let b = element(&mut tx);

    assert!(
        connect_elements(&mut tx, GUID_A, a, a).is_err(),
        "an element cannot connect to itself"
    );
    assert!(
        interfere_elements(&mut tx, GUID_A, a, a, None).is_err(),
        "an element cannot interfere with itself"
    );

    let id = connect_elements(&mut tx, GUID_A, a, b).expect("distinct elements connect");

    let mut model = model;
    tx.commit(&mut model).expect("commit");

    let entity = model.get(id).expect("staged");
    assert_eq!(entity.attributes[5], Value::Ref(a), "RelatingElement");
    assert_eq!(entity.attributes[6], Value::Ref(b), "RelatedElement");
}

/// Realizing elements are what make the subtype meaningful.
///
/// Without them the record is an `IfcRelConnectsElements` wearing a
/// more specific type name, which is why an empty set is refused
/// rather than written as `$`.
#[test]
fn a_realizing_connection_needs_realizing_elements() {
    let model = Model::default();
    let mut tx = Transaction::new(&model);

    let a = element(&mut tx);
    let b = element(&mut tx);
    let weld = element(&mut tx);

    assert!(
        connect_with_realizing_elements(&mut tx, GUID_A, a, b, &[]).is_err(),
        "the subtype exists to name the realizing elements"
    );

    let id = connect_with_realizing_elements(&mut tx, GUID_A, a, b, &[weld])
        .expect("a well formed realizing connection is accepted");

    let mut model = model;
    tx.commit(&mut model).expect("commit");

    let entity = model.get(id).expect("staged");
    assert_eq!(
        entity.attributes[7],
        Value::List(vec![Value::Ref(weld)]),
        "RealizingElements is slot 7"
    );
}

/// `ImpliedOrder` is a three-valued logical, and UNKNOWN is honest.
///
/// A clash detector that reports an overlap without deciding which
/// element gives way has no basis for TRUE or FALSE. Writing `$`
/// instead of UNKNOWN would be a different claim: absent rather than
/// indeterminate.
#[test]
fn implied_order_writes_unknown_rather_than_null() {
    let model = Model::default();
    let mut tx = Transaction::new(&model);

    let a = element(&mut tx);
    let b = element(&mut tx);

    let unknown = interfere_elements(&mut tx, GUID_A, a, b, None).expect("accepted");
    let ordered = interfere_elements(&mut tx, GUID_B, a, b, Some(true)).expect("accepted");

    let mut model = model;
    tx.commit(&mut model).expect("commit");

    assert_eq!(
        model.get(unknown).expect("staged").attributes[8],
        Value::LogicalUnknown,
        "no decision means UNKNOWN, not absent"
    );
    assert_eq!(
        model.get(ordered).expect("staged").attributes[8],
        Value::Bool(true),
        "an explicit order is written as TRUE"
    );
}

/// The factor is a ratio, and a non-finite one is refused.
#[test]
fn a_group_factor_must_be_finite() {
    let model = Model::default();
    let mut tx = Transaction::new(&model);

    let group = element(&mut tx);
    let member = element(&mut tx);

    assert!(
        assign_to_group_by_factor(&mut tx, GUID_A, group, &[member], f64::NAN).is_err(),
        "a non-finite ratio is refused"
    );

    let id = assign_to_group_by_factor(&mut tx, GUID_A, group, &[member], 0.25)
        .expect("a finite ratio is accepted");

    let mut model = model;
    tx.commit(&mut model).expect("commit");

    let entity = model.get(id).expect("staged");
    assert_eq!(
        entity.attributes[7],
        Value::Real(0.25),
        "Factor is slot 7, after RelatingGroup"
    );
}

/// Each set-valued writer stages its own entity name.
///
/// These all funnel through one `relate` helper, so a writer
/// passing the wrong `RelSlots` constant produces a perfectly valid
/// record of the wrong type. Only the type name catches it.
#[test]
fn each_set_writer_stages_its_own_type() {
    let model = Model::default();
    let mut tx = Transaction::new(&model);

    let parent = element(&mut tx);
    let child = element(&mut tx);

    let staged = [
        (
            cover_elements(&mut tx, GUID_A, parent, &[child]),
            "IFCRELCOVERSBLDGELEMENTS",
        ),
        (
            cover_spaces(&mut tx, GUID_A, parent, &[child]),
            "IFCRELCOVERSSPACES",
        ),
        (declare(&mut tx, GUID_A, parent, &[child]), "IFCRELDECLARES"),
        (
            serve_buildings(&mut tx, GUID_A, parent, &[child]),
            "IFCRELSERVICESBUILDINGS",
        ),
        (
            control_flow_element(&mut tx, GUID_A, parent, &[child]),
            "IFCRELFLOWCONTROLELEMENTS",
        ),
        (
            assign_to_actor(&mut tx, GUID_A, parent, &[child]),
            "IFCRELASSIGNSTOACTOR",
        ),
        (
            assign_to_product(&mut tx, GUID_A, parent, &[child]),
            "IFCRELASSIGNSTOPRODUCT",
        ),
        (
            assign_to_process(&mut tx, GUID_A, parent, &[child]),
            "IFCRELASSIGNSTOPROCESS",
        ),
    ];

    let ids: Vec<_> = staged
        .into_iter()
        .map(|(r, name)| (r.expect("accepted"), name))
        .collect();

    let mut model = model;
    tx.commit(&mut model).expect("commit");

    for (id, name) in ids {
        assert_eq!(
            model.get(id).expect("staged").type_name.as_ref(),
            name,
            "each writer must stage {name}"
        );
    }
}

/// The pair writers refuse a malformed GlobalId.
///
/// They do not go through `relate`, so they carry their own guid
/// check and need their own coverage.
#[test]
fn pair_writers_refuse_a_malformed_guid() {
    let model = Model::default();
    let mut tx = Transaction::new(&model);

    let a = element(&mut tx);
    let b = element(&mut tx);

    assert!(
        connect_elements(&mut tx, "not-a-guid", a, b).is_err(),
        "a 22-character base64 GlobalId is required"
    );
    assert!(
        interfere_elements(&mut tx, "still-not-a-guid", a, b, None).is_err(),
        "a clash record carries a GlobalId too"
    );
    assert!(
        connect_with_realizing_elements(&mut tx, "nope", a, b, &[a]).is_err(),
        "the realizing subtype carries one as well"
    );
}
