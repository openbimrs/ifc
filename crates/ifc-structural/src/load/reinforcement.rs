//! `IfcSurfaceReinforcementArea` projection (IFC4 and IFC4X3).
//!
//! The entity is an `IfcStructuralLoadOrResult` beside, not under,
//! `IfcStructuralLoadStatic`, and IFC2X3 does not declare it: the view
//! refuses it there with [`StructuralError::UnsupportedSchema`] rather than
//! reading a same-named record by guesswork.

use ifc_model::{EntityId, Value};

use crate::error::{StructuralError, StructuralResult};
use crate::view::Record;

/// Borrowed projection of an `IfcSurfaceReinforcementArea`: required
/// reinforcement areas per unit length of a surface member, as a design
/// result. It preserves the authored values; it does not design anything.
#[derive(Debug, Clone, Copy)]
pub struct SurfaceReinforcementArea<'m, 's> {
    record: Record<'m, 's>,
}

impl<'m, 's> SurfaceReinforcementArea<'m, 's> {
    /// Wrap `record`, enforcing `SurfaceAndOrShearAreaSpecified`: at least
    /// one of the three areas exists.
    pub(crate) fn from_record(record: Record<'m, 's>) -> StructuralResult<Self> {
        let area = Self { record };
        let specified = [
            "SurfaceReinforcement1",
            "SurfaceReinforcement2",
            "ShearReinforcement",
        ]
        .into_iter()
        .map(|attribute| {
            area.record
                .value(attribute)
                .map(|value| !matches!(value.unwrap_typed(), Value::Null | Value::Derived))
        })
        .collect::<StructuralResult<Vec<_>>>()?;
        if !specified.contains(&true) {
            return Err(StructuralError::SemanticViolation {
                entity: Some(area.record.id),
                rule: "SurfaceAndOrShearAreaSpecified",
            });
        }
        Ok(area)
    }

    #[must_use]
    /// The `IfcSurfaceReinforcementArea` entity id.
    pub fn id(&self) -> EntityId {
        self.record.id
    }

    /// `Name`, inherited from `IfcStructuralLoad`. Legally absent.
    pub fn name(&self) -> StructuralResult<Option<&'m str>> {
        self.record.optional_text("Name")
    }

    /// `SurfaceReinforcement1`: `LIST [2:3] OF IfcLengthMeasure`, the
    /// reinforcement area in the first direction. Legally absent.
    ///
    /// Refused with [`StructuralError::InvalidCardinality`] outside two or
    /// three entries, and with [`StructuralError::SemanticViolation`]
    /// (`NonnegativeArea1`) when the first or second entry is negative, the
    /// entries the published rule constrains.
    pub fn surface_reinforcement_1(&self) -> StructuralResult<Option<Vec<f64>>> {
        self.area_list("SurfaceReinforcement1", "NonnegativeArea1")
    }

    /// `SurfaceReinforcement2`: `LIST [2:3] OF IfcLengthMeasure`, the
    /// reinforcement area in the second direction. Legally absent.
    ///
    /// Refused as [`SurfaceReinforcementArea::surface_reinforcement_1`] is,
    /// with rule `NonnegativeArea2`.
    pub fn surface_reinforcement_2(&self) -> StructuralResult<Option<Vec<f64>>> {
        self.area_list("SurfaceReinforcement2", "NonnegativeArea2")
    }

    /// `ShearReinforcement`: `IfcRatioMeasure`. Legally absent; refused with
    /// [`StructuralError::SemanticViolation`] (`NonnegativeArea3`) when
    /// negative.
    pub fn shear_reinforcement(&self) -> StructuralResult<Option<f64>> {
        let value = self.record.optional_number("ShearReinforcement")?;
        if value.is_some_and(|number| number < 0.0) {
            return Err(StructuralError::SemanticViolation {
                entity: Some(self.record.id),
                rule: "NonnegativeArea3",
            });
        }
        Ok(value)
    }

    fn area_list(
        &self,
        attribute: &'static str,
        rule: &'static str,
    ) -> StructuralResult<Option<Vec<f64>>> {
        let values = match self.record.value(attribute)?.unwrap_typed() {
            Value::Null | Value::Derived => return Ok(None),
            Value::List(values) => values,
            _ => {
                return Err(StructuralError::InvalidValue {
                    entity: self.record.id,
                    attribute,
                    expected: "LIST [2:3] OF IfcLengthMeasure or null",
                })
            }
        };
        if !(2..=3).contains(&values.len()) {
            return Err(StructuralError::InvalidCardinality {
                entity: self.record.id,
                attribute,
                minimum: 2,
                maximum: Some(3),
                actual: values.len(),
            });
        }
        let numbers = values
            .iter()
            .map(|value| match value.unwrap_typed() {
                Value::Integer(number) => Ok(*number as f64),
                Value::Real(number) if number.is_finite() => Ok(*number),
                _ => Err(StructuralError::InvalidValue {
                    entity: self.record.id,
                    attribute,
                    expected: "LIST [2:3] OF finite IfcLengthMeasure",
                }),
            })
            .collect::<StructuralResult<Vec<f64>>>()?;
        if numbers[..2].iter().any(|number| *number < 0.0) {
            return Err(StructuralError::SemanticViolation {
                entity: Some(self.record.id),
                rule,
            });
        }
        Ok(Some(numbers))
    }
}
