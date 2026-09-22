//! Type definitions across all three supertype layouts.
//!
//! The catalogue is generated, so these tests check the properties
//! that generation could get wrong: that each entity keeps its own
//! enum, that the two slot layouts stay apart, and that USERDEFINED
//! is refused without its fallback.

use ifc_element_type::table::{
    IFCBEAMTYPE, IFCCREWRESOURCETYPE, IFCDOORTYPE, IFCFURNITURETYPE, IFCPUMPTYPE, IFCSPACETYPE,
    IFCTASKTYPE,
};
use ifc_element_type::{create_type, Family, Slot6, TypeDraft, ALL};
use ifc_model::{Model, Transaction, Value};

const GUID: &str = "3Ss_vDJfz2dgLcbZKkqAd$";

/// An element type writes its predefined type at slot 9 and its tag
/// at slot 7.
#[test]
fn an_element_type_uses_the_element_layout() {
    let model = Model::new();
    let mut tx = Transaction::new(&model);

    let id = create_type(
        &mut tx,
        IFCBEAMTYPE,
        GUID,
        Some("JOIST"),
        TypeDraft {
            name: Some("IPE 300"),
            tag_or_long_description: Some("B-01"),
            ..TypeDraft {
                name: Some("T"),
                ..TypeDraft::default()
            }
        },
    )
    .expect("a well formed beam type is accepted");

    let mut model = model;
    tx.commit(&mut model).expect("commit");

    let e = model.get(id).expect("staged");
    assert_eq!(e.type_name.as_ref(), "IFCBEAMTYPE");
    assert_eq!(e.attributes.len(), 10);
    assert_eq!(e.attributes[2], Value::Text("IPE 300".into()));
    assert_eq!(e.attributes[7], Value::Text("B-01".into()), "Tag at 7");
    assert_eq!(e.attributes[9], Value::Enum("JOIST".into()), "slot 9");
}

/// A resource type puts its predefined type at 11, not 9, because
/// BaseCosts and BaseQuantity sit in between.
#[test]
fn a_resource_type_uses_the_later_predefined_slot() {
    let model = Model::new();
    let mut tx = Transaction::new(&model);

    let id = create_type(
        &mut tx,
        IFCCREWRESOURCETYPE,
        GUID,
        Some("SITE"),
        TypeDraft {
            maps_or_identification: Some(Slot6::Identification("CREW-7")),
            tag_or_long_description: Some("day shift"),
            ..TypeDraft {
                name: Some("T"),
                ..TypeDraft::default()
            }
        },
    )
    .expect("a well formed crew resource type is accepted");

    let mut model = model;
    tx.commit(&mut model).expect("commit");

    let e = model.get(id).expect("staged");
    assert_eq!(e.attributes.len(), 12);
    assert_eq!(
        e.attributes[6],
        Value::Text("CREW-7".into()),
        "Identification at 6"
    );
    assert_eq!(
        e.attributes[7],
        Value::Text("day shift".into()),
        "LongDescription at 7"
    );
    assert_eq!(e.attributes[9], Value::Null, "BaseCosts stays empty");
    assert_eq!(e.attributes[11], Value::Enum("SITE".into()), "slot 11");
}

/// USERDEFINED asserts a name given elsewhere. Without that name the
/// value resolves to nothing, so it is refused.
#[test]
fn userdefined_without_its_fallback_is_refused() {
    let model = Model::new();
    let mut tx = Transaction::new(&model);

    assert!(
        create_type(
            &mut tx,
            IFCPUMPTYPE,
            GUID,
            Some("USERDEFINED"),
            TypeDraft {
                name: Some("T"),
                ..TypeDraft::default()
            }
        )
        .is_err(),
        "USERDEFINED with no ElementType"
    );
    assert!(
        create_type(
            &mut tx,
            IFCPUMPTYPE,
            GUID,
            Some("USERDEFINED"),
            TypeDraft {
                fallback: Some("   "),
                ..TypeDraft {
                    name: Some("T"),
                    ..TypeDraft::default()
                }
            },
        )
        .is_err(),
        "blank is not a name"
    );

    let ok = create_type(
        &mut tx,
        IFCPUMPTYPE,
        GUID,
        Some("USERDEFINED"),
        TypeDraft {
            fallback: Some("borehole pump"),
            ..TypeDraft {
                name: Some("T"),
                ..TypeDraft::default()
            }
        },
    );
    assert!(ok.is_ok(), "named USERDEFINED is accepted");
}

/// Each entity keeps its own enum. A token that is valid on one type
/// is not valid on another merely because both are pump-like.
#[test]
fn a_token_from_another_enum_is_refused() {
    let model = Model::new();
    let mut tx = Transaction::new(&model);

    assert!(
        create_type(
            &mut tx,
            IFCPUMPTYPE,
            GUID,
            Some("JOIST"),
            TypeDraft {
                name: Some("T"),
                ..TypeDraft::default()
            }
        )
        .is_err(),
        "JOIST is a beam token"
    );
    assert!(
        create_type(
            &mut tx,
            IFCBEAMTYPE,
            GUID,
            Some("SITE"),
            TypeDraft {
                name: Some("T"),
                ..TypeDraft::default()
            }
        )
        .is_err(),
        "SITE is a crew token"
    );
    assert!(
        create_type(
            &mut tx,
            IFCBEAMTYPE,
            GUID,
            None,
            TypeDraft {
                name: Some("T"),
                ..TypeDraft::default()
            }
        )
        .is_err(),
        "a required predefined type cannot be omitted"
    );
    assert!(
        create_type(
            &mut tx,
            IFCFURNITURETYPE,
            GUID,
            None,
            TypeDraft {
                name: Some("T"),
                ..TypeDraft::default()
            }
        )
        .is_ok(),
        "furniture declares it optional"
    );
    assert!(
        create_type(
            &mut tx,
            IFCBEAMTYPE,
            "not-a-guid",
            Some("JOIST"),
            TypeDraft {
                name: Some("T"),
                ..TypeDraft::default()
            }
        )
        .is_err(),
        "malformed GlobalId"
    );
}

/// Slot 6 means different things per family, so a value of the wrong
/// shape is refused rather than silently written.
#[test]
fn slot_six_shape_must_match_the_family() {
    let model = Model::new();
    let mut tx = Transaction::new(&model);
    let map = tx.create(ifc_model::Entity::new(
        "IFCREPRESENTATIONMAP",
        vec![Value::Null; 2],
    ));

    assert!(
        create_type(
            &mut tx,
            IFCTASKTYPE,
            GUID,
            Some("CONSTRUCTION"),
            TypeDraft {
                maps_or_identification: Some(Slot6::RepresentationMaps(&[map])),
                ..TypeDraft {
                    name: Some("T"),
                    ..TypeDraft::default()
                }
            },
        )
        .is_err(),
        "a task type has no representation maps"
    );
    assert!(
        create_type(
            &mut tx,
            IFCBEAMTYPE,
            GUID,
            Some("JOIST"),
            TypeDraft {
                maps_or_identification: Some(Slot6::Identification("X")),
                ..TypeDraft {
                    name: Some("T"),
                    ..TypeDraft::default()
                }
            },
        )
        .is_err(),
        "a beam type has no identification at 6"
    );
}

/// Catalogue-wide invariants. Generation is mechanical, so these
/// assert the properties a bad regenerate would break rather than
/// restating 132 rows.
#[test]
fn the_catalogue_holds_its_invariants() {
    assert_eq!(ALL.len(), 132, "concrete IfcTypeObject subtypes");

    for t in ALL {
        assert_eq!(
            t.fallback_slot, 8,
            "{}: fallback is always slot 8",
            t.type_name
        );
        assert!(
            t.predefined_slot > t.fallback_slot,
            "{}: predefined type follows the fallback",
            t.type_name
        );
        assert!(
            t.predefined_slot < t.arity,
            "{}: predefined slot inside the entity",
            t.type_name
        );
        assert!(
            t.members.contains(&"USERDEFINED"),
            "{}: the rule is meaningless without the token",
            t.type_name
        );
        assert!(t.type_name.starts_with("IFC"), "{}", t.type_name);
        assert!(t.type_name.ends_with("TYPE"), "{}", t.type_name);
    }

    let resource = ALL
        .iter()
        .filter(|t| t.fallback_attr == "ResourceType")
        .count();
    assert_eq!(resource, 6, "six resource types");
    let process = ALL
        .iter()
        .filter(|t| t.fallback_attr == "ProcessType")
        .count();
    assert_eq!(process, 3, "event, procedure, task");

    assert_eq!(IFCSPACETYPE.fallback_attr, "ElementType");
    assert_eq!(IFCDOORTYPE.predefined_slot, 9);
    assert_eq!(IFCFURNITURETYPE.predefined_slot, 10);
    assert_eq!(
        ALL.iter().filter(|t| t.predefined_optional).count(),
        2,
        "only furniture and system furniture make it optional"
    );
    assert_eq!(IFCCREWRESOURCETYPE.family, Family::ResourceOrProcess);
    assert_eq!(IFCBEAMTYPE.family, Family::Element);
}

/// The USERDEFINED fallback is written, not merely validated.
///
/// Checking the rule and then dropping the value would satisfy every
/// refusal test while emitting the exact file the rule forbids.
#[test]
fn the_fallback_name_reaches_slot_eight() {
    let model = Model::new();
    let mut tx = Transaction::new(&model);

    let pump = create_type(
        &mut tx,
        IFCPUMPTYPE,
        GUID,
        Some("USERDEFINED"),
        TypeDraft {
            fallback: Some("borehole pump"),
            ..TypeDraft {
                name: Some("T"),
                ..TypeDraft::default()
            }
        },
    )
    .expect("named USERDEFINED");

    let crew = create_type(
        &mut tx,
        IFCCREWRESOURCETYPE,
        GUID,
        Some("USERDEFINED"),
        TypeDraft {
            fallback: Some("night gang"),
            ..TypeDraft {
                name: Some("T"),
                ..TypeDraft::default()
            }
        },
    )
    .expect("named USERDEFINED");

    let mut model = model;
    tx.commit(&mut model).expect("commit");

    assert_eq!(
        model.get(pump).expect("staged").attributes[8],
        Value::Text("borehole pump".into()),
        "ElementType at 8"
    );
    assert_eq!(
        model.get(crew).expect("staged").attributes[8],
        Value::Text("night gang".into()),
        "ResourceType at 8, same slot, different name"
    );
}
