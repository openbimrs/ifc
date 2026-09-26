//! One unit entity to its SI scale and dimensions.
//!
//! Slot positions are the `IfcNamedUnit` family's; `assignment.rs` documents
//! them against the EXPRESS schema. Each record is checked for exact arity
//! first, so indexing a slot is safe.
//!
//! A conversion-based unit is `factor × UnitComponent` (IFC4
//! `IfcConversionBasedUnit`: "inch, length measure equal to 25.4 mm"), and its
//! `UnitComponent` may itself be conversion-based or derived. The walk keeps
//! the chain of units in progress, so a cycle is reported with its members,
//! and it stops at [`ifc_model::Budget::DEFAULT`]'s depth.

use std::collections::BTreeSet;
use std::sync::Arc;

use ifc_model::{Budget, Entity, EntityId, Model, Value};
use ifc_schema::TypeKind;

use super::super::refs::{nonempty_refs_at, ref_at};
use super::super::release::Release;
use super::super::value::{select_accepts_entity, select_accepts_type, typed_payload_matches};
use super::super::ExactPropertyError;
use super::ExactUnitError;
use crate::unit::prefix_exponent;
use crate::unit::si::{si_name_dimensions, si_to_base, unit_enum_dimensions, Dimensions};

const NAMED_DIMENSIONS: usize = 0;
const NAMED_UNIT_TYPE: usize = 1;
const SI_PREFIX: usize = 2;
const SI_NAME: usize = 3;
const CONVERSION_FACTOR: usize = 3;
const MEASURE_VALUE: usize = 0;
const MEASURE_UNIT: usize = 1;
const DERIVED_ELEMENTS: usize = 0;
const DERIVED_TYPE: usize = 1;
const ELEMENT_UNIT: usize = 0;
const ELEMENT_EXPONENT: usize = 1;

/// A unit converted to SI base units.
pub(super) struct Resolved {
    pub(super) unit_type: Arc<str>,
    pub(super) dimensions: Dimensions,
    pub(super) scale: f64,
    pub(super) offset: f64,
}

pub(super) struct Resolver<'m> {
    model: &'m Model,
    release: Release,
    chain: Vec<EntityId>,
}

impl<'m> Resolver<'m> {
    pub(super) fn new(model: &'m Model, release: Release) -> Self {
        Self {
            model,
            release,
            chain: Vec::new(),
        }
    }

    pub(super) fn resolve(&mut self, unit: EntityId) -> Result<Resolved, ExactUnitError> {
        self.resolve_from(unit, unit)
    }

    fn resolve_from(&mut self, from: EntityId, unit: EntityId) -> Result<Resolved, ExactUnitError> {
        if let Some(position) = self.chain.iter().position(|seen| *seen == unit) {
            let mut cycle = self.chain[position..].to_vec();
            cycle.push(unit);
            return Err(ExactUnitError::CyclicConversion { cycle });
        }
        let max_depth = Budget::DEFAULT.max_depth;
        if self.chain.len() >= max_depth {
            return Err(ExactUnitError::ConversionChainTooDeep { unit, max_depth });
        }
        let entity = self
            .model
            .get(unit)
            .ok_or(ExactPropertyError::MissingReference { from, to: unit })?;
        self.release.require_exact_slots(unit, entity)?;
        if !select_accepts_entity(self.release.schema, "IFCUNIT", &entity.type_name) {
            return Err(unsupported(unit, entity));
        }
        self.chain.push(unit);
        let result = self.dispatch(unit, entity);
        self.chain.pop();
        result
    }

    fn dispatch(&mut self, unit: EntityId, entity: &Entity) -> Result<Resolved, ExactUnitError> {
        let is_a = |ancestor| self.release.schema.is_a(&entity.type_name, ancestor);
        if is_a("IFCSIUNIT") {
            self.si(unit, entity)
        } else if is_a("IFCCONVERSIONBASEDUNITWITHOFFSET") {
            Err(ExactUnitError::UnsupportedOffset { unit })
        } else if is_a("IFCCONVERSIONBASEDUNIT") {
            self.conversion(unit, entity)
        } else if is_a("IFCDERIVEDUNIT") {
            self.derived(unit, entity)
        } else {
            Err(unsupported(unit, entity))
        }
    }

    fn si(&self, unit: EntityId, entity: &Entity) -> Result<Resolved, ExactUnitError> {
        let unit_type = named_unit_type(self.release, unit, entity)?;
        let exponent = match &entity.attributes[SI_PREFIX] {
            Value::Null => 0,
            Value::Enum(prefix) => {
                prefix_exponent(&prefix.to_ascii_uppercase()).ok_or_else(|| {
                    ExactUnitError::UnknownPrefix {
                        unit,
                        prefix: prefix.clone(),
                    }
                })?
            }
            _ => return Err(malformed(unit, "Prefix")),
        };
        let Value::Enum(name) = &entity.attributes[SI_NAME] else {
            return Err(malformed(unit, "Name"));
        };
        let dimensions = si_name_dimensions(self.release.version, name).ok_or_else(|| {
            ExactUnitError::UnknownUnitName {
                unit,
                name: name.clone(),
            }
        })?;
        require_dimensions(self.release, unit, &unit_type, dimensions)?;
        let (scale, offset) = si_to_base(exponent, name);
        Ok(Resolved {
            unit_type,
            dimensions,
            scale,
            offset,
        })
    }

    fn conversion(&mut self, unit: EntityId, entity: &Entity) -> Result<Resolved, ExactUnitError> {
        let unit_type = named_unit_type(self.release, unit, entity)?;
        let declared = self.declared_dimensions(unit, entity)?;
        require_dimensions(self.release, unit, &unit_type, declared)?;
        let measure = ref_at(
            unit,
            entity.attributes.get(CONVERSION_FACTOR),
            "ConversionFactor",
        )?;
        let measure_entity = self.record(unit, measure)?;
        if !self
            .release
            .schema
            .is_a(&measure_entity.type_name, "IFCMEASUREWITHUNIT")
        {
            return Err(malformed(unit, "ConversionFactor"));
        }
        let factor = self.factor(measure, &measure_entity.attributes[MEASURE_VALUE])?;
        let component = ref_at(
            measure,
            measure_entity.attributes.get(MEASURE_UNIT),
            "UnitComponent",
        )?;
        let inner = self.resolve_from(measure, component)?;
        if inner.offset != 0.0 {
            return Err(ExactUnitError::UnsupportedOffset { unit: component });
        }
        if inner.dimensions != declared {
            return Err(ExactUnitError::DimensionMismatch {
                unit,
                expected: declared,
                found: inner.dimensions,
            });
        }
        Ok(Resolved {
            unit_type,
            dimensions: declared,
            scale: factor * inner.scale,
            offset: 0.0,
        })
    }

    fn derived(&mut self, unit: EntityId, entity: &Entity) -> Result<Resolved, ExactUnitError> {
        let unit_type = enum_constant(
            self.release,
            unit,
            &entity.attributes[DERIVED_TYPE],
            "IFCDERIVEDUNITENUM",
            "UnitType",
        )?;
        let mut dimensions = [0; 7];
        let mut scale = 1.0;
        for element in nonempty_refs_at(unit, entity.attributes.get(DERIVED_ELEMENTS), "Elements")?
        {
            let element_entity = self.record(unit, element)?;
            if !element_entity.is_type("IFCDERIVEDUNITELEMENT") {
                return Err(malformed(unit, "Elements"));
            }
            let Value::Integer(exponent) = element_entity.attributes[ELEMENT_EXPONENT] else {
                return Err(malformed(element, "Exponent"));
            };
            let exponent = i32::try_from(exponent).map_err(|_| malformed(element, "Exponent"))?;
            let named = ref_at(element, element_entity.attributes.get(ELEMENT_UNIT), "Unit")?;
            let inner = self.resolve_from(element, named)?;
            if inner.offset != 0.0 {
                return Err(ExactUnitError::UnsupportedOffset { unit: named });
            }
            for (total, part) in dimensions.iter_mut().zip(inner.dimensions) {
                *total += exponent * part;
            }
            scale *= inner.scale.powi(exponent);
        }
        Ok(Resolved {
            unit_type,
            dimensions,
            scale,
            offset: 0.0,
        })
    }

    /// A referenced record that exists and has the release's exact arity.
    fn record(&self, from: EntityId, id: EntityId) -> Result<&'m Entity, ExactUnitError> {
        let entity = self
            .model
            .get(id)
            .ok_or(ExactPropertyError::MissingReference { from, to: id })?;
        self.release.require_exact_slots(id, entity)?;
        Ok(entity)
    }

    /// `IfcNamedUnit.Dimensions` as an explicit `IfcDimensionalExponents`.
    fn declared_dimensions(
        &self,
        unit: EntityId,
        entity: &Entity,
    ) -> Result<Dimensions, ExactUnitError> {
        let id = ref_at(unit, entity.attributes.get(NAMED_DIMENSIONS), "Dimensions")?;
        let exponents = self.record(unit, id)?;
        if !exponents.is_type("IFCDIMENSIONALEXPONENTS") {
            return Err(malformed(unit, "Dimensions"));
        }
        let mut dimensions = [0; 7];
        for (slot, value) in dimensions.iter_mut().zip(&exponents.attributes) {
            let Value::Integer(exponent) = value else {
                return Err(malformed(id, "Dimensions"));
            };
            *slot = i32::try_from(*exponent).map_err(|_| malformed(id, "Dimensions"))?;
        }
        Ok(dimensions)
    }

    /// `IfcMeasureWithUnit.ValueComponent` as a positive finite factor.
    fn factor(&self, measure: EntityId, value: &Value) -> Result<f64, ExactUnitError> {
        let schema = self.release.schema;
        let Value::Typed { type_name, value } = value else {
            return Err(malformed(measure, "ValueComponent"));
        };
        if !select_accepts_type(schema, "IFCVALUE", type_name)
            || !typed_payload_matches(schema, type_name, value, &mut BTreeSet::new())
        {
            return Err(malformed(measure, "ValueComponent"));
        }
        match value.unwrap_typed().as_f64() {
            Some(factor) if factor.is_finite() && factor > 0.0 => Ok(factor),
            _ => Err(malformed(measure, "ValueComponent")),
        }
    }
}

/// The unit type a project unit declares: an `IfcUnitEnum` for a named unit,
/// an `IfcDerivedUnitEnum` for a derived one, none for a monetary unit.
pub(super) fn declared_unit_type(
    release: Release,
    unit: EntityId,
    entity: &Entity,
) -> Result<Option<Arc<str>>, ExactUnitError> {
    if release.schema.is_a(&entity.type_name, "IFCNAMEDUNIT") {
        named_unit_type(release, unit, entity).map(Some)
    } else if release.schema.is_a(&entity.type_name, "IFCDERIVEDUNIT") {
        enum_constant(
            release,
            unit,
            &entity.attributes[DERIVED_TYPE],
            "IFCDERIVEDUNITENUM",
            "UnitType",
        )
        .map(Some)
    } else {
        Ok(None)
    }
}

fn named_unit_type(
    release: Release,
    unit: EntityId,
    entity: &Entity,
) -> Result<Arc<str>, ExactUnitError> {
    enum_constant(
        release,
        unit,
        &entity.attributes[NAMED_UNIT_TYPE],
        "IFCUNITENUM",
        "UnitType",
    )
}

/// An enumeration constant the release declares for `enumeration`.
fn enum_constant(
    release: Release,
    entity: EntityId,
    value: &Value,
    enumeration: &str,
    attribute: &'static str,
) -> Result<Arc<str>, ExactUnitError> {
    let Value::Enum(constant) = value else {
        return Err(malformed(entity, attribute));
    };
    let declared = match release
        .schema
        .type_def(enumeration)
        .map(|definition| &definition.kind)
    {
        Some(TypeKind::Enumeration(members)) => members
            .iter()
            .any(|member| member.eq_ignore_ascii_case(constant)),
        _ => false,
    };
    if declared {
        Ok(constant.to_ascii_uppercase().into())
    } else {
        Err(malformed(entity, attribute))
    }
}

/// `IfcCorrectDimensions`: a named unit's dimensions must match its type.
fn require_dimensions(
    release: Release,
    unit: EntityId,
    unit_type: &Arc<str>,
    found: Dimensions,
) -> Result<(), ExactUnitError> {
    let expected = unit_enum_dimensions(release.version, unit_type).ok_or_else(|| {
        ExactUnitError::UnknownUnitName {
            unit,
            name: unit_type.clone(),
        }
    })?;
    if expected == found {
        Ok(())
    } else {
        Err(ExactUnitError::DimensionMismatch {
            unit,
            expected,
            found,
        })
    }
}

fn malformed(entity: EntityId, attribute: &'static str) -> ExactUnitError {
    ExactUnitError::MalformedUnit { entity, attribute }
}

fn unsupported(unit: EntityId, entity: &Entity) -> ExactUnitError {
    ExactUnitError::UnsupportedUnit {
        unit,
        type_name: entity.type_name.clone(),
    }
}
