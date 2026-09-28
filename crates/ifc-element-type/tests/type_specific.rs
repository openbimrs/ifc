//! #214: the type-specific attributes four types require, and the refusals
//! around them.
//!
//! From the EXPRESS sources, IFC4 ADD2 TC1 and IFC4X3 ADD2 declare
//! `IfcDoorType.OperationType`, `IfcWindowType.PartitioningType`,
//! `IfcEventType.EventTriggerType` and `IfcFurnitureType.AssemblyPlace`
//! without `OPTIONAL` (IFC2X3 TC1 declares only the last). `create_type`
//! wrote `$` into them before; every writer now refuses instead, staging
//! nothing.

use ifc_element_type::table::{
    IFCDOORTYPE, IFCEVENTTYPE, IFCFURNITURETYPE, IFCWALLTYPE, IFCWINDOWTYPE,
};
use ifc_element_type::{create_type, create_type_in, ElementTypeError, TypeDraft};
use ifc_model::{Model, Transaction};
use ifc_schema::SchemaVersion;

const GUID: &str = "1hqA$FMcT8$hVvcqsRDBzZ";

fn refused(
    kind: ifc_element_type::ElementType,
    token: &str,
    draft: TypeDraft<'_>,
) -> ElementTypeError {
    let model = Model::new();
    let mut tx = Transaction::new(&model);
    let error = create_type(&mut tx, kind, GUID, Some(token), draft)
        .expect_err("refused, not written with `$`");
    assert!(tx.edits().is_empty(), "nothing staged on {error:?}");
    error
}

#[test]
fn a_required_type_specific_attribute_left_unset_is_refused() {
    for (kind, token, attribute) in [
        (IFCDOORTYPE, "DOOR", "OperationType"),
        (IFCWINDOWTYPE, "WINDOW", "PartitioningType"),
        (IFCEVENTTYPE, "STARTEVENT", "EventTriggerType"),
        (IFCFURNITURETYPE, "CHAIR", "AssemblyPlace"),
    ] {
        assert_eq!(
            refused(kind, token, TypeDraft::new().name("T")),
            ElementTypeError::AuthoringRequired {
                entity: kind.type_name,
                attribute,
                schema: SchemaVersion::Ifc4x3,
            }
        );
    }
}

#[test]
fn a_token_outside_the_attributes_enumeration_is_refused() {
    let error = refused(
        IFCDOORTYPE,
        "DOOR",
        TypeDraft::new().name("T").operation_type("SINGLE_PANEL"),
    );
    assert_eq!(
        error,
        ElementTypeError::Invalid {
            entity: "IFCDOORTYPE",
            attribute: "OperationType",
            value: "SINGLE_PANEL".into(),
        }
    );
    let error = refused(
        IFCFURNITURETYPE,
        "CHAIR",
        TypeDraft::new().name("T").assembly_place("GARAGE"),
    );
    assert!(
        matches!(
            error,
            ElementTypeError::Invalid {
                attribute: "AssemblyPlace",
                ..
            }
        ),
        "{error:?}"
    );
}

#[test]
fn a_userdefined_trigger_needs_its_name() {
    for label in [None, Some("  ")] {
        let mut draft = TypeDraft::new().name("T").event_trigger_type("USERDEFINED");
        draft.user_defined_event_trigger_type = label;
        assert_eq!(
            refused(IFCEVENTTYPE, "STARTEVENT", draft),
            ElementTypeError::Invalid {
                entity: "IFCEVENTTYPE",
                attribute: "UserDefinedEventTriggerType",
                value: "required by USERDEFINED".into(),
            }
        );
    }
}

#[test]
fn a_value_for_an_attribute_the_type_does_not_declare_is_refused() {
    let error = refused(
        IFCWALLTYPE,
        "SOLIDWALL",
        TypeDraft::new()
            .name("T")
            .operation_type("SINGLE_SWING_LEFT"),
    );
    assert_eq!(
        error,
        ElementTypeError::AuthoringNotInSchema {
            entity: "IFCWALLTYPE",
            attribute: "OperationType",
            schema: SchemaVersion::Ifc4x3,
        }
    );
    let error = refused(
        IFCWALLTYPE,
        "SOLIDWALL",
        TypeDraft::new().name("T").parameter_takes_precedence(true),
    );
    assert_eq!(
        error,
        ElementTypeError::AuthoringNotInSchema {
            entity: "IFCWALLTYPE",
            attribute: "ParameterTakesPrecedence",
            schema: SchemaVersion::Ifc4x3,
        }
    );
}

/// The model-bound writer refuses the same way in the bound release.
#[test]
fn the_model_bound_writer_refuses_in_its_release() {
    let model = Model::new(); // binds IFC4
    let mut tx = Transaction::new(&model);
    let error = create_type_in(
        &mut tx,
        &model,
        IFCDOORTYPE,
        GUID,
        Some("DOOR"),
        TypeDraft::new().name("T"),
    )
    .expect_err("OperationType is required");
    assert_eq!(
        error,
        ElementTypeError::AuthoringRequired {
            entity: "IFCDOORTYPE",
            attribute: "OperationType",
            schema: SchemaVersion::Ifc4,
        }
    );
    assert!(tx.edits().is_empty());
}
