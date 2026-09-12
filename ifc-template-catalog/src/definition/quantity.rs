//! Quantity-template definitions.

use super::LocalizedText;

/// External physical quantity template.
#[derive(Debug, Clone, PartialEq, Eq, bincode::Encode, bincode::Decode)]
pub struct QuantityTemplate {
    /// `Name` as declared by the publication.
    pub name: String,
    /// `Definition` prose, absent when the publication omitted it.
    pub definition: Option<String>,
    /// Localized alternates for `name` in other publication languages.
    pub name_aliases: Vec<LocalizedText>,
    /// Localized alternates for `definition` in other publication languages.
    pub definition_aliases: Vec<LocalizedText>,
    /// Which `IfcPhysicalQuantity` subtype this template describes.
    pub kind: QuantityKind,
}

/// QTO quantity value family, corresponding to an `IfcPhysicalSimpleQuantity` subtype.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, bincode::Encode, bincode::Decode)]
#[non_exhaustive]
pub enum QuantityKind {
    /// `IfcQuantityLength`.
    Length,
    /// `IfcQuantityArea`.
    Area,
    /// `IfcQuantityVolume`.
    Volume,
    /// `IfcQuantityWeight`.
    Weight,
    /// `IfcQuantityTime`.
    Time,
    /// `IfcQuantityCount`.
    Count,
    /// `IfcQuantityNumber`.
    Number,
}
