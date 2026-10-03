//! Lexical forms: scalars as their XSD datatypes spell them, and the header.

use super::unrepresentable;
use crate::error::XmlError;
use crate::typing::Leaf;
use ifc_model::{Header, Value};

/// Escape text for an XML attribute value or element content.
pub(super) fn escape(out: &mut String, text: &str) {
    crate::writer::escape_into(out, text);
}

/// Whether XML 1.0 can carry the character at all, even as a reference.
const fn xml_char(c: char) -> bool {
    matches!(c, '\t' | '\n' | '\r' | '\u{20}'..='\u{D7FF}' | '\u{E000}'..='\u{FFFD}' | '\u{10000}'..)
}

/// Refuse a string XML 1.0 cannot carry.
fn xml_text(text: &str) -> Result<(), XmlError> {
    match text.chars().find(|c| !xml_char(*c)) {
        Some(c) => Err(unrepresentable(format!(
            "the character U+{:04X}, which XML 1.0 does not allow, in {text:?}",
            u32::from(c)
        ))),
        None => Ok(()),
    }
}

/// The text of one simple value, as the XSD datatype of its leaf spells it.
///
/// The XSD maps every IFC string type to `xs:normalizedString`, which has no
/// tab or line break, with `STRING(n)`'s width as its `maxLength`; `REAL`
/// and `NUMBER` to `xs:double`, which reads back as a real; `BINARY` to
/// `xs:hexBinary`, whole bytes only; enumerations to their members in
/// lower case.
pub(super) fn scalar(leaf: &Leaf, value: &Value) -> Result<String, XmlError> {
    let text = match (leaf, value) {
        (Leaf::Integer, Value::Integer(integer)) => integer.to_string(),
        (Leaf::Real | Leaf::Number, Value::Real(real)) => {
            if !real.is_finite() {
                return Err(XmlError::InvalidScalar {
                    kind: "real".into(),
                    value: real.to_string(),
                });
            }
            // Shortest round-trip form, always with '.' and, for large or
            // small magnitudes, an exponent: both are `xs:double` lexicals.
            format!("{real:?}")
        }
        (Leaf::Real | Leaf::Number, Value::Integer(integer)) => {
            return Err(unrepresentable(format!(
                "the integer {integer} where {} is declared: as an xs:double it reads back as a real",
                leaf_name(leaf)
            )))
        }
        (Leaf::Text { width, .. }, Value::Text(text)) => {
            xml_text(text)?;
            if text.contains(['\t', '\n', '\r']) {
                return Err(unrepresentable(format!(
                    "the string {text:?}: the XSD types strings as xs:normalizedString, \
                     which has no tab or line break"
                )));
            }
            if let Some(width) = width {
                let length = text.chars().count();
                if length > *width {
                    return Err(unrepresentable(format!(
                        "a string of {length} characters where STRING({width}) is declared"
                    )));
                }
            }
            text.to_string()
        }
        (Leaf::Boolean | Leaf::Logical, Value::Bool(flag)) => flag.to_string(),
        (Leaf::Logical, Value::LogicalUnknown) => "unknown".into(),
        (Leaf::Binary, Value::Binary(binary)) => hex(binary)?,
        (Leaf::Enumeration { members, .. }, Value::Enum(member)) => members
            .iter()
            .find(|declared| declared.eq_ignore_ascii_case(member))
            .map(|declared| declared.to_ascii_lowercase())
            .ok_or_else(|| XmlError::InvalidScalar {
                kind: leaf_name(leaf),
                value: member.to_string(),
            })?,
        _ => {
            return Err(XmlError::TypeMismatch {
                declared: leaf_name(leaf),
                found: format!("{value:?}"),
            })
        }
    };
    Ok(text)
}

fn leaf_name(leaf: &Leaf) -> String {
    match leaf {
        Leaf::Integer => "INTEGER".into(),
        Leaf::Real => "REAL".into(),
        Leaf::Number => "NUMBER".into(),
        Leaf::Text { .. } => "STRING".into(),
        Leaf::Boolean => "BOOLEAN".into(),
        Leaf::Logical => "LOGICAL".into(),
        Leaf::Binary => "BINARY".into(),
        Leaf::Enumeration { name, .. } | Leaf::Entity(name) | Leaf::Select(name) => {
            name.to_string()
        }
    }
}

/// A STEP binary body (`"0ABC"`: unused leading bits, then hex digits) as
/// `xs:hexBinary`, which holds whole bytes only.
fn hex(binary: &str) -> Result<String, XmlError> {
    let (unused, digits) = binary.split_at(binary.len().min(1));
    if unused != "0" {
        return Err(unrepresentable(format!(
            "the binary \"{binary}\", whose length is not a whole number of bytes"
        )));
    }
    if digits.len() % 2 != 0 || !digits.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(unrepresentable(format!(
            "the binary \"{binary}\", which is not a whole number of bytes in hex"
        )));
    }
    Ok(digits.to_ascii_uppercase())
}

/// A whitespace-separated list token: never empty, never with whitespace,
/// or the list would split differently when read.
pub(super) fn token(text: String) -> Result<String, XmlError> {
    if text.is_empty() || text.contains([' ', '\t', '\n', '\r']) {
        return Err(unrepresentable(format!(
            "the string {text:?} in a whitespace-separated list"
        )));
    }
    Ok(text)
}

/// The `<header>` the XSD declares, or nothing when every field is empty.
pub(super) fn header(out: &mut String, header: &Header) -> Result<(), XmlError> {
    let fields = header_fields(header)?;
    if fields.is_empty() {
        return Ok(());
    }
    out.push_str("  <header>\n");
    for (name, text) in fields {
        xml_text(text).map_err(|error| error.at(format!("/ifcXML/header/{name}")))?;
        out.push_str("    <");
        out.push_str(name);
        out.push('>');
        escape(out, text);
        out.push_str("</");
        out.push_str(name);
        out.push_str(">\n");
    }
    out.push_str("  </header>\n");
    Ok(())
}

/// The header fields the XSD declares, in its order: each at most once,
/// the time stamp an `xs:dateTime`, the STEP description as
/// `documentation`. Empty strings in the single-valued fields are left out,
/// as the reader leaves them empty; a list field keeps its one empty entry.
fn header_fields(header: &Header) -> Result<Vec<(&'static str, &str)>, XmlError> {
    fn at(name: &'static str) -> impl Fn(XmlError) -> XmlError {
        move |error| error.at(format!("/ifcXML/header/{name}"))
    }
    fn nonempty(text: &str) -> Option<&str> {
        (!text.is_empty()).then_some(text)
    }
    let time_stamp = nonempty(&header.time_stamp);
    if let Some(time_stamp) = time_stamp {
        if !date_time(time_stamp) {
            return Err(XmlError::InvalidScalar {
                kind: "xs:dateTime".into(),
                value: time_stamp.into(),
            }
            .at("/ifcXML/header/time_stamp".into()));
        }
    }
    let fields = [
        ("name", nonempty(&header.name)),
        ("time_stamp", time_stamp),
        (
            "author",
            single("author", &header.author).map_err(at("author"))?,
        ),
        (
            "organization",
            single("organization", &header.organization).map_err(at("organization"))?,
        ),
        (
            "preprocessor_version",
            nonempty(&header.preprocessor_version),
        ),
        ("originating_system", nonempty(&header.originating_system)),
        ("authorization", nonempty(&header.authorization)),
        (
            "documentation",
            single("description", &header.description).map_err(at("documentation"))?,
        ),
    ];
    Ok(fields
        .into_iter()
        .filter_map(|(name, value)| value.map(|value| (name, value)))
        .collect())
}

/// A list header field: the XSD holds at most one entry.
fn single<'h>(field: &str, values: &'h [String]) -> Result<Option<&'h str>, XmlError> {
    match values {
        [] => Ok(None),
        [value] => Ok(Some(value)),
        _ => Err(unrepresentable(format!(
            "{} header `{field}` entries; the XSD header holds one",
            values.len()
        ))),
    }
}

/// Whether `text` is an `xs:dateTime` lexical:
/// `-?YYYY-MM-DDThh:mm:ss(.s+)?(Z|(+|-)hh:mm)?`, with a real calendar date.
fn date_time(text: &str) -> bool {
    let text = text.strip_prefix('-').unwrap_or(text);
    let Some((date, time)) = text.split_once('T') else {
        return false;
    };
    let mut date = date.splitn(3, '-');
    let (Some(year), Some(month), Some(day)) = (date.next(), date.next(), date.next()) else {
        return false;
    };
    let digits = |part: &str, width: usize| {
        part.len() == width && part.bytes().all(|byte| byte.is_ascii_digit())
    };
    if year.len() < 4
        || !year.bytes().all(|byte| byte.is_ascii_digit())
        || (year.len() > 4 && year.starts_with('0'))
        || !digits(month, 2)
        || !digits(day, 2)
    {
        return false;
    }
    let (Ok(year), Ok(month), Ok(day)) = (
        year.parse::<u64>(),
        month.parse::<u32>(),
        day.parse::<u32>(),
    ) else {
        return false;
    };
    let leap = year % 4 == 0 && (year % 100 != 0 || year % 400 == 0);
    let days = match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if leap => 29,
        2 => 28,
        _ => return false,
    };
    if year == 0 || day == 0 || day > days {
        return false;
    }
    // The zone: `Z` or `+hh:mm` / `-hh:mm` at the end.
    let (clock, zone) = if let Some(clock) = time.strip_suffix('Z') {
        (clock, None)
    } else if time.len() > 6 && matches!(&time.as_bytes()[time.len() - 6], b'+' | b'-') {
        (&time[..time.len() - 6], Some(&time[time.len() - 5..]))
    } else {
        (time, None)
    };
    if let Some(zone) = zone {
        let Some((hours, minutes)) = zone.split_once(':') else {
            return false;
        };
        if !digits(hours, 2) || !digits(minutes, 2) {
            return false;
        }
        let (hours, minutes) = (
            hours.parse::<u32>().unwrap_or(99),
            minutes.parse::<u32>().unwrap_or(99),
        );
        if minutes > 59 || hours > 14 || (hours == 14 && minutes != 0) {
            return false;
        }
    }
    let (whole, fraction) = clock.split_once('.').unwrap_or((clock, ""));
    if clock.contains('.')
        && (fraction.is_empty() || !fraction.bytes().all(|byte| byte.is_ascii_digit()))
    {
        return false;
    }
    let mut parts = whole.split(':');
    let (Some(hours), Some(minutes), Some(seconds), None) =
        (parts.next(), parts.next(), parts.next(), parts.next())
    else {
        return false;
    };
    if !digits(hours, 2) || !digits(minutes, 2) || !digits(seconds, 2) {
        return false;
    }
    let (hours, minutes, seconds) = (
        hours.parse::<u32>().unwrap_or(99),
        minutes.parse::<u32>().unwrap_or(99),
        seconds.parse::<u32>().unwrap_or(99),
    );
    let midnight =
        hours == 24 && minutes == 0 && seconds == 0 && fraction.bytes().all(|b| b == b'0');
    (hours < 24 || midnight) && minutes < 60 && seconds < 60
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn date_times_follow_the_xsd_lexical_space() {
        for valid in [
            "2026-08-18T12:00:00",
            "2024-08-30T12:34:53+05:00",
            "1970-01-01T00:00:00Z",
            "2024-02-29T23:59:59.125",
            "2024-01-01T24:00:00",
            "-0044-03-15T12:00:00",
        ] {
            assert!(date_time(valid), "{valid}");
        }
        for invalid in [
            "",
            "2026-08-18",
            "2026-08-18T12:00",
            "2023-02-29T00:00:00",
            "2026-13-01T00:00:00",
            "2026-08-18 12:00:00",
            "2026-08-18T12:00:00.",
            "2026-08-18T12:60:00",
            "2026-08-18T12:00:00+15:00",
            "0000-01-01T00:00:00",
            "2024-01-01T24:00:01",
        ] {
            assert!(!date_time(invalid), "{invalid}");
        }
    }

    #[test]
    fn binaries_are_whole_bytes_in_hex() {
        assert_eq!(hex("0").unwrap(), "");
        assert_eq!(hex("0abCD").unwrap(), "ABCD");
        assert!(hex("1ABC").is_err());
        assert!(hex("0ABC").is_err());
        assert!(hex("").is_err());
    }
}
