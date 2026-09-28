//! Staging `IfcRelAssignsToControl` for the controls this crate owns.
//!
//! # Who writes the assignment
//!
//! The crate that owns the relating control writes the assignment.
//! `ifc-cost` binds items to its cost schedules, `ifc-schedule` binds
//! tasks to its work controls, and this module binds work to a permit,
//! a project order, an action request or a performance history. A
//! relating control of any other type is refused here, even though the
//! schema admits every `IfcControl`: the owning crate knows which
//! objects make sense under it, this one does not.
//!
//! `IfcPerformanceHistory` is included because it is an `IfcControl`
//! subtype in IFC4 ADD2 TC1 and IFC4X3 ADD2, so `RelatingControl :
//! IfcControl` admits it; the schema draws no distinction among the four.
//!
//! # What the schema requires
//!
//! `IfcRelAssigns.RelatedObjects` is `SET [1:?] OF IfcObjectDefinition`:
//! at least one member, no duplicates, each an object definition.
//! `IfcRelAssignsToControl.NoSelfReference` forbids the relating control
//! among its own related objects. All four are refused before staging.
//!
//! # RelatedObjectsType
//!
//! Always left unset. IFC4 ADD2 TC1 declares it `OPTIONAL
//! IfcObjectTypeEnum` under `WR1 : IfcCorrectObjectAssignment`, which
//! returns TRUE when it is absent; IFC4X3 ADD2 redeclares it `OPTIONAL
//! IfcStrippedOptional`, a BOOLEAN kept only so older files parse. Null
//! is the one value valid in both, and naming the related objects' type
//! would add a constraint the caller never asked for.

use std::collections::HashSet;

use ifc_model::guid::Guid;
use ifc_model::{Entity, EntityId, Model, Transaction, Value};
use ifc_schema::Schema;

use crate::authoring::ControlKind;
use crate::error::{ControlError, ControlResult};
use crate::release::{bind, projected_type, Release};

const RELATION: &str = "IFCRELASSIGNSTOCONTROL";

/// One `IfcRelAssignsToControl` to stage.
#[derive(Debug, Clone, Copy)]
pub struct ControlAssignmentDraft<'a> {
    /// `GlobalId`: a 22-character compressed IFC GUID.
    pub global_id: &'a str,
    /// `Name`.
    pub name: Option<&'a str>,
    /// `Description`.
    pub description: Option<&'a str>,
    /// `RelatingControl`: a permit, project order, action request or
    /// performance history, in the model or staged earlier in the
    /// transaction.
    pub control: EntityId,
    /// `RelatedObjects`: the work the control governs.
    pub related_objects: &'a [EntityId],
}

/// Stage one `IfcRelAssignsToControl` relating work to an owned control.
///
/// References may target the model or entities staged earlier in `tx`.
/// The record is laid out by attribute name in `schema`, so a schema that
/// moves a slot produces a correct record rather than a shifted one.
/// `OwnerHistory` is left unset, which IFC4 and IFC4X3 allow; an IFC2X3
/// `schema` requires it and is refused with
/// [`ControlError::AuthoringRequired`] (use
/// [`assign_to_control_with_owner_history`]).
///
/// # Errors
///
/// Refuses a malformed GlobalId; a relating control or related object
/// that exists neither in `model` nor in `tx`; a relating control that
/// is not one of the four [`ControlKind`]s; an empty or duplicated
/// `RelatedObjects`; the control among its own related objects; a
/// related object that is not an `IfcObjectDefinition`; a schema that
/// does not declare the relationship; and an IFC2X3 schema. Nothing is
/// staged on an error.
pub fn assign_to_control(
    tx: &mut Transaction,
    model: &Model,
    schema: &Schema,
    draft: ControlAssignmentDraft<'_>,
) -> ControlResult<EntityId> {
    let record = assignment_record(tx, model, Release::of_schema(schema), draft, Value::Null)?;
    Ok(tx.create(record))
}

/// [`assign_to_control`] in `model`'s declared release, with a
/// caller-supplied `IfcOwnerHistory`, which IFC2X3 requires (#202).
///
/// The release is bound from `FILE_SCHEMA` (none binds IFC4); in IFC4 and
/// IFC4X3 the record is that of [`assign_to_control`] with the reference in
/// the optional slot. The owner history is never invented.
///
/// # Errors
///
/// Those of [`assign_to_control`] except the IFC2X3 refusal, and the
/// release and owner-history refusals of
/// [`create_control_with_owner_history`](crate::create_control_with_owner_history).
/// Nothing is staged on an error.
pub fn assign_to_control_with_owner_history(
    tx: &mut Transaction,
    model: &Model,
    draft: ControlAssignmentDraft<'_>,
    owner_history: EntityId,
) -> ControlResult<EntityId> {
    let release = bind(model)?;
    let record = assignment_record(tx, model, release, draft, Value::Ref(owner_history))?;
    release.require_owner_history(tx, model, RELATION, owner_history)?;
    Ok(tx.create(record))
}

fn assignment_record(
    tx: &Transaction,
    model: &Model,
    release: Release<'_>,
    draft: ControlAssignmentDraft<'_>,
    owner_history: Value,
) -> ControlResult<Entity> {
    let schema = release.schema();
    if schema.attributes(RELATION).is_empty() {
        return Err(ControlError::UnsupportedEntity {
            schema: schema.name().to_owned(),
            entity: RELATION,
        });
    }

    if Guid::parse(draft.global_id).is_none() {
        return Err(invalid("GlobalId", draft.global_id));
    }

    let control_type = projected_type(tx, model, draft.control)
        .ok_or(ControlError::UnknownEntity { id: draft.control })?;
    if !ControlKind::ALL
        .iter()
        .any(|kind| control_type.eq_ignore_ascii_case(kind.type_name()))
    {
        return Err(ControlError::ForeignControl {
            id: draft.control,
            actual: control_type,
        });
    }

    if draft.related_objects.is_empty() {
        return Err(invalid("RelatedObjects", "expected at least one member"));
    }
    let mut seen = HashSet::new();
    for &object in draft.related_objects {
        if !seen.insert(object) {
            return Err(invalid(
                "RelatedObjects",
                format!("{object} is listed twice"),
            ));
        }
        if object == draft.control {
            // `NoSelfReference`.
            return Err(invalid(
                "RelatedObjects",
                format!("{object} is the relating control"),
            ));
        }
        let actual =
            projected_type(tx, model, object).ok_or(ControlError::UnknownEntity { id: object })?;
        if !schema.is_a(&actual, "IFCOBJECTDEFINITION") {
            return Err(invalid(
                "RelatedObjects",
                format!("{object} is {actual}, not an IfcObjectDefinition"),
            ));
        }
    }

    release.record(
        RELATION,
        vec![
            ("GlobalId", Value::Text(draft.global_id.into())),
            ("OwnerHistory", owner_history),
            ("Name", text(draft.name)),
            ("Description", text(draft.description)),
            (
                "RelatedObjects",
                Value::List(
                    draft
                        .related_objects
                        .iter()
                        .copied()
                        .map(Value::Ref)
                        .collect(),
                ),
            ),
            // RelatedObjectsType: always unset, see the module docs.
            ("RelatedObjectsType", Value::Null),
            ("RelatingControl", Value::Ref(draft.control)),
        ],
    )
}

fn invalid(attribute: &'static str, value: impl Into<String>) -> ControlError {
    ControlError::AuthoringInvalid {
        entity: RELATION,
        attribute,
        value: value.into(),
    }
}

fn text(value: Option<&str>) -> Value {
    value.map_or(Value::Null, |v| Value::Text(v.into()))
}
