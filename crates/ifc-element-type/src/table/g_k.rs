//! Element types G-K. Generated; do not edit.

use super::{ElementType, Family};

/// `IfcGeographicElementType`, 5 permitted tokens.
pub const IFCGEOGRAPHICELEMENTTYPE: ElementType = ElementType {
    type_name: "IFCGEOGRAPHICELEMENTTYPE",
    arity: 10,
    predefined_slot: 9,
    predefined_optional: false,
    fallback_attr: "ElementType",
    fallback_slot: 8,
    family: Family::Element,
    members: &[
        "SOIL_BORING_POINT",
        "TERRAIN",
        "VEGETATION",
        "USERDEFINED",
        "NOTDEFINED",
    ],
};

/// `IfcHeatExchangerType`, 5 permitted tokens.
pub const IFCHEATEXCHANGERTYPE: ElementType = ElementType {
    type_name: "IFCHEATEXCHANGERTYPE",
    arity: 10,
    predefined_slot: 9,
    predefined_optional: false,
    fallback_attr: "ElementType",
    fallback_slot: 8,
    family: Family::Element,
    members: &[
        "PLATE",
        "SHELLANDTUBE",
        "TURNOUTHEATING",
        "USERDEFINED",
        "NOTDEFINED",
    ],
};

/// `IfcHumidifierType`, 15 permitted tokens.
pub const IFCHUMIDIFIERTYPE: ElementType = ElementType {
    type_name: "IFCHUMIDIFIERTYPE",
    arity: 10,
    predefined_slot: 9,
    predefined_optional: false,
    fallback_attr: "ElementType",
    fallback_slot: 8,
    family: Family::Element,
    members: &[
        "ADIABATICAIRWASHER",
        "ADIABATICATOMIZING",
        "ADIABATICCOMPRESSEDAIRNOZZLE",
        "ADIABATICPAN",
        "ADIABATICRIGIDMEDIA",
        "ADIABATICULTRASONIC",
        "ADIABATICWETTEDELEMENT",
        "ASSISTEDBUTANE",
        "ASSISTEDELECTRIC",
        "ASSISTEDNATURALGAS",
        "ASSISTEDPROPANE",
        "ASSISTEDSTEAM",
        "STEAMINJECTION",
        "USERDEFINED",
        "NOTDEFINED",
    ],
};

/// `IfcImpactProtectionDeviceType`, 6 permitted tokens.
pub const IFCIMPACTPROTECTIONDEVICETYPE: ElementType = ElementType {
    type_name: "IFCIMPACTPROTECTIONDEVICETYPE",
    arity: 10,
    predefined_slot: 9,
    predefined_optional: false,
    fallback_attr: "ElementType",
    fallback_slot: 8,
    family: Family::Element,
    members: &[
        "BUMPER",
        "CRASHCUSHION",
        "DAMPINGSYSTEM",
        "FENDER",
        "USERDEFINED",
        "NOTDEFINED",
    ],
};

/// `IfcInterceptorType`, 6 permitted tokens.
pub const IFCINTERCEPTORTYPE: ElementType = ElementType {
    type_name: "IFCINTERCEPTORTYPE",
    arity: 10,
    predefined_slot: 9,
    predefined_optional: false,
    fallback_attr: "ElementType",
    fallback_slot: 8,
    family: Family::Element,
    members: &[
        "CYCLONIC",
        "GREASE",
        "OIL",
        "PETROL",
        "USERDEFINED",
        "NOTDEFINED",
    ],
};

/// `IfcJunctionBoxType`, 4 permitted tokens.
pub const IFCJUNCTIONBOXTYPE: ElementType = ElementType {
    type_name: "IFCJUNCTIONBOXTYPE",
    arity: 10,
    predefined_slot: 9,
    predefined_optional: false,
    fallback_attr: "ElementType",
    fallback_slot: 8,
    family: Family::Element,
    members: &["DATA", "POWER", "USERDEFINED", "NOTDEFINED"],
};

/// `IfcKerbType`, 2 permitted tokens.
pub const IFCKERBTYPE: ElementType = ElementType {
    type_name: "IFCKERBTYPE",
    arity: 10,
    predefined_slot: 9,
    predefined_optional: false,
    fallback_attr: "ElementType",
    fallback_slot: 8,
    family: Family::Element,
    members: &["USERDEFINED", "NOTDEFINED"],
};
