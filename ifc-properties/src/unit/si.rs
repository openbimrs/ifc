//! SI dimensions and scales, transcribed from the normative EXPRESS functions.
//!
//! Dimensions are `[L, M, T, I, Θ, N, J]`, the order of
//! `IfcDimensionalExponents`. Two tables come from each release:
//!
//! - `IfcDimensionsForSiUnit`: the dimensions of an `IfcSIUnitName`.
//! - `IfcCorrectDimensions`: the dimensions an `IfcUnitEnum` requires.
//!
//! The releases disagree. IFC2X3 TC1 gives `FARAD` the current exponent 1
//! and requires `ELECTRICCAPACITANCEUNIT` to have mass exponent +1; IFC4
//! corrected both to `(-2, -1, 4, 2, 0, 0, 0)`. So an IFC2X3 farad fails its
//! own schema's check, and the resolver reports that instead of borrowing
//! IFC4's answer. The tests below re-parse both functions from
//! `references/ifc-spec` and assert these tables against them.

use ifc_schema::SchemaVersion;

/// SI dimensional exponents, `[L, M, T, I, Θ, N, J]`.
pub(crate) type Dimensions = [i32; 7];

const IFC4_FARAD: Dimensions = [-2, -1, 4, 2, 0, 0, 0];
const IFC2X3_FARAD: Dimensions = [-2, -1, 4, 1, 0, 0, 0];
const IFC4_CAPACITANCE: Dimensions = [-2, -1, 4, 2, 0, 0, 0];
const IFC2X3_CAPACITANCE: Dimensions = [-2, 1, 4, 1, 0, 0, 0];

/// The power a prefix is raised to: `SQUARE_METRE` 2, `CUBIC_METRE` 3.
///
/// Every other `IfcSIUnitName` is a first-power unit.
pub(crate) fn name_power(name: &str) -> i32 {
    match name.to_ascii_uppercase().as_str() {
        "SQUARE_METRE" => 2,
        "CUBIC_METRE" => 3,
        _ => 1,
    }
}

/// `IfcDimensionsForSiUnit` of the given release, or `None` for a name the
/// function does not list (its `OTHERWISE` branch is not a unit).
pub(crate) fn si_name_dimensions(version: SchemaVersion, name: &str) -> Option<Dimensions> {
    Some(match name.to_ascii_uppercase().as_str() {
        "METRE" => [1, 0, 0, 0, 0, 0, 0],
        "SQUARE_METRE" => [2, 0, 0, 0, 0, 0, 0],
        "CUBIC_METRE" => [3, 0, 0, 0, 0, 0, 0],
        "GRAM" => [0, 1, 0, 0, 0, 0, 0],
        "SECOND" => [0, 0, 1, 0, 0, 0, 0],
        "AMPERE" => [0, 0, 0, 1, 0, 0, 0],
        "KELVIN" | "DEGREE_CELSIUS" => [0, 0, 0, 0, 1, 0, 0],
        "MOLE" => [0, 0, 0, 0, 0, 1, 0],
        "CANDELA" | "LUMEN" => [0, 0, 0, 0, 0, 0, 1],
        "RADIAN" | "STERADIAN" => [0; 7],
        "HERTZ" | "BECQUEREL" => [0, 0, -1, 0, 0, 0, 0],
        "NEWTON" => [1, 1, -2, 0, 0, 0, 0],
        "PASCAL" => [-1, 1, -2, 0, 0, 0, 0],
        "JOULE" => [2, 1, -2, 0, 0, 0, 0],
        "WATT" => [2, 1, -3, 0, 0, 0, 0],
        "COULOMB" => [0, 0, 1, 1, 0, 0, 0],
        "VOLT" => [2, 1, -3, -1, 0, 0, 0],
        "FARAD" => match version {
            SchemaVersion::Ifc2x3 => IFC2X3_FARAD,
            SchemaVersion::Ifc4 => IFC4_FARAD,
            SchemaVersion::Ifc4x3 => return None,
        },
        "OHM" => [2, 1, -3, -2, 0, 0, 0],
        "SIEMENS" => [-2, -1, 3, 2, 0, 0, 0],
        "WEBER" => [2, 1, -2, -1, 0, 0, 0],
        "TESLA" => [0, 1, -2, -1, 0, 0, 0],
        "HENRY" => [2, 1, -2, -2, 0, 0, 0],
        "LUX" => [-2, 0, 0, 0, 0, 0, 1],
        "GRAY" | "SIEVERT" => [2, 0, -2, 0, 0, 0, 0],
        _ => return None,
    })
}

/// `IfcCorrectDimensions` of the given release: the dimensions a named unit
/// of this `IfcUnitEnum` must have.
///
/// `None` for `USERDEFINED` (the function answers `UNKNOWN`) and for any
/// constant it does not list.
pub(crate) fn unit_enum_dimensions(version: SchemaVersion, unit_type: &str) -> Option<Dimensions> {
    Some(match unit_type.to_ascii_uppercase().as_str() {
        "LENGTHUNIT" => [1, 0, 0, 0, 0, 0, 0],
        "MASSUNIT" => [0, 1, 0, 0, 0, 0, 0],
        "TIMEUNIT" => [0, 0, 1, 0, 0, 0, 0],
        "ELECTRICCURRENTUNIT" => [0, 0, 0, 1, 0, 0, 0],
        "THERMODYNAMICTEMPERATUREUNIT" => [0, 0, 0, 0, 1, 0, 0],
        "AMOUNTOFSUBSTANCEUNIT" => [0, 0, 0, 0, 0, 1, 0],
        "LUMINOUSINTENSITYUNIT" | "LUMINOUSFLUXUNIT" => [0, 0, 0, 0, 0, 0, 1],
        "PLANEANGLEUNIT" | "SOLIDANGLEUNIT" => [0; 7],
        "AREAUNIT" => [2, 0, 0, 0, 0, 0, 0],
        "VOLUMEUNIT" => [3, 0, 0, 0, 0, 0, 0],
        "ABSORBEDDOSEUNIT" | "DOSEEQUIVALENTUNIT" => [2, 0, -2, 0, 0, 0, 0],
        "RADIOACTIVITYUNIT" | "FREQUENCYUNIT" => [0, 0, -1, 0, 0, 0, 0],
        "ELECTRICCAPACITANCEUNIT" => match version {
            SchemaVersion::Ifc2x3 => IFC2X3_CAPACITANCE,
            SchemaVersion::Ifc4 => IFC4_CAPACITANCE,
            SchemaVersion::Ifc4x3 => return None,
        },
        "ELECTRICCHARGEUNIT" => [0, 0, 1, 1, 0, 0, 0],
        "ELECTRICCONDUCTANCEUNIT" => [-2, -1, 3, 2, 0, 0, 0],
        "ELECTRICVOLTAGEUNIT" => [2, 1, -3, -1, 0, 0, 0],
        "ELECTRICRESISTANCEUNIT" => [2, 1, -3, -2, 0, 0, 0],
        "ENERGYUNIT" => [2, 1, -2, 0, 0, 0, 0],
        "FORCEUNIT" => [1, 1, -2, 0, 0, 0, 0],
        "INDUCTANCEUNIT" => [2, 1, -2, -2, 0, 0, 0],
        "ILLUMINANCEUNIT" => [-2, 0, 0, 0, 0, 0, 1],
        "MAGNETICFLUXUNIT" => [2, 1, -2, -1, 0, 0, 0],
        "MAGNETICFLUXDENSITYUNIT" => [0, 1, -2, -1, 0, 0, 0],
        "POWERUNIT" => [2, 1, -3, 0, 0, 0, 0],
        "PRESSUREUNIT" => [-1, 1, -2, 0, 0, 0, 0],
        _ => return None,
    })
}

/// Scale and offset taking a value in this SI unit to the SI base unit.
///
/// `value_base = value * scale + offset`. Two names are not their own base:
/// the kilogram, not the gram, is the SI base unit of mass (the `IfcSIUnit`
/// definition says an unprefixed mass unit is the gram), and degrees Celsius
/// sit 273.15 above kelvin.
pub(crate) fn si_to_base(prefix_exponent: i32, name: &str) -> (f64, f64) {
    let upper = name.to_ascii_uppercase();
    let power = name_power(&upper);
    let base_exponent = if upper == "GRAM" { -3 } else { 0 };
    let scale = 10f64.powi(prefix_exponent * power + base_exponent);
    let offset = if upper == "DEGREE_CELSIUS" {
        273.15
    } else {
        0.0
    };
    (scale, offset)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    /// `references/ifc-spec` sits one level up standalone and three levels up
    /// inside the openbim superproject.
    fn express(rel: &str) -> Option<String> {
        let crate_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let Some(root) = ["../../../references/ifc-spec", "../references/ifc-spec"]
            .into_iter()
            .map(|path| crate_dir.join(path))
            .find(|path| path.is_dir())
        else {
            assert!(
                std::env::var_os("IFC_SPEC_REQUIRED").is_none(),
                "IFC_SPEC_REQUIRED is set but references/ifc-spec was not found; \
                 run scripts/fetch-ifc-schemas.sh"
            );
            return None;
        };
        let bytes = std::fs::read(root.join(rel)).ok()?;
        Some(bytes.iter().map(|&b| b as char).collect())
    }

    /// `(case label, exponents)` rows of one EXPRESS function's `CASE`.
    fn case_rows(source: &str, function: &str) -> Vec<(String, Dimensions)> {
        let start = source
            .find(&format!("FUNCTION {function}"))
            .expect("function declared");
        let body = &source[start..];
        let body = &body[..body.find("END_FUNCTION").expect("function ends")];
        // Skip the signature, which also names IfcDimensionalExponents.
        let body = &body[body.find("CASE").expect("a CASE statement")..];
        let mut rows = Vec::new();
        for arm in body.split(';') {
            let Some((label, rest)) = arm.split_once(':') else {
                continue;
            };
            let Some(open) = rest.find("IfcDimensionalExponents") else {
                continue;
            };
            let numbers = &rest[open..];
            let inner = &numbers[numbers.find('(').unwrap() + 1..numbers.find(')').unwrap()];
            let values: Vec<i32> = inner
                .split(',')
                .map(|n| n.trim().parse().expect("integer exponent"))
                .collect();
            let label = label.split_whitespace().last().unwrap().to_owned();
            rows.push((label, values.try_into().expect("seven exponents")));
        }
        rows
    }

    fn check(version: SchemaVersion, rel: &str) {
        let Some(source) = express(rel) else {
            return;
        };
        let si = case_rows(&source, "IfcDimensionsForSiUnit");
        assert_eq!(si.len(), 31, "{rel}: 30 names plus OTHERWISE");
        for (name, dimensions) in si {
            if name == "OTHERWISE" {
                assert_eq!(si_name_dimensions(version, &name), None);
                continue;
            }
            assert_eq!(
                si_name_dimensions(version, &name),
                Some(dimensions),
                "{rel}: {name}"
            );
        }
        let units = case_rows(&source, "IfcCorrectDimensions");
        assert_eq!(units.len(), 29, "{rel}: every IfcUnitEnum but USERDEFINED");
        for (unit_type, dimensions) in units {
            assert_eq!(
                unit_enum_dimensions(version, &unit_type),
                Some(dimensions),
                "{rel}: {unit_type}"
            );
        }
        assert_eq!(unit_enum_dimensions(version, "USERDEFINED"), None);
    }

    #[test]
    fn ifc4_tables_match_the_normative_functions() {
        check(SchemaVersion::Ifc4, "ifc4-add2-tc1/IFC4.exp");
    }

    #[test]
    fn ifc2x3_tables_match_the_normative_functions() {
        check(SchemaVersion::Ifc2x3, "ifc2x3-tc1/IFC2X3_TC1.exp");
    }

    #[test]
    fn prefixes_apply_before_the_power_and_mass_is_based_on_the_kilogram() {
        assert_eq!(si_to_base(-3, "SQUARE_METRE"), (1e-6, 0.0));
        assert_eq!(si_to_base(-3, "CUBIC_METRE").0, 1e-9);
        assert_eq!(si_to_base(0, "GRAM").0, 1e-3);
        assert_eq!(si_to_base(3, "GRAM").0, 1.0);
        assert_eq!(si_to_base(0, "DEGREE_CELSIUS"), (1.0, 273.15));
    }
}
