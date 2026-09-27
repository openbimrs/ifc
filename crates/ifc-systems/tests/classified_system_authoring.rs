//! Authoring the three classified systems.
//!
//! The three share slots 0-4 and then disagree: a building or
//! built system keeps PredefinedType at 5 and LongName at 6, a
//! distribution circuit has them the other way round. A writer
//! that assumes one layout files a system classification as a
//! display name on the other.

use ifc_model::{Model, Transaction, Value};
use ifc_schema::{ifc4, ifc4x3};
use ifc_systems::authoring::{create_classified_system, ClassifiedSystemDraft, SystemKind};

const GUID: &str = "1jQ2A$rnvCJhUvFV5RxFtz";

/// The slot swap between the systems and the circuit.
#[test]
fn each_class_lands_its_tail_in_the_declared_slots() {
    let mut model = Model::default();
    let mut tx = Transaction::new(&model);
    let building = create_classified_system(
        &mut tx,
        ifc4x3(),
        SystemKind::Building,
        GUID,
        Some("Shell"),
        ClassifiedSystemDraft {
            long_name: Some("Outer shell system"),
            predefined_type: Some("OUTERSHELL"),
            ..ClassifiedSystemDraft::default()
        },
    )
    .expect("building system");
    let circuit = create_classified_system(
        &mut tx,
        ifc4x3(),
        SystemKind::DistributionCircuit,
        "3rT4B$mkwDKiVwGW6SyGua",
        Some("Ring"),
        ClassifiedSystemDraft {
            long_name: Some("Lighting ring main"),
            predefined_type: Some("ELECTRICAL"),
            ..ClassifiedSystemDraft::default()
        },
    )
    .expect("distribution circuit");
    tx.commit(&mut model).expect("commit");

    let b = model.get(building).expect("building");
    assert_eq!(b.type_name.as_ref(), "IFCBUILDINGSYSTEM");
    assert_eq!(
        b.attributes[5],
        Value::Enum("OUTERSHELL".into()),
        "slot 5 is the type"
    );
    assert_eq!(
        b.attributes[6],
        Value::Text("Outer shell system".into()),
        "slot 6 is the name"
    );

    let c = model.get(circuit).expect("circuit");
    assert_eq!(c.type_name.as_ref(), "IFCDISTRIBUTIONCIRCUIT");
    assert_eq!(
        c.attributes[5],
        Value::Text("Lighting ring main".into()),
        "slot 5 is the name"
    );
    assert_eq!(
        c.attributes[6],
        Value::Enum("ELECTRICAL".into()),
        "slot 6 is the type"
    );
}

/// Each class carries its own enum.
///
/// `IfcBuiltSystemTypeEnum` has MOORING; the building enum does
/// not, and a token borrowed across them is refused.
#[test]
fn a_token_from_a_sibling_enum_is_refused() {
    let model = Model::default();
    let mut tx = Transaction::new(&model);
    create_classified_system(
        &mut tx,
        ifc4x3(),
        SystemKind::Built,
        GUID,
        None,
        ClassifiedSystemDraft {
            predefined_type: Some("MOORING"),
            ..ClassifiedSystemDraft::default()
        },
    )
    .expect("MOORING is a built-system token");
    create_classified_system(
        &mut tx,
        ifc4x3(),
        SystemKind::Building,
        GUID,
        None,
        ClassifiedSystemDraft {
            predefined_type: Some("MOORING"),
            ..ClassifiedSystemDraft::default()
        },
    )
    .expect_err("not a building-system token");
}

/// USERDEFINED without ObjectType names a kind and withholds it.
#[test]
fn userdefined_without_an_object_type_is_refused() {
    let model = Model::default();
    let mut tx = Transaction::new(&model);
    for class in [SystemKind::Building, SystemKind::Built] {
        create_classified_system(
            &mut tx,
            ifc4x3(),
            class,
            GUID,
            None,
            ClassifiedSystemDraft {
                predefined_type: Some("USERDEFINED"),
                ..ClassifiedSystemDraft::default()
            },
        )
        .expect_err("CorrectPredefinedType");
        create_classified_system(
            &mut tx,
            ifc4x3(),
            class,
            GUID,
            None,
            ClassifiedSystemDraft {
                predefined_type: Some("USERDEFINED"),
                object_type: Some("Green roof assembly"),
                ..ClassifiedSystemDraft::default()
            },
        )
        .expect("named kind is accepted");
    }
    assert!(tx.len() == 2, "only the two named drafts staged");
}

/// `IfcBuiltSystem` is IFC4X3 only.
///
/// It replaces `IfcBuildingSystem`, which IFC4X3 deprecates but
/// still declares. A file targeting IFC4 must use the older name.
#[test]
fn the_built_system_is_absent_from_ifc4() {
    assert!(ifc4().entity("IfcBuildingSystem").is_some(), "IFC4 has it");
    assert!(ifc4().entity("IfcBuiltSystem").is_none(), "IFC4 does not");
    assert!(
        ifc4x3().entity("IfcBuiltSystem").is_some(),
        "IFC4X3 adds it"
    );
    assert!(
        ifc4x3().entity("IfcBuildingSystem").is_some(),
        "IFC4X3 keeps the deprecated name declared"
    );
}

/// The hardcoded slots and enums match the shipped schemas.
#[test]
fn declared_layouts_match_the_schema() {
    for (class, entity, type_slot, name_slot) in [
        (SystemKind::Building, "IfcBuildingSystem", 5, 6),
        (SystemKind::Built, "IfcBuiltSystem", 5, 6),
        (
            SystemKind::DistributionCircuit,
            "IfcDistributionCircuit",
            6,
            5,
        ),
    ] {
        let _ = class;
        let names = ifc4x3().attribute_names(entity);
        assert_eq!(names.len(), 7, "{entity} arity");
        assert!(
            names[type_slot].eq_ignore_ascii_case("PredefinedType"),
            "{entity} type slot"
        );
        assert!(
            names[name_slot].eq_ignore_ascii_case("LongName"),
            "{entity} name slot"
        );
    }
}
