//! Quantity-template definitions.

use super::LocalizedText;

/// External physical quantity template.
#[derive(Debug, Clone, PartialEq, Eq, bincode::Encode, bincode::Decode)]
pub struct QuantityTemplate {
    pub name: String,
    pub definition: Option<String>,
    pub name_aliases: Vec<LocalizedText>,
    pub definition_aliases: Vec<LocalizedText>,
    pub kind: QuantityKind,
}

/// QTO quantity value family.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, bincode::Encode, bincode::Decode)]
#[non_exhaustive]
pub enum QuantityKind {
    Length,
    Area,
    Volume,
    Weight,
    Time,
    Count,
    Number,
}
