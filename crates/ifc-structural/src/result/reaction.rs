//! Borrowed structural-reaction projections.

use ifc_model::EntityId;

use crate::action::CoordinateSystem;
use crate::error::{StructuralError, StructuralResult};
use crate::view::Record;

/// Concrete structural-reaction family.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReactionKind {
    /// `IfcStructuralPointReaction`.
    Point,
    /// IFC4+ `IfcStructuralCurveReaction`.
    Curve,
    /// IFC4+ `IfcStructuralSurfaceReaction`.
    Surface,
}

/// Borrowed authored structural reaction.
///
/// The referenced load is returned literally. This type does not compute,
/// transform, combine, or solve reaction values.
#[derive(Debug, Clone, Copy)]
pub struct Reaction<'m, 's> {
    record: Record<'m, 's>,
    kind: ReactionKind,
}

impl<'m, 's> Reaction<'m, 's> {
    pub(crate) fn from_record(record: Record<'m, 's>) -> StructuralResult<Self> {
        let kind = if record
            .schema
            .is_a(&record.entity.type_name, "IfcStructuralPointReaction")
        {
            ReactionKind::Point
        } else if record
            .schema
            .is_a(&record.entity.type_name, "IfcStructuralCurveReaction")
        {
            ReactionKind::Curve
        } else if record
            .schema
            .is_a(&record.entity.type_name, "IfcStructuralSurfaceReaction")
        {
            ReactionKind::Surface
        } else {
            return Err(StructuralError::WrongType {
                id: record.id,
                expected: "concrete IfcStructuralReaction",
                actual: record.entity.type_name.to_string(),
            });
        };
        Ok(Self { record, kind })
    }

    /// Entity identifier in the shared model graph.
    #[must_use]
    pub fn id(&self) -> EntityId {
        self.record.id
    }

    /// Concrete reaction family.
    #[must_use]
    pub fn kind(&self) -> ReactionKind {
        self.kind
    }

    /// Optional authored name after semantic validation.
    pub fn name(&self) -> StructuralResult<Option<&'m str>> {
        self.validate_semantics()?;
        self.record.optional_text("Name")
    }

    /// Authored applied-load/result record.
    pub fn applied_load(&self) -> StructuralResult<EntityId> {
        self.validate_semantics()
    }

    /// Authored global/local coordinate selector.
    pub fn coordinate_system(&self) -> StructuralResult<CoordinateSystem> {
        self.validate_semantics()?;
        match self
            .record
            .required_enum("GlobalOrLocal")?
            .to_ascii_uppercase()
            .as_str()
        {
            "GLOBAL_COORDS" => Ok(CoordinateSystem::Global),
            "LOCAL_COORDS" => Ok(CoordinateSystem::Local),
            _ => Err(StructuralError::InvalidValue {
                entity: self.record.id,
                attribute: "GlobalOrLocal",
                expected: "IfcGlobalOrLocalEnum",
            }),
        }
    }

    /// IFC4+ curve/surface predefined type, absent on point reactions.
    pub fn predefined_type(&self) -> StructuralResult<Option<&'m str>> {
        self.validate_semantics()?;
        if !self.record.has_attribute("PredefinedType") {
            return Ok(None);
        }
        self.record.required_enum("PredefinedType").map(Some)
    }

    fn validate_semantics(&self) -> StructuralResult<EntityId> {
        let applied_load = if self.kind == ReactionKind::Point {
            self.record.required_ref_select(
                "AppliedLoad",
                "point-reaction structural load",
                &[
                    "IfcStructuralLoadSingleForce",
                    "IfcStructuralLoadSingleDisplacement",
                ],
            )?
        } else {
            self.record
                .required_ref("AppliedLoad", "IfcStructuralLoad")?
        };
        if self.record.has_attribute("PredefinedType") {
            let predefined_type = self.record.required_enum("PredefinedType")?;
            self.record.require_object_type_if(
                predefined_type.eq_ignore_ascii_case("USERDEFINED"),
                "USERDEFINED structural reaction requires ObjectType",
            )?;
            if self.kind == ReactionKind::Curve
                && (predefined_type.eq_ignore_ascii_case("SINUS")
                    || predefined_type.eq_ignore_ascii_case("PARABOLA"))
            {
                return Err(StructuralError::SemanticViolation {
                    entity: Some(self.record.id),
                    rule: "IfcStructuralCurveReaction.SuitablePredefinedType",
                });
            }
        }
        Ok(applied_load)
    }
}
