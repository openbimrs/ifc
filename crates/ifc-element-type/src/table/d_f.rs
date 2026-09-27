//! Element types D-F. Generated; do not edit.

use super::{ElementType, Family};

/// `IfcDamperType`, 13 permitted tokens.
pub const IFCDAMPERTYPE: ElementType = ElementType {
    type_name: "IFCDAMPERTYPE",
    arity: 10,
    predefined_slot: 9,
    predefined_optional: false,
    fallback_attr: "ElementType",
    fallback_slot: 8,
    family: Family::Element,
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
};

/// `IfcDiscreteAccessoryType`, 23 permitted tokens.
pub const IFCDISCRETEACCESSORYTYPE: ElementType = ElementType {
    type_name: "IFCDISCRETEACCESSORYTYPE",
    arity: 10,
    predefined_slot: 9,
    predefined_optional: false,
    fallback_attr: "ElementType",
    fallback_slot: 8,
    family: Family::Element,
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
};

/// `IfcDistributionBoardType`, 8 permitted tokens.
pub const IFCDISTRIBUTIONBOARDTYPE: ElementType = ElementType {
    type_name: "IFCDISTRIBUTIONBOARDTYPE",
    arity: 10,
    predefined_slot: 9,
    predefined_optional: false,
    fallback_attr: "ElementType",
    fallback_slot: 8,
    family: Family::Element,
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
};

/// `IfcDistributionChamberElementType`, 10 permitted tokens.
pub const IFCDISTRIBUTIONCHAMBERELEMENTTYPE: ElementType = ElementType {
    type_name: "IFCDISTRIBUTIONCHAMBERELEMENTTYPE",
    arity: 10,
    predefined_slot: 9,
    predefined_optional: false,
    fallback_attr: "ElementType",
    fallback_slot: 8,
    family: Family::Element,
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
};

/// `IfcDoorType`, 7 permitted tokens.
pub const IFCDOORTYPE: ElementType = ElementType {
    type_name: "IFCDOORTYPE",
    arity: 13,
    predefined_slot: 9,
    predefined_optional: false,
    fallback_attr: "ElementType",
    fallback_slot: 8,
    family: Family::Element,
    members: &[
        "BOOM_BARRIER",
        "DOOR",
        "GATE",
        "TRAPDOOR",
        "TURNSTILE",
        "USERDEFINED",
        "NOTDEFINED",
    ],
};

/// `IfcDuctFittingType`, 9 permitted tokens.
pub const IFCDUCTFITTINGTYPE: ElementType = ElementType {
    type_name: "IFCDUCTFITTINGTYPE",
    arity: 10,
    predefined_slot: 9,
    predefined_optional: false,
    fallback_attr: "ElementType",
    fallback_slot: 8,
    family: Family::Element,
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
};

/// `IfcDuctSegmentType`, 4 permitted tokens.
pub const IFCDUCTSEGMENTTYPE: ElementType = ElementType {
    type_name: "IFCDUCTSEGMENTTYPE",
    arity: 10,
    predefined_slot: 9,
    predefined_optional: false,
    fallback_attr: "ElementType",
    fallback_slot: 8,
    family: Family::Element,
    members: &[
        "FLEXIBLESEGMENT",
        "RIGIDSEGMENT",
        "USERDEFINED",
        "NOTDEFINED",
    ],
};

/// `IfcDuctSilencerType`, 5 permitted tokens.
pub const IFCDUCTSILENCERTYPE: ElementType = ElementType {
    type_name: "IFCDUCTSILENCERTYPE",
    arity: 10,
    predefined_slot: 9,
    predefined_optional: false,
    fallback_attr: "ElementType",
    fallback_slot: 8,
    family: Family::Element,
    members: &[
        "FLATOVAL",
        "RECTANGULAR",
        "ROUND",
        "USERDEFINED",
        "NOTDEFINED",
    ],
};

/// `IfcElectricApplianceType`, 18 permitted tokens.
pub const IFCELECTRICAPPLIANCETYPE: ElementType = ElementType {
    type_name: "IFCELECTRICAPPLIANCETYPE",
    arity: 10,
    predefined_slot: 9,
    predefined_optional: false,
    fallback_attr: "ElementType",
    fallback_slot: 8,
    family: Family::Element,
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
};

/// `IfcElectricDistributionBoardType`, 6 permitted tokens.
pub const IFCELECTRICDISTRIBUTIONBOARDTYPE: ElementType = ElementType {
    type_name: "IFCELECTRICDISTRIBUTIONBOARDTYPE",
    arity: 10,
    predefined_slot: 9,
    predefined_optional: false,
    fallback_attr: "ElementType",
    fallback_slot: 8,
    family: Family::Element,
    members: &[
        "CONSUMERUNIT",
        "DISTRIBUTIONBOARD",
        "MOTORCONTROLCENTRE",
        "SWITCHBOARD",
        "USERDEFINED",
        "NOTDEFINED",
    ],
};

/// `IfcElectricFlowStorageDeviceType`, 11 permitted tokens.
pub const IFCELECTRICFLOWSTORAGEDEVICETYPE: ElementType = ElementType {
    type_name: "IFCELECTRICFLOWSTORAGEDEVICETYPE",
    arity: 10,
    predefined_slot: 9,
    predefined_optional: false,
    fallback_attr: "ElementType",
    fallback_slot: 8,
    family: Family::Element,
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
};

/// `IfcElectricFlowTreatmentDeviceType`, 3 permitted tokens.
pub const IFCELECTRICFLOWTREATMENTDEVICETYPE: ElementType = ElementType {
    type_name: "IFCELECTRICFLOWTREATMENTDEVICETYPE",
    arity: 10,
    predefined_slot: 9,
    predefined_optional: false,
    fallback_attr: "ElementType",
    fallback_slot: 8,
    family: Family::Element,
    members: &["ELECTRONICFILTER", "USERDEFINED", "NOTDEFINED"],
};

/// `IfcElectricGeneratorType`, 5 permitted tokens.
pub const IFCELECTRICGENERATORTYPE: ElementType = ElementType {
    type_name: "IFCELECTRICGENERATORTYPE",
    arity: 10,
    predefined_slot: 9,
    predefined_optional: false,
    fallback_attr: "ElementType",
    fallback_slot: 8,
    family: Family::Element,
    members: &[
        "CHP",
        "ENGINEGENERATOR",
        "STANDALONE",
        "USERDEFINED",
        "NOTDEFINED",
    ],
};

/// `IfcElectricMotorType`, 7 permitted tokens.
pub const IFCELECTRICMOTORTYPE: ElementType = ElementType {
    type_name: "IFCELECTRICMOTORTYPE",
    arity: 10,
    predefined_slot: 9,
    predefined_optional: false,
    fallback_attr: "ElementType",
    fallback_slot: 8,
    family: Family::Element,
    members: &[
        "DC",
        "INDUCTION",
        "POLYPHASE",
        "RELUCTANCESYNCHRONOUS",
        "SYNCHRONOUS",
        "USERDEFINED",
        "NOTDEFINED",
    ],
};

/// `IfcElectricTimeControlType`, 5 permitted tokens.
pub const IFCELECTRICTIMECONTROLTYPE: ElementType = ElementType {
    type_name: "IFCELECTRICTIMECONTROLTYPE",
    arity: 10,
    predefined_slot: 9,
    predefined_optional: false,
    fallback_attr: "ElementType",
    fallback_slot: 8,
    family: Family::Element,
    members: &[
        "RELAY",
        "TIMECLOCK",
        "TIMEDELAY",
        "USERDEFINED",
        "NOTDEFINED",
    ],
};

/// `IfcElementAssemblyType`, 30 permitted tokens.
pub const IFCELEMENTASSEMBLYTYPE: ElementType = ElementType {
    type_name: "IFCELEMENTASSEMBLYTYPE",
    arity: 10,
    predefined_slot: 9,
    predefined_optional: false,
    fallback_attr: "ElementType",
    fallback_slot: 8,
    family: Family::Element,
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
};

/// `IfcEngineType`, 4 permitted tokens.
pub const IFCENGINETYPE: ElementType = ElementType {
    type_name: "IFCENGINETYPE",
    arity: 10,
    predefined_slot: 9,
    predefined_optional: false,
    fallback_attr: "ElementType",
    fallback_slot: 8,
    family: Family::Element,
    members: &[
        "EXTERNALCOMBUSTION",
        "INTERNALCOMBUSTION",
        "USERDEFINED",
        "NOTDEFINED",
    ],
};

/// `IfcEvaporativeCoolerType`, 11 permitted tokens.
pub const IFCEVAPORATIVECOOLERTYPE: ElementType = ElementType {
    type_name: "IFCEVAPORATIVECOOLERTYPE",
    arity: 10,
    predefined_slot: 9,
    predefined_optional: false,
    fallback_attr: "ElementType",
    fallback_slot: 8,
    family: Family::Element,
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
};

/// `IfcEvaporatorType`, 8 permitted tokens.
pub const IFCEVAPORATORTYPE: ElementType = ElementType {
    type_name: "IFCEVAPORATORTYPE",
    arity: 10,
    predefined_slot: 9,
    predefined_optional: false,
    fallback_attr: "ElementType",
    fallback_slot: 8,
    family: Family::Element,
    members: &[
        "DIRECTEXPANSION",
        "DIRECTEXPANSIONBRAZEDPLATE",
        "DIRECTEXPANSIONSHELLANDTUBE",
        "DIRECTEXPANSIONTUBEINTUBE",
        "FLOODEDSHELLANDTUBE",
        "SHELLANDCOIL",
        "USERDEFINED",
        "NOTDEFINED",
    ],
};

/// `IfcEventType`, 5 permitted tokens.
pub const IFCEVENTTYPE: ElementType = ElementType {
    type_name: "IFCEVENTTYPE",
    arity: 12,
    predefined_slot: 9,
    predefined_optional: false,
    fallback_attr: "ProcessType",
    fallback_slot: 8,
    family: Family::ResourceOrProcess,
    members: &[
        "ENDEVENT",
        "INTERMEDIATEEVENT",
        "STARTEVENT",
        "USERDEFINED",
        "NOTDEFINED",
    ],
};

/// `IfcFanType`, 9 permitted tokens.
pub const IFCFANTYPE: ElementType = ElementType {
    type_name: "IFCFANTYPE",
    arity: 10,
    predefined_slot: 9,
    predefined_optional: false,
    fallback_attr: "ElementType",
    fallback_slot: 8,
    family: Family::Element,
    members: &[
        "CENTRIFUGALAIRFOIL",
        "CENTRIFUGALBACKWARDINCLINEDCURVED",
        "CENTRIFUGALFORWARDCURVED",
        "CENTRIFUGALRADIAL",
        "PROPELLORAXIAL",
        "TUBEAXIAL",
        "VANEAXIAL",
        "USERDEFINED",
        "NOTDEFINED",
    ],
};

/// `IfcFastenerType`, 5 permitted tokens.
pub const IFCFASTENERTYPE: ElementType = ElementType {
    type_name: "IFCFASTENERTYPE",
    arity: 10,
    predefined_slot: 9,
    predefined_optional: false,
    fallback_attr: "ElementType",
    fallback_slot: 8,
    family: Family::Element,
    members: &["GLUE", "MORTAR", "WELD", "USERDEFINED", "NOTDEFINED"],
};

/// `IfcFilterType`, 8 permitted tokens.
pub const IFCFILTERTYPE: ElementType = ElementType {
    type_name: "IFCFILTERTYPE",
    arity: 10,
    predefined_slot: 9,
    predefined_optional: false,
    fallback_attr: "ElementType",
    fallback_slot: 8,
    family: Family::Element,
    members: &[
        "AIRPARTICLEFILTER",
        "COMPRESSEDAIRFILTER",
        "ODORFILTER",
        "OILFILTER",
        "STRAINER",
        "WATERFILTER",
        "USERDEFINED",
        "NOTDEFINED",
    ],
};

/// `IfcFireSuppressionTerminalType`, 8 permitted tokens.
pub const IFCFIRESUPPRESSIONTERMINALTYPE: ElementType = ElementType {
    type_name: "IFCFIRESUPPRESSIONTERMINALTYPE",
    arity: 10,
    predefined_slot: 9,
    predefined_optional: false,
    fallback_attr: "ElementType",
    fallback_slot: 8,
    family: Family::Element,
    members: &[
        "BREECHINGINLET",
        "FIREHYDRANT",
        "FIREMONITOR",
        "HOSEREEL",
        "SPRINKLER",
        "SPRINKLERDEFLECTOR",
        "USERDEFINED",
        "NOTDEFINED",
    ],
};

/// `IfcFlowInstrumentType`, 12 permitted tokens.
pub const IFCFLOWINSTRUMENTTYPE: ElementType = ElementType {
    type_name: "IFCFLOWINSTRUMENTTYPE",
    arity: 10,
    predefined_slot: 9,
    predefined_optional: false,
    fallback_attr: "ElementType",
    fallback_slot: 8,
    family: Family::Element,
    members: &[
        "AMMETER",
        "COMBINED",
        "FREQUENCYMETER",
        "PHASEANGLEMETER",
        "POWERFACTORMETER",
        "PRESSUREGAUGE",
        "THERMOMETER",
        "VOLTMETER",
        "VOLTMETER_PEAK",
        "VOLTMETER_RMS",
        "USERDEFINED",
        "NOTDEFINED",
    ],
};

/// `IfcFlowMeterType`, 6 permitted tokens.
pub const IFCFLOWMETERTYPE: ElementType = ElementType {
    type_name: "IFCFLOWMETERTYPE",
    arity: 10,
    predefined_slot: 9,
    predefined_optional: false,
    fallback_attr: "ElementType",
    fallback_slot: 8,
    family: Family::Element,
    members: &[
        "ENERGYMETER",
        "GASMETER",
        "OILMETER",
        "WATERMETER",
        "USERDEFINED",
        "NOTDEFINED",
    ],
};

/// `IfcFootingType`, 7 permitted tokens.
pub const IFCFOOTINGTYPE: ElementType = ElementType {
    type_name: "IFCFOOTINGTYPE",
    arity: 10,
    predefined_slot: 9,
    predefined_optional: false,
    fallback_attr: "ElementType",
    fallback_slot: 8,
    family: Family::Element,
    members: &[
        "CAISSON_FOUNDATION",
        "FOOTING_BEAM",
        "PAD_FOOTING",
        "PILE_CAP",
        "STRIP_FOOTING",
        "USERDEFINED",
        "NOTDEFINED",
    ],
};

/// `IfcFurnitureType`, 10 permitted tokens.
pub const IFCFURNITURETYPE: ElementType = ElementType {
    type_name: "IFCFURNITURETYPE",
    arity: 11,
    predefined_slot: 10,
    predefined_optional: true,
    fallback_attr: "ElementType",
    fallback_slot: 8,
    family: Family::Element,
    members: &[
        "BED",
        "CHAIR",
        "DESK",
        "FILECABINET",
        "SHELF",
        "SOFA",
        "TABLE",
        "TECHNICALCABINET",
        "USERDEFINED",
        "NOTDEFINED",
    ],
};
