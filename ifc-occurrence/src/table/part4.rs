//! Generated occurrence rows. Do not edit by hand.

use super::Occurrence;

/// `IfcNavigationElement`, 4 permitted tokens.
pub const IFCNAVIGATIONELEMENT: Occurrence = Occurrence {
    type_name: "IFCNAVIGATIONELEMENT",
    arity: 9,
    predefined_slot: Some(8),
    members: &["BEACON", "BUOY", "USERDEFINED", "NOTDEFINED"],
    type_class: Some("IFCNAVIGATIONELEMENTTYPE"),
};

/// `IfcOpeningElement`, 4 permitted tokens.
pub const IFCOPENINGELEMENT: Occurrence = Occurrence {
    type_name: "IFCOPENINGELEMENT",
    arity: 9,
    predefined_slot: Some(8),
    members: &["OPENING", "RECESS", "USERDEFINED", "NOTDEFINED"],
    type_class: None,
};

/// `IfcOutlet`, 7 permitted tokens.
pub const IFCOUTLET: Occurrence = Occurrence {
    type_name: "IFCOUTLET",
    arity: 9,
    predefined_slot: Some(8),
    members: &[
        "AUDIOVISUALOUTLET",
        "COMMUNICATIONSOUTLET",
        "DATAOUTLET",
        "POWEROUTLET",
        "TELEPHONEOUTLET",
        "USERDEFINED",
        "NOTDEFINED",
    ],
    type_class: Some("IFCOUTLETTYPE"),
};

/// `IfcPavement`, 4 permitted tokens.
pub const IFCPAVEMENT: Occurrence = Occurrence {
    type_name: "IFCPAVEMENT",
    arity: 9,
    predefined_slot: Some(8),
    members: &["FLEXIBLE", "RIGID", "USERDEFINED", "NOTDEFINED"],
    type_class: Some("IFCPAVEMENTTYPE"),
};

/// `IfcPile`, 8 permitted tokens.
pub const IFCPILE: Occurrence = Occurrence {
    type_name: "IFCPILE",
    arity: 10,
    predefined_slot: Some(8),
    members: &[
        "BORED",
        "COHESION",
        "DRIVEN",
        "FRICTION",
        "JETGROUTING",
        "SUPPORT",
        "USERDEFINED",
        "NOTDEFINED",
    ],
    type_class: Some("IFCPILETYPE"),
};

/// `IfcPipeFitting`, 9 permitted tokens.
pub const IFCPIPEFITTING: Occurrence = Occurrence {
    type_name: "IFCPIPEFITTING",
    arity: 9,
    predefined_slot: Some(8),
    members: &[
        "BEND",
        "CONNECTOR",
        "ENTRY",
        "EXIT",
        "JUNCTION",
        "OBSTRUCTION",
        "TRANSITION",
        "USERDEFINED",
        "NOTDEFINED",
    ],
    type_class: Some("IFCPIPEFITTINGTYPE"),
};

/// `IfcPipeSegment`, 7 permitted tokens.
pub const IFCPIPESEGMENT: Occurrence = Occurrence {
    type_name: "IFCPIPESEGMENT",
    arity: 9,
    predefined_slot: Some(8),
    members: &[
        "CULVERT",
        "FLEXIBLESEGMENT",
        "GUTTER",
        "RIGIDSEGMENT",
        "SPOOL",
        "USERDEFINED",
        "NOTDEFINED",
    ],
    type_class: Some("IFCPIPESEGMENTTYPE"),
};

/// `IfcPlate`, 11 permitted tokens.
pub const IFCPLATE: Occurrence = Occurrence {
    type_name: "IFCPLATE",
    arity: 9,
    predefined_slot: Some(8),
    members: &[
        "BASE_PLATE",
        "COVER_PLATE",
        "CURTAIN_PANEL",
        "FLANGE_PLATE",
        "GUSSET_PLATE",
        "SHEET",
        "SPLICE_PLATE",
        "STIFFENER_PLATE",
        "WEB_PLATE",
        "USERDEFINED",
        "NOTDEFINED",
    ],
    type_class: Some("IFCPLATETYPE"),
};

/// `IfcProjectionElement`, 4 permitted tokens.
pub const IFCPROJECTIONELEMENT: Occurrence = Occurrence {
    type_name: "IFCPROJECTIONELEMENT",
    arity: 9,
    predefined_slot: Some(8),
    members: &["BLISTER", "DEVIATOR", "USERDEFINED", "NOTDEFINED"],
    type_class: None,
};

/// `IfcProtectiveDevice`, 12 permitted tokens.
pub const IFCPROTECTIVEDEVICE: Occurrence = Occurrence {
    type_name: "IFCPROTECTIVEDEVICE",
    arity: 9,
    predefined_slot: Some(8),
    members: &[
        "ANTI_ARCING_DEVICE",
        "CIRCUITBREAKER",
        "EARTHINGSWITCH",
        "EARTHLEAKAGECIRCUITBREAKER",
        "FUSEDISCONNECTOR",
        "RESIDUALCURRENTCIRCUITBREAKER",
        "RESIDUALCURRENTSWITCH",
        "SPARKGAP",
        "VARISTOR",
        "VOLTAGELIMITER",
        "USERDEFINED",
        "NOTDEFINED",
    ],
    type_class: Some("IFCPROTECTIVEDEVICETYPE"),
};

/// `IfcProtectiveDeviceTrippingUnit`, 6 permitted tokens.
pub const IFCPROTECTIVEDEVICETRIPPINGUNIT: Occurrence = Occurrence {
    type_name: "IFCPROTECTIVEDEVICETRIPPINGUNIT",
    arity: 9,
    predefined_slot: Some(8),
    members: &[
        "ELECTROMAGNETIC",
        "ELECTRONIC",
        "RESIDUALCURRENT",
        "THERMAL",
        "USERDEFINED",
        "NOTDEFINED",
    ],
    type_class: Some("IFCPROTECTIVEDEVICETRIPPINGUNITTYPE"),
};

/// `IfcPump`, 9 permitted tokens.
pub const IFCPUMP: Occurrence = Occurrence {
    type_name: "IFCPUMP",
    arity: 9,
    predefined_slot: Some(8),
    members: &[
        "CIRCULATOR",
        "ENDSUCTION",
        "SPLITCASE",
        "SUBMERSIBLEPUMP",
        "SUMPPUMP",
        "VERTICALINLINE",
        "VERTICALTURBINE",
        "USERDEFINED",
        "NOTDEFINED",
    ],
    type_class: Some("IFCPUMPTYPE"),
};

/// `IfcRail`, 8 permitted tokens.
pub const IFCRAIL: Occurrence = Occurrence {
    type_name: "IFCRAIL",
    arity: 9,
    predefined_slot: Some(8),
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
    type_class: Some("IFCRAILTYPE"),
};

/// `IfcRailing`, 6 permitted tokens.
pub const IFCRAILING: Occurrence = Occurrence {
    type_name: "IFCRAILING",
    arity: 9,
    predefined_slot: Some(8),
    members: &[
        "BALUSTRADE",
        "FENCE",
        "GUARDRAIL",
        "HANDRAIL",
        "USERDEFINED",
        "NOTDEFINED",
    ],
    type_class: Some("IFCRAILINGTYPE"),
};

/// `IfcRamp`, 8 permitted tokens.
pub const IFCRAMP: Occurrence = Occurrence {
    type_name: "IFCRAMP",
    arity: 9,
    predefined_slot: Some(8),
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
    type_class: Some("IFCRAMPTYPE"),
};

/// `IfcRampFlight`, 4 permitted tokens.
pub const IFCRAMPFLIGHT: Occurrence = Occurrence {
    type_name: "IFCRAMPFLIGHT",
    arity: 9,
    predefined_slot: Some(8),
    members: &["SPIRAL", "STRAIGHT", "USERDEFINED", "NOTDEFINED"],
    type_class: Some("IFCRAMPFLIGHTTYPE"),
};

/// `IfcReinforcedSoil`, 8 permitted tokens.
pub const IFCREINFORCEDSOIL: Occurrence = Occurrence {
    type_name: "IFCREINFORCEDSOIL",
    arity: 9,
    predefined_slot: Some(8),
    members: &[
        "DYNAMICALLYCOMPACTED",
        "GROUTED",
        "REPLACED",
        "ROLLERCOMPACTED",
        "SURCHARGEPRELOADED",
        "VERTICALLYDRAINED",
        "USERDEFINED",
        "NOTDEFINED",
    ],
    type_class: None,
};

/// `IfcReinforcingBar`, 11 permitted tokens.
pub const IFCREINFORCINGBAR: Occurrence = Occurrence {
    type_name: "IFCREINFORCINGBAR",
    arity: 14,
    predefined_slot: Some(12),
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
    type_class: Some("IFCREINFORCINGBARTYPE"),
};

/// `IfcReinforcingMesh`, 2 permitted tokens.
pub const IFCREINFORCINGMESH: Occurrence = Occurrence {
    type_name: "IFCREINFORCINGMESH",
    arity: 18,
    predefined_slot: Some(17),
    members: &["USERDEFINED", "NOTDEFINED"],
    type_class: Some("IFCREINFORCINGMESHTYPE"),
};

/// `IfcRoof`, 15 permitted tokens.
pub const IFCROOF: Occurrence = Occurrence {
    type_name: "IFCROOF",
    arity: 9,
    predefined_slot: Some(8),
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
    type_class: Some("IFCROOFTYPE"),
};

/// `IfcSanitaryTerminal`, 12 permitted tokens.
pub const IFCSANITARYTERMINAL: Occurrence = Occurrence {
    type_name: "IFCSANITARYTERMINAL",
    arity: 9,
    predefined_slot: Some(8),
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
    type_class: Some("IFCSANITARYTERMINALTYPE"),
};

/// `IfcSensor`, 34 permitted tokens.
pub const IFCSENSOR: Occurrence = Occurrence {
    type_name: "IFCSENSOR",
    arity: 9,
    predefined_slot: Some(8),
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
    type_class: Some("IFCSENSORTYPE"),
};

/// `IfcShadingDevice`, 5 permitted tokens.
pub const IFCSHADINGDEVICE: Occurrence = Occurrence {
    type_name: "IFCSHADINGDEVICE",
    arity: 9,
    predefined_slot: Some(8),
    members: &["AWNING", "JALOUSIE", "SHUTTER", "USERDEFINED", "NOTDEFINED"],
    type_class: Some("IFCSHADINGDEVICETYPE"),
};

/// `IfcSign`, 5 permitted tokens.
pub const IFCSIGN: Occurrence = Occurrence {
    type_name: "IFCSIGN",
    arity: 9,
    predefined_slot: Some(8),
    members: &["MARKER", "MIRROR", "PICTORAL", "USERDEFINED", "NOTDEFINED"],
    type_class: Some("IFCSIGNTYPE"),
};

/// `IfcSignal`, 5 permitted tokens.
pub const IFCSIGNAL: Occurrence = Occurrence {
    type_name: "IFCSIGNAL",
    arity: 9,
    predefined_slot: Some(8),
    members: &["AUDIO", "MIXED", "VISUAL", "USERDEFINED", "NOTDEFINED"],
    type_class: Some("IFCSIGNALTYPE"),
};

/// `IfcSlab`, 11 permitted tokens.
pub const IFCSLAB: Occurrence = Occurrence {
    type_name: "IFCSLAB",
    arity: 9,
    predefined_slot: Some(8),
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
    type_class: Some("IFCSLABTYPE"),
};

/// `IfcSolarDevice`, 4 permitted tokens.
pub const IFCSOLARDEVICE: Occurrence = Occurrence {
    type_name: "IFCSOLARDEVICE",
    arity: 9,
    predefined_slot: Some(8),
    members: &["SOLARCOLLECTOR", "SOLARPANEL", "USERDEFINED", "NOTDEFINED"],
    type_class: Some("IFCSOLARDEVICETYPE"),
};

/// `IfcSpaceHeater`, 4 permitted tokens.
pub const IFCSPACEHEATER: Occurrence = Occurrence {
    type_name: "IFCSPACEHEATER",
    arity: 9,
    predefined_slot: Some(8),
    members: &["CONVECTOR", "RADIATOR", "USERDEFINED", "NOTDEFINED"],
    type_class: Some("IFCSPACEHEATERTYPE"),
};

/// `IfcStackTerminal`, 5 permitted tokens.
pub const IFCSTACKTERMINAL: Occurrence = Occurrence {
    type_name: "IFCSTACKTERMINAL",
    arity: 9,
    predefined_slot: Some(8),
    members: &[
        "BIRDCAGE",
        "COWL",
        "RAINWATERHOPPER",
        "USERDEFINED",
        "NOTDEFINED",
    ],
    type_class: Some("IFCSTACKTERMINALTYPE"),
};
