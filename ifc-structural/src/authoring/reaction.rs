//! Authoring for structural reactions.
//!
//! A reaction is what the analysis produces: the force a support
//! returns, as opposed to the action a designer applies. The crate
//! could read all three forms and author none, so a result set could
//! be inspected but never written back.
//!
//! The three forms share `IfcStructuralActivity`'s AppliedLoad and
//! GlobalOrLocal, and differ only in whether they add a
//! PredefinedType and which enum it draws from. The point form adds
//! none, so its arity is one short of the other two.

use ifc_model::guid::Guid;
use ifc_model::{EntityId, Model, Transaction, Value};
use ifc_schema::Schema;

use super::action::validate_activity_token;
use super::item::{root_fields, validate_root, StructuralRootDraft};
use super::load_group::validate_enum_token;
use super::{build_named, optional_ref, optional_text, validate_optional_ref, validate_ref_select};
use crate::action::CoordinateSystem;
use crate::error::StructuralError;
use crate::error::StructuralResult;

/// Which `IfcStructuralReaction` subtype to stage.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReactionDraftKind<'a> {
    /// `IfcStructuralPointReaction`: adds no PredefinedType.
    Point,
    /// `IfcStructuralCurveReaction`, typed by
    /// `IfcStructuralCurveActivityTypeEnum`.
    Curve {
        /// `PredefinedType`. Required; the slot is not optional.
        predefined_type: &'a str,
    },
    /// `IfcStructuralSurfaceReaction`, typed by
    /// `IfcStructuralSurfaceActivityTypeEnum`.
    Surface {
        /// `PredefinedType`. Required; the slot is not optional.
        predefined_type: &'a str,
    },
}

/// Authored fields for one structural reaction.
#[derive(Debug, Clone)]
pub struct ReactionDraft<'a> {
    /// `IfcRoot` attributes shared with other structural entities.
    pub root: StructuralRootDraft,
    /// `AppliedLoad`; the reaction force the analysis produced.
    pub applied_load: EntityId,
    /// `GlobalOrLocal`.
    pub coordinate_system: CoordinateSystem,
    /// Which subtype, and its PredefinedType where one applies.
    pub kind: ReactionDraftKind<'a>,
}

/// Stage an `IfcStructuralReaction` subtype.
///
/// # Errors
///
/// Refuses an `applied_load` that is not an `IfcStructuralLoad`, and a
/// `predefined_type` the target schema does not declare for the
/// subtype's own enum. The two typed forms draw from *different*
/// enums, so a token valid on a curve may be undeclared on a surface.
pub fn stage_reaction(
    tx: &mut Transaction,
    model: &Model,
    schema: &Schema,
    draft: ReactionDraft<'_>,
) -> StructuralResult<EntityId> {
    validate_root(tx, model, schema, &draft.root)?;
    let (entity_type, predefined_type) = match draft.kind {
        ReactionDraftKind::Point => ("IfcStructuralPointReaction", None),
        ReactionDraftKind::Curve { predefined_type } => {
            ("IfcStructuralCurveReaction", Some(predefined_type))
        }
        ReactionDraftKind::Surface { predefined_type } => {
            ("IfcStructuralSurfaceReaction", Some(predefined_type))
        }
    };
    validate_ref_select(
        tx,
        model,
        schema,
        draft.applied_load,
        "structural load",
        &["IfcStructuralLoad"],
    )?;

    let mut fields = root_fields(draft.root);
    fields.push(("AppliedLoad", Value::Ref(draft.applied_load)));
    fields.push((
        "GlobalOrLocal",
        Value::Enum(match draft.coordinate_system {
            CoordinateSystem::Global => "GLOBAL_COORDS".into(),
            CoordinateSystem::Local => "LOCAL_COORDS".into(),
        }),
    ));
    if let Some(token) = predefined_type {
        // The curve and surface forms draw from different activity
        // enums with overlapping tokens, so the token is checked
        // against the slot's own declared enum.
        validate_activity_token(schema, entity_type, token)?;
        fields.push(("PredefinedType", Value::Enum(token.into())));
    }
    Ok(tx.create(build_named(schema, entity_type, fields)?))
}

/// Authored fields for an `IfcStructuralResultGroup`.
///
/// A result group is an `IfcGroup`, not an `IfcProduct`: the schema
/// declares no `ObjectPlacement` or `Representation`, so this draft has
/// no fields for them rather than accepting and dropping them. Same
/// reasoning as [`crate::LoadGroupDraft`].
#[derive(Debug, Clone)]
pub struct ResultGroupDraft {
    /// `GlobalId`; must parse as a 22-character IFC GUID.
    pub global_id: String,
    /// `OwnerHistory`.
    pub owner_history: Option<EntityId>,
    /// `Name`.
    pub name: Option<String>,
    /// `Description`.
    pub description: Option<String>,
    /// `ObjectType`; required when `theory_type` is `USERDEFINED`.
    pub object_type: Option<String>,
    /// `TheoryType`, an `IfcAnalysisTheoryTypeEnum` token.
    pub theory_type: String,
    /// `ResultForLoadGroup`, an `IfcStructuralLoadGroup` reference.
    pub result_for_load_group: Option<EntityId>,
    /// `IsLinear`.
    pub is_linear: bool,
}

/// Stage an `IfcStructuralResultGroup`.
///
/// # Errors
///
/// Refuses a `TheoryType` the schema does not declare, a
/// `result_for_load_group` that is not an `IfcStructuralLoadGroup`, and a
/// `USERDEFINED` theory with no `ObjectType` to name it (`HasObjectType`).
pub fn stage_result_group(
    tx: &mut Transaction,
    model: &Model,
    schema: &Schema,
    draft: ResultGroupDraft,
) -> StructuralResult<EntityId> {
    const ENTITY: &str = "IfcStructuralResultGroup";
    if Guid::parse(&draft.global_id).is_none() {
        return Err(StructuralError::InvalidDraftValue {
            entity_type: ENTITY,
            attribute: "GlobalId",
            expected: "a 22-character IFC GUID",
        });
    }
    validate_optional_ref(tx, model, schema, draft.owner_history, "IfcOwnerHistory")?;
    validate_optional_ref(
        tx,
        model,
        schema,
        draft.result_for_load_group,
        "IfcStructuralLoadGroup",
    )?;
    validate_enum_token(schema, ENTITY, "TheoryType", &draft.theory_type)?;
    // HasObjectType: a USERDEFINED theory names itself in ObjectType or
    // says nothing at all.
    if draft.theory_type.eq_ignore_ascii_case("USERDEFINED")
        && draft
            .object_type
            .as_deref()
            .is_none_or(|text| text.trim().is_empty())
    {
        return Err(StructuralError::SemanticViolation {
            entity: None,
            rule: "HasObjectType",
        });
    }
    let fields = vec![
        ("GlobalId", Value::Text(draft.global_id.into())),
        ("OwnerHistory", optional_ref(draft.owner_history)),
        ("Name", optional_text(draft.name)),
        ("Description", optional_text(draft.description)),
        ("ObjectType", optional_text(draft.object_type)),
        ("TheoryType", Value::Enum(draft.theory_type.into())),
        (
            "ResultForLoadGroup",
            optional_ref(draft.result_for_load_group),
        ),
        ("IsLinear", Value::Bool(draft.is_linear)),
    ];
    Ok(tx.create(build_named(schema, ENTITY, fields)?))
}
