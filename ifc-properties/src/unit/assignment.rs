//! SI, conversion-based and derived units, and the project unit context.
//!
//! # Slots, verified against the IFC4 EXPRESS schema
//!
//! ```text
//! IfcUnitAssignment        0 = Units
//! IfcNamedUnit             0 = Dimensions   1 = UnitType
//! IfcSIUnit                2 = Prefix       3 = Name
//! IfcConversionBasedUnit   2 = Name         3 = ConversionFactor
//! IfcMeasureWithUnit       0 = ValueComponent  1 = UnitComponent
//! IfcDerivedUnit           0 = Elements     1 = UnitType  2 = UserDefinedType
//! IfcDerivedUnitElement    0 = Unit         1 = Exponent
//! IfcMonetaryUnit          0 = Currency
//! ```
//!
//! `IfcSIUnit` inherits `Dimensions`/`UnitType` from `IfcNamedUnit`, so its
//! own `Prefix` and `Name` start at slot 2. `IfcDerivedUnit` is NOT a
//! `IfcNamedUnit`, so its slots start at 0 -- the two cannot share a reader.
//!
//! # Prefixes are exact
//!
//! `MILLI` is 1e-3 exactly as a decimal, and `f64` cannot hold it exactly.
//! The factor is therefore returned as a power of ten and applied by the
//! caller, so `mm -> m` is one multiplication rather than a chain of
//! roundings.

use std::sync::Arc;

use ifc_model::{EntityId, Model, Value};

const NAMED_UNIT_TYPE: usize = 1;
const SI_PREFIX: usize = 2;
const SI_NAME: usize = 3;
const CONVERSION_NAME: usize = 2;
const CONVERSION_FACTOR: usize = 3;
const MEASURE_VALUE: usize = 0;
const MEASURE_UNIT: usize = 1;
const DERIVED_ELEMENTS: usize = 0;
const DERIVED_TYPE: usize = 1;
const DERIVED_ELEMENT_UNIT: usize = 0;
const DERIVED_ELEMENT_EXPONENT: usize = 1;
const MONETARY_CURRENCY: usize = 0;
const ASSIGNMENT_UNITS: usize = 0;

/// A unit as the file states it.
#[derive(Debug, Clone, PartialEq)]
pub enum UnitKind {
    /// `IfcSIUnit`: a base SI unit with an optional decimal prefix.
    Si {
        /// `IfcUnitEnum`, e.g. `LENGTHUNIT`.
        unit_type: Arc<str>,
        /// `IfcSIUnitName`, e.g. `METRE`.
        name: Arc<str>,
        /// `IfcSIPrefix`, e.g. `MILLI`. `None` means unprefixed.
        prefix: Option<Arc<str>>,
        /// Decimal exponent of the prefix: `MILLI` is -3, absent is 0.
        ///
        /// Exposed as an exponent rather than a factor so callers can apply
        /// it exactly instead of multiplying by a rounded 0.001.
        prefix_exponent: i32,
    },
    /// `IfcConversionBasedUnit`: a named unit defined by a factor.
    Conversion {
        /// `IfcUnitEnum`.
        unit_type: Arc<str>,
        /// The unit's name, e.g. `inch`.
        name: Option<Arc<str>>,
        /// The numeric conversion factor, when readable.
        factor: Option<f64>,
        /// The unit the factor is expressed in.
        factor_unit: Option<EntityId>,
    },
    /// `IfcDerivedUnit`: a product of powers of other units.
    Derived {
        /// `IfcDerivedUnitEnum`, e.g. `VOLUMETRICFLOWRATEUNIT`.
        unit_type: Arc<str>,
        /// The (unit, exponent) elements.
        elements: Vec<(EntityId, i64)>,
    },
    /// `IfcMonetaryUnit`: a currency, with no dimension.
    Monetary {
        /// ISO currency code as stated.
        currency: Option<Arc<str>>,
    },
    /// `IfcContextDependentUnit` or another named unit form.
    ContextDependent {
        /// `IfcUnitEnum`.
        unit_type: Arc<str>,
    },
}

impl UnitKind {
    /// The `IfcUnitEnum` this unit declares, when it has one.
    ///
    /// `IfcMonetaryUnit` has none: it is not an `IfcNamedUnit`.
    pub fn unit_type(&self) -> Option<&str> {
        match self {
            Self::Si { unit_type, .. }
            | Self::Conversion { unit_type, .. }
            | Self::ContextDependent { unit_type } => Some(unit_type),
            Self::Derived { unit_type, .. } => Some(unit_type),
            Self::Monetary { .. } => None,
        }
    }

    /// Multiplier converting a value in this unit to the unprefixed SI unit.
    ///
    /// Only defined for `Si`: a conversion-based unit needs its factor unit
    /// resolved too, and a derived unit needs its elements combined, so
    /// neither can answer honestly on its own.
    pub fn si_scale(&self) -> Option<f64> {
        match self {
            Self::Si {
                prefix_exponent, ..
            } => Some(10f64.powi(*prefix_exponent)),
            _ => None,
        }
    }
}

/// The decimal exponent of an `IfcSIPrefix`.
///
/// Returns `None` for an unrecognised constant rather than assuming 0: a
/// silent 1.0 would misreport every value using it.
pub fn prefix_exponent(prefix: &str) -> Option<i32> {
    Some(match prefix {
        "EXA" => 18,
        "PETA" => 15,
        "TERA" => 12,
        "GIGA" => 9,
        "MEGA" => 6,
        "KILO" => 3,
        "HECTO" => 2,
        "DECA" => 1,
        "DECI" => -1,
        "CENTI" => -2,
        "MILLI" => -3,
        "MICRO" => -6,
        "NANO" => -9,
        "PICO" => -12,
        "FEMTO" => -15,
        "ATTO" => -18,
        _ => return None,
    })
}

/// Read one unit by id.
pub fn unit(model: &Model, id: EntityId) -> Option<UnitKind> {
    let entity = model.get(id)?;
    let ty = entity.type_name.to_ascii_uppercase();
    match ty.as_str() {
        "IFCSIUNIT" => {
            let prefix = entity.attributes.get(SI_PREFIX).and_then(enum_text);
            Some(UnitKind::Si {
                unit_type: entity
                    .attributes
                    .get(NAMED_UNIT_TYPE)
                    .and_then(enum_text)
                    .unwrap_or_else(|| "".into()),
                name: entity
                    .attributes
                    .get(SI_NAME)
                    .and_then(enum_text)
                    .unwrap_or_else(|| "".into()),
                prefix_exponent: prefix.as_deref().and_then(prefix_exponent).unwrap_or(0),
                prefix,
            })
        }
        "IFCCONVERSIONBASEDUNIT" | "IFCCONVERSIONBASEDUNITWITHOFFSET" => {
            let factor_entity = entity.attributes.get(CONVERSION_FACTOR).and_then(one_ref);
            let (factor, factor_unit) = match factor_entity.and_then(|f| model.get(f)) {
                Some(measure) => (
                    measure
                        .attributes
                        .get(MEASURE_VALUE)
                        .and_then(|v| v.unwrap_typed().as_f64()),
                    measure.attributes.get(MEASURE_UNIT).and_then(one_ref),
                ),
                None => (None, None),
            };
            Some(UnitKind::Conversion {
                unit_type: entity
                    .attributes
                    .get(NAMED_UNIT_TYPE)
                    .and_then(enum_text)
                    .unwrap_or_else(|| "".into()),
                name: entity.attributes.get(CONVERSION_NAME).and_then(text),
                factor,
                factor_unit,
            })
        }
        "IFCDERIVEDUNIT" => {
            let elements = entity
                .attributes
                .get(DERIVED_ELEMENTS)
                .and_then(refs)
                .unwrap_or_default()
                .into_iter()
                .filter_map(|e| {
                    let element = model.get(e)?;
                    let unit = element
                        .attributes
                        .get(DERIVED_ELEMENT_UNIT)
                        .and_then(one_ref)?;
                    let exponent = match element
                        .attributes
                        .get(DERIVED_ELEMENT_EXPONENT)?
                        .unwrap_typed()
                    {
                        Value::Integer(v) => *v,
                        Value::Real(v) => *v as i64,
                        _ => return None,
                    };
                    Some((unit, exponent))
                })
                .collect();
            Some(UnitKind::Derived {
                unit_type: entity
                    .attributes
                    .get(DERIVED_TYPE)
                    .and_then(enum_text)
                    .unwrap_or_else(|| "".into()),
                elements,
            })
        }
        "IFCMONETARYUNIT" => Some(UnitKind::Monetary {
            currency: entity.attributes.get(MONETARY_CURRENCY).and_then(text),
        }),
        "IFCCONTEXTDEPENDENTUNIT" => Some(UnitKind::ContextDependent {
            unit_type: entity
                .attributes
                .get(NAMED_UNIT_TYPE)
                .and_then(enum_text)
                .unwrap_or_else(|| "".into()),
        }),
        _ => None,
    }
}

/// The `IfcUnitEnum` a unit declares, without building the whole value.
///
/// Used by the quantity reader to check `WR21` cheaply.
pub fn unit_type(model: &Model, id: EntityId) -> Option<Arc<str>> {
    let entity = model.get(id)?;
    let ty = entity.type_name.to_ascii_uppercase();
    let slot = if ty == "IFCDERIVEDUNIT" {
        DERIVED_TYPE
    } else {
        NAMED_UNIT_TYPE
    };
    entity.attributes.get(slot).and_then(enum_text)
}

/// Project default units, from `IfcProject.UnitsInContext`.
///
/// Returned in file order. These are the units a measure without its own
/// unit is expressed in, so a consumer needs them to interpret bare values.
pub fn project_units(model: &Model) -> Vec<(EntityId, UnitKind)> {
    let mut out = Vec::new();
    let mut ids: Vec<_> = model.ids_of_type("IFCUNITASSIGNMENT").to_vec();
    ids.sort_unstable();
    for id in ids {
        let Some(assignment) = model.get(id) else {
            continue;
        };
        for unit_id in assignment
            .attributes
            .get(ASSIGNMENT_UNITS)
            .and_then(refs)
            .unwrap_or_default()
        {
            if let Some(kind) = unit(model, unit_id) {
                out.push((unit_id, kind));
            }
        }
    }
    out
}

/// The project default unit for a given `IfcUnitEnum`.
///
/// `IfcCorrectUnitAssignment` makes at most one named unit per type, so the
/// first match is the only match in a well-formed file.
pub fn project_unit_for(model: &Model, unit_type_name: &str) -> Option<(EntityId, UnitKind)> {
    project_units(model)
        .into_iter()
        .find(|(_, kind)| kind.unit_type() == Some(unit_type_name))
}

fn text(value: &Value) -> Option<Arc<str>> {
    match value.unwrap_typed() {
        Value::Text(t) => Some(t.clone()),
        _ => None,
    }
}

fn enum_text(value: &Value) -> Option<Arc<str>> {
    match value.unwrap_typed() {
        Value::Enum(t) | Value::Text(t) => Some(t.clone()),
        _ => None,
    }
}

fn one_ref(value: &Value) -> Option<EntityId> {
    match value.unwrap_typed() {
        Value::Ref(id) => Some(*id),
        _ => None,
    }
}

fn refs(value: &Value) -> Option<Vec<EntityId>> {
    match value {
        Value::List(items) => Some(items.iter().filter_map(one_ref).collect()),
        _ => None,
    }
}
