//! Property sets authored, then resolved back.
//!
//! The crate could resolve property sets and not write one. These
//! tests assert through the real resolver, not by reading slots back:
//! an authored set is only correct if the resolver finds it.

use ifc_model::codec::Codec;
use ifc_model::{Entity, EntityId, Model, Transaction, Value};
use ifc_properties::{
    add_property_set, add_property_single_value, attach_property_set, properties_of,
};
use ifc_step::StepCodec;

/// A wall to hang properties on.
fn wall(tx: &mut Transaction, guid: &str) -> EntityId {
    let mut attributes = vec![Value::Null; 8];
    attributes[0] = Value::Text(guid.into());
    tx.create(Entity::new("IFCWALL", attributes))
}

/// An authored set attaches to its object and resolves with values.
#[test]
fn an_authored_set_resolves_onto_its_object() {
    let mut model = Model::default();
    let mut tx = Transaction::new(&model);
    let target = wall(&mut tx, "0aBcDeFgHiJkLmNoPqRsTu");
    tx.commit(&mut model).expect("commit the wall");

    let mut tx = Transaction::new(&model);
    let height = add_property_single_value(
        &mut tx,
        "Height",
        Some("clear height"),
        Some(Value::Typed {
            type_name: "IFCLENGTHMEASURE".into(),
            value: Box::new(Value::Real(3.2)),
        }),
        None,
    )
    .expect("authored height");
    let fire = add_property_single_value(&mut tx, "FireRating", None, None, None)
        .expect("authored fire rating");
    let pset = add_property_set(
        &mut tx,
        "1aBcDeFgHiJkLmNoPqRsTu",
        "Pset_WallCommon",
        None,
        &[("Height", height), ("FireRating", fire)],
    )
    .expect("authored pset");
    attach_property_set(&mut tx, &model, "2aBcDeFgHiJkLmNoPqRsTu", &[target], pset)
        .expect("attached");
    tx.commit(&mut model).expect("commit");

    let sets = properties_of(&model, target);
    assert_eq!(sets.len(), 1, "one set resolves onto the wall");
    assert_eq!(sets[0].set.name.as_deref(), Some("Pset_WallCommon"));
    assert_eq!(sets[0].set.properties.len(), 2);
    let height_prop = &sets[0].set.properties[0];
    assert_eq!(height_prop.description.as_deref(), Some("clear height"));
}

/// The schema WHERE rules are enforced at authoring time.
///
/// Each of these produces a file that parses. None of them
/// survives the rules the schema states about it.
#[test]
fn the_schema_where_rules_are_enforced() {
    let model = Model::default();
    let mut tx = Transaction::new(&model);
    let p = add_property_single_value(&mut tx, "A", None, None, None).expect("property");

    // ExistsName: a nameless set cannot be looked up by name.
    let g = "1aBcDeFgHiJkLmNoPqRsTu";
    assert!(add_property_set(&mut tx, g, "   ", None, &[("A", p)]).is_err());

    // HasProperties is required: an empty set states nothing.
    assert!(add_property_set(&mut tx, g, "Pset_X", None, &[]).is_err());

    // UniquePropertyNames: a duplicate name makes lookup ambiguous,
    // and the resolver would silently pick one.
    let q = add_property_single_value(&mut tx, "A", None, None, None).expect("property");
    let dup = add_property_set(&mut tx, g, "Pset_X", None, &[("A", p), ("A", q)]);
    assert!(dup.is_err(), "duplicate property names are refused");

    // A blank property name names nothing.
    assert!(add_property_single_value(&mut tx, "  ", None, None, None).is_err());
}

/// A type object is refused: NoRelatedTypeObject.
///
/// A type carries properties through IfcRelDefinesByType. Attaching
/// one here parses, and is then read by nothing.
#[test]
fn a_type_object_cannot_take_a_property_set() {
    let mut model = Model::default();
    let mut tx = Transaction::new(&model);
    let mut attributes = vec![Value::Null; 10];
    attributes[0] = Value::Text("3aBcDeFgHiJkLmNoPqRsTu".into());
    let wall_type = tx.create(Entity::new("IFCWALLTYPE", attributes));
    let occurrence = wall(&mut tx, "04BcDeFgHiJkLmNoPqRsTu");
    tx.commit(&mut model).expect("commit");

    let mut tx = Transaction::new(&model);
    let p = add_property_single_value(&mut tx, "A", None, None, None).expect("property");
    let g = "05BcDeFgHiJkLmNoPqRsTu";
    let pset = add_property_set(&mut tx, g, "Pset_X", None, &[("A", p)]).expect("pset");
    let h = "06BcDeFgHiJkLmNoPqRsTu";
    let refused = attach_property_set(&mut tx, &model, h, &[wall_type], pset);
    assert!(refused.is_err(), "a type object is refused");

    // The same call on the occurrence is accepted.
    let ok = attach_property_set(&mut tx, &model, h, &[occurrence], pset);
    assert!(ok.is_ok(), "an occurrence takes the set");
}

/// An attachment to nothing is refused.
///
/// RelatedObjects is required. An empty set parses, and then the
/// pset is attached to nothing while looking attached.
#[test]
fn an_attachment_to_no_objects_is_refused() {
    let model = Model::default();
    let mut tx = Transaction::new(&model);
    let p = add_property_single_value(&mut tx, "A", None, None, None).expect("property");
    let g = "07BcDeFgHiJkLmNoPqRsTu";
    let pset = add_property_set(&mut tx, g, "Pset_X", None, &[("A", p)]).expect("pset");
    let h = "08BcDeFgHiJkLmNoPqRsTu";
    let refused = attach_property_set(&mut tx, &model, h, &[], pset);
    assert!(refused.is_err(), "an empty object list is refused");
}

/// The authored set survives STEP text and still resolves.
///
/// The tests above stay in memory. A real consumer writes a file
/// and parses it, which is where a slot mistake shows up.
#[test]
fn the_authored_set_survives_step_text() {
    let mut model = Model::default();
    let mut tx = Transaction::new(&model);
    let target = wall(&mut tx, "09BcDeFgHiJkLmNoPqRsTu");
    tx.commit(&mut model).expect("commit the wall");

    let mut tx = Transaction::new(&model);
    let p = add_property_single_value(&mut tx, "Height", None, None, None).expect("property");
    let g = "0ABcDeFgHiJkLmNoPqRsTu";
    let pset =
        add_property_set(&mut tx, g, "Pset_WallCommon", None, &[("Height", p)]).expect("pset");
    let h = "0BBcDeFgHiJkLmNoPqRsTu";
    attach_property_set(&mut tx, &model, h, &[target], pset).expect("attached");
    tx.commit(&mut model).expect("commit");

    let mut bytes = Vec::new();
    StepCodec.write(&model, &mut bytes).expect("written");
    let reparsed = StepCodec.read_bytes(&bytes).expect("reparsed");

    // Resolve on the REPARSED model: slots survived the text.
    let sets = properties_of(&reparsed, target);
    assert_eq!(sets.len(), 1, "the set still resolves after a round trip");
    assert_eq!(sets[0].set.name.as_deref(), Some("Pset_WallCommon"));
}
