//! Property-set and quantity-set template definitions.

use super::{Applicability, LocalizedText, PropertyTemplate, QuantityTemplate, TemplateSource};

/// External PSD/QTO set template.
#[derive(Debug, Clone, PartialEq, Eq, bincode::Encode, bincode::Decode)]
pub struct SetTemplate {
    /// `Name` as declared by the publication, e.g. `Pset_WallCommon`.
    pub name: String,
    /// Publication `GlobalId`, absent when the source did not assign one.
    pub guid: Option<String>,
    /// `Definition` prose, absent when the publication omitted it.
    pub definition: Option<String>,
    /// Localized alternates for `name` in other publication languages.
    pub name_aliases: Vec<LocalizedText>,
    /// Localized alternates for `definition` in other publication languages.
    pub definition_aliases: Vec<LocalizedText>,
    /// Which source publication contributed this template, absent for
    /// synthesized or corrected templates with no direct source file.
    pub source: Option<TemplateSource>,
    /// Publication `ApplicableTypeValue` before normalization.
    pub raw_applicability: Option<String>,
    /// Parsed entity/predefined-type selectors derived from `raw_applicability`.
    pub applicability: Vec<Applicability>,
    /// Whether this is a property set or quantity set, and its typed payload.
    pub kind: SetTemplateKind,
}

impl SetTemplate {
    /// True when `kind` is [`SetTemplateKind::Property`].
    pub fn is_property_set(&self) -> bool {
        matches!(self.kind, SetTemplateKind::Property { .. })
    }

    /// True when `kind` is [`SetTemplateKind::Quantity`].
    pub fn is_quantity_set(&self) -> bool {
        matches!(self.kind, SetTemplateKind::Quantity { .. })
    }
}

/// Set-level PSD or QTO payload.
#[derive(Debug, Clone, PartialEq, Eq, bincode::Encode, bincode::Decode)]
#[non_exhaustive]
pub enum SetTemplateKind {
    /// An `IfcPropertySetTemplate`: `TemplateType` category plus its property templates.
    Property {
        /// Applicability mode declared by `TemplateType`.
        set_type: PropertySetType,
        /// Property templates in publication order.
        properties: Vec<PropertyTemplate>,
    },
    /// An `IfcElementQuantity` template: `TemplateType` category, measurement
    /// method, and quantity templates.
    Quantity {
        /// Applicability mode declared by `TemplateType`.
        set_type: QuantitySetType,
        /// Free-text `MethodOfMeasurement`, absent when the publication omitted it.
        method_of_measurement: Option<String>,
        /// Quantity templates in publication order.
        quantities: Vec<QuantityTemplate>,
    },
}

/// IFC quantity-set template applicability mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, bincode::Encode, bincode::Decode)]
#[non_exhaustive]
pub enum QuantitySetType {
    /// `QTO_TYPEDRIVENOVERRIDE`: values from the type may be overridden per occurrence.
    TypeDrivenOverride,
    /// `QTO_TYPEDRIVENONLY`: values come only from the type, never the occurrence.
    TypeDrivenOnly,
    /// `QTO_OCCURRENCEDRIVEN`: values are authored per occurrence, not from a type.
    OccurrenceDriven,
    /// The publication omitted its optional `templatetype` classification.
    Unspecified,
}

/// IFC property-set template applicability mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, bincode::Encode, bincode::Decode)]
#[non_exhaustive]
pub enum PropertySetType {
    /// `PSD_TYPEDRIVENOVERRIDE`: values from the type may be overridden per occurrence.
    TypeDrivenOverride,
    /// `PSD_TYPEDRIVENONLY`: values come only from the type, never the occurrence.
    TypeDrivenOnly,
    /// `PSD_OCCURRENCEDRIVEN`: values are authored per occurrence, not from a type.
    OccurrenceDriven,
    /// `PSD_PERFORMANCEDRIVEN`: values describe required/target performance,
    /// distinct from as-built occurrence values.
    PerformanceDriven,
    /// The publication omitted its optional `templatetype` classification.
    Unspecified,
}
