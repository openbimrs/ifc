//! Property-template value forms.

use super::LocalizedText;

/// IFC value type and optional publication unit category.
#[derive(Debug, Clone, PartialEq, Eq, bincode::Encode, bincode::Decode)]
pub struct PropertyDataType {
    /// IFC value type; `None` preserves malformed official entries with an empty `DataType`.
    pub type_name: Option<String>,
    /// Publication unit category (e.g. `LENGTHUNIT`), absent when unspecified.
    pub unit_type: Option<String>,
}

impl PropertyDataType {
    /// Build a data type with a known `type_name` and no declared unit category.
    pub fn new(type_name: impl Into<String>) -> Self {
        Self {
            type_name: Some(type_name.into()),
            unit_type: None,
        }
    }
}

/// One documented value from a PSD `ConstantList`.
#[derive(Debug, Clone, PartialEq, Eq, bincode::Encode, bincode::Decode)]
pub struct EnumerationConstant {
    /// Constant's `Name` as declared by the `ConstantList` entry.
    pub name: String,
    /// Constant's `Definition` prose, absent when the publication omitted it.
    pub definition: Option<String>,
    /// Localized alternates for `name` in other publication languages.
    pub name_aliases: Vec<LocalizedText>,
    /// Localized alternates for `definition` in other publication languages.
    pub definition_aliases: Vec<LocalizedText>,
}

/// External property template.
#[derive(Debug, Clone, PartialEq, Eq, bincode::Encode, bincode::Decode)]
pub struct PropertyTemplate {
    /// `Name` as declared by the publication.
    pub name: String,
    /// Publication `GlobalId`, absent when the source did not assign one.
    pub guid: Option<String>,
    /// `Definition` prose, absent when the publication omitted it.
    pub definition: Option<String>,
    /// Localized alternates for `name` in other publication languages.
    pub name_aliases: Vec<LocalizedText>,
    /// Localized alternates for `definition` in other publication languages.
    pub definition_aliases: Vec<LocalizedText>,
    /// Value form, corresponding to an `IfcSimplePropertyTemplateTypeEnum` case.
    pub kind: PropertyKind,
}

/// PSD property value form, mirroring `IfcSimplePropertyTemplateTypeEnum`
/// plus the `IfcComplexPropertyTemplate` case.
#[derive(Debug, Clone, PartialEq, Eq, bincode::Encode, bincode::Decode)]
#[non_exhaustive]
pub enum PropertyKind {
    /// `P_SINGLEVALUE`: one scalar value of `data_type`.
    SingleValue {
        /// Value type and unit category for the scalar.
        data_type: PropertyDataType,
    },
    /// `P_BOUNDEDVALUE`: an upper/lower/set-point triple of `data_type`.
    BoundedValue {
        /// Value type and unit category shared by the bound values.
        data_type: PropertyDataType,
    },
    /// `P_ENUMERATEDVALUE`: a selection from a fixed list of lexical values
    /// or documented constants.
    EnumeratedValue {
        /// Name of the referenced `IfcPropertyEnumeration`, absent when inline.
        enumeration_name: Option<String>,
        /// Value type and unit category of the enumeration members, absent
        /// when the publication left it implicit.
        data_type: Option<PropertyDataType>,
        /// Lexical values from `EnumList`, in publication order.
        values: Vec<String>,
        /// Documented constants from `ConstantList`, kept distinct from lexical values.
        constants: Vec<EnumerationConstant>,
    },
    /// `P_LISTVALUE`: an ordered list of values of `data_type`.
    ListValue {
        /// Value type and unit category shared by every list element.
        data_type: PropertyDataType,
    },
    /// `P_REFERENCEVALUE`: a reference to another entity of `reference_type`.
    ReferenceValue {
        /// IFC entity type the reference must resolve to.
        reference_type: String,
    },
    /// `P_TABLEVALUE`: a lookup table pairing `defining_type` keys with
    /// `defined_type` values, optionally constrained by `expression`.
    TableValue {
        /// Value type of the table's defining (key) column.
        defining_type: PropertyDataType,
        /// Value type of the table's defined (value) column.
        defined_type: PropertyDataType,
        /// Optional formula constraining defined values from defining values.
        expression: Option<String>,
    },
    /// An `IfcComplexPropertyTemplate`: a named grouping of nested property templates.
    Complex {
        /// `UsageName` distinguishing this grouping's role.
        usage_name: String,
        /// Nested property templates that make up this grouping.
        properties: Vec<PropertyTemplate>,
    },
}
