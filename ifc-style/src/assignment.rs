//! Style assignment, item binding, and deterministic cascade resolution.
//!
//! IFC2x3 `IfcPresentationStyleAssignment` wrappers are projected explicitly
//! and flattened only by the resolver. IFC4/IFC4X3 direct style selections keep
//! their original IDs.
//!
//! A unique direct item assignment wins over layer styles. Multiple direct
//! assignments are an ambiguity error; layer candidates remain inspectable.

use ifc_model::{EntityId, Value};

use crate::error::{StyleError, StyleResult};
use crate::view::Record;

mod layer;
mod resolution;
mod styled_item;

pub use layer::PresentationStyleAssignment;
pub use resolution::{ResolvedStyle, StyleSource};
pub use styled_item::StyledItem;

/// One member of IFC2x3/IFC4's legacy presentation-style select.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PresentationStyleMember {
    Style(EntityId),
    Null,
}

pub(crate) fn presentation_style_members(
    record: Record<'_, '_>,
    attribute: &'static str,
    minimum: usize,
) -> StyleResult<Vec<PresentationStyleMember>> {
    let value = record.value(attribute)?;
    let Value::List(items) = value.unwrap_typed() else {
        return Err(record.invalid(attribute, value));
    };
    if items.len() < minimum {
        return Err(StyleError::InvalidValue {
            entity: record.entity.type_name.to_string(),
            id: record.id,
            attribute,
            value: format!("{} member(s), expected at least {minimum}", items.len()),
        });
    }

    items
        .iter()
        .map(|item| match item {
            Value::Ref(target) => {
                record.check_reference(*target, "IfcPresentationStyle")?;
                Ok(PresentationStyleMember::Style(*target))
            }
            Value::Typed { type_name, value }
                if type_name.eq_ignore_ascii_case("IFCNULLSTYLE")
                    && matches!(value.as_ref(), Value::Enum(member) if member.eq_ignore_ascii_case("NULL")) =>
            {
                Ok(PresentationStyleMember::Null)
            }
            _ => Err(record.invalid(attribute, item)),
        })
        .collect()
}
