//! Element types Q-S. Generated; do not edit.

use super::{ElementType, Family};

/// `IfcRailType`, 8 permitted tokens.
pub const IFCRAILTYPE: ElementType = ElementType {
    type_name: "IFCRAILTYPE",
    arity: 10,
    predefined_slot: 9,
    predefined_optional: false,
    fallback_attr: "ElementType",
    fallback_slot: 8,
    family: Family::Element,
    members: &[
        "BLADE",
        "CHECKRAIL",
        "GUARDRAIL",
        "RACKRAIL",
        "RAIL",
        "STOCKRAIL",
        "USERDEFINED",
        "NOTDEFINED",
    ],
};

/// `IfcRailingType`, 6 permitted tokens.
pub const IFCRAILINGTYPE: ElementType = ElementType {
    type_name: "IFCRAILINGTYPE",
    arity: 10,
    predefined_slot: 9,
    predefined_optional: false,
    fallback_attr: "ElementType",
    fallback_slot: 8,
    family: Family::Element,
    members: &[
        "BALUSTRADE",
        "FENCE",
        "GUARDRAIL",
        "HANDRAIL",
        "USERDEFINED",
        "NOTDEFINED",
    ],
};

/// `IfcRampFlightType`, 4 permitted tokens.
pub const IFCRAMPFLIGHTTYPE: ElementType = ElementType {
    type_name: "IFCRAMPFLIGHTTYPE",
    arity: 10,
    predefined_slot: 9,
    predefined_optional: false,
    fallback_attr: "ElementType",
    fallback_slot: 8,
    family: Family::Element,
    members: &["SPIRAL", "STRAIGHT", "USERDEFINED", "NOTDEFINED"],
};

/// `IfcRampType`, 8 permitted tokens.
pub const IFCRAMPTYPE: ElementType = ElementType {
    type_name: "IFCRAMPTYPE",
    arity: 10,
    predefined_slot: 9,
    predefined_optional: false,
    fallback_attr: "ElementType",
    fallback_slot: 8,
    family: Family::Element,
    members: &[
        "HALF_TURN_RAMP",
        "QUARTER_TURN_RAMP",
        "SPIRAL_RAMP",
        "STRAIGHT_RUN_RAMP",
        "TWO_QUARTER_TURN_RAMP",
        "TWO_STRAIGHT_RUN_RAMP",
        "USERDEFINED",
        "NOTDEFINED",
    ],
};

/// `IfcReinforcingBarType`, 11 permitted tokens.
pub const IFCREINFORCINGBARTYPE: ElementType = ElementType {
    type_name: "IFCREINFORCINGBARTYPE",
    arity: 16,
    predefined_slot: 9,
    predefined_optional: false,
    fallback_attr: "ElementType",
    fallback_slot: 8,
    family: Family::Element,
    members: &[
        "ANCHORING",
        "EDGE",
        "LIGATURE",
        "MAIN",
        "PUNCHING",
        "RING",
        "SHEAR",
        "SPACEBAR",
        "STUD",
        "USERDEFINED",
        "NOTDEFINED",
    ],
};

/// `IfcReinforcingMeshType`, 2 permitted tokens.
pub const IFCREINFORCINGMESHTYPE: ElementType = ElementType {
    type_name: "IFCREINFORCINGMESHTYPE",
    arity: 20,
    predefined_slot: 9,
    predefined_optional: false,
    fallback_attr: "ElementType",
    fallback_slot: 8,
    family: Family::Element,
    members: &["USERDEFINED", "NOTDEFINED"],
};

/// `IfcRoofType`, 15 permitted tokens.
pub const IFCROOFTYPE: ElementType = ElementType {
    type_name: "IFCROOFTYPE",
    arity: 10,
    predefined_slot: 9,
    predefined_optional: false,
    fallback_attr: "ElementType",
    fallback_slot: 8,
    family: Family::Element,
    members: &[
        "BARREL_ROOF",
        "BUTTERFLY_ROOF",
        "DOME_ROOF",
        "FLAT_ROOF",
        "FREEFORM",
        "GABLE_ROOF",
        "GAMBREL_ROOF",
        "HIPPED_GABLE_ROOF",
        "HIP_ROOF",
        "MANSARD_ROOF",
        "PAVILION_ROOF",
        "RAINBOW_ROOF",
        "SHED_ROOF",
        "USERDEFINED",
        "NOTDEFINED",
    ],
};

/// `IfcSanitaryTerminalType`, 12 permitted tokens.
pub const IFCSANITARYTERMINALTYPE: ElementType = ElementType {
    type_name: "IFCSANITARYTERMINALTYPE",
    arity: 10,
    predefined_slot: 9,
    predefined_optional: false,
    fallback_attr: "ElementType",
    fallback_slot: 8,
    family: Family::Element,
    members: &[
        "BATH",
        "BIDET",
        "CISTERN",
        "SANITARYFOUNTAIN",
        "SHOWER",
        "SINK",
        "TOILETPAN",
        "URINAL",
        "WASHHANDBASIN",
        "WCSEAT",
        "USERDEFINED",
        "NOTDEFINED",
    ],
};

/// `IfcSensorType`, 34 permitted tokens.
pub const IFCSENSORTYPE: ElementType = ElementType {
    type_name: "IFCSENSORTYPE",
    arity: 10,
    predefined_slot: 9,
    predefined_optional: false,
    fallback_attr: "ElementType",
    fallback_slot: 8,
    family: Family::Element,
    members: &[
        "CO2SENSOR",
        "CONDUCTANCESENSOR",
        "CONTACTSENSOR",
        "COSENSOR",
        "EARTHQUAKESENSOR",
        "FIRESENSOR",
        "FLOWSENSOR",
        "FOREIGNOBJECTDETECTIONSENSOR",
        "FROSTSENSOR",
        "GASSENSOR",
        "HEATSENSOR",
        "HUMIDITYSENSOR",
        "IDENTIFIERSENSOR",
        "IONCONCENTRATIONSENSOR",
        "LEVELSENSOR",
        "LIGHTSENSOR",
        "MOISTURESENSOR",
        "MOVEMENTSENSOR",
        "OBSTACLESENSOR",
        "PHSENSOR",
        "PRESSURESENSOR",
        "RADIATIONSENSOR",
        "RADIOACTIVITYSENSOR",
        "RAINSENSOR",
        "SMOKESENSOR",
        "SNOWDEPTHSENSOR",
        "SOUNDSENSOR",
        "TEMPERATURESENSOR",
        "TRAINSENSOR",
        "TURNOUTCLOSURESENSOR",
        "WHEELSENSOR",
        "WINDSENSOR",
        "USERDEFINED",
        "NOTDEFINED",
    ],
};

/// `IfcShadingDeviceType`, 5 permitted tokens.
pub const IFCSHADINGDEVICETYPE: ElementType = ElementType {
    type_name: "IFCSHADINGDEVICETYPE",
    arity: 10,
    predefined_slot: 9,
    predefined_optional: false,
    fallback_attr: "ElementType",
    fallback_slot: 8,
    family: Family::Element,
    members: &["AWNING", "JALOUSIE", "SHUTTER", "USERDEFINED", "NOTDEFINED"],
};

/// `IfcSignType`, 5 permitted tokens.
pub const IFCSIGNTYPE: ElementType = ElementType {
    type_name: "IFCSIGNTYPE",
    arity: 10,
    predefined_slot: 9,
    predefined_optional: false,
    fallback_attr: "ElementType",
    fallback_slot: 8,
    family: Family::Element,
    members: &["MARKER", "MIRROR", "PICTORAL", "USERDEFINED", "NOTDEFINED"],
};

/// `IfcSignalType`, 5 permitted tokens.
pub const IFCSIGNALTYPE: ElementType = ElementType {
    type_name: "IFCSIGNALTYPE",
    arity: 10,
    predefined_slot: 9,
    predefined_optional: false,
    fallback_attr: "ElementType",
    fallback_slot: 8,
    family: Family::Element,
    members: &["AUDIO", "MIXED", "VISUAL", "USERDEFINED", "NOTDEFINED"],
};

/// `IfcSlabType`, 11 permitted tokens.
pub const IFCSLABTYPE: ElementType = ElementType {
    type_name: "IFCSLABTYPE",
    arity: 10,
    predefined_slot: 9,
    predefined_optional: false,
    fallback_attr: "ElementType",
    fallback_slot: 8,
    family: Family::Element,
    members: &[
        "APPROACH_SLAB",
        "BASESLAB",
        "FLOOR",
        "LANDING",
        "PAVING",
        "ROOF",
        "SIDEWALK",
        "TRACKSLAB",
        "WEARING",
        "USERDEFINED",
        "NOTDEFINED",
    ],
};

/// `IfcSolarDeviceType`, 4 permitted tokens.
pub const IFCSOLARDEVICETYPE: ElementType = ElementType {
    type_name: "IFCSOLARDEVICETYPE",
    arity: 10,
    predefined_slot: 9,
    predefined_optional: false,
    fallback_attr: "ElementType",
    fallback_slot: 8,
    family: Family::Element,
    members: &["SOLARCOLLECTOR", "SOLARPANEL", "USERDEFINED", "NOTDEFINED"],
};

/// `IfcSpaceHeaterType`, 4 permitted tokens.
pub const IFCSPACEHEATERTYPE: ElementType = ElementType {
    type_name: "IFCSPACEHEATERTYPE",
    arity: 10,
    predefined_slot: 9,
    predefined_optional: false,
    fallback_attr: "ElementType",
    fallback_slot: 8,
    family: Family::Element,
    members: &["CONVECTOR", "RADIATOR", "USERDEFINED", "NOTDEFINED"],
};

/// `IfcSpaceType`, 8 permitted tokens.
pub const IFCSPACETYPE: ElementType = ElementType {
    type_name: "IFCSPACETYPE",
    arity: 11,
    predefined_slot: 9,
    predefined_optional: false,
    fallback_attr: "ElementType",
    fallback_slot: 8,
    family: Family::Element,
    members: &[
        "BERTH",
        "EXTERNAL",
        "GFA",
        "INTERNAL",
        "PARKING",
        "SPACE",
        "USERDEFINED",
        "NOTDEFINED",
    ],
};

/// `IfcSpatialZoneType`, 12 permitted tokens.
pub const IFCSPATIALZONETYPE: ElementType = ElementType {
    type_name: "IFCSPATIALZONETYPE",
    arity: 11,
    predefined_slot: 9,
    predefined_optional: false,
    fallback_attr: "ElementType",
    fallback_slot: 8,
    family: Family::Element,
    members: &[
        "CONSTRUCTION",
        "FIRESAFETY",
        "INTERFERENCE",
        "LIGHTING",
        "OCCUPANCY",
        "RESERVATION",
        "SECURITY",
        "THERMAL",
        "TRANSPORT",
        "VENTILATION",
        "USERDEFINED",
        "NOTDEFINED",
    ],
};

/// `IfcStackTerminalType`, 5 permitted tokens.
pub const IFCSTACKTERMINALTYPE: ElementType = ElementType {
    type_name: "IFCSTACKTERMINALTYPE",
    arity: 10,
    predefined_slot: 9,
    predefined_optional: false,
    fallback_attr: "ElementType",
    fallback_slot: 8,
    family: Family::Element,
    members: &[
        "BIRDCAGE",
        "COWL",
        "RAINWATERHOPPER",
        "USERDEFINED",
        "NOTDEFINED",
    ],
};

/// `IfcStairFlightType`, 7 permitted tokens.
pub const IFCSTAIRFLIGHTTYPE: ElementType = ElementType {
    type_name: "IFCSTAIRFLIGHTTYPE",
    arity: 10,
    predefined_slot: 9,
    predefined_optional: false,
    fallback_attr: "ElementType",
    fallback_slot: 8,
    family: Family::Element,
    members: &[
        "CURVED",
        "FREEFORM",
        "SPIRAL",
        "STRAIGHT",
        "WINDER",
        "USERDEFINED",
        "NOTDEFINED",
    ],
};

/// `IfcStairType`, 17 permitted tokens.
pub const IFCSTAIRTYPE: ElementType = ElementType {
    type_name: "IFCSTAIRTYPE",
    arity: 10,
    predefined_slot: 9,
    predefined_optional: false,
    fallback_attr: "ElementType",
    fallback_slot: 8,
    family: Family::Element,
    members: &[
        "CURVED_RUN_STAIR",
        "DOUBLE_RETURN_STAIR",
        "HALF_TURN_STAIR",
        "HALF_WINDING_STAIR",
        "LADDER",
        "QUARTER_TURN_STAIR",
        "QUARTER_WINDING_STAIR",
        "SPIRAL_STAIR",
        "STRAIGHT_RUN_STAIR",
        "THREE_QUARTER_TURN_STAIR",
        "THREE_QUARTER_WINDING_STAIR",
        "TWO_CURVED_RUN_STAIR",
        "TWO_QUARTER_TURN_STAIR",
        "TWO_QUARTER_WINDING_STAIR",
        "TWO_STRAIGHT_RUN_STAIR",
        "USERDEFINED",
        "NOTDEFINED",
    ],
};

/// `IfcSubContractResourceType`, 4 permitted tokens.
pub const IFCSUBCONTRACTRESOURCETYPE: ElementType = ElementType {
    type_name: "IFCSUBCONTRACTRESOURCETYPE",
    arity: 12,
    predefined_slot: 11,
    predefined_optional: false,
    fallback_attr: "ResourceType",
    fallback_slot: 8,
    family: Family::ResourceOrProcess,
    members: &["PURCHASE", "WORK", "USERDEFINED", "NOTDEFINED"],
};

/// `IfcSwitchingDeviceType`, 13 permitted tokens.
pub const IFCSWITCHINGDEVICETYPE: ElementType = ElementType {
    type_name: "IFCSWITCHINGDEVICETYPE",
    arity: 10,
    predefined_slot: 9,
    predefined_optional: false,
    fallback_attr: "ElementType",
    fallback_slot: 8,
    family: Family::Element,
    members: &[
        "CONTACTOR",
        "DIMMERSWITCH",
        "EMERGENCYSTOP",
        "KEYPAD",
        "MOMENTARYSWITCH",
        "RELAY",
        "SELECTORSWITCH",
        "STARTER",
        "START_AND_STOP_EQUIPMENT",
        "SWITCHDISCONNECTOR",
        "TOGGLESWITCH",
        "USERDEFINED",
        "NOTDEFINED",
    ],
};

/// `IfcSystemFurnitureElementType`, 5 permitted tokens.
pub const IFCSYSTEMFURNITUREELEMENTTYPE: ElementType = ElementType {
    type_name: "IFCSYSTEMFURNITUREELEMENTTYPE",
    arity: 10,
    predefined_slot: 9,
    predefined_optional: true,
    fallback_attr: "ElementType",
    fallback_slot: 8,
    family: Family::Element,
    members: &[
        "PANEL",
        "SUBRACK",
        "WORKSURFACE",
        "USERDEFINED",
        "NOTDEFINED",
    ],
};
