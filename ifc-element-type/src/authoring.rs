//! Authoring element, resource, and process type definitions.
//!
//! # What a type definition is for
//!
//! An `IfcXxxType` carries what every occurrence of a product shares:
//! a door type names the operation and panel layout that each door
//! placed from it inherits. Authoring one wrong does not corrupt a
//! single door; it corrupts every door of that type.
//!
//! # The rule this module exists to enforce
//!
//! All 132 types carry `CorrectPredefinedType`:
//!
//! ```text
//! (PredefinedType <> USERDEFINED) OR
//! ((PredefinedType = USERDEFINED) AND EXISTS(<fallback>))
//! ```
//!
//! `USERDEFINED` means "the enum has no token for this, the name is
//! given elsewhere". Without that elsewhere the value asserts a name
//! exists and then withholds it, which no reader can resolve.

use ifc_model::guid::Guid;
use ifc_model::{Entity, EntityId, Transaction, Value};

use crate::table::{ElementType, Family};

/// Why a type definition was refused.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ElementTypeError {
    /// An attribute value the schema does not permit.
    Invalid {
        /// STEP type name.
        entity: &'static str,
        /// Attribute that was rejected.
        attribute: &'static str,
        /// The offending value.
        value: String,
    },
}

impl std::fmt::Display for ElementTypeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let Self::Invalid {
            entity,
            attribute,
            value,
        } = self;
        write!(f, "{entity}.{attribute}: {value}")
    }
}

impl std::error::Error for ElementTypeError {}

/// Result of staging a type definition.
pub type ElementTypeResult<T> = Result<T, ElementTypeError>;

fn invalid(
    entity: &'static str,
    attribute: &'static str,
    value: impl Into<String>,
) -> ElementTypeError {
    ElementTypeError::Invalid {
        entity,
        attribute,
        value: value.into(),
    }
}

/// Attributes shared by every type definition.
///
/// `tag_or_long_description` and `maps_or_identification` occupy slots
/// 7 and 6, whose meaning depends on [`Family`]. Naming them for both
/// readings keeps a caller from assuming the element-type reading on a
/// resource type, where it would file a tag as a description.
#[derive(Debug, Clone, Copy, Default)]
pub struct TypeDraft<'a> {
    /// `Name`.
    pub name: Option<&'a str>,
    /// `Description`.
    pub description: Option<&'a str>,
    /// `ApplicableOccurrence`, slot 4.
    pub applicable_occurrence: Option<&'a str>,
    /// Slot 6: `RepresentationMaps` refs, or `Identification` text.
    pub maps_or_identification: Option<Slot6<'a>>,
    /// Slot 7: `Tag` on element types, `LongDescription` otherwise.
    pub tag_or_long_description: Option<&'a str>,
    /// Slot 8: the `USERDEFINED` fallback. Required when the
    /// predefined type is `USERDEFINED`.
    pub fallback: Option<&'a str>,
}

/// What slot 6 holds, which differs by [`Family`].
#[derive(Debug, Clone, Copy)]
pub enum Slot6<'a> {
    /// `RepresentationMaps`: shape definitions the occurrences map.
    RepresentationMaps(&'a [EntityId]),
    /// `Identification`: a catalogue or article number.
    Identification(&'a str),
}

/// Stage one type definition.
///
/// `predefined_type` must be a token the entity's own enum declares.
/// A token borrowed from a sibling enum is refused: `IfcPumpTypeEnum`
/// has no `SUBMERSIBLEPUMP` member merely because some other pump-like
/// enum does.
///
/// # Errors
///
/// Refuses a malformed GlobalId, a token outside the entity's enum, a
/// missing predefined type where the schema requires one, `USERDEFINED`
/// without the fallback attribute, and a slot-6 value of the wrong
/// shape for the entity's family.
pub fn create_type(
    tx: &mut Transaction,
    kind: ElementType,
    global_id: &str,
    predefined_type: Option<&str>,
    draft: TypeDraft<'_>,
) -> ElementTypeResult<EntityId> {
    let entity = kind.type_name;
    if Guid::parse(global_id).is_none() {
        return Err(invalid(entity, "GlobalId", global_id));
    }
    // `IfcTypeObject.NameRequired` is inherited by all 132 catalogue
    // types. `Name` is OPTIONAL in the slot table and mandatory by
    // rule, so a writer trusting the slot table alone files a nameless
    // type that parses and cannot be referred to.
    if blank(draft.name) {
        return Err(invalid(entity, "Name", "NameRequired"));
    }

    match predefined_type {
        None if !kind.predefined_optional => {
            return Err(invalid(entity, "PredefinedType", "required"));
        }
        Some(token) if !kind.members.contains(&token) => {
            return Err(invalid(entity, "PredefinedType", token));
        }
        Some(token) if token == "USERDEFINED" && blank(draft.fallback) => {
            return Err(invalid(
                entity,
                kind.fallback_attr,
                "required by USERDEFINED",
            ));
        }
        _ => {}
    }

    let mut attrs = vec![Value::Null; kind.arity];
    attrs[0] = Value::Text(global_id.into());
    attrs[2] = text(draft.name);
    attrs[3] = text(draft.description);
    attrs[4] = text(draft.applicable_occurrence);

    match (draft.maps_or_identification, kind.family) {
        (Some(Slot6::RepresentationMaps(maps)), Family::Element) => {
            if maps.is_empty() {
                return Err(invalid(entity, "RepresentationMaps", "empty"));
            }
            attrs[6] = Value::List(maps.iter().copied().map(Value::Ref).collect());
        }
        (Some(Slot6::Identification(id)), Family::ResourceOrProcess) => {
            attrs[6] = Value::Text(id.into());
        }
        (Some(Slot6::RepresentationMaps(_)), Family::ResourceOrProcess) => {
            return Err(invalid(entity, "Identification", "expected text, got maps"));
        }
        (Some(Slot6::Identification(_)), Family::Element) => {
            return Err(invalid(
                entity,
                "RepresentationMaps",
                "expected maps, got text",
            ));
        }
        (None, _) => {}
    }

    attrs[7] = text(draft.tag_or_long_description);
    attrs[kind.fallback_slot] = text(draft.fallback);
    attrs[kind.predefined_slot] = predefined_type.map_or(Value::Null, |t| Value::Enum(t.into()));

    Ok(tx.create(Entity::new(entity, attrs)))
}

fn text(value: Option<&str>) -> Value {
    value.map_or(Value::Null, |v| Value::Text(v.into()))
}

fn blank(value: Option<&str>) -> bool {
    value.is_none_or(|v| v.trim().is_empty())
}
