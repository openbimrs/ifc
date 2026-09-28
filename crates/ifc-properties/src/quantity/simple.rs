//! `IfcPhysicalSimpleQuantity`: length, area, volume, count, weight, time.
//!
//! # Slots, verified against the EXPRESS schemas
//!
//! ```text
//! IfcPhysicalQuantity        0 = Name   1 = Description   (IFC2X3, IFC4, IFC4X3)
//! IfcPhysicalSimpleQuantity  2 = Unit   (OPTIONAL IfcNamedUnit)
//! IfcQuantity<Kind>          3 = <Kind>Value              (not OPTIONAL)
//!                            4 = Formula                  (IFC4, IFC4X3 only)
//! ```
//!
//! `LengthValue`, `AreaValue`, `VolumeValue`, `CountValue`, `WeightValue`
//! and `TimeValue` are declared without `OPTIONAL` in all three releases.
//! IFC2X3 has no `Formula`, so its records end at slot 3 and `formula`
//! reads as `None`.
//!
//! IFC4X3 adds `IfcQuantityNumber` (`NumberValue : IfcNumericMeasure`,
//! `Formula : OPTIONAL IfcLabel`, no WHERE rule; `IfcNumericMeasure =
//! NUMBER`). It is [`QuantityKind::Number`] only in a model whose declared
//! release declares the entity, and both slots are then looked up by name in
//! that release's table. IFC2X3 and IFC4 do not declare it, so there, and in
//! a model with no single known release, it reads as
//! [`Quantity::Unsupported`], as any entity the crate cannot place does.
//!
//! # A quantity without a number still exists
//!
//! A value slot that is `$`, missing from a truncated record, or not a
//! number leaves the quantity without a value, not without an identity.
//! It is returned as [`Quantity::Unresolved`], in place, so a caller listing
//! a set's quantities still sees it, and the fault is also reported as
//! [`PropertyAnomaly::QuantityValueMissing`] or
//! [`PropertyAnomaly::QuantityValueNotNumeric`]. It is never read as 0.

use std::sync::Arc;

use ifc_model::{Entity, EntityId, Model, Value};
use ifc_schema::{for_version, SchemaVersion};

use crate::error::PropertyAnomaly;
use crate::nesting::Nesting;
use crate::quantity::set::{Quantity, QuantityKind};
use crate::unit::unit_type;

const DESCRIPTION: usize = 1;
const UNIT: usize = 2;
const VALUE: usize = 3;
const FORMULA: usize = 4;

/// Where a simple quantity keeps its value and its `Formula`.
#[derive(Debug, Clone, Copy)]
pub(super) struct ValueSlots {
    value: usize,
    formula: usize,
}

/// The slots of `kind` in `model`, or `None` when the model's declared
/// release does not declare the entity.
///
/// The six kinds every release declares share fixed slots. `IfcQuantityNumber`
/// is looked up by name in the declared release's table, which must be one
/// release that has it.
pub(super) fn value_slots(model: &Model, kind: QuantityKind) -> Option<ValueSlots> {
    if kind != QuantityKind::Number {
        return Some(ValueSlots {
            value: VALUE,
            formula: FORMULA,
        });
    }
    let [token] = model.header().schema.as_slice() else {
        return None;
    };
    let schema = for_version(SchemaVersion::from_header_token(token)?).ok()?;
    let names = schema.attribute_names("IFCQUANTITYNUMBER");
    let slot = |name: &str| names.iter().position(|n| n.eq_ignore_ascii_case(name));
    Some(ValueSlots {
        value: slot("NumberValue")?,
        formula: slot("Formula")?,
    })
}

/// Why a simple quantity has no value to report.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum UnresolvedValue {
    /// The value attribute is `$`, or the record ends before it.
    Missing,
    /// The value attribute holds something other than a number, such as
    /// text or a reference.
    NotNumeric {
        /// The value found, rendered for the message.
        found: String,
    },
}

/// Read simple quantity `id` of `kind`, reporting schema-rule anomalies.
pub(super) fn read_simple(
    model: &Model,
    id: EntityId,
    entity: &Entity,
    kind: QuantityKind,
    slots: ValueSlots,
    name: Option<Arc<str>>,
    nesting: &mut Nesting<'_>,
) -> Quantity {
    let description = entity.attributes.get(DESCRIPTION).and_then(text);
    let formula = entity.attributes.get(slots.formula).and_then(text);
    let unit = entity.attributes.get(UNIT).and_then(one_ref);

    let value = match entity.attributes.get(slots.value) {
        None | Some(Value::Null) => Err(UnresolvedValue::Missing),
        Some(stated) => stated
            .unwrap_typed()
            .as_f64()
            .ok_or_else(|| UnresolvedValue::NotNumeric {
                found: format!("{stated:?}"),
            }),
    };
    match &value {
        Err(UnresolvedValue::Missing) => {
            nesting.report(PropertyAnomaly::QuantityValueMissing { quantity: id });
        }
        Err(UnresolvedValue::NotNumeric { found }) => {
            nesting.report(PropertyAnomaly::QuantityValueNotNumeric {
                quantity: id,
                found: found.clone(),
            });
        }
        // WR22 (WR21 for a count); IfcQuantityNumber has no such rule.
        Ok(value) if *value < 0.0 && kind.requires_non_negative() => {
            nesting.report(PropertyAnomaly::NegativeQuantity {
                quantity: id,
                value: *value,
            })
        }
        Ok(_) => {}
    }
    // WR21: a stated unit must match the quantity kind, whether or not the
    // value could be read.
    if let (Some(unit_id), Some(expected)) = (unit, kind.required_unit()) {
        if let Some(found) = unit_type(model, unit_id) {
            if &*found != expected {
                nesting.report(PropertyAnomaly::QuantityUnitMismatch {
                    quantity: id,
                    unit: unit_id,
                    expected,
                    found: found.to_string(),
                });
            }
        }
    }

    match value {
        Ok(value) => Quantity::Simple {
            id,
            name,
            description,
            kind,
            value,
            unit,
            formula,
        },
        Err(reason) => Quantity::Unresolved {
            id,
            name,
            description,
            kind,
            unit,
            formula,
            reason,
        },
    }
}

pub(super) fn text(value: &Value) -> Option<Arc<str>> {
    match value.unwrap_typed() {
        Value::Text(t) => Some(t.clone()),
        _ => None,
    }
}

fn one_ref(value: &Value) -> Option<EntityId> {
    match value.unwrap_typed() {
        Value::Ref(id) => Some(*id),
        _ => None,
    }
}
