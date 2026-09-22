//! Staging `IfcStructuralLoadGroup` and `IfcStructuralLoadCase`.
//!
//! # A load case is a load group that says so
//!
//! `IfcStructuralLoadCase` adds one attribute to its supertype and
//! one rule:
//!
//! ```text
//! IsLoadCasePredefinedType :
//!   SELF\\IfcStructuralLoadGroup.PredefinedType = IfcLoadGroupTypeEnum.LOAD_CASE;
//! ```
//!
//! The rule pins an *inherited* attribute. A load case whose
//! `PredefinedType` says `LOAD_COMBINATION` is a contradiction the
//! schema forbids, so the caller does not choose that token for a
//! case: the writer sets it.
//!
//! # Three enums, one rule
//!
//! `HasObjectType` fires when *any* of `PredefinedType`,
//! `ActionType` or `ActionSource` is `USERDEFINED`. One
//! `ObjectType` covers all three, so the check is on the
//! disjunction rather than per attribute.

use ifc_model::guid::Guid;
use ifc_model::{EntityId, Model, Transaction, Value};
use ifc_schema::{Schema, TypeKind};

use super::{build_named, optional_text, validate_optional_ref};
use crate::error::{StructuralError, StructuralResult};

/// Which of the two group forms to stage.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LoadGroupKind {
    /// `IfcStructuralLoadGroup`, whose `PredefinedType` the caller chooses.
    Group {
        /// `PredefinedType`, an `IfcLoadGroupTypeEnum` token.
        predefined_type: &'static str,
    },
    /// `IfcStructuralLoadCase`, whose `PredefinedType` is pinned to
    /// `LOAD_CASE` by `IsLoadCasePredefinedType`.
    Case {
        /// `SelfWeightCoefficients`, a `LIST [3:3]` of ratios when given.
        ///
        /// Exactly three: one per global axis. Any other length is
        /// refused rather than padded, because a two-entry list
        /// leaves an axis with no stated self-weight.
        self_weight_coefficients: Option<[f64; 3]>,
    },
}

/// Staged fields for [`stage_load_group`].
///
/// A load group is an `IfcGroup`, not an `IfcProduct`: it declares
/// no `ObjectPlacement` or `Representation`, so this draft has no
/// fields for them rather than accepting and dropping them.
#[derive(Debug, Clone)]
pub struct LoadGroupDraft {
    /// `GlobalId`; must parse as a 22-character IFC GUID.
    pub global_id: String,
    /// `OwnerHistory`, validated against the model/transaction if present.
    pub owner_history: Option<EntityId>,
    /// `Name`.
    pub name: Option<String>,
    /// `Description`.
    pub description: Option<String>,
    /// `ObjectType`; required non-blank when any of the three
    /// enum attributes is `USERDEFINED`.
    pub object_type: Option<String>,
    /// `ActionType`, an `IfcActionTypeEnum` token.
    pub action_type: &'static str,
    /// `ActionSource`, an `IfcActionSourceTypeEnum` token.
    pub action_source: &'static str,
    /// `Coefficient`, a ratio applied to every load in the group.
    pub coefficient: Option<f64>,
    /// `Purpose`.
    pub purpose: Option<String>,
    /// Which group form, and its form-specific attributes.
    pub kind: LoadGroupKind,
}

/// Stage an `IfcStructuralLoadGroup` or `IfcStructuralLoadCase`.
///
/// # Errors
///
/// Refuses a malformed `GlobalId`; an enum token the target schema
/// does not declare for that attribute; a `USERDEFINED` token with
/// no non-blank `ObjectType` (`HasObjectType`); a non-finite
/// `Coefficient` or self-weight ratio; and a `LOAD_CASE`
/// `PredefinedType` requested for the plain group form, which
/// would duplicate the case form with the rule unenforced.
pub fn stage_load_group(
    tx: &mut Transaction,
    model: &Model,
    schema: &Schema,
    draft: LoadGroupDraft,
) -> StructuralResult<EntityId> {
    if Guid::parse(&draft.global_id).is_none() {
        return Err(StructuralError::InvalidGlobalId);
    }
    let entity_type = match draft.kind {
        LoadGroupKind::Group { .. } => "IfcStructuralLoadGroup",
        LoadGroupKind::Case { .. } => "IfcStructuralLoadCase",
    };
    // IsLoadCasePredefinedType: the case form's inherited
    // PredefinedType is not the caller's to choose.
    let predefined_type = match draft.kind {
        LoadGroupKind::Group { predefined_type } => {
            if predefined_type.eq_ignore_ascii_case("LOAD_CASE") {
                return Err(StructuralError::SemanticViolation {
                    entity: None,
                    rule: "LOAD_CASE PredefinedType requires IfcStructuralLoadCase",
                });
            }
            predefined_type
        }
        LoadGroupKind::Case { .. } => "LOAD_CASE",
    };

    for (attribute, token) in [
        ("PredefinedType", predefined_type),
        ("ActionType", draft.action_type),
        ("ActionSource", draft.action_source),
    ] {
        validate_enum_token(schema, entity_type, attribute, token)?;
    }

    // HasObjectType fires on the disjunction: one USERDEFINED
    // anywhere among the three makes ObjectType the only place
    // the intended kind is stated.
    let user_defined = [predefined_type, draft.action_type, draft.action_source]
        .iter()
        .any(|token| token.eq_ignore_ascii_case("USERDEFINED"));
    if user_defined
        && draft
            .object_type
            .as_deref()
            .is_none_or(|value| value.trim().is_empty())
    {
        return Err(StructuralError::SemanticViolation {
            entity: None,
            rule: "USERDEFINED load group requires an ObjectType",
        });
    }

    validate_finite(draft.coefficient, entity_type, "Coefficient")?;
    let self_weight = match draft.kind {
        LoadGroupKind::Case {
            self_weight_coefficients: Some(ratios),
        } => {
            for ratio in ratios {
                validate_finite(Some(ratio), entity_type, "SelfWeightCoefficients")?;
            }
            Some(Value::List(
                ratios.iter().copied().map(Value::Real).collect(),
            ))
        }
        _ => None,
    };

    validate_root_refs(tx, model, schema, draft.owner_history)?;

    let mut fields = vec![
        ("GlobalId", Value::Text(draft.global_id.into())),
        ("Name", optional_text(draft.name)),
        ("Description", optional_text(draft.description)),
        ("ObjectType", optional_text(draft.object_type)),
        ("PredefinedType", Value::Enum(predefined_type.into())),
        ("ActionType", Value::Enum(draft.action_type.into())),
        ("ActionSource", Value::Enum(draft.action_source.into())),
        ("Purpose", optional_text(draft.purpose)),
        (
            "Coefficient",
            draft.coefficient.map_or(Value::Null, Value::Real),
        ),
    ];
    if let Some(owner_history) = draft.owner_history {
        fields.push(("OwnerHistory", Value::Ref(owner_history)));
    }
    // SelfWeightCoefficients belongs to the case form only;
    // build_named refuses it on the plain group rather than
    // dropping it, so it is pushed only when present.
    if let Some(values) = self_weight {
        fields.push(("SelfWeightCoefficients", values));
    }
    Ok(tx.create(build_named(schema, entity_type, fields)?))
}

/// Refuse an enum token the target schema does not declare.
///
/// Tokens are read from the schema rather than a local list, so a
/// token added or withdrawn between schemas needs no edit here.
fn validate_enum_token(
    schema: &Schema,
    entity_type: &'static str,
    attribute: &'static str,
    token: &str,
) -> StructuralResult<()> {
    let declared = schema
        .attributes(entity_type)
        .iter()
        .find(|candidate| candidate.name.eq_ignore_ascii_case(attribute))
        .and_then(|candidate| schema.type_def(&candidate.type_name))
        .is_some_and(|definition| match &definition.kind {
            TypeKind::Enumeration(values) => values.iter().any(|member| member == token),
            _ => false,
        });
    if declared {
        return Ok(());
    }
    Err(StructuralError::InvalidDraftValue {
        entity_type,
        attribute,
        expected: "a token the schema declares for this attribute",
    })
}

/// Refuse a non-finite ratio.
///
/// NaN and the infinities all survive a `f64` slot and reach the
/// file as text no reader can act on.
fn validate_finite(
    value: Option<f64>,
    entity_type: &'static str,
    attribute: &'static str,
) -> StructuralResult<()> {
    if value.is_some_and(|number| !number.is_finite()) {
        return Err(StructuralError::InvalidDraftValue {
            entity_type,
            attribute,
            expected: "a finite ratio",
        });
    }
    Ok(())
}

/// Validate the one reference an `IfcGroup`-rooted record carries.
fn validate_root_refs(
    tx: &Transaction,
    model: &Model,
    schema: &Schema,
    owner_history: Option<EntityId>,
) -> StructuralResult<()> {
    validate_optional_ref(tx, model, schema, owner_history, "IfcOwnerHistory")
}
