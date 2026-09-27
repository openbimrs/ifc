//! Typed structural boundary and connection conditions.
//!
//! Values are preserved as authored. Boolean stiffness selectors are not
//! interpreted as solver constraints, and numeric values are not unit-converted.

use ifc_model::{EntityId, Value};
use ifc_schema::SchemaVersion;

use crate::error::{StructuralError, StructuralResult};
use crate::view::Record;

mod rotation;
mod translation;

/// Three values ordered by the structural X, Y, and Z axes.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AxisValues<T> {
    /// X-axis value.
    pub x: T,
    /// Y-axis value.
    pub y: T,
    /// Z-axis value.
    pub z: T,
}

impl<T: Default> Default for AxisValues<T> {
    fn default() -> Self {
        Self {
            x: T::default(),
            y: T::default(),
            z: T::default(),
        }
    }
}

/// One authored IFC stiffness SELECT member.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum StiffnessValue {
    /// IFC4/IFC4X3 `IfcBoolean`, preserved literally without interpreting it.
    Boolean(bool),
    /// One finite stiffness or subgrade-reaction measure.
    Measure(f64),
}

/// Concrete boundary-condition family.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BoundaryConditionKind {
    /// `IfcBoundaryEdgeCondition`.
    Edge,
    /// `IfcBoundaryFaceCondition`.
    Face,
    /// `IfcBoundaryNodeCondition`.
    Node,
    /// `IfcBoundaryNodeConditionWarping`.
    NodeWarping,
}

/// Strict borrowed projection of a concrete IFC boundary condition.
#[derive(Debug, Clone, Copy)]
pub struct BoundaryCondition<'m, 's> {
    record: Record<'m, 's>,
    kind: BoundaryConditionKind,
}

impl<'m, 's> BoundaryCondition<'m, 's> {
    pub(crate) fn from_record(record: Record<'m, 's>) -> StructuralResult<Self> {
        let kind = if record
            .schema
            .is_a(&record.entity.type_name, "IfcBoundaryNodeConditionWarping")
        {
            BoundaryConditionKind::NodeWarping
        } else if record
            .schema
            .is_a(&record.entity.type_name, "IfcBoundaryNodeCondition")
        {
            BoundaryConditionKind::Node
        } else if record
            .schema
            .is_a(&record.entity.type_name, "IfcBoundaryEdgeCondition")
        {
            BoundaryConditionKind::Edge
        } else if record
            .schema
            .is_a(&record.entity.type_name, "IfcBoundaryFaceCondition")
        {
            BoundaryConditionKind::Face
        } else {
            return Err(StructuralError::WrongType {
                id: record.id,
                expected: "concrete IfcBoundaryCondition",
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

    /// Concrete boundary-condition family.
    #[must_use]
    pub fn kind(&self) -> BoundaryConditionKind {
        self.kind
    }

    /// Optional authored name.
    pub fn name(&self) -> StructuralResult<Option<&'m str>> {
        self.record.optional_text("Name")
    }

    /// Translational stiffness values in X/Y/Z order.
    pub fn translational_stiffnesses(
        &self,
    ) -> StructuralResult<AxisValues<Option<StiffnessValue>>> {
        let (current, legacy) = match self.kind {
            BoundaryConditionKind::Edge => (
                [
                    "TranslationalStiffnessByLengthX",
                    "TranslationalStiffnessByLengthY",
                    "TranslationalStiffnessByLengthZ",
                ],
                [
                    "LinearStiffnessByLengthX",
                    "LinearStiffnessByLengthY",
                    "LinearStiffnessByLengthZ",
                ],
            ),
            BoundaryConditionKind::Face => (
                [
                    "TranslationalStiffnessByAreaX",
                    "TranslationalStiffnessByAreaY",
                    "TranslationalStiffnessByAreaZ",
                ],
                [
                    "LinearStiffnessByAreaX",
                    "LinearStiffnessByAreaY",
                    "LinearStiffnessByAreaZ",
                ],
            ),
            BoundaryConditionKind::Node | BoundaryConditionKind::NodeWarping => (
                [
                    "TranslationalStiffnessX",
                    "TranslationalStiffnessY",
                    "TranslationalStiffnessZ",
                ],
                ["LinearStiffnessX", "LinearStiffnessY", "LinearStiffnessZ"],
            ),
        };
        self.stiffness_axes(current, legacy)
    }

    /// Rotational stiffness values in X/Y/Z order.
    ///
    /// Face conditions do not declare rotational values and return three `None`
    /// entries rather than manufacturing an unsupported field.
    pub fn rotational_stiffnesses(&self) -> StructuralResult<AxisValues<Option<StiffnessValue>>> {
        if self.kind == BoundaryConditionKind::Face {
            return Ok(AxisValues::default());
        }
        let names = if self.kind == BoundaryConditionKind::Edge {
            [
                "RotationalStiffnessByLengthX",
                "RotationalStiffnessByLengthY",
                "RotationalStiffnessByLengthZ",
            ]
        } else {
            [
                "RotationalStiffnessX",
                "RotationalStiffnessY",
                "RotationalStiffnessZ",
            ]
        };
        self.stiffness_axes(names, names)
    }

    /// Optional warping stiffness for `IfcBoundaryNodeConditionWarping`.
    pub fn warping_stiffness(&self) -> StructuralResult<Option<StiffnessValue>> {
        if self.kind != BoundaryConditionKind::NodeWarping {
            return Ok(None);
        }
        decode_stiffness(&self.record, "WarpingStiffness")
    }

    fn stiffness_axes(
        &self,
        current: [&'static str; 3],
        legacy: [&'static str; 3],
    ) -> StructuralResult<AxisValues<Option<StiffnessValue>>> {
        let value = |index| {
            let selected = if self.record.has_attribute(current[index]) {
                current[index]
            } else {
                legacy[index]
            };
            decode_stiffness(&self.record, selected)
        };
        Ok(AxisValues {
            x: value(0)?,
            y: value(1)?,
            z: value(2)?,
        })
    }
}

fn decode_stiffness(
    record: &Record<'_, '_>,
    attribute: &'static str,
) -> StructuralResult<Option<StiffnessValue>> {
    match record.value(attribute)?.unwrap_typed() {
        Value::Null | Value::Derived => Ok(None),
        Value::Bool(value) if record.schema.version() != Some(SchemaVersion::Ifc2x3) => {
            Ok(Some(StiffnessValue::Boolean(*value)))
        }
        Value::Integer(value) => Ok(Some(StiffnessValue::Measure(*value as f64))),
        Value::Real(value) if value.is_finite() => Ok(Some(StiffnessValue::Measure(*value))),
        _ => Err(StructuralError::InvalidValue {
            entity: record.id,
            attribute,
            expected: "finite stiffness measure or schema-supported boolean",
        }),
    }
}

/// Concrete structural connection-condition family.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectionConditionKind {
    /// `IfcFailureConnectionCondition`.
    Failure,
    /// `IfcSlippageConnectionCondition`.
    Slippage,
}

/// Authored force limits for a failure connection condition.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FailureLimits {
    /// Tension limits in X/Y/Z order.
    pub tension: AxisValues<Option<f64>>,
    /// Compression limits in X/Y/Z order.
    pub compression: AxisValues<Option<f64>>,
}

/// Strict borrowed projection of a concrete connection condition.
#[derive(Debug, Clone, Copy)]
pub struct ConnectionCondition<'m, 's> {
    record: Record<'m, 's>,
    kind: ConnectionConditionKind,
}

impl<'m, 's> ConnectionCondition<'m, 's> {
    pub(crate) fn from_record(record: Record<'m, 's>) -> StructuralResult<Self> {
        let kind = if record
            .schema
            .is_a(&record.entity.type_name, "IfcFailureConnectionCondition")
        {
            ConnectionConditionKind::Failure
        } else if record
            .schema
            .is_a(&record.entity.type_name, "IfcSlippageConnectionCondition")
        {
            ConnectionConditionKind::Slippage
        } else {
            return Err(StructuralError::WrongType {
                id: record.id,
                expected: "concrete IfcStructuralConnectionCondition",
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

    /// Concrete connection-condition family.
    #[must_use]
    pub fn kind(&self) -> ConnectionConditionKind {
        self.kind
    }

    /// Optional authored name.
    pub fn name(&self) -> StructuralResult<Option<&'m str>> {
        self.record.optional_text("Name")
    }

    /// Failure limits, or `None` for a slippage condition.
    pub fn failure_limits(&self) -> StructuralResult<Option<FailureLimits>> {
        if self.kind != ConnectionConditionKind::Failure {
            return Ok(None);
        }
        Ok(Some(FailureLimits {
            tension: number_axes(
                &self.record,
                ["TensionFailureX", "TensionFailureY", "TensionFailureZ"],
            )?,
            compression: number_axes(
                &self.record,
                [
                    "CompressionFailureX",
                    "CompressionFailureY",
                    "CompressionFailureZ",
                ],
            )?,
        }))
    }

    /// Slippage lengths in X/Y/Z order, or `None` for a failure condition.
    pub fn slippages(&self) -> StructuralResult<Option<AxisValues<Option<f64>>>> {
        if self.kind != ConnectionConditionKind::Slippage {
            return Ok(None);
        }
        Ok(Some(number_axes(
            &self.record,
            ["SlippageX", "SlippageY", "SlippageZ"],
        )?))
    }
}

fn number_axes(
    record: &Record<'_, '_>,
    names: [&'static str; 3],
) -> StructuralResult<AxisValues<Option<f64>>> {
    Ok(AxisValues {
        x: record.optional_number(names[0])?,
        y: record.optional_number(names[1])?,
        z: record.optional_number(names[2])?,
    })
}
