//! `IfcStructuralLoadConfiguration` projections.

use ifc_model::{EntityId, Value};

use crate::error::{StructuralError, StructuralResult};
use crate::view::Record;

/// Ordered load/result values with optional one- or two-coordinate locations.
///
/// This preserves authored configuration data. It does not interpolate,
/// combine, envelope, or otherwise evaluate the values.
#[derive(Debug, Clone, Copy)]
pub struct LoadConfiguration<'m, 's> {
    record: Record<'m, 's>,
}

impl<'m, 's> LoadConfiguration<'m, 's> {
    pub(crate) fn from_record(record: Record<'m, 's>) -> Self {
        Self { record }
    }

    /// Entity identifier in the shared model graph.
    #[must_use]
    pub fn id(&self) -> EntityId {
        self.record.id
    }

    /// Optional authored name.
    pub fn name(&self) -> StructuralResult<Option<&'m str>> {
        self.record.optional_text("Name")
    }

    /// Ordered references to `IfcStructuralLoadOrResult` values.
    pub fn values(&self) -> StructuralResult<Vec<EntityId>> {
        let values = match self.record.value("Values")?.unwrap_typed() {
            Value::List(values) if !values.is_empty() => values,
            Value::List(values) => {
                return Err(StructuralError::InvalidCardinality {
                    entity: self.record.id,
                    attribute: "Values",
                    minimum: 1,
                    maximum: None,
                    actual: values.len(),
                })
            }
            _ => {
                return Err(StructuralError::InvalidValue {
                    entity: self.record.id,
                    attribute: "Values",
                    expected: "LIST [1:?] of IfcStructuralLoadOrResult references",
                })
            }
        };
        values
            .iter()
            .map(|value| self.load_or_result_ref(value))
            .collect()
    }

    /// Optional authored locations, preserving outer and inner list order.
    ///
    /// Each location contains one or two finite length coordinates. The outer
    /// list is unique and, when present, has the same length as `values`.
    pub fn locations(&self) -> StructuralResult<Option<Vec<Vec<f64>>>> {
        let raw = match self.record.value("Locations")?.unwrap_typed() {
            Value::Null | Value::Derived => return Ok(None),
            Value::List(values) if !values.is_empty() => values,
            Value::List(values) => {
                return Err(StructuralError::InvalidCardinality {
                    entity: self.record.id,
                    attribute: "Locations",
                    minimum: 1,
                    maximum: None,
                    actual: values.len(),
                })
            }
            _ => {
                return Err(StructuralError::InvalidValue {
                    entity: self.record.id,
                    attribute: "Locations",
                    expected: "LIST [1:?] of unique LIST [1:2] finite lengths",
                })
            }
        };
        let value_count = self.values()?.len();
        if raw.len() != value_count {
            return Err(StructuralError::SemanticViolation {
                entity: Some(self.record.id),
                rule: "IfcStructuralLoadConfiguration.ValidListSize",
            });
        }
        let mut locations = Vec::with_capacity(raw.len());
        for raw_location in raw {
            let values = match raw_location.unwrap_typed() {
                Value::List(values) if (1..=2).contains(&values.len()) => values,
                Value::List(values) => {
                    return Err(StructuralError::InvalidCardinality {
                        entity: self.record.id,
                        attribute: "Locations",
                        minimum: 1,
                        maximum: Some(2),
                        actual: values.len(),
                    })
                }
                _ => {
                    return Err(StructuralError::InvalidValue {
                        entity: self.record.id,
                        attribute: "Locations",
                        expected: "LIST [1:2] of finite length measures",
                    })
                }
            };
            let mut location = Vec::with_capacity(values.len());
            for value in values {
                let number = match value.unwrap_typed() {
                    Value::Integer(value) => *value as f64,
                    Value::Real(value) => *value,
                    _ => {
                        return Err(StructuralError::InvalidValue {
                            entity: self.record.id,
                            attribute: "Locations",
                            expected: "finite length measure",
                        })
                    }
                };
                if !number.is_finite() {
                    return Err(StructuralError::InvalidValue {
                        entity: self.record.id,
                        attribute: "Locations",
                        expected: "finite length measure",
                    });
                }
                location.push(number);
            }
            if locations.contains(&location) {
                return Err(StructuralError::SemanticViolation {
                    entity: Some(self.record.id),
                    rule: "IfcStructuralLoadConfiguration.UniqueLocations",
                });
            }
            locations.push(location);
        }
        Ok(Some(locations))
    }

    fn load_or_result_ref(&self, value: &Value) -> StructuralResult<EntityId> {
        let Value::Ref(id) = value.unwrap_typed() else {
            return Err(StructuralError::InvalidValue {
                entity: self.record.id,
                attribute: "Values",
                expected: "IfcStructuralLoadOrResult reference",
            });
        };
        let target = self
            .record
            .model
            .get(*id)
            .ok_or(StructuralError::DanglingReference {
                entity: self.record.id,
                attribute: "Values",
                target: *id,
            })?;
        if !self
            .record
            .schema
            .is_a(&target.type_name, "IfcStructuralLoadOrResult")
        {
            return Err(StructuralError::WrongType {
                id: *id,
                expected: "IfcStructuralLoadOrResult",
                actual: target.type_name.to_string(),
            });
        }
        Ok(*id)
    }
}
