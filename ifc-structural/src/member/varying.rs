//! Version-aware varying-member payloads.

use ifc_model::{EntityId, Value};

use crate::error::{StructuralError, StructuralResult};
use crate::member::{Member, MemberKind};

impl Member<'_, '_> {
    /// Whether the entity is one of the varying-member marker subtypes.
    #[must_use]
    pub fn is_varying(&self) -> bool {
        matches!(
            self.kind,
            MemberKind::CurveVarying | MemberKind::SurfaceVarying
        )
    }

    /// IFC2X3 surface-varying thicknesses after the inherited first thickness.
    ///
    /// IFC4 and IFC4X3 retain the varying subtype as a marker without these
    /// attributes, so this returns `None` for those schemas and for curve members.
    pub fn subsequent_thicknesses(&self) -> StructuralResult<Option<Vec<f64>>> {
        if self.kind != MemberKind::SurfaceVarying
            || !self.record.has_attribute("SubsequentThickness")
        {
            return Ok(None);
        }
        if self.thickness()?.is_none() {
            return Err(StructuralError::SemanticViolation {
                entity: Some(self.record.id),
                rule: "IfcStructuralSurfaceMemberVarying.WR61",
            });
        }
        let values = match self.record.value("SubsequentThickness")?.unwrap_typed() {
            Value::List(values) => values,
            _ => {
                return Err(StructuralError::InvalidValue {
                    entity: self.record.id,
                    attribute: "SubsequentThickness",
                    expected: "LIST [2:?] of positive finite thickness measures",
                })
            }
        };
        if values.len() < 2 {
            return Err(StructuralError::InvalidCardinality {
                entity: self.record.id,
                attribute: "SubsequentThickness",
                minimum: 2,
                maximum: None,
                actual: values.len(),
            });
        }
        let mut thicknesses = Vec::with_capacity(values.len());
        for value in values {
            let number = match value.unwrap_typed() {
                Value::Integer(value) => *value as f64,
                Value::Real(value) => *value,
                _ => {
                    return Err(StructuralError::InvalidValue {
                        entity: self.record.id,
                        attribute: "SubsequentThickness",
                        expected: "LIST [2:?] of positive finite thickness measures",
                    })
                }
            };
            if !number.is_finite() || number <= 0.0 {
                return Err(StructuralError::InvalidValue {
                    entity: self.record.id,
                    attribute: "SubsequentThickness",
                    expected: "LIST [2:?] of positive finite thickness measures",
                });
            }
            thicknesses.push(number);
        }
        Ok(Some(thicknesses))
    }

    /// IFC2X3 shape aspect locating the varying surface-member thicknesses.
    ///
    /// Returns `None` when the selected schema does not declare this attribute.
    pub fn varying_thickness_location(&self) -> StructuralResult<Option<EntityId>> {
        if self.kind != MemberKind::SurfaceVarying
            || !self.record.has_attribute("VaryingThicknessLocation")
        {
            return Ok(None);
        }
        self.record
            .required_ref("VaryingThicknessLocation", "IfcShapeAspect")
            .map(Some)
    }
}
