//! Idealized point, curve and surface connections.

use ifc_model::EntityId;

use crate::error::{StructuralError, StructuralResult};
use crate::view::Record;

mod curve;
mod point;
mod surface;

/// Which `IfcStructuralConnection` application geometry a connection carries.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectionKind {
    /// Point support: `IfcStructuralPointConnection`.
    Point,
    /// Line support: `IfcStructuralCurveConnection`.
    Curve,
    /// Surface support: `IfcStructuralSurfaceConnection`.
    Surface,
}

/// Borrowed projection of an `IfcStructuralConnection` (point, curve, or surface).
#[derive(Debug, Clone, Copy)]
pub struct StructuralConnection<'m, 's> {
    record: Record<'m, 's>,
    kind: ConnectionKind,
}

impl<'m, 's> StructuralConnection<'m, 's> {
    /// Classify `record`'s [`ConnectionKind`] from its declared IFC type.
    ///
    /// Fails with [`StructuralError::WrongType`] if the type is none of
    /// `IfcStructuralPointConnection`, `IfcStructuralCurveConnection`, or
    /// `IfcStructuralSurfaceConnection`.
    pub(crate) fn from_record(record: Record<'m, 's>) -> StructuralResult<Self> {
        let kind = if record
            .schema
            .is_a(&record.entity.type_name, "IfcStructuralPointConnection")
        {
            ConnectionKind::Point
        } else if record
            .schema
            .is_a(&record.entity.type_name, "IfcStructuralCurveConnection")
        {
            ConnectionKind::Curve
        } else if record
            .schema
            .is_a(&record.entity.type_name, "IfcStructuralSurfaceConnection")
        {
            ConnectionKind::Surface
        } else {
            return Err(StructuralError::WrongType {
                id: record.id,
                expected: "point, curve or surface structural connection",
                actual: record.entity.type_name.to_string(),
            });
        };
        Ok(Self { record, kind })
    }

    #[must_use]
    /// The `IfcStructuralConnection` entity id.
    pub fn id(&self) -> EntityId {
        self.record.id
    }

    #[must_use]
    /// Which application geometry this connection carries.
    pub fn kind(&self) -> ConnectionKind {
        self.kind
    }

    /// `Name`, inherited from `IfcRoot`. Legally absent.
    pub fn name(&self) -> StructuralResult<Option<&'m str>> {
        self.record.optional_text("Name")
    }

    /// `AppliedCondition`, the `IfcBoundaryCondition` fixing this connection's degrees of freedom. Legally absent.
    pub fn applied_condition(&self) -> StructuralResult<Option<EntityId>> {
        self.record
            .optional_ref("AppliedCondition", "IfcBoundaryCondition")
    }

    /// The local axis orientation: `AxisDirection`/`Axis` for a curve connection, or
    /// `ConditionCoordinateSystem` for a point connection. Surface connections have
    /// no comparable attribute here and always return `Ok(None)`.
    pub fn axis(&self) -> StructuralResult<Option<EntityId>> {
        if self.kind == ConnectionKind::Curve {
            for attribute in ["AxisDirection", "Axis"] {
                if self.record.has_attribute(attribute) {
                    return self
                        .record
                        .required_ref(attribute, "IfcDirection")
                        .map(Some);
                }
            }
        }
        if self.kind == ConnectionKind::Point
            && self.record.has_attribute("ConditionCoordinateSystem")
        {
            return self
                .record
                .optional_ref("ConditionCoordinateSystem", "IfcAxis2Placement3D");
        }
        Ok(None)
    }
}
