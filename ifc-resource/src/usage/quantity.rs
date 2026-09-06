//! `IfcPhysicalQuantity` — bounded IFC4 usage-quantity projections.
//!
//! Scope: simple physical quantities (area, count, length, time, volume,
//! weight) and complex composite quantities, as reachable from
//! `IfcConstructionResource.BaseQuantity` or `IfcConstructionResourceType.BaseQuantity`.
//! This module preserves authored measure values; it does not evaluate
//! formulas, resolve units against a project unit assignment, or compute
//! derived quantities.

use ifc_model::EntityId;

use crate::error::{ResourceError, ResourceResult};
use crate::view::Record;

/// A concrete `IfcPhysicalSimpleQuantity` measure kind.
#[derive(Debug, Clone, Copy, PartialEq)]
#[non_exhaustive]
pub enum SimpleQuantityValue {
    Area(f64),
    Count(f64),
    Length(f64),
    Time(f64),
    Volume(f64),
    Weight(f64),
}

fn simple_kind_from_type(type_name: &str) -> Option<&'static str> {
    if type_name.eq_ignore_ascii_case("IfcQuantityArea") {
        Some("AreaValue")
    } else if type_name.eq_ignore_ascii_case("IfcQuantityCount") {
        Some("CountValue")
    } else if type_name.eq_ignore_ascii_case("IfcQuantityLength") {
        Some("LengthValue")
    } else if type_name.eq_ignore_ascii_case("IfcQuantityTime") {
        Some("TimeValue")
    } else if type_name.eq_ignore_ascii_case("IfcQuantityVolume") {
        Some("VolumeValue")
    } else if type_name.eq_ignore_ascii_case("IfcQuantityWeight") {
        Some("WeightValue")
    } else {
        None
    }
}

fn simple_value(type_name: &str, raw: f64) -> SimpleQuantityValue {
    if type_name.eq_ignore_ascii_case("IfcQuantityArea") {
        SimpleQuantityValue::Area(raw)
    } else if type_name.eq_ignore_ascii_case("IfcQuantityCount") {
        SimpleQuantityValue::Count(raw)
    } else if type_name.eq_ignore_ascii_case("IfcQuantityLength") {
        SimpleQuantityValue::Length(raw)
    } else if type_name.eq_ignore_ascii_case("IfcQuantityTime") {
        SimpleQuantityValue::Time(raw)
    } else if type_name.eq_ignore_ascii_case("IfcQuantityVolume") {
        SimpleQuantityValue::Volume(raw)
    } else {
        SimpleQuantityValue::Weight(raw)
    }
}

/// A borrowed, schema-resolved `IfcPhysicalSimpleQuantity` projection.
///
/// Enforces the shared non-negativity rule declared on every concrete
/// simple-quantity subtype (`WR21`/`WR22` in the official schema).
#[derive(Debug, Clone, Copy)]
pub struct SimpleQuantity<'m, 's> {
    record: Record<'m, 's>,
    attribute: &'static str,
}

impl<'m, 's> SimpleQuantity<'m, 's> {
    pub(crate) fn from_record(record: Record<'m, 's>) -> ResourceResult<Self> {
        let attribute = simple_kind_from_type(&record.entity.type_name).ok_or_else(|| {
            ResourceError::WrongType {
                id: record.id,
                expected: "concrete IfcPhysicalSimpleQuantity",
                actual: record.entity.type_name.to_string(),
            }
        })?;
        Ok(Self { record, attribute })
    }

    #[must_use]
    pub fn id(&self) -> EntityId {
        self.record.id
    }

    pub fn name(&self) -> ResourceResult<&'m str> {
        self.record.required_text("Name")
    }

    pub fn description(&self) -> ResourceResult<Option<&'m str>> {
        self.record.optional_text("Description")
    }

    pub fn unit(&self) -> ResourceResult<Option<EntityId>> {
        self.record.optional_ref("Unit", "IfcNamedUnit")
    }

    pub fn formula(&self) -> ResourceResult<Option<&'m str>> {
        self.record.optional_text("Formula")
    }

    /// The authored measure value, typed by the concrete quantity kind.
    pub fn value(&self) -> ResourceResult<SimpleQuantityValue> {
        let raw = self.record.required_non_negative_number(self.attribute)?;
        Ok(simple_value(&self.record.entity.type_name, raw))
    }
}

/// A borrowed, schema-resolved `IfcPhysicalComplexQuantity` projection.
///
/// `HasQuantities` is a `SET`: the official schema forbids a complex
/// quantity from listing itself among its own members (`NoSelfReference`).
/// This projection enforces that rule and reports authored-order members as
/// entity references rather than eagerly recursing, since a complex
/// quantity may nest further complex quantities without a fixed bound.
#[derive(Debug, Clone, Copy)]
pub struct ComplexQuantity<'m, 's> {
    record: Record<'m, 's>,
}

impl<'m, 's> ComplexQuantity<'m, 's> {
    pub(crate) fn from_record(record: Record<'m, 's>) -> ResourceResult<Self> {
        let quantity = Self { record };
        let members = quantity.member_ids()?;
        if members.contains(&quantity.record.id) {
            return Err(ResourceError::SemanticViolation {
                entity: Some(quantity.record.id),
                rule: "IfcPhysicalComplexQuantity.NoSelfReference",
            });
        }
        Ok(quantity)
    }

    #[must_use]
    pub fn id(&self) -> EntityId {
        self.record.id
    }

    pub fn name(&self) -> ResourceResult<&'m str> {
        self.record.required_text("Name")
    }

    pub fn description(&self) -> ResourceResult<Option<&'m str>> {
        self.record.optional_text("Description")
    }

    pub fn discrimination(&self) -> ResourceResult<&'m str> {
        self.record.required_text("Discrimination")
    }

    pub fn quality(&self) -> ResourceResult<Option<&'m str>> {
        self.record.optional_text("Quality")
    }

    pub fn usage(&self) -> ResourceResult<Option<&'m str>> {
        self.record.optional_text("Usage")
    }

    pub fn member_ids(&self) -> ResourceResult<Vec<EntityId>> {
        self.record
            .refs("HasQuantities", "IfcPhysicalQuantity", 1, false, true)
    }
}
