use ifc_model::{EntityId, Model, Transaction, Value};
use ifc_schema::Schema;

use super::item::{root_fields, validate_root, StructuralRootDraft};
use super::{build_named, optional_ref, validate_optional_ref, validate_ref_select};
use crate::action::CoordinateSystem;
use crate::error::{StructuralError, StructuralResult};

/// Value of `IfcProjectedOrTrueLengthEnum` naming how a linear/planar action's magnitude is measured.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProjectedOrTrue {
    /// `PROJECTED_LENGTH`: magnitude given per unit of the projected length/area.
    ProjectedLength,
    /// `TRUE_LENGTH`: magnitude given per unit of the true (unprojected) length/area.
    TrueLength,
}

impl ProjectedOrTrue {
    fn token(self) -> &'static str {
        match self {
            Self::ProjectedLength => "PROJECTED_LENGTH",
            Self::TrueLength => "TRUE_LENGTH",
        }
    }
}

/// Staged action subtype for [`ActionDraft::kind`].
#[derive(Debug, Clone)]
pub enum ActionDraftKind {
    /// Stages an `IfcStructuralPointAction`.
    Point,
    /// Stages an `IfcStructuralLinearAction`.
    Linear {
        /// `ProjectedOrTrue`; staged only when the target schema declares the attribute.
        projected_or_true: Option<ProjectedOrTrue>,
    },
    /// Stages an `IfcStructuralPlanarAction`.
    Planar {
        /// `ProjectedOrTrue`; staged only when the target schema declares the attribute.
        projected_or_true: Option<ProjectedOrTrue>,
    },
}

/// Staged fields for creating an `IfcStructuralAction` via [`stage_action`].
#[derive(Debug, Clone)]
pub struct ActionDraft {
    /// `IfcRoot` attributes shared with other staged structural entities.
    pub root: StructuralRootDraft,
    /// `AppliedLoad`; must reference a load type compatible with `kind`.
    pub applied_load: EntityId,
    /// `GlobalOrLocal`.
    pub coordinate_system: CoordinateSystem,
    /// `DestabilizingLoad`; required when the target schema (IFC2X3) declares it mandatory.
    pub destabilizing_load: Option<bool>,
    /// `CausedBy`; only staged when the target schema declares the attribute.
    pub caused_by: Option<EntityId>,
    /// Which `IfcStructuralAction` subtype to create, and its subtype-specific attributes.
    pub kind: ActionDraftKind,
}

/// Stage an `IfcStructuralAction` create edit on `tx`.
///
/// Fails with [`StructuralError::WrongReferenceType`] if `draft.applied_load`
/// is not one of the load types `kind` permits, [`StructuralError::SemanticViolation`]
/// if `ProjectedOrTrue` is `PROJECTED_LENGTH` while `coordinate_system` is not
/// `Global`, and [`StructuralError::MissingRequired`] if `destabilizing_load`
/// is unset while the target schema requires it. Returns the id staged for
/// the new entity.
pub fn stage_action(
    tx: &mut Transaction,
    model: &Model,
    schema: &Schema,
    draft: ActionDraft,
) -> StructuralResult<EntityId> {
    validate_root(tx, model, schema, &draft.root)?;
    let (entity_type, projected_or_true, load_members): (&str, Option<ProjectedOrTrue>, &[&str]) =
        match draft.kind {
            ActionDraftKind::Point => (
                "IfcStructuralPointAction",
                None,
                &[
                    "IfcStructuralLoadSingleForce",
                    "IfcStructuralLoadSingleDisplacement",
                ],
            ),
            ActionDraftKind::Linear { projected_or_true } => (
                "IfcStructuralLinearAction",
                projected_or_true,
                &[
                    "IfcStructuralLoadLinearForce",
                    "IfcStructuralLoadTemperature",
                ],
            ),
            ActionDraftKind::Planar { projected_or_true } => (
                "IfcStructuralPlanarAction",
                projected_or_true,
                &[
                    "IfcStructuralLoadPlanarForce",
                    "IfcStructuralLoadTemperature",
                ],
            ),
        };
    validate_ref_select(
        tx,
        model,
        schema,
        draft.applied_load,
        "compatible structural load",
        load_members,
    )?;
    validate_optional_ref(tx, model, schema, draft.caused_by, "IfcStructuralReaction")?;
    if projected_or_true == Some(ProjectedOrTrue::ProjectedLength)
        && draft.coordinate_system != CoordinateSystem::Global
    {
        return Err(StructuralError::SemanticViolation {
            entity: None,
            rule: "PROJECTED_LENGTH structural action requires GLOBAL_COORDS",
        });
    }
    let attributes = schema.attributes(entity_type);

    let destabilizing_required = attributes
        .iter()
        .find(|a| a.name.eq_ignore_ascii_case("DestabilizingLoad"))
        .is_some_and(|attribute| !attribute.optional);
    if destabilizing_required && draft.destabilizing_load.is_none() {
        return Err(StructuralError::MissingRequired {
            entity_type: entity_type.into(),
            attribute: "DestabilizingLoad".into(),
        });
    }
    let mut fields = root_fields(draft.root);
    fields.push(("AppliedLoad", Value::Ref(draft.applied_load)));
    fields.push((
        "GlobalOrLocal",
        Value::Enum(match draft.coordinate_system {
            CoordinateSystem::Global => "GLOBAL_COORDS".into(),
            CoordinateSystem::Local => "LOCAL_COORDS".into(),
        }),
    ));
    if attributes
        .iter()
        .any(|a| a.name.eq_ignore_ascii_case("DestabilizingLoad"))
    {
        fields.push((
            "DestabilizingLoad",
            draft.destabilizing_load.map_or(Value::Null, Value::Bool),
        ));
    }
    if attributes
        .iter()
        .any(|a| a.name.eq_ignore_ascii_case("CausedBy"))
    {
        fields.push(("CausedBy", optional_ref(draft.caused_by)));
    }
    if attributes
        .iter()
        .any(|a| a.name.eq_ignore_ascii_case("ProjectedOrTrue"))
    {
        fields.push((
            "ProjectedOrTrue",
            projected_or_true.map_or(Value::Null, |value| Value::Enum(value.token().into())),
        ));
    }
    if attributes
        .iter()
        .any(|a| a.name.eq_ignore_ascii_case("PredefinedType"))
    {
        fields.push(("PredefinedType", Value::Enum("CONST".into())));
    }
    Ok(tx.create(build_named(schema, entity_type, fields)?))
}
