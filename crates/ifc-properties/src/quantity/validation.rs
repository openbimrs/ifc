//! Comparing an authored quantity against an externally computed value.
//!
//! # This crate never measures anything
//!
//! `AGENTS.md` states the invariant: an IFC quantity is an authored
//! assertion, and applications compute shape elsewhere and pass the typed
//! result in. So this module takes a value the CALLER computed and reports
//! whether the file agrees. It does not open a geometry crate, and it cannot:
//! `ifc-properties` has no geometry dependency and the boundary gate enforces
//! that.
//!
//! # Comparison needs a tolerance, and the honest default is relative
//!
//! An absolute epsilon that suits a 0.2 m thickness is meaningless for a
//! 4000 m3 volume. Comparison is therefore relative to the larger magnitude,
//! with an absolute floor so values near zero do not demand infinite
//! precision.
//!
//! # Units are not converted silently
//!
//! If the authored quantity states millimetres and the caller computed
//! metres, the numbers differ by 1000 and no tolerance makes that agreement.
//! Rather than guess, [`Comparison::UnitMismatch`] reports it: a silent
//! conversion is how a 1000x error becomes a passing check.

use ifc_model::{EntityId, Model};

use crate::quantity::{Quantity, QuantityKind};
use crate::unit::{unit, UnitKind};

/// How an authored quantity compares to an externally computed value.
#[derive(Debug, Clone, PartialEq)]
pub enum Comparison {
    /// The file agrees with the computed value within tolerance.
    Agrees {
        /// The authored value.
        authored: f64,
        /// The value supplied by the caller.
        computed: f64,
        /// Relative difference actually observed.
        relative_difference: f64,
    },
    /// The file states a different value.
    Disagrees {
        /// The authored value.
        authored: f64,
        /// The value supplied by the caller.
        computed: f64,
        /// Relative difference actually observed.
        relative_difference: f64,
    },
    /// The quantity and the computed value are in different units.
    ///
    /// Reported rather than converted: see the module note.
    UnitMismatch {
        /// The unit the quantity states.
        authored_unit: String,
        /// The unit the caller says its value is in.
        computed_unit: String,
    },
    /// The quantity does not measure what the caller computed.
    ///
    /// Comparing an authored area to a computed volume is a caller error, and
    /// silently returning `Disagrees` would hide it.
    KindMismatch {
        /// What the quantity measures.
        authored: QuantityKind,
        /// What the caller says it computed.
        computed: QuantityKind,
    },
    /// The quantity carries no comparable scalar (complex or unsupported).
    NotComparable,
}

/// A value computed outside this crate, with its meaning attached.
///
/// Constructing one requires stating the kind and unit, so a bare `f64`
/// cannot cross the boundary -- the crate invariant that units stay explicit.
#[derive(Debug, Clone, PartialEq)]
pub struct ComputedQuantity {
    /// What was measured.
    pub kind: QuantityKind,
    /// The magnitude.
    pub value: f64,
    /// The `IfcUnitEnum`-style unit name the value is expressed in, e.g.
    /// `METRE` with no prefix. Compared textually against the authored unit.
    pub unit: String,
}

/// Tolerance for comparing two measurements.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Tolerance {
    /// Fractional difference allowed, e.g. `1e-6`.
    pub relative: f64,
    /// Absolute floor, so near-zero values stay comparable.
    pub absolute: f64,
}

impl Default for Tolerance {
    /// A tolerance suited to authored building data.
    ///
    /// 1e-6 relative is far tighter than any exporter's rounding, so a
    /// disagreement means a real difference rather than float noise.
    fn default() -> Self {
        Self {
            relative: 1e-6,
            absolute: 1e-9,
        }
    }
}

/// Compare one authored quantity against a computed value.
pub fn compare(
    model: &Model,
    quantity: &Quantity,
    computed: &ComputedQuantity,
    tolerance: Tolerance,
) -> Comparison {
    let Quantity::Simple {
        kind,
        value,
        unit: unit_id,
        ..
    } = quantity
    else {
        return Comparison::NotComparable;
    };

    if *kind != computed.kind {
        return Comparison::KindMismatch {
            authored: *kind,
            computed: computed.kind,
        };
    }

    if let Some(authored_unit) = unit_id.and_then(|id| unit_name(model, id)) {
        if !authored_unit.eq_ignore_ascii_case(&computed.unit) {
            return Comparison::UnitMismatch {
                authored_unit,
                computed_unit: computed.unit.clone(),
            };
        }
    }

    let difference = (value - computed.value).abs();
    let magnitude = value.abs().max(computed.value.abs());
    // Relative to the larger magnitude, with an absolute floor near zero.
    let relative_difference = if magnitude > 0.0 {
        difference / magnitude
    } else {
        0.0
    };
    let agrees = difference <= tolerance.absolute || relative_difference <= tolerance.relative;

    if agrees {
        Comparison::Agrees {
            authored: *value,
            computed: computed.value,
            relative_difference,
        }
    } else {
        Comparison::Disagrees {
            authored: *value,
            computed: computed.value,
            relative_difference,
        }
    }
}

/// A comparable name for a unit, including its prefix.
///
/// `MILLI` + `METRE` becomes `MILLIMETRE`, so a prefixed unit does not
/// compare equal to its base. That is the point: mm and m are different
/// units and a check that conflates them is worse than no check.
fn unit_name(model: &Model, id: EntityId) -> Option<String> {
    match unit(model, id)? {
        UnitKind::Si { name, prefix, .. } => Some(match prefix {
            Some(p) => format!("{p}{name}"),
            None => name.to_string(),
        }),
        UnitKind::Conversion { name, .. } => name.map(|n| n.to_string()),
        UnitKind::Monetary { currency } => currency.map(|c| c.to_string()),
        UnitKind::Derived { unit_type, .. } | UnitKind::ContextDependent { unit_type } => {
            Some(unit_type.to_string())
        }
    }
}
