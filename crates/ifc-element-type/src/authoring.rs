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
//!
//! This module is stricter than `EXISTS`: a blank fallback string satisfies
//! EXPRESS but names nothing, so it is refused too. And the writer takes an
//! [`ElementType`] from the catalogue rather than a type-name string, so an
//! entity the catalogue does not know cannot be written at all.
//!
//! # Which release is written
//!
//! [`create_type`] takes no model and writes the catalogue's IFC4X3
//! layout. [`create_type_in`] and [`create_type_with_owner_history`] write
//! the model's declared release (#202): see `release.rs`.

use ifc_model::guid::Guid;
use ifc_model::{EntityId, Model, Transaction, Value};

use crate::error::{ElementTypeError, ElementTypeResult};
use crate::release::{bind, require_owner_history, Layout};
use crate::table::{ElementType, Family};

pub(crate) fn invalid(
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
/// # Release
///
/// Takes no model, so it writes the catalogue's IFC4X3 layout with
/// `OwnerHistory` `$`, and cannot refuse a model that declares another
/// release. That record is valid in IFC4X3 and, where IFC4 declares the
/// type with the same layout and token, in IFC4; it is never valid IFC2X3,
/// which requires `OwnerHistory`. Use [`create_type_in`] or
/// [`create_type_with_owner_history`] to write the model's declared
/// release.
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
    let request = Request {
        kind,
        global_id,
        predefined_type,
        draft,
    };
    author(tx, Layout::catalogue()?, request, None)
}

/// [`create_type`] in the model's declared release (#202).
///
/// The record is laid out by attribute name from that release's table, and
/// `predefined_type` is checked against that release's enumeration.
/// `OwnerHistory` is left `$`, which IFC4 and IFC4X3 allow and IFC2X3 does
/// not, so an IFC2X3 model is refused with
/// [`ElementTypeError::AuthoringRequired`]; use
/// [`create_type_with_owner_history`] there. A header without
/// `FILE_SCHEMA` binds IFC4.
///
/// # Errors
///
/// Those of [`create_type`], checked against the bound release, and:
/// [`ElementTypeError::MultipleSchemas`] or
/// [`ElementTypeError::UnsupportedSchema`] for a model that binds no single
/// known release; [`ElementTypeError::EntityNotInSchema`] for a type the
/// release does not declare (IFC2X3 has no `IfcDoorType`, IFC4 no
/// `IfcBearingType`); [`ElementTypeError::AuthoringNotInSchema`] for a
/// token where the release declares no `PredefinedType`, and
/// [`ElementTypeError::AuthoringRequired`] for any other attribute the
/// release requires that the draft cannot carry. Nothing is staged on an
/// error.
pub fn create_type_in(
    tx: &mut Transaction,
    model: &Model,
    kind: ElementType,
    global_id: &str,
    predefined_type: Option<&str>,
    draft: TypeDraft<'_>,
) -> ElementTypeResult<EntityId> {
    let request = Request {
        kind,
        global_id,
        predefined_type,
        draft,
    };
    author(tx, bind(model)?, request, None)
}

/// [`create_type_in`] with a caller-supplied `IfcOwnerHistory`, which
/// IFC2X3 requires on every `IfcRoot`.
///
/// `owner_history` must be in the model or staged earlier on `tx`, and must
/// be an `IfcOwnerHistory`; one is never invented here (build it with
/// `ifc-author`). In IFC4 and IFC4X3 the reference fills the optional slot.
///
/// # Errors
///
/// Those of [`create_type_in`] except the IFC2X3 `OwnerHistory` refusal,
/// and [`ElementTypeError::MissingEntity`] for an `owner_history` that does
/// not resolve or [`ElementTypeError::Invalid`] on `OwnerHistory` for one
/// that is another entity. Nothing is staged on an error.
pub fn create_type_with_owner_history(
    tx: &mut Transaction,
    model: &Model,
    kind: ElementType,
    global_id: &str,
    predefined_type: Option<&str>,
    draft: TypeDraft<'_>,
    owner_history: EntityId,
) -> ElementTypeResult<EntityId> {
    let layout = bind(model)?;
    // Checked before the draft so a wrong reference is reported as such.
    require_owner_history(tx, model, kind.type_name, owner_history)?;
    let request = Request {
        kind,
        global_id,
        predefined_type,
        draft,
    };
    author(tx, layout, request, Some(owner_history))
}

/// The caller's arguments, bundled.
struct Request<'a> {
    kind: ElementType,
    global_id: &'a str,
    predefined_type: Option<&'a str>,
    draft: TypeDraft<'a>,
}

/// Stage one type definition in `layout`; `None` leaves `OwnerHistory` `$`.
/// The owner history, if any, has been checked by the caller.
fn author(
    tx: &mut Transaction,
    layout: Layout,
    request: Request<'_>,
    owner_history: Option<EntityId>,
) -> ElementTypeResult<EntityId> {
    let Request {
        kind,
        global_id,
        predefined_type,
        draft,
    } = request;
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
    layout.require_entity(entity)?;

    // The release's own `PredefinedType`: whether it is required, and its
    // tokens. For the catalogue's release these are the row's own.
    match layout.attribute(entity, "PredefinedType") {
        None if predefined_type.is_some() => {
            return Err(ElementTypeError::AuthoringNotInSchema {
                entity,
                attribute: "PredefinedType",
                schema: layout.version(),
            });
        }
        None => {}
        Some(declared) => {
            let members = layout.members(entity, "PredefinedType").unwrap_or_default();
            match predefined_type {
                None if !declared.optional => {
                    return Err(invalid(entity, "PredefinedType", "required"));
                }
                Some(token) if !members.contains(&token) => {
                    return Err(invalid(entity, "PredefinedType", token));
                }
                _ => {}
            }
        }
    }
    if predefined_type == Some("USERDEFINED") && blank(draft.fallback) {
        return Err(invalid(
            entity,
            kind.fallback_attr,
            "required by USERDEFINED",
        ));
    }

    let (slot6_name, slot6) = match (draft.maps_or_identification, kind.family) {
        (Some(Slot6::RepresentationMaps(maps)), Family::Element) => {
            if maps.is_empty() {
                return Err(invalid(entity, "RepresentationMaps", "empty"));
            }
            (
                "RepresentationMaps",
                Value::List(maps.iter().copied().map(Value::Ref).collect()),
            )
        }
        (Some(Slot6::Identification(id)), Family::ResourceOrProcess) => {
            ("Identification", Value::Text(id.into()))
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
        (None, _) => ("RepresentationMaps", Value::Null),
    };
    let slot7_name = match kind.family {
        Family::Element => "Tag",
        Family::ResourceOrProcess => "LongDescription",
    };

    let record = layout.named_record(
        entity,
        vec![
            ("GlobalId", Value::Text(global_id.into())),
            (
                "OwnerHistory",
                owner_history.map_or(Value::Null, Value::Ref),
            ),
            ("Name", text(draft.name)),
            ("Description", text(draft.description)),
            ("ApplicableOccurrence", text(draft.applicable_occurrence)),
            (slot6_name, slot6),
            (slot7_name, text(draft.tag_or_long_description)),
            (kind.fallback_attr, text(draft.fallback)),
            (
                "PredefinedType",
                predefined_type.map_or(Value::Null, |t| Value::Enum(t.into())),
            ),
        ],
    )?;
    Ok(tx.create(record))
}

fn text(value: Option<&str>) -> Value {
    value.map_or(Value::Null, |v| Value::Text(v.into()))
}

fn blank(value: Option<&str>) -> bool {
    value.is_none_or(|v| v.trim().is_empty())
}
