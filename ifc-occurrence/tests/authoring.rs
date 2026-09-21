//! Occurrences, and the link to their type definition.
//!
//! The catalogue is generated, so these check what generation could
//! get wrong: each class keeping its own enum, the predefined slot
//! moving when extra attributes precede it, and the type pairing.

use ifc_model::{Entity, EntityId, Model, Transaction, Value};
use ifc_occurrence::table::{
    ALL, IFCDOOR, IFCELEMENTASSEMBLY, IFCFURNISHINGELEMENT, IFCPILE, IFCPUMP, IFCVALVE, IFCWALL,
};
use ifc_occurrence::{create, OccurrenceDraft, OccurrenceError};

const GUID: &str = "3vB2YO$MX4xv5uCqZZG05x";
const OTHER: &str = "1kTvXnbbzCWw8lcMd1dR4o";

/// A wall writes its predefined type at slot 8 and its tag at 7.
#[test]
fn an_occurrence_keeps_its_slots() {
    let mut model = Model::new();
    let mut tx = Transaction::new(&model);
    let id = create(
        &mut tx,
        &model,
        IFCWALL,
        GUID,
        Some("SOLIDWALL"),
        None,
        OccurrenceDraft {
            name: Some("W-01"),
            tag: Some("wall-tag"),
            ..OccurrenceDraft::default()
        },
    )
    .expect("a well formed wall is accepted");
    tx.commit(&mut model).expect("commit");

    let e = model.get(id).expect("staged");
    assert_eq!(e.type_name.as_ref(), "IFCWALL");
    assert_eq!(e.attributes.len(), 9);
    assert_eq!(e.attributes[0], Value::Text(GUID.into()));
    assert_eq!(e.attributes[2], Value::Text("W-01".into()));
    assert_eq!(e.attributes[7], Value::Text("wall-tag".into()), "Tag at 7");
    assert_eq!(e.attributes[8], Value::Enum("SOLIDWALL".into()), "slot 8");
}
/// A door puts its predefined type at 10, not 8: OverallHeight and
/// OverallWidth precede it. A fixed slot would write the enum into a
/// length measure.
#[test]
fn extra_attributes_push_the_predefined_slot() {
    let mut model = Model::new();
    let mut tx = Transaction::new(&model);
    let door = create(
        &mut tx,
        &model,
        IFCDOOR,
        GUID,
        Some("DOOR"),
        None,
        OccurrenceDraft::default(),
    )
    .expect("a well formed door");
    let pile = create(
        &mut tx,
        &model,
        IFCPILE,
        OTHER,
        Some("BORED"),
        None,
        OccurrenceDraft::default(),
    )
    .expect("a well formed pile");
    tx.commit(&mut model).expect("commit");

    assert_eq!(IFCDOOR.predefined_slot, Some(10));
    let d = model.get(door).expect("door");
    assert_eq!(d.attributes.len(), 13);
    assert_eq!(d.attributes[10], Value::Enum("DOOR".into()));
    assert_eq!(d.attributes[8], Value::Null, "OverallHeight stays empty");

    // IfcPile keeps slot 8 but adds ConstructionType after it.
    assert_eq!(IFCPILE.predefined_slot, Some(8));
    let p = model.get(pile).expect("pile");
    assert_eq!(p.attributes.len(), 10);
    assert_eq!(p.attributes[8], Value::Enum("BORED".into()));
}

/// An occurrence may only be typed by its own type class.
///
/// A pump typed by a valve type is the schema violation this crate
/// exists to prevent: it silently reassigns meaning, and nothing
/// downstream can tell the pump was ever intended.
#[test]
fn an_occurrence_refuses_a_foreign_type_class() {
    let mut model = Model::new();
    let valve_type = EntityId(1);
    model.insert(
        valve_type,
        Entity::new("IFCVALVETYPE", vec![Value::Text(OTHER.into()); 10]),
    );

    let mut tx = Transaction::new(&model);
    let err = create(
        &mut tx,
        &model,
        IFCPUMP,
        GUID,
        None,
        Some(valve_type),
        OccurrenceDraft::default(),
    )
    .expect_err("a pump is not a valve");
    match err {
        OccurrenceError::WrongTypeClass {
            expected, found, ..
        } => {
            assert_eq!(expected, "IFCPUMPTYPE");
            assert_eq!(found, "IFCVALVETYPE");
        }
        other => panic!("expected WrongTypeClass, got {other:?}"),
    }
}

/// The matching type class is accepted, and the wrong enum token and
/// a bare USERDEFINED are both refused.
#[test]
fn the_paired_type_is_accepted_and_bad_tokens_are_not() {
    let mut model = Model::new();
    let pump_type = EntityId(1);
    model.insert(
        pump_type,
        Entity::new("IFCPUMPTYPE", vec![Value::Text(OTHER.into()); 10]),
    );

    let mut tx = Transaction::new(&model);
    create(
        &mut tx,
        &model,
        IFCPUMP,
        GUID,
        Some("SUBMERSIBLEPUMP"),
        Some(pump_type),
        OccurrenceDraft::default(),
    )
    .expect("a pump typed by a pump type");

    let wrong = create(
        &mut tx,
        &model,
        IFCPUMP,
        GUID,
        Some("SOLIDWALL"),
        None,
        OccurrenceDraft::default(),
    )
    .expect_err("a wall token is not a pump token");
    assert!(matches!(
        wrong,
        OccurrenceError::UnknownPredefinedType { .. }
    ));

    let bare = create(
        &mut tx,
        &model,
        IFCPUMP,
        GUID,
        Some("USERDEFINED"),
        None,
        OccurrenceDraft::default(),
    )
    .expect_err("USERDEFINED names nothing without ObjectType");
    assert!(matches!(
        bare,
        OccurrenceError::UserDefinedWithoutObjectType { .. }
    ));
}

/// Ten classes carry no PredefinedType at all. Offering one is a
/// caller error, not something to write into a slot that is not there.
#[test]
fn a_class_without_a_predefined_type_refuses_one() {
    let model = Model::new();
    let mut tx = Transaction::new(&model);
    assert_eq!(IFCFURNISHINGELEMENT.predefined_slot, None);
    let err = create(
        &mut tx,
        &model,
        IFCFURNISHINGELEMENT,
        GUID,
        Some("ANYTHING"),
        None,
        OccurrenceDraft::default(),
    )
    .expect_err("no slot to write it into");
    assert!(matches!(err, OccurrenceError::NoPredefinedType { .. }));

    create(
        &mut tx,
        &model,
        IFCFURNISHINGELEMENT,
        GUID,
        None,
        None,
        OccurrenceDraft::default(),
    )
    .expect("but the class itself is writable");
}

/// Catalogue-wide invariants a bad regeneration would break.
#[test]
fn the_catalogue_holds_its_invariants() {
    assert_eq!(ALL.len(), 141);

    for t in ALL {
        // Slots 0..=7 are the IfcElement prefix every class shares.
        assert!(t.arity >= 8, "{} has arity {}", t.type_name, t.arity);
        if let Some(slot) = t.predefined_slot {
            assert!(slot >= 8, "{} predefined at {slot}", t.type_name);
            assert!(slot < t.arity, "{} slot out of range", t.type_name);
            assert!(!t.members.is_empty(), "{} has no tokens", t.type_name);
        } else {
            assert!(t.members.is_empty(), "{} tokens with no slot", t.type_name);
        }
        if let Some(class) = t.type_class {
            assert!(class.ends_with("TYPE"), "{class} is not a type class");
        }
    }

    let typed = ALL.iter().filter(|t| t.type_class.is_some()).count();
    assert_eq!(typed, 122, "CorrectTypeAssigned pairs 122 classes");
    let no_enum = ALL.iter().filter(|t| t.predefined_slot.is_none()).count();
    assert_eq!(no_enum, 10);

    assert_eq!(IFCPUMP.type_class, Some("IFCPUMPTYPE"));
    assert_eq!(IFCVALVE.type_class, Some("IFCVALVETYPE"));
    assert_eq!(IFCELEMENTASSEMBLY.predefined_slot, Some(9));
}

/// A blank ObjectType names nothing, so it does not satisfy
/// USERDEFINED. Whitespace is not a name.
#[test]
fn a_blank_object_type_does_not_satisfy_userdefined() {
    let model = Model::new();
    let mut tx = Transaction::new(&model);
    let err = create(
        &mut tx,
        &model,
        IFCWALL,
        GUID,
        Some("USERDEFINED"),
        None,
        OccurrenceDraft {
            object_type: Some("   "),
            ..OccurrenceDraft::default()
        },
    )
    .expect_err("blank is not a name");
    assert!(matches!(
        err,
        OccurrenceError::UserDefinedWithoutObjectType { .. }
    ));
}

/// A GlobalId must be a real 22-character IFC GUID.
#[test]
fn a_malformed_global_id_is_refused() {
    let model = Model::new();
    let mut tx = Transaction::new(&model);
    let err = create(
        &mut tx,
        &model,
        IFCWALL,
        "nope",
        None,
        None,
        OccurrenceDraft::default(),
    )
    .expect_err("four characters is not a GUID");
    assert!(matches!(err, OccurrenceError::MalformedGuid { .. }));
}
