//! Property-set and quantity-set template definitions.

use super::{Applicability, LocalizedText, PropertyTemplate, QuantityTemplate, TemplateSource};

/// External PSD/QTO set template.
#[derive(Debug, Clone, PartialEq, Eq, bincode::Encode, bincode::Decode)]
pub struct SetTemplate {
    pub name: String,
    pub guid: Option<String>,
    pub definition: Option<String>,
    pub name_aliases: Vec<LocalizedText>,
    pub definition_aliases: Vec<LocalizedText>,
    pub source: Option<TemplateSource>,
    /// Publication `ApplicableTypeValue` before normalization.
    pub raw_applicability: Option<String>,
    pub applicability: Vec<Applicability>,
    pub kind: SetTemplateKind,
}

impl SetTemplate {
    pub fn is_property_set(&self) -> bool {
        matches!(self.kind, SetTemplateKind::Property { .. })
    }

    pub fn is_quantity_set(&self) -> bool {
        matches!(self.kind, SetTemplateKind::Quantity { .. })
    }
}

/// Set-level PSD or QTO payload.
#[derive(Debug, Clone, PartialEq, Eq, bincode::Encode, bincode::Decode)]
#[non_exhaustive]
pub enum SetTemplateKind {
    Property {
        set_type: PropertySetType,
        properties: Vec<PropertyTemplate>,
    },
    Quantity {
        set_type: QuantitySetType,
        method_of_measurement: Option<String>,
        quantities: Vec<QuantityTemplate>,
    },
}

/// IFC quantity-set template applicability mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, bincode::Encode, bincode::Decode)]
#[non_exhaustive]
pub enum QuantitySetType {
    TypeDrivenOverride,
    TypeDrivenOnly,
    OccurrenceDriven,
    /// The publication omitted its optional `templatetype` classification.
    Unspecified,
}

/// IFC property-set template applicability mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, bincode::Encode, bincode::Decode)]
#[non_exhaustive]
pub enum PropertySetType {
    TypeDrivenOverride,
    TypeDrivenOnly,
    OccurrenceDriven,
    PerformanceDriven,
    Unspecified,
}
