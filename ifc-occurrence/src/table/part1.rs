//! Generated occurrence rows. Do not edit by hand.

use super::Occurrence;

/// `IfcActuator`, 7 permitted tokens.
pub const IFCACTUATOR: Occurrence = Occurrence {
    type_name: "IFCACTUATOR",
    arity: 9,
    predefined_slot: Some(8),
    members: &[
        "ELECTRICACTUATOR",
        "HANDOPERATEDACTUATOR",
        "HYDRAULICACTUATOR",
        "PNEUMATICACTUATOR",
        "THERMOSTATICACTUATOR",
        "USERDEFINED",
        "NOTDEFINED",
    ],
    type_class: Some("IFCACTUATORTYPE"),
};

/// `IfcAirTerminal`, 6 permitted tokens.
pub const IFCAIRTERMINAL: Occurrence = Occurrence {
    type_name: "IFCAIRTERMINAL",
    arity: 9,
    predefined_slot: Some(8),
    members: &[
        "DIFFUSER",
        "GRILLE",
        "LOUVRE",
        "REGISTER",
        "USERDEFINED",
        "NOTDEFINED",
    ],
    type_class: Some("IFCAIRTERMINALTYPE"),
};

/// `IfcAirTerminalBox`, 5 permitted tokens.
pub const IFCAIRTERMINALBOX: Occurrence = Occurrence {
    type_name: "IFCAIRTERMINALBOX",
    arity: 9,
    predefined_slot: Some(8),
    members: &[
        "CONSTANTFLOW",
        "VARIABLEFLOWPRESSUREDEPENDANT",
        "VARIABLEFLOWPRESSUREINDEPENDANT",
        "USERDEFINED",
        "NOTDEFINED",
    ],
    type_class: Some("IFCAIRTERMINALBOXTYPE"),
};

/// `IfcAirToAirHeatRecovery`, 11 permitted tokens.
pub const IFCAIRTOAIRHEATRECOVERY: Occurrence = Occurrence {
    type_name: "IFCAIRTOAIRHEATRECOVERY",
    arity: 9,
    predefined_slot: Some(8),
    members: &[
        "FIXEDPLATECOUNTERFLOWEXCHANGER",
        "FIXEDPLATECROSSFLOWEXCHANGER",
        "FIXEDPLATEPARALLELFLOWEXCHANGER",
        "HEATPIPE",
        "ROTARYWHEEL",
        "RUNAROUNDCOILLOOP",
        "THERMOSIPHONCOILTYPEHEATEXCHANGERS",
        "THERMOSIPHONSEALEDTUBEHEATEXCHANGERS",
        "TWINTOWERENTHALPYRECOVERYLOOPS",
        "USERDEFINED",
        "NOTDEFINED",
    ],
    type_class: Some("IFCAIRTOAIRHEATRECOVERYTYPE"),
};

/// `IfcAlarm`, 10 permitted tokens.
pub const IFCALARM: Occurrence = Occurrence {
    type_name: "IFCALARM",
    arity: 9,
    predefined_slot: Some(8),
    members: &[
        "BELL",
        "BREAKGLASSBUTTON",
        "LIGHT",
        "MANUALPULLBOX",
        "RAILWAYCROCODILE",
        "RAILWAYDETONATOR",
        "SIREN",
        "WHISTLE",
        "USERDEFINED",
        "NOTDEFINED",
    ],
    type_class: Some("IFCALARMTYPE"),
};

/// `IfcAudioVisualAppliance`, 15 permitted tokens.
pub const IFCAUDIOVISUALAPPLIANCE: Occurrence = Occurrence {
    type_name: "IFCAUDIOVISUALAPPLIANCE",
    arity: 9,
    predefined_slot: Some(8),
    members: &[
        "AMPLIFIER",
        "CAMERA",
        "COMMUNICATIONTERMINAL",
        "DISPLAY",
        "MICROPHONE",
        "PLAYER",
        "PROJECTOR",
        "RECEIVER",
        "RECORDINGEQUIPMENT",
        "SPEAKER",
        "SWITCHER",
        "TELEPHONE",
        "TUNER",
        "USERDEFINED",
        "NOTDEFINED",
    ],
    type_class: Some("IFCAUDIOVISUALAPPLIANCETYPE"),
};

/// `IfcBeam`, 14 permitted tokens.
pub const IFCBEAM: Occurrence = Occurrence {
    type_name: "IFCBEAM",
    arity: 9,
    predefined_slot: Some(8),
    members: &[
        "BEAM",
        "CORNICE",
        "DIAPHRAGM",
        "EDGEBEAM",
        "GIRDER_SEGMENT",
        "HATSTONE",
        "HOLLOWCORE",
        "JOIST",
        "LINTEL",
        "PIERCAP",
        "SPANDREL",
        "T_BEAM",
        "USERDEFINED",
        "NOTDEFINED",
    ],
    type_class: Some("IFCBEAMTYPE"),
};

/// `IfcBearing`, 10 permitted tokens.
pub const IFCBEARING: Occurrence = Occurrence {
    type_name: "IFCBEARING",
    arity: 9,
    predefined_slot: Some(8),
    members: &[
        "CYLINDRICAL",
        "DISK",
        "ELASTOMERIC",
        "GUIDE",
        "POT",
        "ROCKER",
        "ROLLER",
        "SPHERICAL",
        "USERDEFINED",
        "NOTDEFINED",
    ],
    type_class: Some("IFCBEARINGTYPE"),
};

/// `IfcBoiler`, 4 permitted tokens.
pub const IFCBOILER: Occurrence = Occurrence {
    type_name: "IFCBOILER",
    arity: 9,
    predefined_slot: Some(8),
    members: &["STEAM", "WATER", "USERDEFINED", "NOTDEFINED"],
    type_class: Some("IFCBOILERTYPE"),
};

/// `IfcBorehole`, 0 permitted tokens.
pub const IFCBOREHOLE: Occurrence = Occurrence {
    type_name: "IFCBOREHOLE",
    arity: 8,
    predefined_slot: None,
    members: &[],
    type_class: None,
};

/// `IfcBuildingElementPart`, 7 permitted tokens.
pub const IFCBUILDINGELEMENTPART: Occurrence = Occurrence {
    type_name: "IFCBUILDINGELEMENTPART",
    arity: 9,
    predefined_slot: Some(8),
    members: &[
        "APRON",
        "ARMOURUNIT",
        "INSULATION",
        "PRECASTPANEL",
        "SAFETYCAGE",
        "USERDEFINED",
        "NOTDEFINED",
    ],
    type_class: Some("IFCBUILDINGELEMENTPARTTYPE"),
};

/// `IfcBuildingElementProxy`, 7 permitted tokens.
pub const IFCBUILDINGELEMENTPROXY: Occurrence = Occurrence {
    type_name: "IFCBUILDINGELEMENTPROXY",
    arity: 9,
    predefined_slot: Some(8),
    members: &[
        "COMPLEX",
        "ELEMENT",
        "PARTIAL",
        "PROVISIONFORSPACE",
        "PROVISIONFORVOID",
        "USERDEFINED",
        "NOTDEFINED",
    ],
    type_class: Some("IFCBUILDINGELEMENTPROXYTYPE"),
};

/// `IfcBuiltElement`, 0 permitted tokens.
pub const IFCBUILTELEMENT: Occurrence = Occurrence {
    type_name: "IFCBUILTELEMENT",
    arity: 8,
    predefined_slot: None,
    members: &[],
    type_class: None,
};

/// `IfcBurner`, 2 permitted tokens.
pub const IFCBURNER: Occurrence = Occurrence {
    type_name: "IFCBURNER",
    arity: 9,
    predefined_slot: Some(8),
    members: &["USERDEFINED", "NOTDEFINED"],
    type_class: Some("IFCBURNERTYPE"),
};

/// `IfcCableCarrierFitting`, 9 permitted tokens.
pub const IFCCABLECARRIERFITTING: Occurrence = Occurrence {
    type_name: "IFCCABLECARRIERFITTING",
    arity: 9,
    predefined_slot: Some(8),
    members: &[
        "BEND",
        "CONNECTOR",
        "CROSS",
        "JUNCTION",
        "REDUCER",
        "TEE",
        "TRANSITION",
        "USERDEFINED",
        "NOTDEFINED",
    ],
    type_class: Some("IFCCABLECARRIERFITTINGTYPE"),
};

/// `IfcCableCarrierSegment`, 9 permitted tokens.
pub const IFCCABLECARRIERSEGMENT: Occurrence = Occurrence {
    type_name: "IFCCABLECARRIERSEGMENT",
    arity: 9,
    predefined_slot: Some(8),
    members: &[
        "CABLEBRACKET",
        "CABLELADDERSEGMENT",
        "CABLETRAYSEGMENT",
        "CABLETRUNKINGSEGMENT",
        "CATENARYWIRE",
        "CONDUITSEGMENT",
        "DROPPER",
        "USERDEFINED",
        "NOTDEFINED",
    ],
    type_class: Some("IFCCABLECARRIERSEGMENTTYPE"),
};

/// `IfcCableFitting`, 8 permitted tokens.
pub const IFCCABLEFITTING: Occurrence = Occurrence {
    type_name: "IFCCABLEFITTING",
    arity: 9,
    predefined_slot: Some(8),
    members: &[
        "CONNECTOR",
        "ENTRY",
        "EXIT",
        "FANOUT",
        "JUNCTION",
        "TRANSITION",
        "USERDEFINED",
        "NOTDEFINED",
    ],
    type_class: Some("IFCCABLEFITTINGTYPE"),
};

/// `IfcCableSegment`, 12 permitted tokens.
pub const IFCCABLESEGMENT: Occurrence = Occurrence {
    type_name: "IFCCABLESEGMENT",
    arity: 9,
    predefined_slot: Some(8),
    members: &[
        "BUSBARSEGMENT",
        "CABLESEGMENT",
        "CONDUCTORSEGMENT",
        "CONTACTWIRESEGMENT",
        "CORESEGMENT",
        "FIBERSEGMENT",
        "FIBERTUBE",
        "OPTICALCABLESEGMENT",
        "STITCHWIRE",
        "WIREPAIRSEGMENT",
        "USERDEFINED",
        "NOTDEFINED",
    ],
    type_class: Some("IFCCABLESEGMENTTYPE"),
};

/// `IfcCaissonFoundation`, 4 permitted tokens.
pub const IFCCAISSONFOUNDATION: Occurrence = Occurrence {
    type_name: "IFCCAISSONFOUNDATION",
    arity: 9,
    predefined_slot: Some(8),
    members: &["CAISSON", "WELL", "USERDEFINED", "NOTDEFINED"],
    type_class: Some("IFCCAISSONFOUNDATIONTYPE"),
};

/// `IfcChiller`, 5 permitted tokens.
pub const IFCCHILLER: Occurrence = Occurrence {
    type_name: "IFCCHILLER",
    arity: 9,
    predefined_slot: Some(8),
    members: &[
        "AIRCOOLED",
        "HEATRECOVERY",
        "WATERCOOLED",
        "USERDEFINED",
        "NOTDEFINED",
    ],
    type_class: Some("IFCCHILLERTYPE"),
};

/// `IfcChimney`, 2 permitted tokens.
pub const IFCCHIMNEY: Occurrence = Occurrence {
    type_name: "IFCCHIMNEY",
    arity: 9,
    predefined_slot: Some(8),
    members: &["USERDEFINED", "NOTDEFINED"],
    type_class: Some("IFCCHIMNEYTYPE"),
};

/// `IfcCivilElement`, 0 permitted tokens.
pub const IFCCIVILELEMENT: Occurrence = Occurrence {
    type_name: "IFCCIVILELEMENT",
    arity: 8,
    predefined_slot: None,
    members: &[],
    type_class: None,
};

/// `IfcCoil`, 9 permitted tokens.
pub const IFCCOIL: Occurrence = Occurrence {
    type_name: "IFCCOIL",
    arity: 9,
    predefined_slot: Some(8),
    members: &[
        "DXCOOLINGCOIL",
        "ELECTRICHEATINGCOIL",
        "GASHEATINGCOIL",
        "HYDRONICCOIL",
        "STEAMHEATINGCOIL",
        "WATERCOOLINGCOIL",
        "WATERHEATINGCOIL",
        "USERDEFINED",
        "NOTDEFINED",
    ],
    type_class: Some("IFCCOILTYPE"),
};

/// `IfcColumn`, 7 permitted tokens.
pub const IFCCOLUMN: Occurrence = Occurrence {
    type_name: "IFCCOLUMN",
    arity: 9,
    predefined_slot: Some(8),
    members: &[
        "COLUMN",
        "PIERSTEM",
        "PIERSTEM_SEGMENT",
        "PILASTER",
        "STANDCOLUMN",
        "USERDEFINED",
        "NOTDEFINED",
    ],
    type_class: Some("IFCCOLUMNTYPE"),
};

/// `IfcCommunicationsAppliance`, 26 permitted tokens.
pub const IFCCOMMUNICATIONSAPPLIANCE: Occurrence = Occurrence {
    type_name: "IFCCOMMUNICATIONSAPPLIANCE",
    arity: 9,
    predefined_slot: Some(8),
    members: &[
        "ANTENNA",
        "AUTOMATON",
        "COMPUTER",
        "FAX",
        "GATEWAY",
        "INTELLIGENTPERIPHERAL",
        "IPNETWORKEQUIPMENT",
        "LINESIDEELECTRONICUNIT",
        "MODEM",
        "NETWORKAPPLIANCE",
        "NETWORKBRIDGE",
        "NETWORKHUB",
        "OPTICALLINETERMINAL",
        "OPTICALNETWORKUNIT",
        "PRINTER",
        "RADIOBLOCKCENTER",
        "REPEATER",
        "ROUTER",
        "SCANNER",
        "TELECOMMAND",
        "TELEPHONYEXCHANGE",
        "TRANSITIONCOMPONENT",
        "TRANSPONDER",
        "TRANSPORTEQUIPMENT",
        "USERDEFINED",
        "NOTDEFINED",
    ],
    type_class: Some("IFCCOMMUNICATIONSAPPLIANCETYPE"),
};

/// `IfcCompressor`, 17 permitted tokens.
pub const IFCCOMPRESSOR: Occurrence = Occurrence {
    type_name: "IFCCOMPRESSOR",
    arity: 9,
    predefined_slot: Some(8),
    members: &[
        "BOOSTER",
        "DYNAMIC",
        "HERMETIC",
        "OPENTYPE",
        "RECIPROCATING",
        "ROLLINGPISTON",
        "ROTARY",
        "ROTARYVANE",
        "SCROLL",
        "SEMIHERMETIC",
        "SINGLESCREW",
        "SINGLESTAGE",
        "TROCHOIDAL",
        "TWINSCREW",
        "WELDEDSHELLHERMETIC",
        "USERDEFINED",
        "NOTDEFINED",
    ],
    type_class: Some("IFCCOMPRESSORTYPE"),
};

/// `IfcCondenser`, 9 permitted tokens.
pub const IFCCONDENSER: Occurrence = Occurrence {
    type_name: "IFCCONDENSER",
    arity: 9,
    predefined_slot: Some(8),
    members: &[
        "AIRCOOLED",
        "EVAPORATIVECOOLED",
        "WATERCOOLED",
        "WATERCOOLEDBRAZEDPLATE",
        "WATERCOOLEDSHELLCOIL",
        "WATERCOOLEDSHELLTUBE",
        "WATERCOOLEDTUBEINTUBE",
        "USERDEFINED",
        "NOTDEFINED",
    ],
    type_class: Some("IFCCONDENSERTYPE"),
};

/// `IfcController`, 7 permitted tokens.
pub const IFCCONTROLLER: Occurrence = Occurrence {
    type_name: "IFCCONTROLLER",
    arity: 9,
    predefined_slot: Some(8),
    members: &[
        "FLOATING",
        "MULTIPOSITION",
        "PROGRAMMABLE",
        "PROPORTIONAL",
        "TWOPOSITION",
        "USERDEFINED",
        "NOTDEFINED",
    ],
    type_class: Some("IFCCONTROLLERTYPE"),
};

/// `IfcConveyorSegment`, 6 permitted tokens.
pub const IFCCONVEYORSEGMENT: Occurrence = Occurrence {
    type_name: "IFCCONVEYORSEGMENT",
    arity: 9,
    predefined_slot: Some(8),
    members: &[
        "BELTCONVEYOR",
        "BUCKETCONVEYOR",
        "CHUTECONVEYOR",
        "SCREWCONVEYOR",
        "USERDEFINED",
        "NOTDEFINED",
    ],
    type_class: Some("IFCCONVEYORSEGMENTTYPE"),
};
