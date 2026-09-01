use ifc_model::{EntityId, Model, Transaction, Value};
use ifc_schema::Schema;

use super::item::{root_fields, validate_root, StructuralRootDraft};
use super::{build_named, optional_ref, validate_optional_ref, validate_ref_select};
use crate::action::CoordinateSystem;
use crate::error::{StructuralError, StructuralResult};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProjectedOrTrue {
    ProjectedLength,
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

#[derive(Debug, Clone)]
pub enum ActionDraftKind {
    Point,
    Linear {
        projected_or_true: Option<ProjectedOrTrue>,
    },
    Planar {
        projected_or_true: Option<ProjectedOrTrue>,
    },
}

#[derive(Debug, Clone)]
pub struct ActionDraft {
    pub root: StructuralRootDraft,
    pub applied_load: EntityId,
    pub coordinate_system: CoordinateSystem,
    pub destabilizing_load: Option<bool>,
    pub caused_by: Option<EntityId>,
    pub kind: ActionDraftKind,
}

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
