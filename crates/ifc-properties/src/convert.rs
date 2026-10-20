//! Conversion between units (#469): exact factors, dimensional analysis and
//! length formatting.
//!
//! A [`ConversionUnit`] is a unit reduced to what conversion needs: its SI
//! dimensional exponents and its factor into SI base units, split as an
//! integer power of ten and a mantissa. It comes from a model
//! ([`crate::resolve_unit`], which follows `IfcSIUnit` prefixes,
//! `IfcConversionBasedUnit` factors and `IfcDerivedUnit` exponents through
//! the exact, release-bound resolver), or without one from an SI name and
//! prefix ([`ConversionUnit::si`]), a named conversion-based unit
//! ([`ConversionUnit::named`], IFC4 Table 673 with exact factors) or text
//! ([`ConversionUnit::parse`]).
//!
//! [`unit_conversion`] between two units checks that their dimensions are
//! equal and gives the [`UnitConversion`] that takes a value from one to the
//! other. Units that differ only by prefixes convert by an exact power of
//! ten: millimetres to metres divides by 1000, which `value * 0.001` does
//! not (0.001 is not a binary fraction).
//!
//! # Refused
//!
//! - Units of different dimensions ([`UnitConversionError::DimensionMismatch`]):
//!   an area does not convert to a length.
//! - Monetary units ([`UnitConversionError::MonetaryUnit`]): currencies have
//!   no fixed rate.
//! - Context-dependent units ([`UnitConversionError::ContextDependentUnit`]):
//!   they state no SI factor.
//! - Conversion-based units with an offset (`IfcConversionBasedUnitWithOffset`,
//!   degree Fahrenheit by name): where IFC4 applies the offset contradicts
//!   its own example, so nothing applies one until buildingSMART settles it
//!   (IFC4.x-development#1193, tracked in #110).
//!
//! The SI `DEGREE_CELSIUS` is the one offset unit converted: SI defines it
//! as kelvin less 273.15, and `IfcSIUnit` states no offset to dispute. A
//! conversion to or from it is affine ([`UnitConversion::offset`]) and is
//! for absolute temperatures, not temperature differences.

mod format;
mod named;

use std::fmt;
use std::sync::Arc;

use ifc_model::EntityId;

use crate::exact::ExactUnitError;
use crate::unit::prefix_exponent;
use crate::unit::si::{si_decimal_exponent, si_name_dimensions, si_to_base};

pub use format::{format_length, LengthFormat};
pub use named::{named_units, NamedUnit};

/// A unit reduced to its dimensions and its exact factor into SI base units.
///
/// `value_si = value * mantissa * 10^decimal_exponent + offset`.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct ConversionUnit {
    /// How the unit was named: `MILLI METRE`, `foot`, or the model entity's
    /// type and name.
    pub name: Arc<str>,
    /// Its `IfcUnitEnum` or `IfcDerivedUnitEnum` constant, when it has one.
    pub unit_type: Option<Arc<str>>,
    /// SI dimensional exponents `[L, M, T, I, Θ, N, J]`.
    pub dimensions: [i32; 7],
    /// See the type note.
    pub mantissa: f64,
    /// See the type note.
    pub decimal_exponent: i32,
    /// Added after scaling; nonzero only for the SI `DEGREE_CELSIUS`.
    pub offset: f64,
}

impl ConversionUnit {
    /// An `IfcSIUnit` by name and optional prefix, e.g. `(Some("MILLI"),
    /// "METRE")` or `(None, "SQUARE_METRE")`.
    ///
    /// Names and dimensions are IFC4 ADD2 TC1's `IfcDimensionsForSiUnit`
    /// (IFC4X3 ADD2 agrees); the unprefixed mass unit is the gram, as
    /// `IfcSIUnit` defines it.
    ///
    /// # Errors
    ///
    /// [`UnitConversionError::UnknownPrefix`] or
    /// [`UnitConversionError::UnknownUnit`].
    pub fn si(prefix: Option<&str>, name: &str) -> Result<Self, UnitConversionError> {
        let upper = name.trim().to_ascii_uppercase().replace("METER", "METRE");
        let dimensions = si_name_dimensions(ifc_schema::SchemaVersion::Ifc4, &upper)
            .ok_or_else(|| UnitConversionError::UnknownUnit { spec: name.into() })?;
        let exponent = match prefix {
            None => 0,
            Some(prefix) => {
                prefix_exponent(&prefix.trim().to_ascii_uppercase()).ok_or_else(|| {
                    UnitConversionError::UnknownPrefix {
                        prefix: prefix.into(),
                    }
                })?
            }
        };
        let (_, offset) = si_to_base(exponent, &upper);
        let name = match prefix {
            Some(prefix) => format!("{} {upper}", prefix.trim().to_ascii_uppercase()),
            None => upper.clone(),
        };
        Ok(Self {
            name: name.into(),
            unit_type: si_unit_type(&upper).map(Into::into),
            dimensions,
            mantissa: 1.0,
            decimal_exponent: si_decimal_exponent(exponent, &upper),
            offset,
        })
    }

    /// A named conversion-based unit: an IFC4 Table 673 designation
    /// (`inch`, `foot`, `square foot`, `gallon US`, `pound`, `degree` ...,
    /// case-insensitive) or `US survey foot`, with the exact factors of
    /// [`named_units`].
    ///
    /// # Errors
    ///
    /// [`UnitConversionError::OffsetUnit`] for degree Fahrenheit (#110),
    /// [`UnitConversionError::UnknownUnit`] for any other name.
    pub fn named(name: &str) -> Result<Self, UnitConversionError> {
        if let Some(unit) = named::named_unit(name) {
            return Ok(Self::from_named(unit));
        }
        if named::OFFSET_UNIT_NAMES.contains(&named::normalise(name).as_str()) {
            return Err(UnitConversionError::OffsetUnit {
                name: name.into(),
                unit: None,
            });
        }
        Err(UnitConversionError::UnknownUnit { spec: name.into() })
    }

    /// A unit from text: an SI unit, with its prefix written before it
    /// (`METRE`, `MILLIMETRE`, `MILLI METRE`, `MILLI_METRE`, `square
    /// millimetre`, `CUBIC_METRE`, `KILOGRAM`; `METER` spellings accepted),
    /// or a named conversion-based unit ([`Self::named`]). Case-insensitive.
    ///
    /// A prefix joined to a square or cubic name applies before the power,
    /// as in IFC: `MILLI SQUARE_METRE` and `SQUARE_MILLIMETRE` are both
    /// mm².
    ///
    /// # Errors
    ///
    /// As [`Self::si`] and [`Self::named`].
    pub fn parse(spec: &str) -> Result<Self, UnitConversionError> {
        if let Some((prefix, name)) = split_si(spec) {
            return Self::si(prefix.as_deref(), &name);
        }
        Self::named(spec).map_err(|error| match error {
            UnitConversionError::UnknownUnit { .. } => {
                UnitConversionError::UnknownUnit { spec: spec.into() }
            }
            other => other,
        })
    }

    pub(crate) fn from_named(unit: &NamedUnit) -> Self {
        Self {
            name: unit.name.into(),
            unit_type: Some(unit.unit_type.into()),
            dimensions: unit.dimensions,
            mantissa: unit.mantissa,
            decimal_exponent: unit.decimal_exponent,
            offset: 0.0,
        }
    }

    /// The factor into SI base units, `mantissa * 10^decimal_exponent`.
    #[must_use]
    pub fn scale(&self) -> f64 {
        shift(self.mantissa, self.decimal_exponent)
    }

    /// Whether this is a length unit (dimensions `[1, 0, 0, 0, 0, 0, 0]`).
    #[must_use]
    pub fn is_length(&self) -> bool {
        self.dimensions == [1, 0, 0, 0, 0, 0, 0]
    }
}

/// The `IfcUnitEnum` constant an SI name belongs to when that is unique.
fn si_unit_type(name: &str) -> Option<&'static str> {
    Some(match name {
        "METRE" => "LENGTHUNIT",
        "SQUARE_METRE" => "AREAUNIT",
        "CUBIC_METRE" => "VOLUMEUNIT",
        "GRAM" => "MASSUNIT",
        "SECOND" => "TIMEUNIT",
        "AMPERE" => "ELECTRICCURRENTUNIT",
        "KELVIN" | "DEGREE_CELSIUS" => "THERMODYNAMICTEMPERATUREUNIT",
        "MOLE" => "AMOUNTOFSUBSTANCEUNIT",
        "CANDELA" => "LUMINOUSINTENSITYUNIT",
        "LUMEN" => "LUMINOUSFLUXUNIT",
        "RADIAN" => "PLANEANGLEUNIT",
        "STERADIAN" => "SOLIDANGLEUNIT",
        "HERTZ" => "FREQUENCYUNIT",
        "BECQUEREL" => "RADIOACTIVITYUNIT",
        "NEWTON" => "FORCEUNIT",
        "PASCAL" => "PRESSUREUNIT",
        "JOULE" => "ENERGYUNIT",
        "WATT" => "POWERUNIT",
        "COULOMB" => "ELECTRICCHARGEUNIT",
        "VOLT" => "ELECTRICVOLTAGEUNIT",
        "FARAD" => "ELECTRICCAPACITANCEUNIT",
        "OHM" => "ELECTRICRESISTANCEUNIT",
        "SIEMENS" => "ELECTRICCONDUCTANCEUNIT",
        "WEBER" => "MAGNETICFLUXUNIT",
        "TESLA" => "MAGNETICFLUXDENSITYUNIT",
        "HENRY" => "INDUCTANCEUNIT",
        "LUX" => "ILLUMINANCEUNIT",
        "GRAY" => "ABSORBEDDOSEUNIT",
        "SIEVERT" => "DOSEEQUIVALENTUNIT",
        _ => return None,
    })
}

/// `IfcSIPrefix` constants. No constant is a prefix of another, so the
/// order does not matter.
const PREFIXES: [&str; 16] = [
    "EXA", "PETA", "TERA", "GIGA", "MEGA", "KILO", "HECTO", "DECA", "DECI", "CENTI", "MILLI",
    "MICRO", "NANO", "PICO", "FEMTO", "ATTO",
];

/// An SI unit as `(prefix, name)`, or `None` when the text is not one.
fn split_si(spec: &str) -> Option<(Option<String>, String)> {
    let normal: String = spec
        .trim()
        .to_ascii_uppercase()
        .replace("METER", "METRE")
        .split(|c: char| c.is_whitespace() || c == '_')
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join("_");
    let is_si = |name: &str| si_name_dimensions(ifc_schema::SchemaVersion::Ifc4, name).is_some();
    if is_si(&normal) {
        return Some((None, normal));
    }
    // `SQUARE_MILLIMETRE`, `CUBIC_CENTIMETRE`: the power first.
    for power in ["SQUARE_", "CUBIC_"] {
        if let Some(rest) = normal.strip_prefix(power) {
            if let Some((prefix, base)) = strip_prefix(rest) {
                let name = format!("{power}{base}");
                if is_si(&name) {
                    return Some((Some(prefix.to_owned()), name));
                }
            }
        }
    }
    let (prefix, rest) = strip_prefix(&normal)?;
    let rest = rest.trim_start_matches('_');
    is_si(rest).then(|| (Some(prefix.to_owned()), rest.to_owned()))
}

fn strip_prefix(text: &str) -> Option<(&'static str, &str)> {
    PREFIXES
        .iter()
        .find_map(|prefix| text.strip_prefix(prefix).map(|rest| (*prefix, rest)))
}

/// How a value in one unit becomes a value in another:
/// `to = from * numerator / denominator * 10^decimal_exponent + offset`.
///
/// The numerator and denominator are the two units' mantissas, kept apart
/// so that a conversion to or from a unit whose mantissa is 1 (every SI
/// unit) rounds once: millimetres to inches divides by 254 instead of
/// multiplying by the rounded `1/254`.
#[derive(Debug, Clone, Copy, PartialEq)]
#[non_exhaustive]
pub struct UnitConversion {
    /// The mantissa of the unit converted from.
    pub numerator: f64,
    /// The mantissa of the unit converted to.
    pub denominator: f64,
    /// The power of ten between the two units.
    pub decimal_exponent: i32,
    /// Added after scaling; nonzero only to or from the SI
    /// `DEGREE_CELSIUS`.
    pub offset: f64,
}

impl UnitConversion {
    /// The identity.
    pub const IDENTITY: Self = Self {
        numerator: 1.0,
        denominator: 1.0,
        decimal_exponent: 0,
        offset: 0.0,
    };

    /// Convert one value. A power of ten up to 10^22 is exact in `f64`, so
    /// it is applied as one correctly rounded multiplication or division.
    #[must_use]
    pub fn apply(&self, value: f64) -> f64 {
        let mut scaled = value;
        if self.numerator != self.denominator {
            if self.numerator != 1.0 {
                scaled *= self.numerator;
            }
            if self.denominator != 1.0 {
                scaled /= self.denominator;
            }
        }
        shift(scaled, self.decimal_exponent) + self.offset
    }

    /// The multiplicative factor, `numerator / denominator *
    /// 10^decimal_exponent`.
    #[must_use]
    pub fn factor(&self) -> f64 {
        self.apply(1.0) - self.offset
    }

    /// The conversion raised to an integer power: the factor between the
    /// squares (`2`) or cubes (`3`) of two units. `None` when the
    /// conversion has an offset, which has no power.
    #[must_use]
    pub fn powi(&self, exponent: i32) -> Option<Self> {
        (self.offset == 0.0).then(|| Self {
            numerator: self.numerator.powi(exponent),
            denominator: self.denominator.powi(exponent),
            decimal_exponent: self.decimal_exponent * exponent,
            offset: 0.0,
        })
    }

    /// The conversion back.
    #[must_use]
    pub fn inverse(&self) -> Self {
        Self {
            numerator: self.denominator,
            denominator: self.numerator,
            decimal_exponent: -self.decimal_exponent,
            offset: -shift(
                self.offset * self.denominator / self.numerator,
                -self.decimal_exponent,
            ),
        }
    }

    /// Whether the conversion changes nothing.
    #[must_use]
    pub fn is_identity(&self) -> bool {
        self.numerator == self.denominator && self.decimal_exponent == 0 && self.offset == 0.0
    }
}

/// `(mantissa, exponent)` with the mantissa scaled to the smallest integer
/// that still states the same `f64` (`0.3048, 0` becomes `3048, -4`), so
/// that a factor read from a file and the same factor written as an integer
/// convert into each other exactly. A mantissa whose decimal form does not
/// terminate within 15 digits is kept as it is.
pub(crate) fn normalise_decimal(mantissa: f64, exponent: i32) -> (f64, i32) {
    if !mantissa.is_finite() || mantissa == 0.0 || mantissa.fract() == 0.0 {
        return (mantissa, exponent);
    }
    for digits in 1..=15 {
        let integer = shift(mantissa, digits).round();
        if integer.abs() >= 9_007_199_254_740_992.0 {
            break;
        }
        if shift(integer, -digits) == mantissa {
            return (integer, exponent - digits);
        }
    }
    (mantissa, exponent)
}

/// `value * 10^exponent`, exactly rounded while `10^|exponent|` is exact.
pub(crate) fn shift(value: f64, exponent: i32) -> f64 {
    // 10^22 is the largest power of ten an f64 holds exactly.
    match exponent {
        0 => value,
        1..=22 => value * 10f64.powi(exponent),
        -22..=-1 => value / 10f64.powi(-exponent),
        _ => value * 10f64.powi(exponent),
    }
}

/// The conversion taking a value in `from` to a value in `to`.
///
/// # Errors
///
/// [`UnitConversionError::DimensionMismatch`] when the two units measure
/// different quantities.
pub fn unit_conversion(
    from: &ConversionUnit,
    to: &ConversionUnit,
) -> Result<UnitConversion, UnitConversionError> {
    if from.dimensions != to.dimensions {
        return Err(UnitConversionError::DimensionMismatch {
            from: from.dimensions,
            to: to.dimensions,
        });
    }
    let decimal_exponent = from.decimal_exponent - to.decimal_exponent;
    // value_si = v_from * s_from + o_from = v_to * s_to + o_to
    let offset = if from.offset == to.offset {
        0.0
    } else {
        (from.offset - to.offset) / to.scale()
    };
    Ok(UnitConversion {
        numerator: from.mantissa,
        denominator: to.mantissa,
        decimal_exponent,
        offset,
    })
}

/// Convert `value` from one unit to another.
///
/// # Errors
///
/// As [`unit_conversion`], and [`UnitConversionError::NonFinite`] for a
/// value that is not finite.
pub fn convert_value(
    value: f64,
    from: &ConversionUnit,
    to: &ConversionUnit,
) -> Result<f64, UnitConversionError> {
    if !value.is_finite() {
        return Err(UnitConversionError::NonFinite { value });
    }
    Ok(unit_conversion(from, to)?.apply(value))
}

/// Why a conversion was refused.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum UnitConversionError {
    /// The model's unit could not be resolved exactly (missing, malformed,
    /// cyclic, an `IfcConversionBasedUnitWithOffset` ...): see
    /// [`ExactUnitError`].
    Resolve(ExactUnitError),
    /// The two units measure different quantities.
    DimensionMismatch {
        /// Dimensions of the unit converted from.
        from: [i32; 7],
        /// Dimensions of the unit converted to.
        to: [i32; 7],
    },
    /// An `IfcMonetaryUnit`: currencies have no fixed conversion.
    MonetaryUnit {
        /// The unit entity.
        unit: EntityId,
    },
    /// An `IfcContextDependentUnit`: it states no factor into SI.
    ContextDependentUnit {
        /// The unit entity.
        unit: EntityId,
    },
    /// A conversion-based unit with an offset, refused until buildingSMART
    /// settles where `ConversionOffset` applies (IFC4.x-development#1193,
    /// tracked in #110).
    OffsetUnit {
        /// The unit as named.
        name: Arc<str>,
        /// The unit entity, for a unit read from a model.
        unit: Option<EntityId>,
    },
    /// Text that names no SI unit and no named conversion-based unit.
    UnknownUnit {
        /// The text.
        spec: Arc<str>,
    },
    /// An SI prefix that is not an `IfcSIPrefix` constant.
    UnknownPrefix {
        /// The prefix as given.
        prefix: Arc<str>,
    },
    /// A value that is not finite.
    NonFinite {
        /// The value.
        value: f64,
    },
    /// A formatting request that cannot be met, such as a zero fraction
    /// denominator.
    InvalidFormat {
        /// Why.
        reason: &'static str,
    },
}

impl From<ExactUnitError> for UnitConversionError {
    fn from(error: ExactUnitError) -> Self {
        match error {
            ExactUnitError::UnsupportedOffset { unit } => Self::OffsetUnit {
                name: "IfcConversionBasedUnitWithOffset".into(),
                unit: Some(unit),
            },
            other => Self::Resolve(other),
        }
    }
}

impl fmt::Display for UnitConversionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Resolve(error) => write!(f, "{error}"),
            Self::DimensionMismatch { from, to } => write!(
                f,
                "units of different dimensions do not convert: {from:?} to {to:?}"
            ),
            Self::MonetaryUnit { unit } => {
                write!(
                    f,
                    "#{} is a monetary unit, which has no fixed conversion",
                    unit.0
                )
            }
            Self::ContextDependentUnit { unit } => write!(
                f,
                "#{} is a context-dependent unit, which states no SI factor",
                unit.0
            ),
            Self::OffsetUnit { name, .. } => write!(
                f,
                "{name} carries an offset, which is not applied until buildingSMART settles \
                 where it applies (IFC4.x-development#1193, openbimrs/ifc#110)"
            ),
            Self::UnknownUnit { spec } => write!(f, "{spec:?} names no unit this crate converts"),
            Self::UnknownPrefix { prefix } => write!(f, "{prefix:?} is not an IfcSIPrefix"),
            Self::NonFinite { value } => write!(f, "{value} is not a finite value"),
            Self::InvalidFormat { reason } => write!(f, "invalid length format: {reason}"),
        }
    }
}

impl std::error::Error for UnitConversionError {}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(spec: &str) -> ConversionUnit {
        ConversionUnit::parse(spec).unwrap_or_else(|error| panic!("{spec}: {error}"))
    }

    #[test]
    fn prefixed_si_units_convert_by_an_exact_power_of_ten() {
        let mm = parse("MILLIMETRE");
        let m = parse("metre");
        let conversion = unit_conversion(&mm, &m).unwrap();
        assert_eq!(conversion.numerator, conversion.denominator);
        assert_eq!(conversion.decimal_exponent, -3);
        assert_eq!(conversion.apply(1234.0), 1.234);
        assert_eq!(unit_conversion(&m, &mm).unwrap().apply(1.234), 1234.0);
        // value * 0.001 is not always the correctly rounded quotient.
        assert_ne!(9.0 * 0.001, 0.009);
        assert_eq!(conversion.apply(9.0), 0.009);
    }

    #[test]
    fn spellings_of_an_si_unit_agree() {
        let expected = parse("MILLIMETRE");
        for spec in ["MILLI METRE", "milli_metre", "Millimeter", " MILLI  METRE "] {
            assert_eq!(
                parse(spec).decimal_exponent,
                expected.decimal_exponent,
                "{spec}"
            );
            assert_eq!(parse(spec).dimensions, expected.dimensions, "{spec}");
        }
        let mm2 = parse("SQUARE_MILLIMETRE");
        assert_eq!(mm2.dimensions, [2, 0, 0, 0, 0, 0, 0]);
        assert_eq!(mm2.decimal_exponent, -6);
        assert_eq!(
            parse("MILLI SQUARE_METRE"),
            ConversionUnit {
                name: "MILLI SQUARE_METRE".into(),
                ..mm2
            }
        );
        assert_eq!(parse("cubic centimetre").decimal_exponent, -6);
        assert_eq!(parse("KILOGRAM").decimal_exponent, 0);
        assert_eq!(parse("GRAM").decimal_exponent, -3);
        assert_eq!(parse("DECAMETRE").decimal_exponent, 1);
        assert_eq!(parse("DECIMETRE").decimal_exponent, -1);
    }

    #[test]
    fn imperial_units_convert_exactly() {
        let inch = parse("inch");
        let foot = parse("foot");
        let mm = parse("MILLIMETRE");
        assert_eq!(convert_value(1.0, &inch, &mm).unwrap(), 25.4);
        assert_eq!(convert_value(1.0, &foot, &mm).unwrap(), 304.8);
        assert_eq!(convert_value(12.0, &inch, &foot).unwrap(), 1.0);
        assert_eq!(
            convert_value(1.0, &parse("mile"), &parse("METRE")).unwrap(),
            1609.344
        );
        let sq = unit_conversion(&parse("square foot"), &parse("SQUARE_METRE")).unwrap();
        assert_eq!(sq.factor(), 0.09290304);
        // The square of the foot-to-metre conversion is the square foot's.
        let length = unit_conversion(&foot, &parse("METRE")).unwrap();
        assert_eq!(length.powi(2).unwrap().factor(), sq.factor());
    }

    #[test]
    fn units_of_different_dimensions_are_refused() {
        let error = unit_conversion(&parse("METRE"), &parse("SQUARE_METRE")).unwrap_err();
        assert_eq!(normalise_decimal(0.3048, 0), (3048.0, -4));
        assert_eq!(normalise_decimal(25.4, -3), (254.0, -4));
        assert_eq!(normalise_decimal(1.0 / 3.0, 0), (1.0 / 3.0, 0));
        assert_eq!(
            error,
            UnitConversionError::DimensionMismatch {
                from: [1, 0, 0, 0, 0, 0, 0],
                to: [2, 0, 0, 0, 0, 0, 0]
            }
        );
        assert!(unit_conversion(&parse("pound"), &parse("NEWTON")).is_err());
        assert!(unit_conversion(&parse("lbf"), &parse("KILONEWTON")).is_ok());
    }

    #[test]
    fn offset_units_are_refused_by_name_and_celsius_is_affine() {
        assert!(matches!(
            ConversionUnit::parse("degree Fahrenheit"),
            Err(UnitConversionError::OffsetUnit { .. })
        ));
        let celsius = parse("DEGREE_CELSIUS");
        let kelvin = parse("KELVIN");
        assert_eq!(convert_value(0.0, &celsius, &kelvin).unwrap(), 273.15);
        assert_eq!(convert_value(273.15, &kelvin, &celsius).unwrap(), 0.0);
        assert_eq!(unit_conversion(&celsius, &kelvin).unwrap().powi(2), None);
    }

    #[test]
    fn unknown_text_is_refused() {
        assert!(matches!(
            ConversionUnit::parse("furlong"),
            Err(UnitConversionError::UnknownUnit { .. })
        ));
        assert!(matches!(
            ConversionUnit::si(Some("MICROSCOPIC"), "METRE"),
            Err(UnitConversionError::UnknownPrefix { .. })
        ));
        assert!(matches!(
            convert_value(f64::NAN, &parse("METRE"), &parse("METRE")),
            Err(UnitConversionError::NonFinite { .. })
        ));
    }
}
