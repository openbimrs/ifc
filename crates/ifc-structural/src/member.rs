//! Idealized curve and surface structural members.

use ifc_model::EntityId;
use ifc_schema::SchemaVersion;

use crate::error::{StructuralError, StructuralResult};
use crate::view::Record;

mod curve;
mod surface;
mod varying;

/// Which `IfcStructuralMember` subtype (and mutable-cross-section variant) a member carries.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemberKind {
    /// `IfcStructuralCurveMember` with constant axis properties.
    Curve,
    /// `IfcStructuralCurveMemberVarying`: axis properties vary along the curve.
    CurveVarying,
    /// `IfcStructuralSurfaceMember` with constant thickness.
    Surface,
    /// `IfcStructuralSurfaceMemberVarying`: thickness varies over the surface.
    SurfaceVarying,
}

/// Borrowed projection of an `IfcStructuralMember` (curve or surface, constant or varying).
#[derive(Debug, Clone, Copy)]
pub struct Member<'m, 's> {
    record: Record<'m, 's>,
    kind: MemberKind,
}

impl<'m, 's> Member<'m, 's> {
    /// Classify `record`'s [`MemberKind`] from its declared IFC type.
    ///
    /// Fails with [`StructuralError::WrongType`] if the type is not a subtype
    /// of `IfcStructuralCurveMember` or `IfcStructuralSurfaceMember`.
    pub(crate) fn from_record(record: Record<'m, 's>) -> StructuralResult<Self> {
        let kind = if record
            .schema
            .is_a(&record.entity.type_name, "IfcStructuralCurveMemberVarying")
        {
            MemberKind::CurveVarying
        } else if record.schema.is_a(
            &record.entity.type_name,
            "IfcStructuralSurfaceMemberVarying",
        ) {
            MemberKind::SurfaceVarying
        } else if record
            .schema
            .is_a(&record.entity.type_name, "IfcStructuralCurveMember")
        {
            MemberKind::Curve
        } else if record
            .schema
            .is_a(&record.entity.type_name, "IfcStructuralSurfaceMember")
        {
            MemberKind::Surface
        } else {
            return Err(StructuralError::WrongType {
                id: record.id,
                expected: "IfcStructuralCurveMember or IfcStructuralSurfaceMember",
                actual: record.entity.type_name.to_string(),
            });
        };
        Ok(Self { record, kind })
    }

    #[must_use]
    /// The `IfcStructuralMember` entity id.
    pub fn id(&self) -> EntityId {
        self.record.id
    }

    #[must_use]
    /// Which curve/surface, constant/varying subtype this member carries.
    pub fn kind(&self) -> MemberKind {
        self.kind
    }

    /// `Name`, inherited from `IfcRoot`. Legally absent.
    pub fn name(&self) -> StructuralResult<Option<&'m str>> {
        self.validate_semantics()?;
        self.record.optional_text("Name")
    }

    /// `PredefinedType`, always mandatory on `IfcStructuralMember`.
    ///
    /// Fails with [`StructuralError::SemanticViolation`] if the value is
    /// `USERDEFINED` but `ObjectType` is unset (IFC4/IFC4X3 only; IFC2X3
    /// declares no `PredefinedType` semantic constraint here).
    pub fn predefined_type(&self) -> StructuralResult<&'m str> {
        self.validate_semantics()?;
        self.record.required_enum("PredefinedType")
    }

    /// `Axis`, the local member axis direction. Only meaningful for curve
    /// members; returns `None` for surface members or when the attribute is unset.
    pub fn axis(&self) -> StructuralResult<Option<EntityId>> {
        if !matches!(self.kind, MemberKind::Curve | MemberKind::CurveVarying)
            || !self.record.has_attribute("Axis")
        {
            return Ok(None);
        }
        self.record.required_ref("Axis", "IfcDirection").map(Some)
    }

    /// `Thickness`, the constant surface thickness. Only meaningful for
    /// surface members; returns `None` for curve members or when the
    /// attribute is unset. Fails with [`StructuralError::SemanticViolation`]
    /// if present but not a positive finite value.
    pub fn thickness(&self) -> StructuralResult<Option<f64>> {
        if !matches!(self.kind, MemberKind::Surface | MemberKind::SurfaceVarying)
            || !self.record.has_attribute("Thickness")
        {
            return Ok(None);
        }
        let thickness = self.record.optional_number("Thickness")?;
        if thickness.is_some_and(|value| !value.is_finite() || value <= 0.0) {
            return Err(StructuralError::SemanticViolation {
                entity: Some(self.record.id),
                rule: "structural surface member Thickness must be positive and finite",
            });
        }
        Ok(thickness)
    }

    fn validate_semantics(&self) -> StructuralResult<()> {
        if self.record.schema.version() == Some(SchemaVersion::Ifc2x3) {
            return Ok(());
        }
        let user_defined = self
            .record
            .required_enum("PredefinedType")?
            .eq_ignore_ascii_case("USERDEFINED");
        self.record.require_object_type_if(
            user_defined,
            "USERDEFINED structural member requires ObjectType",
        )
    }
}
