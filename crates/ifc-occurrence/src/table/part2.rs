//! Generated occurrence rows. Do not edit by hand.

use super::Occurrence;

/// `IfcCooledBeam`, 4 permitted tokens.
pub const IFCCOOLEDBEAM: Occurrence = Occurrence {
    type_name: "IFCCOOLEDBEAM",
    arity: 9,
    predefined_slot: Some(8),
    members: &["ACTIVE", "PASSIVE", "USERDEFINED", "NOTDEFINED"],
    type_class: Some("IFCCOOLEDBEAMTYPE"),
};

/// `IfcCoolingTower`, 5 permitted tokens.
pub const IFCCOOLINGTOWER: Occurrence = Occurrence {
    type_name: "IFCCOOLINGTOWER",
    arity: 9,
    predefined_slot: Some(8),
    members: &[
        "MECHANICALFORCEDDRAFT",
        "MECHANICALINDUCEDDRAFT",
        "NATURALDRAFT",
        "USERDEFINED",
        "NOTDEFINED",
    ],
    type_class: Some("IFCCOOLINGTOWERTYPE"),
};

/// `IfcCourse`, 8 permitted tokens.
pub const IFCCOURSE: Occurrence = Occurrence {
    type_name: "IFCCOURSE",
    arity: 9,
    predefined_slot: Some(8),
    members: &[
        "ARMOUR",
        "BALLASTBED",
        "CORE",
        "FILTER",
        "PAVEMENT",
        "PROTECTION",
        "USERDEFINED",
        "NOTDEFINED",
    ],
    type_class: Some("IFCCOURSETYPE"),
};

/// `IfcCovering`, 14 permitted tokens.
pub const IFCCOVERING: Occurrence = Occurrence {
    type_name: "IFCCOVERING",
    arity: 9,
    predefined_slot: Some(8),
    members: &[
        "CEILING",
        "CLADDING",
        "COPING",
        "FLOORING",
        "INSULATION",
        "MEMBRANE",
        "MOLDING",
        "ROOFING",
        "SKIRTINGBOARD",
        "SLEEVING",
        "TOPPING",
        "WRAPPING",
        "USERDEFINED",
        "NOTDEFINED",
    ],
    type_class: Some("IFCCOVERINGTYPE"),
};

/// `IfcCurtainWall`, 2 permitted tokens.
pub const IFCCURTAINWALL: Occurrence = Occurrence {
    type_name: "IFCCURTAINWALL",
    arity: 9,
    predefined_slot: Some(8),
    members: &["USERDEFINED", "NOTDEFINED"],
    type_class: Some("IFCCURTAINWALLTYPE"),
};

/// `IfcDamper`, 13 permitted tokens.
pub const IFCDAMPER: Occurrence = Occurrence {
    type_name: "IFCDAMPER",
    arity: 9,
    predefined_slot: Some(8),
    members: &[
        "BACKDRAFTDAMPER",
        "BALANCINGDAMPER",
        "BLASTDAMPER",
        "CONTROLDAMPER",
        "FIREDAMPER",
        "FIRESMOKEDAMPER",
        "FUMEHOODEXHAUST",
        "GRAVITYDAMPER",
        "GRAVITYRELIEFDAMPER",
        "RELIEFDAMPER",
        "SMOKEDAMPER",
        "USERDEFINED",
        "NOTDEFINED",
    ],
    type_class: Some("IFCDAMPERTYPE"),
};

/// `IfcDeepFoundation`, 0 permitted tokens.
pub const IFCDEEPFOUNDATION: Occurrence = Occurrence {
    type_name: "IFCDEEPFOUNDATION",
    arity: 8,
    predefined_slot: None,
    members: &[],
    type_class: Some("IFCDEEPFOUNDATIONTYPE"),
};

/// `IfcDiscreteAccessory`, 23 permitted tokens.
pub const IFCDISCRETEACCESSORY: Occurrence = Occurrence {
    type_name: "IFCDISCRETEACCESSORY",
    arity: 9,
    predefined_slot: Some(8),
    members: &[
        "ANCHORPLATE",
        "BIRDPROTECTION",
        "BRACKET",
        "CABLEARRANGER",
        "ELASTIC_CUSHION",
        "EXPANSION_JOINT_DEVICE",
        "FILLER",
        "FLASHING",
        "INSULATOR",
        "LOCK",
        "PANEL_STRENGTHENING",
        "POINTMACHINEMOUNTINGDEVICE",
        "POINT_MACHINE_LOCKING_DEVICE",
        "RAILBRACE",
        "RAILPAD",
        "RAIL_LUBRICATION",
        "RAIL_MECHANICAL_EQUIPMENT",
        "SHOE",
        "SLIDINGCHAIR",
        "SOUNDABSORPTION",
        "TENSIONINGEQUIPMENT",
        "USERDEFINED",
        "NOTDEFINED",
    ],
    type_class: Some("IFCDISCRETEACCESSORYTYPE"),
};

/// `IfcDistributionBoard`, 8 permitted tokens.
pub const IFCDISTRIBUTIONBOARD: Occurrence = Occurrence {
    type_name: "IFCDISTRIBUTIONBOARD",
    arity: 9,
    predefined_slot: Some(8),
    members: &[
        "CONSUMERUNIT",
        "DISPATCHINGBOARD",
        "DISTRIBUTIONBOARD",
        "DISTRIBUTIONFRAME",
        "MOTORCONTROLCENTRE",
        "SWITCHBOARD",
        "USERDEFINED",
        "NOTDEFINED",
    ],
    type_class: Some("IFCDISTRIBUTIONBOARDTYPE"),
};

/// `IfcDistributionChamberElement`, 10 permitted tokens.
pub const IFCDISTRIBUTIONCHAMBERELEMENT: Occurrence = Occurrence {
    type_name: "IFCDISTRIBUTIONCHAMBERELEMENT",
    arity: 9,
    predefined_slot: Some(8),
    members: &[
        "FORMEDDUCT",
        "INSPECTIONCHAMBER",
        "INSPECTIONPIT",
        "MANHOLE",
        "METERCHAMBER",
        "SUMP",
        "TRENCH",
        "VALVECHAMBER",
        "USERDEFINED",
        "NOTDEFINED",
    ],
    type_class: Some("IFCDISTRIBUTIONCHAMBERELEMENTTYPE"),
};

/// `IfcDistributionControlElement`, 0 permitted tokens.
pub const IFCDISTRIBUTIONCONTROLELEMENT: Occurrence = Occurrence {
    type_name: "IFCDISTRIBUTIONCONTROLELEMENT",
    arity: 8,
    predefined_slot: None,
    members: &[],
    type_class: None,
};

/// `IfcDistributionFlowElement`, 0 permitted tokens.
pub const IFCDISTRIBUTIONFLOWELEMENT: Occurrence = Occurrence {
    type_name: "IFCDISTRIBUTIONFLOWELEMENT",
    arity: 8,
    predefined_slot: None,
    members: &[],
    type_class: None,
};

/// `IfcDoor`, 7 permitted tokens.
pub const IFCDOOR: Occurrence = Occurrence {
    type_name: "IFCDOOR",
    arity: 13,
    predefined_slot: Some(10),
    members: &[
        "BOOM_BARRIER",
        "DOOR",
        "GATE",
        "TRAPDOOR",
        "TURNSTILE",
        "USERDEFINED",
        "NOTDEFINED",
    ],
    type_class: Some("IFCDOORTYPE"),
};

/// `IfcDuctFitting`, 9 permitted tokens.
pub const IFCDUCTFITTING: Occurrence = Occurrence {
    type_name: "IFCDUCTFITTING",
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
    type_class: Some("IFCDUCTFITTINGTYPE"),
};

/// `IfcDuctSegment`, 4 permitted tokens.
pub const IFCDUCTSEGMENT: Occurrence = Occurrence {
    type_name: "IFCDUCTSEGMENT",
    arity: 9,
    predefined_slot: Some(8),
    members: &[
        "FLEXIBLESEGMENT",
        "RIGIDSEGMENT",
        "USERDEFINED",
        "NOTDEFINED",
    ],
    type_class: Some("IFCDUCTSEGMENTTYPE"),
};

/// `IfcDuctSilencer`, 5 permitted tokens.
pub const IFCDUCTSILENCER: Occurrence = Occurrence {
    type_name: "IFCDUCTSILENCER",
    arity: 9,
    predefined_slot: Some(8),
    members: &[
        "FLATOVAL",
        "RECTANGULAR",
        "ROUND",
        "USERDEFINED",
        "NOTDEFINED",
    ],
    type_class: Some("IFCDUCTSILENCERTYPE"),
};

/// `IfcEarthworksCut`, 11 permitted tokens.
pub const IFCEARTHWORKSCUT: Occurrence = Occurrence {
    type_name: "IFCEARTHWORKSCUT",
    arity: 9,
    predefined_slot: Some(8),
    members: &[
        "BASE_EXCAVATION",
        "CUT",
        "DREDGING",
        "EXCAVATION",
        "OVEREXCAVATION",
        "PAVEMENTMILLING",
        "STEPEXCAVATION",
        "TOPSOILREMOVAL",
        "TRENCH",
        "USERDEFINED",
        "NOTDEFINED",
    ],
    type_class: None,
};

/// `IfcEarthworksElement`, 0 permitted tokens.
pub const IFCEARTHWORKSELEMENT: Occurrence = Occurrence {
    type_name: "IFCEARTHWORKSELEMENT",
    arity: 8,
    predefined_slot: None,
    members: &[],
    type_class: None,
};

/// `IfcEarthworksFill`, 9 permitted tokens.
pub const IFCEARTHWORKSFILL: Occurrence = Occurrence {
    type_name: "IFCEARTHWORKSFILL",
    arity: 9,
    predefined_slot: Some(8),
    members: &[
        "BACKFILL",
        "COUNTERWEIGHT",
        "EMBANKMENT",
        "SLOPEFILL",
        "SUBGRADE",
        "SUBGRADEBED",
        "TRANSITIONSECTION",
        "USERDEFINED",
        "NOTDEFINED",
    ],
    type_class: None,
};

/// `IfcElectricAppliance`, 18 permitted tokens.
pub const IFCELECTRICAPPLIANCE: Occurrence = Occurrence {
    type_name: "IFCELECTRICAPPLIANCE",
    arity: 9,
    predefined_slot: Some(8),
    members: &[
        "DISHWASHER",
        "ELECTRICCOOKER",
        "FREESTANDINGELECTRICHEATER",
        "FREESTANDINGFAN",
        "FREESTANDINGWATERCOOLER",
        "FREESTANDINGWATERHEATER",
        "FREEZER",
        "FRIDGE_FREEZER",
        "HANDDRYER",
        "KITCHENMACHINE",
        "MICROWAVE",
        "PHOTOCOPIER",
        "REFRIGERATOR",
        "TUMBLEDRYER",
        "VENDINGMACHINE",
        "WASHINGMACHINE",
        "USERDEFINED",
        "NOTDEFINED",
    ],
    type_class: Some("IFCELECTRICAPPLIANCETYPE"),
};

/// `IfcElectricDistributionBoard`, 6 permitted tokens.
pub const IFCELECTRICDISTRIBUTIONBOARD: Occurrence = Occurrence {
    type_name: "IFCELECTRICDISTRIBUTIONBOARD",
    arity: 9,
    predefined_slot: Some(8),
    members: &[
        "CONSUMERUNIT",
        "DISTRIBUTIONBOARD",
        "MOTORCONTROLCENTRE",
        "SWITCHBOARD",
        "USERDEFINED",
        "NOTDEFINED",
    ],
    type_class: Some("IFCELECTRICDISTRIBUTIONBOARDTYPE"),
};

/// `IfcElectricFlowStorageDevice`, 11 permitted tokens.
pub const IFCELECTRICFLOWSTORAGEDEVICE: Occurrence = Occurrence {
    type_name: "IFCELECTRICFLOWSTORAGEDEVICE",
    arity: 9,
    predefined_slot: Some(8),
    members: &[
        "BATTERY",
        "CAPACITOR",
        "CAPACITORBANK",
        "COMPENSATOR",
        "HARMONICFILTER",
        "INDUCTOR",
        "INDUCTORBANK",
        "RECHARGER",
        "UPS",
        "USERDEFINED",
        "NOTDEFINED",
    ],
    type_class: Some("IFCELECTRICFLOWSTORAGEDEVICETYPE"),
};

/// `IfcElectricFlowTreatmentDevice`, 3 permitted tokens.
pub const IFCELECTRICFLOWTREATMENTDEVICE: Occurrence = Occurrence {
    type_name: "IFCELECTRICFLOWTREATMENTDEVICE",
    arity: 9,
    predefined_slot: Some(8),
    members: &["ELECTRONICFILTER", "USERDEFINED", "NOTDEFINED"],
    type_class: Some("IFCELECTRICFLOWTREATMENTDEVICETYPE"),
};

/// `IfcElectricGenerator`, 5 permitted tokens.
pub const IFCELECTRICGENERATOR: Occurrence = Occurrence {
    type_name: "IFCELECTRICGENERATOR",
    arity: 9,
    predefined_slot: Some(8),
    members: &[
        "CHP",
        "ENGINEGENERATOR",
        "STANDALONE",
        "USERDEFINED",
        "NOTDEFINED",
    ],
    type_class: Some("IFCELECTRICGENERATORTYPE"),
};

/// `IfcElectricMotor`, 7 permitted tokens.
pub const IFCELECTRICMOTOR: Occurrence = Occurrence {
    type_name: "IFCELECTRICMOTOR",
    arity: 9,
    predefined_slot: Some(8),
    members: &[
        "DC",
        "INDUCTION",
        "POLYPHASE",
        "RELUCTANCESYNCHRONOUS",
        "SYNCHRONOUS",
        "USERDEFINED",
        "NOTDEFINED",
    ],
    type_class: Some("IFCELECTRICMOTORTYPE"),
};

/// `IfcElectricTimeControl`, 5 permitted tokens.
pub const IFCELECTRICTIMECONTROL: Occurrence = Occurrence {
    type_name: "IFCELECTRICTIMECONTROL",
    arity: 9,
    predefined_slot: Some(8),
    members: &[
        "RELAY",
        "TIMECLOCK",
        "TIMEDELAY",
        "USERDEFINED",
        "NOTDEFINED",
    ],
    type_class: Some("IFCELECTRICTIMECONTROLTYPE"),
};

/// `IfcElementAssembly`, 30 permitted tokens.
pub const IFCELEMENTASSEMBLY: Occurrence = Occurrence {
    type_name: "IFCELEMENTASSEMBLY",
    arity: 10,
    predefined_slot: Some(9),
    members: &[
        "ABUTMENT",
        "ACCESSORY_ASSEMBLY",
        "ARCH",
        "BEAM_GRID",
        "BRACED_FRAME",
        "CROSS_BRACING",
        "DECK",
        "DILATATIONPANEL",
        "ENTRANCEWORKS",
        "GIRDER",
        "GRID",
        "MAST",
        "PIER",
        "PYLON",
        "RAIL_MECHANICAL_EQUIPMENT_ASSEMBLY",
        "REINFORCEMENT_UNIT",
        "RIGID_FRAME",
        "SHELTER",
        "SIGNALASSEMBLY",
        "SLAB_FIELD",
        "SUMPBUSTER",
        "SUPPORTINGASSEMBLY",
        "SUSPENSIONASSEMBLY",
        "TRACKPANEL",
        "TRACTION_SWITCHING_ASSEMBLY",
        "TRAFFIC_CALMING_DEVICE",
        "TRUSS",
        "TURNOUTPANEL",
        "USERDEFINED",
        "NOTDEFINED",
    ],
    type_class: Some("IFCELEMENTASSEMBLYTYPE"),
};

/// `IfcEngine`, 4 permitted tokens.
pub const IFCENGINE: Occurrence = Occurrence {
    type_name: "IFCENGINE",
    arity: 9,
    predefined_slot: Some(8),
    members: &[
        "EXTERNALCOMBUSTION",
        "INTERNALCOMBUSTION",
        "USERDEFINED",
        "NOTDEFINED",
    ],
    type_class: Some("IFCENGINETYPE"),
};

/// `IfcEvaporativeCooler`, 11 permitted tokens.
pub const IFCEVAPORATIVECOOLER: Occurrence = Occurrence {
    type_name: "IFCEVAPORATIVECOOLER",
    arity: 9,
    predefined_slot: Some(8),
    members: &[
        "DIRECTEVAPORATIVEAIRWASHER",
        "DIRECTEVAPORATIVEPACKAGEDROTARYAIRCOOLER",
        "DIRECTEVAPORATIVERANDOMMEDIAAIRCOOLER",
        "DIRECTEVAPORATIVERIGIDMEDIAAIRCOOLER",
        "DIRECTEVAPORATIVESLINGERSPACKAGEDAIRCOOLER",
        "INDIRECTDIRECTCOMBINATION",
        "INDIRECTEVAPORATIVECOOLINGTOWERORCOILCOOLER",
        "INDIRECTEVAPORATIVEPACKAGEAIRCOOLER",
        "INDIRECTEVAPORATIVEWETCOIL",
        "USERDEFINED",
        "NOTDEFINED",
    ],
    type_class: Some("IFCEVAPORATIVECOOLERTYPE"),
};
