//! Property-template value forms.

use super::LocalizedText;

/// IFC value type and optional publication unit category.
#[derive(Debug, Clone, PartialEq, Eq, bincode::Encode, bincode::Decode)]
pub struct PropertyDataType {
    /// IFC value type; `None` preserves malformed official entries with an empty `DataType`.
    pub type_name: Option<String>,
    pub unit_type: Option<String>,
}

impl PropertyDataType {
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
    pub name: String,
    pub definition: Option<String>,
    pub name_aliases: Vec<LocalizedText>,
    pub definition_aliases: Vec<LocalizedText>,
}

/// External property template.
#[derive(Debug, Clone, PartialEq, Eq, bincode::Encode, bincode::Decode)]
pub struct PropertyTemplate {
    pub name: String,
    pub guid: Option<String>,
    pub definition: Option<String>,
    pub name_aliases: Vec<LocalizedText>,
    pub definition_aliases: Vec<LocalizedText>,
    pub kind: PropertyKind,
}

/// PSD property value form.
#[derive(Debug, Clone, PartialEq, Eq, bincode::Encode, bincode::Decode)]
#[non_exhaustive]
pub enum PropertyKind {
    SingleValue {
        data_type: PropertyDataType,
    },
    BoundedValue {
        data_type: PropertyDataType,
    },
    EnumeratedValue {
        enumeration_name: Option<String>,
        data_type: Option<PropertyDataType>,
        /// Lexical values from `EnumList`, in publication order.
        values: Vec<String>,
        /// Documented constants from `ConstantList`, kept distinct from lexical values.
        constants: Vec<EnumerationConstant>,
    },
    ListValue {
        data_type: PropertyDataType,
    },
    ReferenceValue {
        reference_type: String,
    },
    TableValue {
        defining_type: PropertyDataType,
        defined_type: PropertyDataType,
        expression: Option<String>,
    },
    Complex {
        usage_name: String,
        properties: Vec<PropertyTemplate>,
    },
}
