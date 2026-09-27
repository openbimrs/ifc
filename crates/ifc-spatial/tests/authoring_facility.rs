//! Facilities and facility parts.
//!
//! The interesting claim is that the two layouts stay apart: a part
//! carries a mandatory UsageType at 9 and its PredefinedType at 10,
//! while a facility carries PredefinedType at 9 and no usage at all.

use ifc_model::{Entity, EntityId, Model, Transaction, Value};
use ifc_spatial::facility::{
    create_facility, FacilityDraft, FacilityError, ALL, IFCBRIDGE, IFCFACILITY,
    IFCFACILITYPARTCOMMON, IFCROAD, IFCROADPART,
};

const GUID: &str = "0EI0MSHbX9gg8Fxwar7lb8";

fn staged(tx: Transaction, model: &mut Model, id: EntityId) -> Entity {
    tx.commit(model).expect("commit");
    model.get(id).expect("staged").clone()
}

/// A facility puts PredefinedType at 9 and has no usage slot.
#[test]
fn a_facility_keeps_its_slots() {
    let mut model = Model::default();
    let mut tx = Transaction::new(&model);
    let id = create_facility(
        &mut tx,
        IFCBRIDGE,
        GUID,
        Some("SUSPENSION"),
        FacilityDraft {
            name: Some("North crossing"),
            long_name: Some("North river crossing"),
            composition: Some("ELEMENT"),
            ..FacilityDraft::default()
        },
    )
    .expect("a well formed bridge is accepted");

    let e = staged(tx, &mut model, id);
    assert_eq!(e.type_name.as_ref(), "IFCBRIDGE");
    assert_eq!(e.attributes.len(), 10);
    assert_eq!(e.attributes[2], Value::Text("North crossing".into()));
    assert_eq!(e.attributes[7], Value::Text("North river crossing".into()));
    assert_eq!(e.attributes[8], Value::Enum("ELEMENT".into()));
    assert_eq!(e.attributes[9], Value::Enum("SUSPENSION".into()), "slot 9");
}

/// A part pushes PredefinedType to 10 and carries usage at 9.
///
/// Writing a part predefined type at 9 would land it in UsageType,
/// where a reader would take a road-part kind for a usage token.
#[test]
fn a_part_keeps_usage_and_predefined_apart() {
    let mut model = Model::default();
    let mut tx = Transaction::new(&model);
    let id = create_facility(
        &mut tx,
        IFCROADPART,
        GUID,
        Some("ROADSEGMENT"),
        FacilityDraft {
            usage: Some("LONGITUDINAL"),
            ..FacilityDraft::default()
        },
    )
    .expect("a well formed road part is accepted");

    let e = staged(tx, &mut model, id);
    assert_eq!(e.attributes.len(), 11);
    assert_eq!(
        e.attributes[9],
        Value::Enum("LONGITUDINAL".into()),
        "UsageType at 9"
    );
    assert_eq!(
        e.attributes[10],
        Value::Enum("ROADSEGMENT".into()),
        "PredefinedType at 10, not 9"
    );
}

/// UsageType is mandatory on a part: the schema does not mark it
/// OPTIONAL, so omitting it writes a file that is missing a required
/// attribute.
#[test]
fn a_part_without_usage_is_refused() {
    let model = Model::default();
    let mut tx = Transaction::new(&model);
    let err = create_facility(
        &mut tx,
        IFCFACILITYPARTCOMMON,
        GUID,
        Some("SEGMENT"),
        FacilityDraft::default(),
    )
    .expect_err("a part without usage is refused");
    assert!(
        matches!(err, FacilityError::MissingUsageType { .. }),
        "{err}"
    );
}

/// A facility has no usage slot, so offering one is a caller error
/// rather than something to write somewhere plausible.
#[test]
fn a_facility_refuses_a_usage_token() {
    let model = Model::default();
    let mut tx = Transaction::new(&model);
    let err = create_facility(
        &mut tx,
        IFCROAD,
        GUID,
        None,
        FacilityDraft {
            usage: Some("LONGITUDINAL"),
            ..FacilityDraft::default()
        },
    )
    .expect_err("a facility has no usage slot");
    assert!(
        matches!(err, FacilityError::UnexpectedUsageType { .. }),
        "{err}"
    );
}

/// Two independent USERDEFINED rules. Either enum may claim a kind
/// of its own, and each needs ObjectType to name it; satisfying one
/// does not satisfy the other.
#[test]
fn both_userdefined_rules_are_enforced() {
    let model = Model::default();

    let mut tx = Transaction::new(&model);
    let err = create_facility(
        &mut tx,
        IFCROADPART,
        GUID,
        Some("USERDEFINED"),
        FacilityDraft {
            usage: Some("LONGITUDINAL"),
            ..FacilityDraft::default()
        },
    )
    .expect_err("USERDEFINED predefined type needs ObjectType");
    assert!(
        matches!(err, FacilityError::UserDefinedWithoutObjectType { .. }),
        "{err}"
    );

    let mut tx = Transaction::new(&model);
    let err = create_facility(
        &mut tx,
        IFCROADPART,
        GUID,
        None,
        FacilityDraft {
            usage: Some("USERDEFINED"),
            ..FacilityDraft::default()
        },
    )
    .expect_err("USERDEFINED usage needs ObjectType too");
    assert!(
        matches!(err, FacilityError::UserDefinedWithoutObjectType { .. }),
        "{err}"
    );

    let mut tx = Transaction::new(&model);
    create_facility(
        &mut tx,
        IFCROADPART,
        GUID,
        Some("USERDEFINED"),
        FacilityDraft {
            usage: Some("USERDEFINED"),
            object_type: Some("Bespoke verge"),
            ..FacilityDraft::default()
        },
    )
    .expect("ObjectType satisfies both rules at once");
}

/// Each class keeps its own enum: a bridge kind is not a road kind.
#[test]
fn enum_membership_is_per_entity() {
    let model = Model::default();
    let mut tx = Transaction::new(&model);
    let err = create_facility(
        &mut tx,
        IFCROAD,
        GUID,
        Some("SUSPENSION"),
        FacilityDraft::default(),
    )
    .expect_err("SUSPENSION is a bridge kind, not a road kind");
    assert!(matches!(err, FacilityError::UnknownToken { .. }), "{err}");

    let mut tx = Transaction::new(&model);
    create_facility(
        &mut tx,
        IFCBRIDGE,
        GUID,
        Some("SUSPENSION"),
        FacilityDraft::default(),
    )
    .expect("the same token is valid on a bridge");
}

/// IfcFacility itself carries neither enum.
#[test]
fn the_base_facility_has_no_enums() {
    let model = Model::default();
    let mut tx = Transaction::new(&model);
    let err = create_facility(
        &mut tx,
        IFCFACILITY,
        GUID,
        Some("ANYTHING"),
        FacilityDraft::default(),
    )
    .expect_err("IfcFacility declares no PredefinedType");
    assert!(
        matches!(err, FacilityError::NoPredefinedType { .. }),
        "{err}"
    );

    let mut tx = Transaction::new(&model);
    create_facility(&mut tx, IFCFACILITY, GUID, None, FacilityDraft::default())
        .expect("a plain facility is fine");
}

/// Catalogue-wide invariants a bad regeneration would break.
#[test]
fn the_catalogue_holds_its_invariants() {
    assert_eq!(ALL.len(), 10);
    let parts = ALL.iter().filter(|f| f.usage_slot.is_some()).count();
    assert_eq!(parts, 5, "five part classes carry UsageType");

    for f in ALL {
        if let Some(usage) = f.usage_slot {
            assert_eq!(usage, 9, "{} usage slot", f.type_name);
            assert_eq!(
                f.predefined_slot,
                Some(10),
                "{} pushes predefined past usage",
                f.type_name
            );
        } else if let Some(slot) = f.predefined_slot {
            assert_eq!(slot, 9, "{} predefined slot", f.type_name);
        }
        assert!(f.arity >= 9, "{} arity", f.type_name);
    }
}

/// UsageType has its own fixed vocabulary, shared by every part.
///
/// A road-part kind is not a usage token: offering one must be
/// refused rather than written into slot 9, where a reader would
/// take it for a usage.
#[test]
fn an_unknown_usage_token_is_refused() {
    let model = Model::default();
    let mut tx = Transaction::new(&model);
    let err = create_facility(
        &mut tx,
        IFCROADPART,
        GUID,
        None,
        FacilityDraft {
            usage: Some("ROADSEGMENT"),
            ..FacilityDraft::default()
        },
    )
    .expect_err("ROADSEGMENT is a part kind, not a usage");
    assert!(matches!(err, FacilityError::UnknownToken { .. }), "{err}");
}

/// GlobalId is how every relationship names this container, so a
/// malformed one is refused at construction rather than written.
#[test]
fn a_malformed_guid_is_refused() {
    let model = Model::default();
    let mut tx = Transaction::new(&model);
    let err = create_facility(&mut tx, IFCBRIDGE, "nope", None, FacilityDraft::default())
        .expect_err("a four character guid is not a guid");
    assert!(matches!(err, FacilityError::MalformedGuid { .. }), "{err}");
}
