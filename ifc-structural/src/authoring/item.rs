use ifc_model::guid::Guid;
use ifc_model::{EntityId, Model, Transaction, Value};
use ifc_schema::Schema;

use super::{build_named, optional_ref, optional_text, validate_optional_ref, validate_ref};
use crate::error::{StructuralError, StructuralResult};

#[derive(Debug, Clone)]
pub struct StructuralRootDraft {
    pub global_id: String,
    pub owner_history: Option<EntityId>,
    pub name: Option<String>,
    pub description: Option<String>,
    pub object_type: Option<String>,
    pub object_placement: Option<EntityId>,
    pub representation: Option<EntityId>,
}

impl Default for StructuralRootDraft {
    fn default() -> Self {
        Self {
            global_id: "0000000000000000000000".into(),
            owner_history: None,
            name: None,
            description: None,
            object_type: None,
            object_placement: None,
            representation: None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemberPredefinedType {
    RigidJoinedMember,
    PinJoinedMember,
    Cable,
    TensionMember,
    CompressionMember,
    BendingElement,
    MembraneElement,
    Shell,
    UserDefined,
    NotDefined,
}

impl MemberPredefinedType {
    fn token(self) -> &'static str {
        match self {
            Self::RigidJoinedMember => "RIGID_JOINED_MEMBER",
            Self::PinJoinedMember => "PIN_JOINED_MEMBER",
            Self::Cable => "CABLE",
            Self::TensionMember => "TENSION_MEMBER",
            Self::CompressionMember => "COMPRESSION_MEMBER",
            Self::BendingElement => "BENDING_ELEMENT",
            Self::MembraneElement => "MEMBRANE_ELEMENT",
            Self::Shell => "SHELL",
            Self::UserDefined => "USERDEFINED",
            Self::NotDefined => "NOTDEFINED",
        }
    }
}

#[derive(Debug, Clone)]
pub enum MemberDraftKind {
    Curve {
        predefined_type: MemberPredefinedType,
        axis: Option<EntityId>,
    },
    Surface {
        predefined_type: MemberPredefinedType,
        thickness: Option<f64>,
    },
}

#[derive(Debug, Clone)]
pub struct MemberDraft {
    pub root: StructuralRootDraft,
    pub kind: MemberDraftKind,
}

#[derive(Debug, Clone)]
pub enum ConnectionDraftKind {
    Point {
        applied_condition: Option<EntityId>,
        condition_coordinate_system: Option<EntityId>,
    },
    Curve {
        applied_condition: Option<EntityId>,
        axis: Option<EntityId>,
    },
    Surface {
        applied_condition: Option<EntityId>,
    },
}

#[derive(Debug, Clone)]
pub struct ConnectionDraft {
    pub root: StructuralRootDraft,
    pub kind: ConnectionDraftKind,
}

pub(super) fn validate_root(
    tx: &Transaction,
    model: &Model,
    schema: &Schema,
    root: &StructuralRootDraft,
) -> StructuralResult<()> {
    if Guid::parse(&root.global_id).is_none() {
        return Err(StructuralError::InvalidGlobalId);
    }
    validate_optional_ref(tx, model, schema, root.owner_history, "IfcOwnerHistory")?;
    validate_optional_ref(
        tx,
        model,
        schema,
        root.object_placement,
        "IfcObjectPlacement",
    )?;
    validate_optional_ref(
        tx,
        model,
        schema,
        root.representation,
        "IfcProductRepresentation",
    )?;
    Ok(())
}

pub(super) fn root_fields(root: StructuralRootDraft) -> Vec<(&'static str, Value)> {
    vec![
        ("GlobalId", Value::Text(root.global_id.into())),
        ("OwnerHistory", optional_ref(root.owner_history)),
        ("Name", optional_text(root.name)),
        ("Description", optional_text(root.description)),
        ("ObjectType", optional_text(root.object_type)),
        ("ObjectPlacement", optional_ref(root.object_placement)),
        ("Representation", optional_ref(root.representation)),
    ]
}

pub fn stage_member(
    tx: &mut Transaction,
    model: &Model,
    schema: &Schema,
    draft: MemberDraft,
) -> StructuralResult<EntityId> {
    validate_root(tx, model, schema, &draft.root)?;
    let (entity_type, predefined_type, axis, thickness, curve) = match draft.kind {
        MemberDraftKind::Curve {
            predefined_type,
            axis,
        } => (
            "IfcStructuralCurveMember",
            predefined_type,
            axis,
            None,
            true,
        ),
        MemberDraftKind::Surface {
            predefined_type,
            thickness,
        } => (
            "IfcStructuralSurfaceMember",
            predefined_type,
            None,
            thickness,
            false,
        ),
    };
    let token = predefined_type.token();
    let valid = if curve {
        matches!(
            predefined_type,
            MemberPredefinedType::RigidJoinedMember
                | MemberPredefinedType::PinJoinedMember
                | MemberPredefinedType::Cable
                | MemberPredefinedType::TensionMember
                | MemberPredefinedType::CompressionMember
                | MemberPredefinedType::UserDefined
                | MemberPredefinedType::NotDefined
        )
    } else {
        matches!(
            predefined_type,
            MemberPredefinedType::BendingElement
                | MemberPredefinedType::MembraneElement
                | MemberPredefinedType::Shell
                | MemberPredefinedType::UserDefined
                | MemberPredefinedType::NotDefined
        )
    };
    if !valid {
        return Err(StructuralError::InvalidDraftValue {
            entity_type,
            attribute: "PredefinedType",
            expected: "member-kind enum value",
        });
    }
    if predefined_type == MemberPredefinedType::UserDefined
        && draft.root.object_type.as_deref().is_none_or(str::is_empty)
    {
        return Err(StructuralError::SemanticViolation {
            entity: None,
            rule: "USERDEFINED structural member requires ObjectType",
        });
    }
    let has_axis = schema
        .attributes(entity_type)
        .iter()
        .any(|attribute| attribute.name.eq_ignore_ascii_case("Axis"));
    let validated_axis = if has_axis {
        let target = axis.ok_or(StructuralError::MissingRequired {
            entity_type: entity_type.into(),
            attribute: "Axis".into(),
        })?;
        validate_ref(tx, model, schema, target, "IfcDirection")?;
        Some(target)
    } else if axis.is_some() {
        return Err(StructuralError::UnsupportedAttribute {
            entity_type: entity_type.into(),
            attribute: "Axis".into(),
        });
    } else {
        None
    };
    if thickness.is_some_and(|value| !value.is_finite() || value <= 0.0) {
        return Err(StructuralError::InvalidDraftValue {
            entity_type,
            attribute: "Thickness",
            expected: "positive finite thickness or null",
        });
    }
    if predefined_type == MemberPredefinedType::Shell && thickness.is_none() {
        return Err(StructuralError::InvalidDraftValue {
            entity_type,
            attribute: "Thickness",
            expected: "SHELL requires positive finite Thickness",
        });
    }
    let mut fields = root_fields(draft.root);
    fields.push(("PredefinedType", Value::Enum(token.into())));
    if let Some(axis) = validated_axis {
        fields.push(("Axis", Value::Ref(axis)));
    }
    if schema
        .attributes(entity_type)
        .iter()
        .any(|a| a.name.eq_ignore_ascii_case("Thickness"))
    {
        fields.push(("Thickness", thickness.map_or(Value::Null, Value::Real)));
    }
    Ok(tx.create(build_named(schema, entity_type, fields)?))
}

pub fn stage_connection(
    tx: &mut Transaction,
    model: &Model,
    schema: &Schema,
    draft: ConnectionDraft,
) -> StructuralResult<EntityId> {
    validate_root(tx, model, schema, &draft.root)?;
    let (entity_type, applied_condition, axis, coordinate_system) = match draft.kind {
        ConnectionDraftKind::Point {
            applied_condition,
            condition_coordinate_system,
        } => (
            "IfcStructuralPointConnection",
            applied_condition,
            None,
            condition_coordinate_system,
        ),
        ConnectionDraftKind::Curve {
            applied_condition,
            axis,
        } => (
            "IfcStructuralCurveConnection",
            applied_condition,
            axis,
            None,
        ),
        ConnectionDraftKind::Surface { applied_condition } => (
            "IfcStructuralSurfaceConnection",
            applied_condition,
            None,
            None,
        ),
    };
    validate_optional_ref(tx, model, schema, applied_condition, "IfcBoundaryCondition")?;
    validate_optional_ref(tx, model, schema, coordinate_system, "IfcAxis2Placement3D")?;
    let has_axis = schema
        .attributes(entity_type)
        .iter()
        .any(|attribute| attribute.name.eq_ignore_ascii_case("Axis"));
    let validated_axis = if has_axis {
        let target = axis.ok_or(StructuralError::MissingRequired {
            entity_type: entity_type.into(),
            attribute: "Axis".into(),
        })?;
        validate_ref(tx, model, schema, target, "IfcDirection")?;
        Some(target)
    } else if axis.is_some() {
        return Err(StructuralError::UnsupportedAttribute {
            entity_type: entity_type.into(),
            attribute: "Axis".into(),
        });
    } else {
        None
    };
    let mut fields = root_fields(draft.root);
    fields.push(("AppliedCondition", optional_ref(applied_condition)));
    if let Some(axis) = validated_axis {
        fields.push(("Axis", Value::Ref(axis)));
    }
    if schema
        .attributes(entity_type)
        .iter()
        .any(|a| a.name.eq_ignore_ascii_case("ConditionCoordinateSystem"))
    {
        fields.push(("ConditionCoordinateSystem", optional_ref(coordinate_system)));
    }
    Ok(tx.create(build_named(schema, entity_type, fields)?))
}
