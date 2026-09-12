use ifc_model::guid::Guid;
use ifc_model::{EntityId, Model, Transaction, Value};
use ifc_schema::Schema;

use super::{build_named, optional_ref, optional_text, validate_optional_ref, validate_ref};
use crate::error::{StructuralError, StructuralResult};

/// Staged `IfcRoot`-level attributes shared by every staged structural entity.
#[derive(Debug, Clone)]
pub struct StructuralRootDraft {
    /// `GlobalId`; must parse as a 22-character IFC GUID.
    pub global_id: String,
    /// `OwnerHistory`, validated against the model/transaction if present.
    pub owner_history: Option<EntityId>,
    /// `Name`.
    pub name: Option<String>,
    /// `Description`.
    pub description: Option<String>,
    /// `ObjectType`; required non-blank on entities whose predefined type is `USERDEFINED`.
    pub object_type: Option<String>,
    /// `ObjectPlacement`, an `IfcObjectPlacement` reference.
    pub object_placement: Option<EntityId>,
    /// `Representation`, an `IfcProductRepresentation` reference.
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

/// Value of `IfcStructuralCurveMemberTypeEnum`/`IfcStructuralSurfaceMemberTypeEnum` naming a member's structural role.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemberPredefinedType {
    /// `RIGID_JOINED_MEMBER` (curve members only).
    RigidJoinedMember,
    /// `PIN_JOINED_MEMBER` (curve members only).
    PinJoinedMember,
    /// `CABLE` (curve members only).
    Cable,
    /// `TENSION_MEMBER` (curve members only).
    TensionMember,
    /// `COMPRESSION_MEMBER` (curve members only).
    CompressionMember,
    /// `BENDING_ELEMENT` (surface members only).
    BendingElement,
    /// `MEMBRANE_ELEMENT` (surface members only).
    MembraneElement,
    /// `SHELL` (surface members only); requires `Thickness` to be set.
    Shell,
    /// `USERDEFINED`: a custom role named by `ObjectType`.
    UserDefined,
    /// `NOTDEFINED`: no structural role classification given.
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

/// Staged member subtype for [`MemberDraft::kind`].
#[derive(Debug, Clone)]
pub enum MemberDraftKind {
    /// Stages an `IfcStructuralCurveMember`.
    Curve {
        /// `PredefinedType`; must be a curve-member role (rigid/pin-joined, cable, tension/compression, user/not defined).
        predefined_type: MemberPredefinedType,
        /// `Axis`; required when the target schema declares the attribute for this type.
        axis: Option<EntityId>,
    },
    /// Stages an `IfcStructuralSurfaceMember`.
    Surface {
        /// `PredefinedType`; must be a surface-member role (bending/membrane element, shell, user/not defined).
        predefined_type: MemberPredefinedType,
        /// `Thickness`; must be a positive finite value, and mandatory when `predefined_type` is `Shell`.
        thickness: Option<f64>,
    },
}

/// Staged fields for creating an `IfcStructuralMember` via [`stage_member`].
#[derive(Debug, Clone)]
pub struct MemberDraft {
    /// `IfcRoot` attributes shared with other staged structural entities.
    pub root: StructuralRootDraft,
    /// Which `IfcStructuralMember` subtype to create, and its subtype-specific attributes.
    pub kind: MemberDraftKind,
}

/// Staged connection subtype for [`ConnectionDraft::kind`].
#[derive(Debug, Clone)]
pub enum ConnectionDraftKind {
    /// Stages an `IfcStructuralPointConnection`.
    Point {
        /// `AppliedCondition`, an `IfcBoundaryCondition` reference.
        applied_condition: Option<EntityId>,
        /// `ConditionCoordinateSystem`, an `IfcAxis2Placement3D` reference.
        condition_coordinate_system: Option<EntityId>,
    },
    /// Stages an `IfcStructuralCurveConnection`.
    Curve {
        /// `AppliedCondition`, an `IfcBoundaryCondition` reference.
        applied_condition: Option<EntityId>,
        /// `Axis`; required when the target schema declares the attribute for this type.
        axis: Option<EntityId>,
    },
    /// Stages an `IfcStructuralSurfaceConnection`.
    Surface {
        /// `AppliedCondition`, an `IfcBoundaryCondition` reference.
        applied_condition: Option<EntityId>,
    },
}

/// Staged fields for creating an `IfcStructuralConnection` via [`stage_connection`].
#[derive(Debug, Clone)]
pub struct ConnectionDraft {
    /// `IfcRoot` attributes shared with other staged structural entities.
    pub root: StructuralRootDraft,
    /// Which `IfcStructuralConnection` subtype to create, and its subtype-specific attributes.
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

/// Stage an `IfcStructuralMember` create edit on `tx`.
///
/// Fails with [`StructuralError::InvalidDraftValue`] if `predefined_type`
/// does not belong to the curve/surface role set matching `kind`, or if
/// `Thickness` is set but not positive and finite;
/// [`StructuralError::SemanticViolation`] if `predefined_type` is
/// `UserDefined` without a non-blank `object_type`; [`StructuralError::MissingRequired`]
/// if `Axis` is required by the schema but unset, or `predefined_type` is
/// `Shell` without a `Thickness`; and [`StructuralError::UnsupportedAttribute`]
/// if `axis` is set but the target schema declares no `Axis` attribute for
/// this type. Returns the id staged for the new entity.
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

/// Stage an `IfcStructuralConnection` create edit on `tx`.
///
/// Fails with [`StructuralError::MissingRequired`] if `Axis` is required by
/// the schema but unset, and with [`StructuralError::UnsupportedAttribute`]
/// if `axis` is set but the target schema declares no `Axis` attribute for
/// this type. Returns the id staged for the new entity.
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
