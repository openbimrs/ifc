//! `IfcColourOrFactor` select projection.

use ifc_model::{EntityId, Value};

use crate::error::{StyleError, StyleResult};
use crate::view::Record;

/// One resolved member of the `IfcColourOrFactor` select.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ColourOrFactor {
    /// A reference to an `IfcColourSpecification` or `IfcPreDefinedColour`.
    Colour(EntityId),
    /// A bare normalized scalar in `[0, 1]`, used directly as a factor
    /// rather than looked up as a colour entity.
    Factor(f64),
}

/// Reads an optional `IfcColourOrFactor` attribute, distinguishing a colour
/// reference from a scalar factor. Returns `Ok(None)` when the attribute is
/// absent (this select is always optional in its IFC usages).
pub(crate) fn optional_colour_or_factor(
    record: &Record<'_, '_>,
    attribute: &'static str,
) -> StyleResult<Option<ColourOrFactor>> {
    let Some(value) = record.optional_raw(attribute)? else {
        return Ok(None);
    };
    match value.unwrap_typed() {
        Value::Ref(target) => {
            record.check_reference_select(
                *target,
                "IfcColour",
                &["IfcColourSpecification", "IfcPreDefinedColour"],
            )?;
            Ok(Some(ColourOrFactor::Colour(*target)))
        }
        Value::Real(factor) if factor.is_finite() && (0.0..=1.0).contains(factor) => {
            Ok(Some(ColourOrFactor::Factor(*factor)))
        }
        Value::Integer(factor) if (0..=1).contains(factor) => {
            Ok(Some(ColourOrFactor::Factor(*factor as f64)))
        }
        value => Err(StyleError::InvalidValue {
            entity: record.entity.type_name.to_string(),
            id: record.id,
            attribute,
            value: format!("{value:?}"),
        }),
    }
}
