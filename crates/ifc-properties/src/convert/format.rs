//! Lengths as text: decimal in any length unit, or feet and inches with
//! fractions.

use super::{convert_value, ConversionUnit, UnitConversionError};

/// How [`format_length`] writes a length.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum LengthFormat {
    /// The value in `unit`, with `decimal_places` digits after the point:
    /// `1.25` for 1250 mm in metres with two places. No unit symbol is
    /// written.
    Decimal {
        /// The unit to write the value in.
        unit: ConversionUnit,
        /// Digits after the decimal point.
        decimal_places: u8,
    },
    /// Feet and inches, the inches rounded to the nearest
    /// `1/denominator`: `5' - 6 1/2"`. With `suppress_zero_inches`, a whole
    /// number of feet is written `5'`.
    FeetInches {
        /// The fraction of an inch to round to: 2, 4, 8, 16, 32, 64 ...
        denominator: u32,
        /// Write `5'` rather than `5' - 0"`.
        suppress_zero_inches: bool,
    },
    /// Inches only, rounded to the nearest `1/denominator`: `66 1/2"`.
    Inches {
        /// The fraction of an inch to round to.
        denominator: u32,
    },
}

impl LengthFormat {
    /// `Decimal` with `decimal_places` digits in `unit`.
    #[must_use]
    pub fn decimal(unit: ConversionUnit, decimal_places: u8) -> Self {
        Self::Decimal {
            unit,
            decimal_places,
        }
    }

    /// `FeetInches` to the nearest `1/denominator` inch.
    #[must_use]
    pub fn feet_inches(denominator: u32, suppress_zero_inches: bool) -> Self {
        Self::FeetInches {
            denominator,
            suppress_zero_inches,
        }
    }

    /// `Inches` to the nearest `1/denominator` inch.
    #[must_use]
    pub fn inches(denominator: u32) -> Self {
        Self::Inches { denominator }
    }
}

/// Write `value`, a length in `unit`, as `format` asks.
///
/// Negative lengths keep their sign in front (`-1' - 6"`). A fraction is
/// reduced (`4/8` is written `1/2`), and inches that round up to twelve
/// carry into the feet. This is IfcOpenShell's `format_length` layout
/// (`util/unit.py`), with the value given in any length unit rather than
/// in feet or inches.
///
/// # Errors
///
/// [`UnitConversionError::DimensionMismatch`] when `unit`, or the decimal
/// format's unit, is not a length; [`UnitConversionError::NonFinite`] for a
/// value that is not finite; [`UnitConversionError::InvalidFormat`] for a
/// zero denominator or a value too large to round to it.
pub fn format_length(
    value: f64,
    unit: &ConversionUnit,
    format: &LengthFormat,
) -> Result<String, UnitConversionError> {
    let length = [1, 0, 0, 0, 0, 0, 0];
    if unit.dimensions != length {
        return Err(UnitConversionError::DimensionMismatch {
            from: unit.dimensions,
            to: length,
        });
    }
    match format {
        LengthFormat::Decimal {
            unit: target,
            decimal_places,
        } => {
            let converted = convert_value(value, unit, target)?;
            let places = usize::from(*decimal_places);
            let text = format!("{converted:.places$}");
            // `-0.00` reads as a negative length.
            Ok(
                if text.starts_with('-') && text[1..].chars().all(|c| c == '0' || c == '.') {
                    text[1..].to_owned()
                } else {
                    text
                },
            )
        }
        LengthFormat::FeetInches {
            denominator,
            suppress_zero_inches,
        } => {
            let (negative, units) = rounded_fractions(value, unit, *denominator)?;
            let per_foot = 12 * u64::from(*denominator);
            let feet = units / per_foot;
            let inches = units % per_foot;
            let sign = if negative && units != 0 { "-" } else { "" };
            if inches == 0 && *suppress_zero_inches {
                return Ok(format!("{sign}{feet}'"));
            }
            Ok(format!(
                "{sign}{feet}' - {}\"",
                inches_text(inches, *denominator)
            ))
        }
        LengthFormat::Inches { denominator } => {
            let (negative, units) = rounded_fractions(value, unit, *denominator)?;
            let sign = if negative && units != 0 { "-" } else { "" };
            Ok(format!("{sign}{}\"", inches_text(units, *denominator)))
        }
    }
}

/// `|value|` in `1/denominator` inches, rounded half away from zero, and
/// whether `value` is negative.
fn rounded_fractions(
    value: f64,
    unit: &ConversionUnit,
    denominator: u32,
) -> Result<(bool, u64), UnitConversionError> {
    if denominator == 0 {
        return Err(UnitConversionError::InvalidFormat {
            reason: "the fraction denominator must be at least 1",
        });
    }
    let inch = ConversionUnit::named("inch")?;
    let inches = convert_value(value, unit, &inch)?;
    let units = (inches.abs() * f64::from(denominator)).round();
    // 2^53: every integer below it is exact in f64.
    if units >= 9_007_199_254_740_992.0 {
        return Err(UnitConversionError::InvalidFormat {
            reason: "the length is too large to round to the requested fraction",
        });
    }
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let units = units as u64;
    Ok((inches < 0.0, units))
}

/// `units / denominator` inches as `whole num/den`, reduced.
fn inches_text(units: u64, denominator: u32) -> String {
    let denominator = u64::from(denominator);
    let whole = units / denominator;
    let numerator = units % denominator;
    if numerator == 0 {
        return whole.to_string();
    }
    let divisor = gcd(numerator, denominator);
    format!("{whole} {}/{}", numerator / divisor, denominator / divisor)
}

fn gcd(mut a: u64, mut b: u64) -> u64 {
    while b != 0 {
        (a, b) = (b, a % b);
    }
    a
}

#[cfg(test)]
mod tests {
    use super::*;

    fn unit(spec: &str) -> ConversionUnit {
        ConversionUnit::parse(spec).unwrap()
    }

    #[test]
    fn feet_and_inches_round_to_the_fraction_and_carry() {
        let foot = unit("foot");
        let format = LengthFormat::feet_inches(8, true);
        assert_eq!(format_length(5.5, &foot, &format).unwrap(), "5' - 6\"");
        assert_eq!(format_length(5.0, &foot, &format).unwrap(), "5'");
        assert_eq!(
            format_length(5.0, &foot, &LengthFormat::feet_inches(8, false)).unwrap(),
            "5' - 0\""
        );
        // 1' 5.5" in millimetres.
        let mm = unit("MILLIMETRE");
        assert_eq!(format_length(444.5, &mm, &format).unwrap(), "1' - 5 1/2\"");
        // 11.99" rounds to a whole foot.
        let inch = unit("inch");
        assert_eq!(format_length(11.99, &inch, &format).unwrap(), "1'");
        assert_eq!(
            format_length(0.125, &inch, &format).unwrap(),
            "0' - 0 1/8\""
        );
        assert_eq!(format_length(-18.0, &inch, &format).unwrap(), "-1' - 6\"");
    }

    #[test]
    fn inches_and_decimals() {
        let foot = unit("foot");
        assert_eq!(
            format_length(5.5417, &foot, &LengthFormat::inches(2)).unwrap(),
            "66 1/2\""
        );
        let mm = unit("MILLIMETRE");
        assert_eq!(
            format_length(1250.0, &mm, &LengthFormat::decimal(unit("METRE"), 2)).unwrap(),
            "1.25"
        );
        assert_eq!(
            format_length(-0.001, &mm, &LengthFormat::decimal(unit("METRE"), 2)).unwrap(),
            "0.00"
        );
    }

    #[test]
    fn non_lengths_and_bad_formats_are_refused() {
        assert!(matches!(
            format_length(1.0, &unit("SQUARE_METRE"), &LengthFormat::inches(2)),
            Err(UnitConversionError::DimensionMismatch { .. })
        ));
        assert!(matches!(
            format_length(1.0, &unit("METRE"), &LengthFormat::inches(0)),
            Err(UnitConversionError::InvalidFormat { .. })
        ));
        assert!(matches!(
            format_length(1.0, &unit("METRE"), &LengthFormat::decimal(unit("GRAM"), 2)),
            Err(UnitConversionError::DimensionMismatch { .. })
        ));
    }
}
