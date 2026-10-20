//! The named conversion-based units: the standard designations IFC4 lists
//! for `IfcConversionBasedUnit.Name`, with exact factors.
//!
//! IFC4 ADD2 TC1 `IfcConversionBasedUnit`, Table 673 "Standard unit names",
//! names the units and describes each with a rounded factor ("'yard' |
//! Length measure equal to 914 mm", "'mile' | ... 1609 m", "'pound' | ...
//! 0.454 kg", "'square mile' | ... 2 588 881 square meters"). The names are
//! the standard's; the factors here are the exact legal definitions instead,
//! because a converter that used the table's rounding would move a mile by
//! 344 mm:
//!
//! - inch, foot, yard, mile and the pound, and every unit built from them:
//!   the international yard and pound (1959), yard = 0.9144 m and pound =
//!   0.453 592 37 kg exactly; the inch is 25.4 mm and the foot 304.8 mm,
//!   which the IFC table states exactly too.
//! - gallon UK = 4.546 09 L (UK Weights and Measures Act 1985); gallon US =
//!   231 cubic inches; pints and fluid ounces are 1/8 and 1/160 (UK) or
//!   1/8 and 1/128 (US) of their gallon.
//! - lbf = pound × standard gravity 9.806 65 m/s² (CGPM 1901); kip = 1000
//!   lbf; psi = lbf per square inch.
//! - btu: the table's 1055.056 J (the ISO 31-4 "International Table" value
//!   rounded as the table rounds it), since several BTUs exist and the
//!   table names this one.
//! - degree = π/180 rad; minute, hour and day are 60, 3600 and 86 400 s.
//!
//! A factor whose decimal expansion terminates is kept as an integer
//! mantissa and a power of ten, so that `foot` to `MILLI METRE` is
//! `3048 / 10` and not `0.3048 * 1000` (#469). The degree, the psi, the ksi
//! and the US survey foot do not terminate and are the nearest `f64`.
//!
//! The US survey foot (1200/3937 m, still found in older US state plane
//! grids) is not in the IFC table; it is listed under the name `US survey
//! foot` so that a file using it can be read, and is never chosen for a
//! name the table defines.
//!
//! Temperature units with an offset (degree Fahrenheit, and degree Celsius
//! as an `IfcConversionBasedUnitWithOffset`) are refused by name: where IFC4
//! applies `ConversionOffset` is not settled (#110).

/// One named conversion-based unit.
#[derive(Debug, Clone, Copy, PartialEq)]
#[non_exhaustive]
pub struct NamedUnit {
    /// The designation, as IFC4 Table 673 spells it (case-insensitive in a
    /// file).
    pub name: &'static str,
    /// The `IfcUnitEnum` constant of the unit.
    pub unit_type: &'static str,
    /// SI dimensional exponents `[L, M, T, I, Θ, N, J]`.
    pub dimensions: [i32; 7],
    /// The factor into SI base units is `mantissa * 10^decimal_exponent`.
    pub mantissa: f64,
    /// See [`Self::mantissa`].
    pub decimal_exponent: i32,
}

impl NamedUnit {
    /// The factor into SI base units (metre, kilogram, second, radian ...).
    #[must_use]
    pub fn scale(&self) -> f64 {
        super::shift(self.mantissa, self.decimal_exponent)
    }
}

const L: [i32; 7] = [1, 0, 0, 0, 0, 0, 0];
const A: [i32; 7] = [2, 0, 0, 0, 0, 0, 0];
const V: [i32; 7] = [3, 0, 0, 0, 0, 0, 0];
const M: [i32; 7] = [0, 1, 0, 0, 0, 0, 0];
const T: [i32; 7] = [0, 0, 1, 0, 0, 0, 0];
const ANGLE: [i32; 7] = [0; 7];
const FORCE: [i32; 7] = [1, 1, -2, 0, 0, 0, 0];
const PRESSURE: [i32; 7] = [-1, 1, -2, 0, 0, 0, 0];
const ENERGY: [i32; 7] = [2, 1, -2, 0, 0, 0, 0];

const fn unit(
    name: &'static str,
    unit_type: &'static str,
    dimensions: [i32; 7],
    mantissa: f64,
    decimal_exponent: i32,
) -> NamedUnit {
    NamedUnit {
        name,
        unit_type,
        dimensions,
        mantissa,
        decimal_exponent,
    }
}

/// 1 lbf = 0.45359237 kg × 9.80665 m/s² = 4.4482216152605 N exactly.
const LBF_MANTISSA: f64 = 44_482_216_152_605.0;
/// 1 psi = 1 lbf / (0.0254 m)² = 4.4482216152605 / 0.00064516 Pa.
const PSI: f64 = 4.448_221_615_260_5 / 0.000_645_16;

/// IFC4 Table 673, in its order, with exact factors; then the US survey
/// foot.
pub(crate) const NAMED_UNITS: [NamedUnit; 33] = [
    unit("inch", "LENGTHUNIT", L, 254.0, -4),
    unit("foot", "LENGTHUNIT", L, 3048.0, -4),
    unit("yard", "LENGTHUNIT", L, 9144.0, -4),
    unit("mile", "LENGTHUNIT", L, 1_609_344.0, -3),
    unit("square inch", "AREAUNIT", A, 64_516.0, -8),
    unit("square foot", "AREAUNIT", A, 9_290_304.0, -8),
    unit("square yard", "AREAUNIT", A, 83_612_736.0, -8),
    unit("acre", "AREAUNIT", A, 40_468_564_224.0, -7),
    unit("square mile", "AREAUNIT", A, 2_589_988_110_336.0, -6),
    unit("cubic inch", "VOLUMEUNIT", V, 16_387_064.0, -12),
    unit("cubic foot", "VOLUMEUNIT", V, 28_316_846_592.0, -12),
    unit("cubic yard", "VOLUMEUNIT", V, 764_554_857_984.0, -12),
    unit("litre", "VOLUMEUNIT", V, 1.0, -3),
    unit("fluid ounce UK", "VOLUMEUNIT", V, 284_130_625.0, -13),
    unit("fluid ounce US", "VOLUMEUNIT", V, 295_735_295_625.0, -16),
    unit("pint UK", "VOLUMEUNIT", V, 56_826_125.0, -11),
    unit("pint US", "VOLUMEUNIT", V, 473_176_473.0, -12),
    unit("gallon UK", "VOLUMEUNIT", V, 454_609.0, -8),
    unit("gallon US", "VOLUMEUNIT", V, 3_785_411_784.0, -12),
    unit(
        "degree",
        "PLANEANGLEUNIT",
        ANGLE,
        std::f64::consts::PI / 180.0,
        0,
    ),
    unit("ounce", "MASSUNIT", M, 28_349_523_125.0, -12),
    unit("pound", "MASSUNIT", M, 45_359_237.0, -8),
    unit("ton UK", "MASSUNIT", M, 10_160_469_088.0, -7),
    unit("ton US", "MASSUNIT", M, 90_718_474.0, -5),
    unit("lbf", "FORCEUNIT", FORCE, LBF_MANTISSA, -13),
    unit("kip", "FORCEUNIT", FORCE, LBF_MANTISSA, -10),
    unit("psi", "PRESSUREUNIT", PRESSURE, PSI, 0),
    unit("ksi", "PRESSUREUNIT", PRESSURE, PSI, 3),
    unit("minute", "TIMEUNIT", T, 60.0, 0),
    unit("hour", "TIMEUNIT", T, 3600.0, 0),
    unit("day", "TIMEUNIT", T, 86_400.0, 0),
    unit("btu", "ENERGYUNIT", ENERGY, 1_055_056.0, -3),
    unit("US survey foot", "LENGTHUNIT", L, 1200.0 / 3937.0, 0),
];

/// Names of conversion-based temperature units that carry an offset,
/// refused by name until #110 settles where `ConversionOffset` applies. The
/// SI `DEGREE_CELSIUS` is not among them: its offset is the SI definition.
pub(crate) const OFFSET_UNIT_NAMES: [&str; 2] = ["fahrenheit", "degree fahrenheit"];

/// The named unit of this designation, ignoring ASCII case and treating
/// `_` as a space.
pub(crate) fn named_unit(name: &str) -> Option<&'static NamedUnit> {
    let wanted = normalise(name);
    NAMED_UNITS
        .iter()
        .find(|unit| normalise(unit.name) == wanted)
}

pub(crate) fn normalise(name: &str) -> String {
    name.trim()
        .chars()
        .map(|c| {
            if c == '_' {
                ' '
            } else {
                c.to_ascii_lowercase()
            }
        })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

/// Every named conversion-based unit this crate converts, in IFC4 Table 673
/// order followed by the US survey foot.
#[must_use]
pub fn named_units() -> &'static [NamedUnit] {
    &NAMED_UNITS
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn terminating_factors_are_the_legal_definitions() {
        let scale = |name| named_unit(name).expect(name).scale();
        assert_eq!(scale("inch"), 0.0254);
        assert_eq!(scale("foot"), 0.3048);
        assert_eq!(scale("yard"), 0.9144);
        assert_eq!(scale("mile"), 1609.344);
        assert_eq!(scale("pound"), 0.453_592_37);
        // 231 cubic inches.
        let gallon = 231.0 * 16_387_064.0;
        assert_eq!(gallon, 3_785_411_784.0);
        // The pint and fluid ounce are fractions of their gallon.
        assert_eq!(454_609.0 * 1e5 / 8.0, 5_682_612_500.0);
        assert_eq!(56_826_125.0 / 20.0 * 100.0, 284_130_625.0);
        assert_eq!(3_785_411_784.0 / 8.0, 473_176_473.0);
        assert!((3_785_411_784.0 / 128.0 * 1e4 - 295_735_295_625.0_f64).abs() < 1e-3);
        // lbf = pound × g0.
        assert_eq!(45_359_237.0 * 980_665.0, LBF_MANTISSA);
        // Areas and volumes are the powers of their length.
        assert_eq!(254.0 * 254.0, 64_516.0);
        assert_eq!(3048.0 * 3048.0, 9_290_304.0);
        assert_eq!(9144.0 * 9144.0, 83_612_736.0);
        assert_eq!(1_609_344.0 * 1_609_344.0, 2_589_988_110_336.0);
        assert_eq!(254.0 * 254.0 * 254.0, 16_387_064.0);
        assert_eq!(3048.0 * 3048.0 * 3048.0, 28_316_846_592.0);
        assert_eq!(9144.0 * 9144.0 * 9144.0, 764_554_857_984.0);
        // An acre is 4840 square yards.
        assert_eq!(4840.0 * 83_612_736.0, 404_685_642_240.0);
    }

    #[test]
    fn names_match_ignoring_case_and_underscores() {
        assert_eq!(named_unit("FOOT").unwrap().name, "foot");
        assert_eq!(named_unit("Square_Foot").unwrap().name, "square foot");
        assert_eq!(named_unit("  gallon   us ").unwrap().name, "gallon US");
        assert!(named_unit("furlong").is_none());
    }
}
