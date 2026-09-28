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
//! The same holds for `IfcEventType.CorrectEventTriggerType`, stated over
//! `EventTriggerType` and `UserDefinedEventTriggerType`.
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

/// Attributes of a type definition: those every type shares, and the few
/// one type requires on top.
///
/// `tag_or_long_description` and `maps_or_identification` occupy slots
/// 7 and 6, whose meaning depends on [`Family`]. Naming them for both
/// readings keeps a caller from assuming the element-type reading on a
/// resource type, where it would file a tag as a description.
///
/// The type-specific fields (#214) carry the attributes IFC4 and IFC4X3
/// require on four types, as `references/ifc-spec` declares them:
///
/// ```text
/// IfcDoorType       OperationType    : IfcDoorTypeOperationEnum;
/// IfcWindowType     PartitioningType : IfcWindowTypePartitioningEnum;
/// IfcEventType      EventTriggerType : IfcEventTriggerTypeEnum;
/// IfcFurnitureType  AssemblyPlace    : IfcAssemblyPlaceEnum;  (IFC2X3 too)
/// ```
///
/// A value for an attribute the type does not declare in the bound release
/// is refused, never dropped.
///
/// The struct is `#[non_exhaustive]`: build it with [`TypeDraft::new`] and
/// the setters, so a field a later release needs can be added without
/// breaking callers.
#[derive(Debug, Clone, Copy, Default)]
#[non_exhaustive]
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
    /// `IfcDoorType.OperationType`, an `IfcDoorTypeOperationEnum` token;
    /// required on `IfcDoorType`.
    pub operation_type: Option<&'a str>,
    /// `IfcDoorType.UserDefinedOperationType`.
    pub user_defined_operation_type: Option<&'a str>,
    /// `IfcWindowType.PartitioningType`, an `IfcWindowTypePartitioningEnum`
    /// token; required on `IfcWindowType`.
    pub partitioning_type: Option<&'a str>,
    /// `IfcWindowType.UserDefinedPartitioningType`.
    pub user_defined_partitioning_type: Option<&'a str>,
    /// `ParameterTakesPrecedence` on `IfcDoorType` and `IfcWindowType`.
    pub parameter_takes_precedence: Option<bool>,
    /// `IfcEventType.EventTriggerType`, an `IfcEventTriggerTypeEnum` token;
    /// required on `IfcEventType`.
    pub event_trigger_type: Option<&'a str>,
    /// `IfcEventType.UserDefinedEventTriggerType`. Required when
    /// `event_trigger_type` is `USERDEFINED` (`CorrectEventTriggerType`).
    pub user_defined_event_trigger_type: Option<&'a str>,
    /// `IfcFurnitureType.AssemblyPlace`, an `IfcAssemblyPlaceEnum` token;
    /// required on `IfcFurnitureType`.
    pub assembly_place: Option<&'a str>,
}

impl<'a> TypeDraft<'a> {
    /// Starts an empty draft with every attribute unset.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets `Name`, which `NameRequired` makes mandatory.
    #[must_use]
    pub fn name(mut self, value: &'a str) -> Self {
        self.name = Some(value);
        self
    }

    /// Sets `Description`.
    #[must_use]
    pub fn description(mut self, value: &'a str) -> Self {
        self.description = Some(value);
        self
    }

    /// Sets `ApplicableOccurrence`.
    #[must_use]
    pub fn applicable_occurrence(mut self, value: &'a str) -> Self {
        self.applicable_occurrence = Some(value);
        self
    }

    /// Sets slot 6: `RepresentationMaps` or `Identification`.
    #[must_use]
    pub fn maps_or_identification(mut self, value: Slot6<'a>) -> Self {
        self.maps_or_identification = Some(value);
        self
    }

    /// Sets slot 7: `Tag` or `LongDescription`.
    #[must_use]
    pub fn tag_or_long_description(mut self, value: &'a str) -> Self {
        self.tag_or_long_description = Some(value);
        self
    }

    /// Sets the slot-8 `USERDEFINED` fallback.
    #[must_use]
    pub fn fallback(mut self, value: &'a str) -> Self {
        self.fallback = Some(value);
        self
    }

    /// Sets `IfcDoorType.OperationType`.
    #[must_use]
    pub fn operation_type(mut self, value: &'a str) -> Self {
        self.operation_type = Some(value);
        self
    }

    /// Sets `IfcDoorType.UserDefinedOperationType`.
    #[must_use]
    pub fn user_defined_operation_type(mut self, value: &'a str) -> Self {
        self.user_defined_operation_type = Some(value);
        self
    }

    /// Sets `IfcWindowType.PartitioningType`.
    #[must_use]
    pub fn partitioning_type(mut self, value: &'a str) -> Self {
        self.partitioning_type = Some(value);
        self
    }

    /// Sets `IfcWindowType.UserDefinedPartitioningType`.
    #[must_use]
    pub fn user_defined_partitioning_type(mut self, value: &'a str) -> Self {
        self.user_defined_partitioning_type = Some(value);
        self
    }

    /// Sets `ParameterTakesPrecedence` (`IfcDoorType`, `IfcWindowType`).
    #[must_use]
    pub fn parameter_takes_precedence(mut self, value: bool) -> Self {
        self.parameter_takes_precedence = Some(value);
        self
    }

    /// Sets `IfcEventType.EventTriggerType`.
    #[must_use]
    pub fn event_trigger_type(mut self, value: &'a str) -> Self {
        self.event_trigger_type = Some(value);
        self
    }

    /// Sets `IfcEventType.UserDefinedEventTriggerType`.
    #[must_use]
    pub fn user_defined_event_trigger_type(mut self, value: &'a str) -> Self {
        self.user_defined_event_trigger_type = Some(value);
        self
    }

    /// Sets `IfcFurnitureType.AssemblyPlace`.
    #[must_use]
    pub fn assembly_place(mut self, value: &'a str) -> Self {
        self.assembly_place = Some(value);
        self
    }
}

/// What slot 6 holds, which differs by [`Family`].
#[derive(Debug, Clone, Copy)]
#[non_exhaustive]
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
/// shape for the entity's family. A type-specific token outside its
/// enumeration is [`ElementTypeError::Invalid`], one for an attribute the
/// type does not declare [`ElementTypeError::AuthoringNotInSchema`], and
/// `USERDEFINED` `event_trigger_type` without
/// `user_defined_event_trigger_type` is refused (`CorrectEventTriggerType`).
/// A required attribute left unset, such as `IfcDoorType.OperationType` or
/// `IfcFurnitureType.AssemblyPlace`, is
/// [`ElementTypeError::AuthoringRequired`], never written `$` (#214).
/// Nothing is staged on an error.
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
/// release requires that the draft leaves unset. Nothing is staged on an
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

    let specific = specific_values(layout, entity, &draft)?;
    let mut values = vec![
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
    ];
    values.extend(specific);
    let record = layout.named_record(entity, values)?;
    Ok(tx.create(record))
}

/// The type-specific attributes of `draft` (#214), checked against the
/// bound release: a token must be a member of the enumeration the release
/// declares for that attribute on `entity`, and a value for an attribute
/// `entity` does not declare there is
/// [`ElementTypeError::AuthoringNotInSchema`]. Unset fields yield `$`,
/// which [`Layout::named_record`] drops for an undeclared attribute and
/// refuses for a required one.
fn specific_values(
    layout: Layout,
    entity: &'static str,
    draft: &TypeDraft<'_>,
) -> ElementTypeResult<Vec<(&'static str, Value)>> {
    let enums = [
        ("OperationType", draft.operation_type),
        ("PartitioningType", draft.partitioning_type),
        ("EventTriggerType", draft.event_trigger_type),
        ("AssemblyPlace", draft.assembly_place),
    ];
    let mut values = Vec::new();
    for (attribute, token) in enums {
        let Some(token) = token else {
            continue;
        };
        let Some(members) = layout.members(entity, attribute) else {
            return Err(ElementTypeError::AuthoringNotInSchema {
                entity,
                attribute,
                schema: layout.version(),
            });
        };
        if !members.contains(&token) {
            return Err(invalid(entity, attribute, token));
        }
        values.push((attribute, Value::Enum(token.into())));
    }
    // `IfcEventType.CorrectEventTriggerType`: USERDEFINED names its trigger
    // in `UserDefinedEventTriggerType`. Blank is refused as for `fallback`.
    if draft.event_trigger_type == Some("USERDEFINED")
        && blank(draft.user_defined_event_trigger_type)
    {
        return Err(invalid(
            entity,
            "UserDefinedEventTriggerType",
            "required by USERDEFINED",
        ));
    }
    values.extend([
        (
            "UserDefinedOperationType",
            text(draft.user_defined_operation_type),
        ),
        (
            "UserDefinedPartitioningType",
            text(draft.user_defined_partitioning_type),
        ),
        (
            "UserDefinedEventTriggerType",
            text(draft.user_defined_event_trigger_type),
        ),
        (
            "ParameterTakesPrecedence",
            draft
                .parameter_takes_precedence
                .map_or(Value::Null, Value::Bool),
        ),
    ]);
    Ok(values)
}

fn text(value: Option<&str>) -> Value {
    value.map_or(Value::Null, |v| Value::Text(v.into()))
}

fn blank(value: Option<&str>) -> bool {
    value.is_none_or(|v| v.trim().is_empty())
}
