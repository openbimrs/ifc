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
//! compares its STEP class against the pairing the model's declared
//! release states (#214): the IFC4 and IFC4X3 WHERE rule of the class,
//! or IFC2X3's documented pairing, which types an `IfcDoor` by an
//! `IfcDoorStyle`. That costs a model lookup, which is why `create`
//! takes a `&Model`: a pairing that is not checked against the real
//! entity is not checked at all.
//!
//! # The release decides the layout
//!
//! The same `&Model` names the release the occurrence is written in
//! (#202). Slots, the `PredefinedType` enumeration and the required
//! attributes come from that release's table by attribute name, never
//! from the IFC4X3 catalogue row; see `release.rs`.

use ifc_model::guid::Guid;
use ifc_model::{EntityId, Model, Transaction, Value};

use crate::draft::{specific_values, OccurrenceDraft};
use crate::error::{OccurrenceError, OccurrenceResult};
use crate::release::{bind, require_owner_history};
use crate::table::Occurrence;

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
/// declared release pairs with `kind`; see
/// [`OccurrenceError::WrongTypeClass`] and
/// [`OccurrenceError::TypeClassNotInSchema`].
///
/// Refuses a malformed `global_id`, a `predefined_type` outside the
/// class enum, `USERDEFINED` without `draft.object_type`, a
/// `typed_by` of the wrong class or one that does not resolve, a
/// `shape_type` or `bar_role` outside its enumeration
/// ([`OccurrenceError::UnknownToken`]), and a measure the attribute does
/// not admit ([`OccurrenceError::InvalidMeasure`]).
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
/// requires and the draft leaves unset (IFC2X3 `IfcStair.ShapeType`, for
/// one) is refused the same way, and a draft value for an attribute the
/// release does not declare on the class with
/// [`OccurrenceError::AuthoringNotInSchema`]. The type pairing is the
/// bound release's own.
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
        let Some(members) = layout.members(entity, "PredefinedType") else {
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
        let Some(expected) = layout.type_class(kind)? else {
            if kind.type_class.is_none() {
                return Err(OccurrenceError::TypingNotPermitted { entity });
            }
            return Err(OccurrenceError::TypeClassNotInSchema {
                entity,
                schema: layout.version(),
            });
        };
        let found = model
            .get(id)
            .ok_or(OccurrenceError::UnresolvedType { id })?
            .type_name
            .to_ascii_uppercase();
        if !layout.is_a(&found, expected) {
            return Err(OccurrenceError::WrongTypeClass {
                entity,
                expected,
                found,
            });
        }
    }

    let specific = specific_values(layout, entity, &draft)?;
    if let Some(owner_history) = owner_history {
        require_owner_history(tx, model, owner_history)?;
    }
    let mut values = vec![
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
    ];
    values.extend(specific);
    let record = layout.named_record(entity, values)?;
    Ok(tx.create(record))
}
