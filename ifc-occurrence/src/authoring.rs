//! Authoring built-element and distribution occurrences.
//!
//! # The type pairing is the interesting rule
//!
//! `CorrectPredefinedType` repeats what the type catalogue already
//! enforces. `CorrectTypeAssigned` does not: it says an occurrence
//! may be typed by at most one type, and that type must be the single
//! class the schema pairs with it. An `IfcPump` typed by an
//! `IfcValveType` is not a pump with an unusual type, it is a
//! contradiction: the occurrence claims to be a pump while its shared
//! definition describes a valve.
//!
//! The writer therefore resolves the referenced type entity and
//! compares its STEP class against the pairing recorded in the
//! catalogue. That costs a model lookup, which is why `create` takes
//! a `&Model`: a pairing that is not checked against the real entity
//! is not checked at all.

use ifc_model::guid::Guid;
use ifc_model::{Entity, EntityId, Model, Transaction, Value};

use crate::table::Occurrence;

/// Why an occurrence was refused.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum OccurrenceError {
    /// `GlobalId` did not parse as a 22-character IFC GUID.
    MalformedGuid {
        /// The offending value.
        value: String,
    },
    /// The token is not a member of this class's own enum.
    UnknownPredefinedType {
        /// STEP class.
        entity: &'static str,
        /// The offending token.
        token: String,
    },
    /// The class has no `PredefinedType` attribute at all.
    NoPredefinedType {
        /// STEP class.
        entity: &'static str,
    },
    /// `USERDEFINED` was given without an `ObjectType` naming it.
    UserDefinedWithoutObjectType {
        /// STEP class.
        entity: &'static str,
    },
    /// The occurrence was typed by a class the schema does not pair with it.
    WrongTypeClass {
        /// STEP class of the occurrence.
        entity: &'static str,
        /// The only class `CorrectTypeAssigned` permits.
        expected: &'static str,
        /// What the referenced entity actually is.
        found: String,
    },
    /// The class permits no type at all, but one was supplied.
    TypingNotPermitted {
        /// STEP class.
        entity: &'static str,
    },
    /// The typed-by reference does not resolve in the model.
    UnresolvedType {
        /// The dangling id.
        id: EntityId,
    },
}

/// Result alias for this crate.
pub type OccurrenceResult<T> = Result<T, OccurrenceError>;

/// Attributes shared by every occurrence.
#[derive(Debug, Clone, Copy, Default)]
pub struct OccurrenceDraft<'a> {
    /// `Name`.
    pub name: Option<&'a str>,
    /// `Description`.
    pub description: Option<&'a str>,
    /// `ObjectType`; required when `PredefinedType` is `USERDEFINED`.
    pub object_type: Option<&'a str>,
    /// `ObjectPlacement`.
    pub placement: Option<EntityId>,
    /// `Representation`.
    pub representation: Option<EntityId>,
    /// `Tag`, slot 7 on every class in this catalogue.
    pub tag: Option<&'a str>,
}

fn text(v: Option<&str>) -> Value {
    v.map_or(Value::Null, |s| Value::Text(s.into()))
}

fn slot_ref(v: Option<EntityId>) -> Value {
    v.map_or(Value::Null, Value::Ref)
}

/// Stage one occurrence.
///
/// `typed_by` is the `IfcTypeObject` this occurrence takes its shared
/// definition from, if any. It is checked against the one class the
/// schema pairs with `kind`; see [`OccurrenceError::WrongTypeClass`].
///
/// Refuses a malformed `global_id`, a `predefined_type` outside the
/// class enum, `USERDEFINED` without `draft.object_type`, and a
/// `typed_by` of the wrong class or one that does not resolve.
pub fn create(
    tx: &mut Transaction,
    model: &Model,
    kind: Occurrence,
    global_id: &str,
    predefined_type: Option<&str>,
    typed_by: Option<EntityId>,
    draft: OccurrenceDraft<'_>,
) -> OccurrenceResult<EntityId> {
    let entity = kind.type_name;
    if Guid::parse(global_id).is_none() {
        return Err(OccurrenceError::MalformedGuid {
            value: global_id.into(),
        });
    }
    if let Some(token) = predefined_type {
        if kind.predefined_slot.is_none() {
            return Err(OccurrenceError::NoPredefinedType { entity });
        }
        if !kind.members.contains(&token) {
            return Err(OccurrenceError::UnknownPredefinedType {
                entity,
                token: token.into(),
            });
        }
        if token == "USERDEFINED" && draft.object_type.is_none_or(|s| s.trim().is_empty()) {
            return Err(OccurrenceError::UserDefinedWithoutObjectType { entity });
        }
    }

    if let Some(id) = typed_by {
        let Some(expected) = kind.type_class else {
            return Err(OccurrenceError::TypingNotPermitted { entity });
        };
        let found = model
            .get(id)
            .ok_or(OccurrenceError::UnresolvedType { id })?
            .type_name
            .to_ascii_uppercase();
        if found != expected {
            return Err(OccurrenceError::WrongTypeClass {
                entity,
                expected,
                found,
            });
        }
    }

    let mut attrs = vec![Value::Null; kind.arity];
    attrs[0] = Value::Text(global_id.into());
    attrs[2] = text(draft.name);
    attrs[3] = text(draft.description);
    attrs[4] = text(draft.object_type);
    attrs[5] = slot_ref(draft.placement);
    attrs[6] = slot_ref(draft.representation);
    attrs[7] = text(draft.tag);
    if let (Some(slot), Some(token)) = (kind.predefined_slot, predefined_type) {
        attrs[slot] = Value::Enum(token.into());
    }
    Ok(tx.create(Entity::new(entity, attrs)))
}
