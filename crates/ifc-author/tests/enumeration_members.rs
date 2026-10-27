//! An enumeration constant must be a member of the declared enumeration, as
//! spelled by the declared release's own tables.

use ifc_author::{AuthorError, EntityBuilder};
use ifc_model::Value;
use ifc_schema::Schema;

fn beam(schema: &Schema, constant: &str) -> Result<ifc_model::Entity, AuthorError> {
    EntityBuilder::new(schema, "IfcBeam")
        .text("GlobalId", "3vB2YO$MX4xv5uCqZZG05x")
        .set("PredefinedType", Value::Enum(constant.into()))
        .build()
}

fn accepted(schema: &Schema, constant: &str) {
    if let Err(error) = beam(schema, constant) {
        panic!("`.{constant}.` refused: {error}");
    }
}

#[test]
fn piercap_is_refused_on_ifc4_and_accepted_on_ifc4x3() {
    match beam(ifc_schema::ifc4(), "PIERCAP") {
        Err(AuthorError::NotEnumerationMember {
            entity,
            attribute,
            enumeration,
            constant,
            allowed,
        }) => {
            assert_eq!(entity, "IfcBeam");
            assert_eq!(attribute, "PredefinedType");
            assert_eq!(enumeration, "IfcBeamTypeEnum");
            assert_eq!(constant, "PIERCAP");
            assert!(allowed.iter().any(|m| m == "JOIST"));
            assert!(!allowed.iter().any(|m| m == "PIERCAP"));
        }
        other => panic!("expected NotEnumerationMember, got {other:?}"),
    }
    accepted(ifc_schema::ifc4x3(), "PIERCAP");
}

#[test]
fn valid_members_are_accepted_in_any_case() {
    accepted(ifc_schema::ifc4(), "JOIST");
    accepted(ifc_schema::ifc4(), "USERDEFINED");
    accepted(ifc_schema::ifc4(), "joist");
}

#[test]
fn refusal_message_names_type_constant_and_members() {
    let message = beam(ifc_schema::ifc4(), "PIERCAP").unwrap_err().to_string();
    assert!(message.contains("IfcBeamTypeEnum"), "{message}");
    assert!(message.contains(".PIERCAP."), "{message}");
    assert!(message.contains("JOIST"), "{message}");
}
