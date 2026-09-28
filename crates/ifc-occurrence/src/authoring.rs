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
//!
//! # The release decides the layout
//!
//! The same `&Model` names the release the occurrence is written in
//! (#202). Slots, the `PredefinedType` enumeration and the required
//! attributes come from that release's table by attribute name, never
//! from the IFC4X3 catalogue row; see `release.rs`.

use ifc_model::guid::Guid;
use ifc_model::{EntityId, Model, Transaction, Value};
use ifc_schema::SchemaVersion;

use crate::release::{bind, require_owner_history};
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
    /// The model's header declares several schemas; authoring binds to
    /// exactly one release.
    MultipleSchemas {
        /// Number of `FILE_SCHEMA` declarations.
        schemas: usize,
    },
    /// The model's header declares one schema with no bundled table, so no
    /// layout can be trusted.
    UnsupportedSchema {
        /// The `FILE_SCHEMA` token as written.
        schema: String,
    },
    /// The model's release does not declare the class, or declares it
    /// abstract, such as `IfcBorehole` (IFC4X3 only) in an IFC4 model.
    EntityNotInSchema {
        /// STEP class.
        entity: &'static str,
        /// The release the model declares.
        schema: SchemaVersion,
    },
    /// A value for an attribute the model's release does not declare. It is
    /// refused rather than dropped.
    AuthoringNotInSchema {
        /// STEP class.
        entity: &'static str,
        /// The attribute.
        attribute: &'static str,
        /// The release the model declares.
        schema: SchemaVersion,
    },
    /// The model's release requires an attribute the call leaves unset,
    /// such as the IFC2X3 `IfcRoot.OwnerHistory`.
    AuthoringRequired {
        /// STEP class.
        entity: &'static str,
        /// The required attribute, as the release names it.
        attribute: &'static str,
        /// The release the model declares.
        schema: SchemaVersion,
    },
    /// The owner-history reference resolves neither in the model nor on the
    /// transaction.
    UnresolvedOwnerHistory {
        /// The dangling id.
        id: EntityId,
    },
    /// The owner-history reference is not an `IfcOwnerHistory`.
    NotAnOwnerHistory {
        /// The referenced id.
        id: EntityId,
        /// What the referenced entity actually is.
        found: String,
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
///
/// # Release
///
/// Written in the model's declared release (#202), laid out by attribute
/// name from its table; a header without `FILE_SCHEMA` binds IFC4.
/// `predefined_type` is checked against that release's enumeration, and a
/// class the release does not declare is refused with
/// [`OccurrenceError::EntityNotInSchema`]. `OwnerHistory` is left `$`,
/// which IFC4 and IFC4X3 allow and IFC2X3 does not, so an IFC2X3 model is
/// refused with [`OccurrenceError::AuthoringRequired`]; use
/// [`create_with_owner_history`] there. Any other attribute the release
/// requires and the draft cannot carry (IFC2X3 `IfcStair.ShapeType`, for
/// one) is refused the same way. The type pairing is the IFC4X3
/// `CorrectTypeAssigned` class of the catalogue row in every release.
///
/// # Errors
///
/// The refusals above; [`OccurrenceError::MultipleSchemas`] or
/// [`OccurrenceError::UnsupportedSchema`] for a model that binds no single
/// known release. Nothing is staged on an error.
pub fn create(
    tx: &mut Transaction,
    model: &Model,
    kind: Occurrence,
    global_id: &str,
    predefined_type: Option<&str>,
    typed_by: Option<EntityId>,
    draft: OccurrenceDraft<'_>,
) -> OccurrenceResult<EntityId> {
    let request = Request {
        kind,
        global_id,
        predefined_type,
        typed_by,
        draft,
    };
    author(tx, model, request, None)
}

/// [`create`] with a caller-supplied `IfcOwnerHistory`, which IFC2X3
/// requires on every `IfcRoot`.
///
/// `owner_history` must be in the model or staged earlier on `tx`, and must
/// be an `IfcOwnerHistory`; one is never invented here (build it with
/// `ifc-author`). In IFC4 and IFC4X3 the reference fills the optional slot.
///
/// # Errors
///
/// Those of [`create`] except the IFC2X3 `OwnerHistory` refusal, and
/// [`OccurrenceError::UnresolvedOwnerHistory`] or
/// [`OccurrenceError::NotAnOwnerHistory`] for an `owner_history` that does
/// not resolve or is another entity. Nothing is staged on an error.
#[allow(clippy::too_many_arguments)]
pub fn create_with_owner_history(
    tx: &mut Transaction,
    model: &Model,
    kind: Occurrence,
    global_id: &str,
    predefined_type: Option<&str>,
    typed_by: Option<EntityId>,
    draft: OccurrenceDraft<'_>,
    owner_history: EntityId,
) -> OccurrenceResult<EntityId> {
    let request = Request {
        kind,
        global_id,
        predefined_type,
        typed_by,
        draft,
    };
    author(tx, model, request, Some(owner_history))
}

/// The caller's arguments, bundled.
struct Request<'a> {
    kind: Occurrence,
    global_id: &'a str,
    predefined_type: Option<&'a str>,
    typed_by: Option<EntityId>,
    draft: OccurrenceDraft<'a>,
}

/// Stage one occurrence; `None` leaves `OwnerHistory` `$`.
fn author(
    tx: &mut Transaction,
    model: &Model,
    request: Request<'_>,
    owner_history: Option<EntityId>,
) -> OccurrenceResult<EntityId> {
    let Request {
        kind,
        global_id,
        predefined_type,
        typed_by,
        draft,
    } = request;
    let entity = kind.type_name;
    if Guid::parse(global_id).is_none() {
        return Err(OccurrenceError::MalformedGuid {
            value: global_id.into(),
        });
    }
    let layout = bind(model)?;
    layout.require_entity(entity)?;
    if let Some(token) = predefined_type {
        let Some(members) = layout.predefined_members(entity) else {
            return Err(OccurrenceError::NoPredefinedType { entity });
        };
        if !members.contains(&token) {
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

    if let Some(owner_history) = owner_history {
        require_owner_history(tx, model, owner_history)?;
    }
    let record = layout.named_record(
        entity,
        vec![
            ("GlobalId", Value::Text(global_id.into())),
            ("OwnerHistory", slot_ref(owner_history)),
            ("Name", text(draft.name)),
            ("Description", text(draft.description)),
            ("ObjectType", text(draft.object_type)),
            ("ObjectPlacement", slot_ref(draft.placement)),
            ("Representation", slot_ref(draft.representation)),
            ("Tag", text(draft.tag)),
            (
                "PredefinedType",
                predefined_type.map_or(Value::Null, |token| Value::Enum(token.into())),
            ),
        ],
    )?;
    Ok(tx.create(record))
}
