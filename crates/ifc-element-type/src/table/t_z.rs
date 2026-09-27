//! Element types T-Z. Generated; do not edit.

use super::{ElementType, Family};

/// `IfcTankType`, 10 permitted tokens.
pub const IFCTANKTYPE: ElementType = ElementType {
    type_name: "IFCTANKTYPE",
    arity: 10,
    predefined_slot: 9,
    predefined_optional: false,
    fallback_attr: "ElementType",
    fallback_slot: 8,
    family: Family::Element,
    members: &[
        "BASIN",
        "BREAKPRESSURE",
        "EXPANSION",
        "FEEDANDEXPANSION",
        "OILRETENTIONTRAY",
        "PRESSUREVESSEL",
        "STORAGE",
        "VESSEL",
        "USERDEFINED",
        "NOTDEFINED",
    ],
};

/// `IfcTaskType`, 23 permitted tokens.
pub const IFCTASKTYPE: ElementType = ElementType {
    type_name: "IFCTASKTYPE",
    arity: 11,
    predefined_slot: 9,
    predefined_optional: false,
    fallback_attr: "ProcessType",
    fallback_slot: 8,
    family: Family::ResourceOrProcess,
    members: &[
        "ADJUSTMENT",
        "ATTENDANCE",
        "CALIBRATION",
        "CONSTRUCTION",
        "DEMOLITION",
        "DISMANTLE",
        "DISPOSAL",
        "EMERGENCY",
        "INSPECTION",
        "INSTALLATION",
        "LOGISTIC",
        "MAINTENANCE",
        "MOVE",
        "OPERATION",
        "REMOVAL",
        "RENOVATION",
        "SAFETY",
        "SHUTDOWN",
        "STARTUP",
        "TESTING",
        "TROUBLESHOOTING",
        "USERDEFINED",
        "NOTDEFINED",
    ],
};

/// `IfcTendonAnchorType`, 5 permitted tokens.
pub const IFCTENDONANCHORTYPE: ElementType = ElementType {
    type_name: "IFCTENDONANCHORTYPE",
    arity: 10,
    predefined_slot: 9,
    predefined_optional: false,
    fallback_attr: "ElementType",
    fallback_slot: 8,
    family: Family::Element,
    members: &[
        "COUPLER",
        "FIXED_END",
        "TENSIONING_END",
        "USERDEFINED",
        "NOTDEFINED",
    ],
};

/// `IfcTendonConduitType`, 7 permitted tokens.
pub const IFCTENDONCONDUITTYPE: ElementType = ElementType {
    type_name: "IFCTENDONCONDUITTYPE",
    arity: 10,
    predefined_slot: 9,
    predefined_optional: false,
    fallback_attr: "ElementType",
    fallback_slot: 8,
    family: Family::Element,
    members: &[
        "COUPLER",
        "DIABOLO",
        "DUCT",
        "GROUTING_DUCT",
        "TRUMPET",
        "USERDEFINED",
        "NOTDEFINED",
    ],
};

/// `IfcTendonType`, 6 permitted tokens.
pub const IFCTENDONTYPE: ElementType = ElementType {
    type_name: "IFCTENDONTYPE",
    arity: 13,
    predefined_slot: 9,
    predefined_optional: false,
    fallback_attr: "ElementType",
    fallback_slot: 8,
    family: Family::Element,
    members: &[
        "BAR",
        "COATED",
        "STRAND",
        "WIRE",
        "USERDEFINED",
        "NOTDEFINED",
    ],
};

/// `IfcTrackElementType`, 10 permitted tokens.
pub const IFCTRACKELEMENTTYPE: ElementType = ElementType {
    type_name: "IFCTRACKELEMENTTYPE",
    arity: 10,
    predefined_slot: 9,
    predefined_optional: false,
    fallback_attr: "ElementType",
    fallback_slot: 8,
    family: Family::Element,
    members: &[
        "BLOCKINGDEVICE",
        "DERAILER",
        "FROG",
        "HALF_SET_OF_BLADES",
        "SLEEPER",
        "SPEEDREGULATOR",
        "TRACKENDOFALIGNMENT",
        "VEHICLESTOP",
        "USERDEFINED",
        "NOTDEFINED",
    ],
};

/// `IfcTransformerType`, 9 permitted tokens.
pub const IFCTRANSFORMERTYPE: ElementType = ElementType {
    type_name: "IFCTRANSFORMERTYPE",
    arity: 10,
    predefined_slot: 9,
    predefined_optional: false,
    fallback_attr: "ElementType",
    fallback_slot: 8,
    family: Family::Element,
    members: &[
        "CHOPPER",
        "COMBINED",
        "CURRENT",
        "FREQUENCY",
        "INVERTER",
        "RECTIFIER",
        "VOLTAGE",
        "USERDEFINED",
        "NOTDEFINED",
    ],
};

/// `IfcTransportElementType`, 8 permitted tokens.
pub const IFCTRANSPORTELEMENTTYPE: ElementType = ElementType {
    type_name: "IFCTRANSPORTELEMENTTYPE",
    arity: 10,
    predefined_slot: 9,
    predefined_optional: false,
    fallback_attr: "ElementType",
    fallback_slot: 8,
    family: Family::Element,
    members: &[
        "CRANEWAY",
        "ELEVATOR",
        "ESCALATOR",
        "HAULINGGEAR",
        "LIFTINGGEAR",
        "MOVINGWALKWAY",
        "USERDEFINED",
        "NOTDEFINED",
    ],
};

/// `IfcTubeBundleType`, 3 permitted tokens.
pub const IFCTUBEBUNDLETYPE: ElementType = ElementType {
    type_name: "IFCTUBEBUNDLETYPE",
    arity: 10,
    predefined_slot: 9,
    predefined_optional: false,
    fallback_attr: "ElementType",
    fallback_slot: 8,
    family: Family::Element,
    members: &["FINNED", "USERDEFINED", "NOTDEFINED"],
};

/// `IfcUnitaryControlElementType`, 12 permitted tokens.
pub const IFCUNITARYCONTROLELEMENTTYPE: ElementType = ElementType {
    type_name: "IFCUNITARYCONTROLELEMENTTYPE",
    arity: 10,
    predefined_slot: 9,
    predefined_optional: false,
    fallback_attr: "ElementType",
    fallback_slot: 8,
    family: Family::Element,
    members: &[
        "ALARMPANEL",
        "BASESTATIONCONTROLLER",
        "COMBINED",
        "CONTROLPANEL",
        "GASDETECTIONPANEL",
        "HUMIDISTAT",
        "INDICATORPANEL",
        "MIMICPANEL",
        "THERMOSTAT",
        "WEATHERSTATION",
        "USERDEFINED",
        "NOTDEFINED",
    ],
};

/// `IfcUnitaryEquipmentType`, 7 permitted tokens.
pub const IFCUNITARYEQUIPMENTTYPE: ElementType = ElementType {
    type_name: "IFCUNITARYEQUIPMENTTYPE",
    arity: 10,
    predefined_slot: 9,
    predefined_optional: false,
    fallback_attr: "ElementType",
    fallback_slot: 8,
    family: Family::Element,
    members: &[
        "AIRCONDITIONINGUNIT",
        "AIRHANDLER",
        "DEHUMIDIFIER",
        "ROOFTOPUNIT",
        "SPLITSYSTEM",
        "USERDEFINED",
        "NOTDEFINED",
    ],
};

/// `IfcValveType`, 23 permitted tokens.
pub const IFCVALVETYPE: ElementType = ElementType {
    type_name: "IFCVALVETYPE",
    arity: 10,
    predefined_slot: 9,
    predefined_optional: false,
    fallback_attr: "ElementType",
    fallback_slot: 8,
    family: Family::Element,
    members: &[
        "AIRRELEASE",
        "ANTIVACUUM",
        "CHANGEOVER",
        "CHECK",
        "COMMISSIONING",
        "DIVERTING",
        "DOUBLECHECK",
        "DOUBLEREGULATING",
        "DRAWOFFCOCK",
        "FAUCET",
        "FLUSHING",
        "GASCOCK",
        "GASTAP",
        "ISOLATING",
        "MIXING",
        "PRESSUREREDUCING",
        "PRESSURERELIEF",
        "REGULATING",
        "SAFETYCUTOFF",
        "STEAMTRAP",
        "STOPCOCK",
        "USERDEFINED",
        "NOTDEFINED",
    ],
};

/// `IfcVehicleType`, 9 permitted tokens.
pub const IFCVEHICLETYPE: ElementType = ElementType {
    type_name: "IFCVEHICLETYPE",
    arity: 10,
    predefined_slot: 9,
    predefined_optional: false,
    fallback_attr: "ElementType",
    fallback_slot: 8,
    family: Family::Element,
    members: &[
        "CARGO",
        "ROLLINGSTOCK",
        "VEHICLE",
        "VEHICLEAIR",
        "VEHICLEMARINE",
        "VEHICLETRACKED",
        "VEHICLEWHEELED",
        "USERDEFINED",
        "NOTDEFINED",
    ],
};

/// `IfcVibrationDamperType`, 8 permitted tokens.
pub const IFCVIBRATIONDAMPERTYPE: ElementType = ElementType {
    type_name: "IFCVIBRATIONDAMPERTYPE",
    arity: 10,
    predefined_slot: 9,
    predefined_optional: false,
    fallback_attr: "ElementType",
    fallback_slot: 8,
    family: Family::Element,
    members: &[
        "AXIAL_YIELD",
        "BENDING_YIELD",
        "FRICTION",
        "RUBBER",
        "SHEAR_YIELD",
        "VISCOUS",
        "USERDEFINED",
        "NOTDEFINED",
    ],
};

/// `IfcVibrationIsolatorType`, 5 permitted tokens.
pub const IFCVIBRATIONISOLATORTYPE: ElementType = ElementType {
    type_name: "IFCVIBRATIONISOLATORTYPE",
    arity: 10,
    predefined_slot: 9,
    predefined_optional: false,
    fallback_attr: "ElementType",
    fallback_slot: 8,
    family: Family::Element,
    members: &["BASE", "COMPRESSION", "SPRING", "USERDEFINED", "NOTDEFINED"],
};

/// `IfcWallType`, 13 permitted tokens.
pub const IFCWALLTYPE: ElementType = ElementType {
    type_name: "IFCWALLTYPE",
    arity: 10,
    predefined_slot: 9,
    predefined_optional: false,
    fallback_attr: "ElementType",
    fallback_slot: 8,
    family: Family::Element,
    members: &[
        "ELEMENTEDWALL",
        "MOVABLE",
        "PARAPET",
        "PARTITIONING",
        "PLUMBINGWALL",
        "POLYGONAL",
        "RETAININGWALL",
        "SHEAR",
        "SOLIDWALL",
        "STANDARD",
        "WAVEWALL",
        "USERDEFINED",
        "NOTDEFINED",
    ],
};

/// `IfcWasteTerminalType`, 9 permitted tokens.
pub const IFCWASTETERMINALTYPE: ElementType = ElementType {
    type_name: "IFCWASTETERMINALTYPE",
    arity: 10,
    predefined_slot: 9,
    predefined_optional: false,
    fallback_attr: "ElementType",
    fallback_slot: 8,
    family: Family::Element,
    members: &[
        "FLOORTRAP",
        "FLOORWASTE",
        "GULLYSUMP",
        "GULLYTRAP",
        "ROOFDRAIN",
        "WASTEDISPOSALUNIT",
        "WASTETRAP",
        "USERDEFINED",
        "NOTDEFINED",
    ],
};

/// `IfcWindowType`, 5 permitted tokens.
pub const IFCWINDOWTYPE: ElementType = ElementType {
    type_name: "IFCWINDOWTYPE",
    arity: 13,
    predefined_slot: 9,
    predefined_optional: false,
    fallback_attr: "ElementType",
    fallback_slot: 8,
    family: Family::Element,
    members: &[
        "LIGHTDOME",
        "SKYLIGHT",
        "WINDOW",
        "USERDEFINED",
        "NOTDEFINED",
    ],
};
