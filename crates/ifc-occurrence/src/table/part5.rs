//! Generated occurrence rows. Do not edit by hand.

use super::Occurrence;

/// `IfcStair`, 17 permitted tokens.
pub const IFCSTAIR: Occurrence = Occurrence {
    type_name: "IFCSTAIR",
    arity: 9,
    predefined_slot: Some(8),
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
    type_class: Some("IFCSTAIRTYPE"),
};

/// `IfcStairFlight`, 7 permitted tokens.
pub const IFCSTAIRFLIGHT: Occurrence = Occurrence {
    type_name: "IFCSTAIRFLIGHT",
    arity: 13,
    predefined_slot: Some(12),
    members: &[
        "CURVED",
        "FREEFORM",
        "SPIRAL",
        "STRAIGHT",
        "WINDER",
        "USERDEFINED",
        "NOTDEFINED",
    ],
    type_class: Some("IFCSTAIRFLIGHTTYPE"),
};

/// `IfcSurfaceFeature`, 13 permitted tokens.
pub const IFCSURFACEFEATURE: Occurrence = Occurrence {
    type_name: "IFCSURFACEFEATURE",
    arity: 9,
    predefined_slot: Some(8),
    members: &[
        "DEFECT",
        "HATCHMARKING",
        "LINEMARKING",
        "MARK",
        "NONSKIDSURFACING",
        "PAVEMENTSURFACEMARKING",
        "RUMBLESTRIP",
        "SYMBOLMARKING",
        "TAG",
        "TRANSVERSERUMBLESTRIP",
        "TREATMENT",
        "USERDEFINED",
        "NOTDEFINED",
    ],
    type_class: None,
};

/// `IfcSwitchingDevice`, 13 permitted tokens.
pub const IFCSWITCHINGDEVICE: Occurrence = Occurrence {
    type_name: "IFCSWITCHINGDEVICE",
    arity: 9,
    predefined_slot: Some(8),
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
    type_class: Some("IFCSWITCHINGDEVICETYPE"),
};

/// `IfcSystemFurnitureElement`, 5 permitted tokens.
pub const IFCSYSTEMFURNITUREELEMENT: Occurrence = Occurrence {
    type_name: "IFCSYSTEMFURNITUREELEMENT",
    arity: 9,
    predefined_slot: Some(8),
    members: &[
        "PANEL",
        "SUBRACK",
        "WORKSURFACE",
        "USERDEFINED",
        "NOTDEFINED",
    ],
    type_class: Some("IFCSYSTEMFURNITUREELEMENTTYPE"),
};

/// `IfcTank`, 10 permitted tokens.
pub const IFCTANK: Occurrence = Occurrence {
    type_name: "IFCTANK",
    arity: 9,
    predefined_slot: Some(8),
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
    type_class: Some("IFCTANKTYPE"),
};

/// `IfcTendon`, 6 permitted tokens.
pub const IFCTENDON: Occurrence = Occurrence {
    type_name: "IFCTENDON",
    arity: 17,
    predefined_slot: Some(9),
    members: &[
        "BAR",
        "COATED",
        "STRAND",
        "WIRE",
        "USERDEFINED",
        "NOTDEFINED",
    ],
    type_class: Some("IFCTENDONTYPE"),
};

/// `IfcTendonAnchor`, 5 permitted tokens.
pub const IFCTENDONANCHOR: Occurrence = Occurrence {
    type_name: "IFCTENDONANCHOR",
    arity: 10,
    predefined_slot: Some(9),
    members: &[
        "COUPLER",
        "FIXED_END",
        "TENSIONING_END",
        "USERDEFINED",
        "NOTDEFINED",
    ],
    type_class: Some("IFCTENDONANCHORTYPE"),
};

/// `IfcTendonConduit`, 7 permitted tokens.
pub const IFCTENDONCONDUIT: Occurrence = Occurrence {
    type_name: "IFCTENDONCONDUIT",
    arity: 10,
    predefined_slot: Some(9),
    members: &[
        "COUPLER",
        "DIABOLO",
        "DUCT",
        "GROUTING_DUCT",
        "TRUMPET",
        "USERDEFINED",
        "NOTDEFINED",
    ],
    type_class: Some("IFCTENDONCONDUITTYPE"),
};

/// `IfcTrackElement`, 10 permitted tokens.
pub const IFCTRACKELEMENT: Occurrence = Occurrence {
    type_name: "IFCTRACKELEMENT",
    arity: 9,
    predefined_slot: Some(8),
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
    type_class: Some("IFCTRACKELEMENTTYPE"),
};

/// `IfcTransformer`, 9 permitted tokens.
pub const IFCTRANSFORMER: Occurrence = Occurrence {
    type_name: "IFCTRANSFORMER",
    arity: 9,
    predefined_slot: Some(8),
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
    type_class: Some("IFCTRANSFORMERTYPE"),
};

/// `IfcTransportElement`, 8 permitted tokens.
pub const IFCTRANSPORTELEMENT: Occurrence = Occurrence {
    type_name: "IFCTRANSPORTELEMENT",
    arity: 9,
    predefined_slot: Some(8),
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
    type_class: Some("IFCTRANSPORTELEMENTTYPE"),
};

/// `IfcTubeBundle`, 3 permitted tokens.
pub const IFCTUBEBUNDLE: Occurrence = Occurrence {
    type_name: "IFCTUBEBUNDLE",
    arity: 9,
    predefined_slot: Some(8),
    members: &["FINNED", "USERDEFINED", "NOTDEFINED"],
    type_class: Some("IFCTUBEBUNDLETYPE"),
};

/// `IfcUnitaryControlElement`, 12 permitted tokens.
pub const IFCUNITARYCONTROLELEMENT: Occurrence = Occurrence {
    type_name: "IFCUNITARYCONTROLELEMENT",
    arity: 9,
    predefined_slot: Some(8),
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
    type_class: Some("IFCUNITARYCONTROLELEMENTTYPE"),
};

/// `IfcUnitaryEquipment`, 7 permitted tokens.
pub const IFCUNITARYEQUIPMENT: Occurrence = Occurrence {
    type_name: "IFCUNITARYEQUIPMENT",
    arity: 9,
    predefined_slot: Some(8),
    members: &[
        "AIRCONDITIONINGUNIT",
        "AIRHANDLER",
        "DEHUMIDIFIER",
        "ROOFTOPUNIT",
        "SPLITSYSTEM",
        "USERDEFINED",
        "NOTDEFINED",
    ],
    type_class: Some("IFCUNITARYEQUIPMENTTYPE"),
};

/// `IfcValve`, 23 permitted tokens.
pub const IFCVALVE: Occurrence = Occurrence {
    type_name: "IFCVALVE",
    arity: 9,
    predefined_slot: Some(8),
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
    type_class: Some("IFCVALVETYPE"),
};

/// `IfcVehicle`, 9 permitted tokens.
pub const IFCVEHICLE: Occurrence = Occurrence {
    type_name: "IFCVEHICLE",
    arity: 9,
    predefined_slot: Some(8),
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
    type_class: Some("IFCVEHICLETYPE"),
};

/// `IfcVibrationDamper`, 8 permitted tokens.
pub const IFCVIBRATIONDAMPER: Occurrence = Occurrence {
    type_name: "IFCVIBRATIONDAMPER",
    arity: 9,
    predefined_slot: Some(8),
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
    type_class: Some("IFCVIBRATIONDAMPERTYPE"),
};

/// `IfcVibrationIsolator`, 5 permitted tokens.
pub const IFCVIBRATIONISOLATOR: Occurrence = Occurrence {
    type_name: "IFCVIBRATIONISOLATOR",
    arity: 9,
    predefined_slot: Some(8),
    members: &["BASE", "COMPRESSION", "SPRING", "USERDEFINED", "NOTDEFINED"],
    type_class: Some("IFCVIBRATIONISOLATORTYPE"),
};

/// `IfcVirtualElement`, 5 permitted tokens.
pub const IFCVIRTUALELEMENT: Occurrence = Occurrence {
    type_name: "IFCVIRTUALELEMENT",
    arity: 9,
    predefined_slot: Some(8),
    members: &[
        "BOUNDARY",
        "CLEARANCE",
        "PROVISIONFORVOID",
        "USERDEFINED",
        "NOTDEFINED",
    ],
    type_class: None,
};

/// `IfcVoidingFeature`, 8 permitted tokens.
pub const IFCVOIDINGFEATURE: Occurrence = Occurrence {
    type_name: "IFCVOIDINGFEATURE",
    arity: 9,
    predefined_slot: Some(8),
    members: &[
        "CHAMFER",
        "CUTOUT",
        "EDGE",
        "HOLE",
        "MITER",
        "NOTCH",
        "USERDEFINED",
        "NOTDEFINED",
    ],
    type_class: None,
};

/// `IfcWall`, 13 permitted tokens.
pub const IFCWALL: Occurrence = Occurrence {
    type_name: "IFCWALL",
    arity: 9,
    predefined_slot: Some(8),
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
    type_class: Some("IFCWALLTYPE"),
};

/// `IfcWallStandardCase`, 13 permitted tokens.
pub const IFCWALLSTANDARDCASE: Occurrence = Occurrence {
    type_name: "IFCWALLSTANDARDCASE",
    arity: 9,
    predefined_slot: Some(8),
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
    type_class: None,
};

/// `IfcWasteTerminal`, 9 permitted tokens.
pub const IFCWASTETERMINAL: Occurrence = Occurrence {
    type_name: "IFCWASTETERMINAL",
    arity: 9,
    predefined_slot: Some(8),
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
    type_class: Some("IFCWASTETERMINALTYPE"),
};

/// `IfcWindow`, 5 permitted tokens.
pub const IFCWINDOW: Occurrence = Occurrence {
    type_name: "IFCWINDOW",
    arity: 13,
    predefined_slot: Some(10),
    members: &[
        "LIGHTDOME",
        "SKYLIGHT",
        "WINDOW",
        "USERDEFINED",
        "NOTDEFINED",
    ],
    type_class: Some("IFCWINDOWTYPE"),
};
