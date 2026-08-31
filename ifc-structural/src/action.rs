//! Applied structural actions.

use ifc_model::EntityId;
use ifc_schema::SchemaVersion;

use crate::error::{StructuralError, StructuralResult};
use crate::view::Record;

mod linear;
mod planar;
mod point;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActionKind {
    Point,
    Curve,
    Surface,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CoordinateSystem {
    Global,
    Local,
}

#[derive(Debug, Clone, Copy)]
pub struct StructuralAction<'m, 's> {
    record: Record<'m, 's>,
    kind: ActionKind,
}

impl<'m, 's> StructuralAction<'m, 's> {
    pub(crate) fn from_record(record: Record<'m, 's>) -> StructuralResult<Self> {
        let kind = if record
            .schema
            .is_a(&record.entity.type_name, "IfcStructuralPointAction")
        {
            ActionKind::Point
        } else if record
            .schema
            .is_a(&record.entity.type_name, "IfcStructuralCurveAction")
            || record
                .schema
                .is_a(&record.entity.type_name, "IfcStructuralLinearAction")
        {
            ActionKind::Curve
        } else if record
            .schema
            .is_a(&record.entity.type_name, "IfcStructuralSurfaceAction")
            || record
                .schema
                .is_a(&record.entity.type_name, "IfcStructuralPlanarAction")
        {
            ActionKind::Surface
        } else {
            return Err(StructuralError::WrongType {
                id: record.id,
                expected: "point, curve/linear or surface/planar structural action",
                actual: record.entity.type_name.to_string(),
            });
        };
        Ok(Self { record, kind })
    }

    #[must_use]
    pub fn id(&self) -> EntityId {
        self.record.id
    }

    #[must_use]
    pub fn kind(&self) -> ActionKind {
        self.kind
    }

    pub fn applied_load(&self) -> StructuralResult<EntityId> {
        self.validate_semantics()
    }

    pub fn coordinate_system(&self) -> StructuralResult<CoordinateSystem> {
        self.validate_semantics()?;
        self.coordinate_system_value()
    }

    fn coordinate_system_value(&self) -> StructuralResult<CoordinateSystem> {
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

    pub fn destabilizing_load(&self) -> StructuralResult<Option<bool>> {
        self.validate_semantics()?;
        if !self.record.has_attribute("DestabilizingLoad") {
            return Ok(None);
        }
        if self.record.schema.version() == Some(ifc_schema::SchemaVersion::Ifc2x3) {
            return self.record.required_bool("DestabilizingLoad").map(Some);
        }
        self.record.optional_bool("DestabilizingLoad")
    }

    pub fn caused_by(&self) -> StructuralResult<Option<EntityId>> {
        self.validate_semantics()?;
        if !self.record.has_attribute("CausedBy") {
            return Ok(None);
        }
        self.record
            .optional_ref("CausedBy", "IfcStructuralReaction")
    }

    pub fn predefined_type(&self) -> StructuralResult<Option<&'m str>> {
        self.validate_semantics()?;
        if !self.record.has_attribute("PredefinedType") {
            return Ok(None);
        }
        self.record.required_enum("PredefinedType").map(Some)
    }

    pub fn projected_or_true(&self) -> StructuralResult<Option<&'m str>> {
        self.validate_semantics()?;
        if !self.record.has_attribute("ProjectedOrTrue") {
            return Ok(None);
        }
        if self.record.schema.version() == Some(SchemaVersion::Ifc2x3)
            && (self.is_linear() || self.is_planar())
        {
            return self.record.required_enum("ProjectedOrTrue").map(Some);
        }
        self.record.optional_enum("ProjectedOrTrue")
    }

    fn validate_semantics(&self) -> StructuralResult<EntityId> {
        let applied_load = self.applied_load_value()?;
        if self.kind == ActionKind::Point
            || self.record.schema.version() == Some(SchemaVersion::Ifc2x3)
        {
            return Ok(applied_load);
        }
        let predefined_type = self.record.required_enum("PredefinedType")?;
        self.record.require_object_type_if(
            predefined_type.eq_ignore_ascii_case("USERDEFINED"),
            "USERDEFINED structural action requires ObjectType",
        )?;
        if self.kind == ActionKind::Curve && predefined_type.eq_ignore_ascii_case("EQUIDISTANT") {
            return Err(StructuralError::SemanticViolation {
                entity: Some(self.record.id),
                rule: "structural curve action PredefinedType must not be EQUIDISTANT",
            });
        }
        if (self.is_linear() || self.is_planar()) && !predefined_type.eq_ignore_ascii_case("CONST")
        {
            return Err(StructuralError::SemanticViolation {
                entity: Some(self.record.id),
                rule: "linear and planar structural actions require CONST PredefinedType",
            });
        }
        if self
            .record
            .optional_enum("ProjectedOrTrue")?
            .is_some_and(|value| value.eq_ignore_ascii_case("PROJECTED_LENGTH"))
            && self.coordinate_system_value()? != CoordinateSystem::Global
        {
            return Err(StructuralError::SemanticViolation {
                entity: Some(self.record.id),
                rule: "PROJECTED_LENGTH structural action requires GLOBAL_COORDS",
            });
        }
        Ok(applied_load)
    }

    fn applied_load_value(&self) -> StructuralResult<EntityId> {
        let (expected, members): (&'static str, &[&str]) = if self.kind == ActionKind::Point {
            (
                "point-action structural load",
                &[
                    "IfcStructuralLoadSingleForce",
                    "IfcStructuralLoadSingleDisplacement",
                ],
            )
        } else if self.is_linear() {
            (
                "linear-action structural load",
                &[
                    "IfcStructuralLoadLinearForce",
                    "IfcStructuralLoadTemperature",
                ],
            )
        } else if self.is_planar() {
            (
                "planar-action structural load",
                &[
                    "IfcStructuralLoadPlanarForce",
                    "IfcStructuralLoadTemperature",
                ],
            )
        } else {
            return self.record.required_ref("AppliedLoad", "IfcStructuralLoad");
        };
        self.record
            .required_ref_select("AppliedLoad", expected, members)
    }

    fn is_linear(&self) -> bool {
        self.record
            .schema
            .is_a(&self.record.entity.type_name, "IfcStructuralLinearAction")
    }

    fn is_planar(&self) -> bool {
        self.record
            .schema
            .is_a(&self.record.entity.type_name, "IfcStructuralPlanarAction")
    }
}
