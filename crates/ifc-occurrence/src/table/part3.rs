//! Generated occurrence rows. Do not edit by hand.

use super::Occurrence;

/// `IfcEvaporator`, 8 permitted tokens.
pub const IFCEVAPORATOR: Occurrence = Occurrence {
    type_name: "IFCEVAPORATOR",
    arity: 9,
    predefined_slot: Some(8),
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
    type_class: Some("IFCEVAPORATORTYPE"),
};

/// `IfcFan`, 9 permitted tokens.
pub const IFCFAN: Occurrence = Occurrence {
    type_name: "IFCFAN",
    arity: 9,
    predefined_slot: Some(8),
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
    type_class: Some("IFCFANTYPE"),
};

/// `IfcFastener`, 5 permitted tokens.
pub const IFCFASTENER: Occurrence = Occurrence {
    type_name: "IFCFASTENER",
    arity: 9,
    predefined_slot: Some(8),
    members: &["GLUE", "MORTAR", "WELD", "USERDEFINED", "NOTDEFINED"],
    type_class: Some("IFCFASTENERTYPE"),
};

/// `IfcFilter`, 8 permitted tokens.
pub const IFCFILTER: Occurrence = Occurrence {
    type_name: "IFCFILTER",
    arity: 9,
    predefined_slot: Some(8),
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
    type_class: Some("IFCFILTERTYPE"),
};

/// `IfcFireSuppressionTerminal`, 8 permitted tokens.
pub const IFCFIRESUPPRESSIONTERMINAL: Occurrence = Occurrence {
    type_name: "IFCFIRESUPPRESSIONTERMINAL",
    arity: 9,
    predefined_slot: Some(8),
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
    type_class: Some("IFCFIRESUPPRESSIONTERMINALTYPE"),
};

/// `IfcFlowInstrument`, 12 permitted tokens.
pub const IFCFLOWINSTRUMENT: Occurrence = Occurrence {
    type_name: "IFCFLOWINSTRUMENT",
    arity: 9,
    predefined_slot: Some(8),
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
    type_class: Some("IFCFLOWINSTRUMENTTYPE"),
};

/// `IfcFlowMeter`, 6 permitted tokens.
pub const IFCFLOWMETER: Occurrence = Occurrence {
    type_name: "IFCFLOWMETER",
    arity: 9,
    predefined_slot: Some(8),
    members: &[
        "ENERGYMETER",
        "GASMETER",
        "OILMETER",
        "WATERMETER",
        "USERDEFINED",
        "NOTDEFINED",
    ],
    type_class: Some("IFCFLOWMETERTYPE"),
};

/// `IfcFooting`, 7 permitted tokens.
pub const IFCFOOTING: Occurrence = Occurrence {
    type_name: "IFCFOOTING",
    arity: 9,
    predefined_slot: Some(8),
    members: &[
        "CAISSON_FOUNDATION",
        "FOOTING_BEAM",
        "PAD_FOOTING",
        "PILE_CAP",
        "STRIP_FOOTING",
        "USERDEFINED",
        "NOTDEFINED",
    ],
    type_class: Some("IFCFOOTINGTYPE"),
};

/// `IfcFurnishingElement`, 0 permitted tokens.
pub const IFCFURNISHINGELEMENT: Occurrence = Occurrence {
    type_name: "IFCFURNISHINGELEMENT",
    arity: 8,
    predefined_slot: None,
    members: &[],
    type_class: None,
};

/// `IfcFurniture`, 10 permitted tokens.
pub const IFCFURNITURE: Occurrence = Occurrence {
    type_name: "IFCFURNITURE",
    arity: 9,
    predefined_slot: Some(8),
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
    type_class: Some("IFCFURNITURETYPE"),
};

/// `IfcGeographicElement`, 5 permitted tokens.
pub const IFCGEOGRAPHICELEMENT: Occurrence = Occurrence {
    type_name: "IFCGEOGRAPHICELEMENT",
    arity: 9,
    predefined_slot: Some(8),
    members: &[
        "SOIL_BORING_POINT",
        "TERRAIN",
        "VEGETATION",
        "USERDEFINED",
        "NOTDEFINED",
    ],
    type_class: Some("IFCGEOGRAPHICELEMENTTYPE"),
};

/// `IfcGeomodel`, 0 permitted tokens.
pub const IFCGEOMODEL: Occurrence = Occurrence {
    type_name: "IFCGEOMODEL",
    arity: 8,
    predefined_slot: None,
    members: &[],
    type_class: None,
};

/// `IfcGeoslice`, 0 permitted tokens.
pub const IFCGEOSLICE: Occurrence = Occurrence {
    type_name: "IFCGEOSLICE",
    arity: 8,
    predefined_slot: None,
    members: &[],
    type_class: None,
};

/// `IfcGeotechnicalStratum`, 5 permitted tokens.
pub const IFCGEOTECHNICALSTRATUM: Occurrence = Occurrence {
    type_name: "IFCGEOTECHNICALSTRATUM",
    arity: 9,
    predefined_slot: Some(8),
    members: &["SOLID", "VOID", "WATER", "USERDEFINED", "NOTDEFINED"],
    type_class: None,
};

/// `IfcHeatExchanger`, 5 permitted tokens.
pub const IFCHEATEXCHANGER: Occurrence = Occurrence {
    type_name: "IFCHEATEXCHANGER",
    arity: 9,
    predefined_slot: Some(8),
    members: &[
        "PLATE",
        "SHELLANDTUBE",
        "TURNOUTHEATING",
        "USERDEFINED",
        "NOTDEFINED",
    ],
    type_class: Some("IFCHEATEXCHANGERTYPE"),
};

/// `IfcHumidifier`, 15 permitted tokens.
pub const IFCHUMIDIFIER: Occurrence = Occurrence {
    type_name: "IFCHUMIDIFIER",
    arity: 9,
    predefined_slot: Some(8),
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
    type_class: Some("IFCHUMIDIFIERTYPE"),
};

/// `IfcImpactProtectionDevice`, 6 permitted tokens.
pub const IFCIMPACTPROTECTIONDEVICE: Occurrence = Occurrence {
    type_name: "IFCIMPACTPROTECTIONDEVICE",
    arity: 9,
    predefined_slot: Some(8),
    members: &[
        "BUMPER",
        "CRASHCUSHION",
        "DAMPINGSYSTEM",
        "FENDER",
        "USERDEFINED",
        "NOTDEFINED",
    ],
    type_class: Some("IFCIMPACTPROTECTIONDEVICETYPE"),
};

/// `IfcInterceptor`, 6 permitted tokens.
pub const IFCINTERCEPTOR: Occurrence = Occurrence {
    type_name: "IFCINTERCEPTOR",
    arity: 9,
    predefined_slot: Some(8),
    members: &[
        "CYCLONIC",
        "GREASE",
        "OIL",
        "PETROL",
        "USERDEFINED",
        "NOTDEFINED",
    ],
    type_class: Some("IFCINTERCEPTORTYPE"),
};

/// `IfcJunctionBox`, 4 permitted tokens.
pub const IFCJUNCTIONBOX: Occurrence = Occurrence {
    type_name: "IFCJUNCTIONBOX",
    arity: 9,
    predefined_slot: Some(8),
    members: &["DATA", "POWER", "USERDEFINED", "NOTDEFINED"],
    type_class: Some("IFCJUNCTIONBOXTYPE"),
};

/// `IfcKerb`, 2 permitted tokens.
pub const IFCKERB: Occurrence = Occurrence {
    type_name: "IFCKERB",
    arity: 9,
    predefined_slot: Some(8),
    members: &["USERDEFINED", "NOTDEFINED"],
    type_class: Some("IFCKERBTYPE"),
};

/// `IfcLamp`, 11 permitted tokens.
pub const IFCLAMP: Occurrence = Occurrence {
    type_name: "IFCLAMP",
    arity: 9,
    predefined_slot: Some(8),
    members: &[
        "COMPACTFLUORESCENT",
        "FLUORESCENT",
        "HALOGEN",
        "HIGHPRESSUREMERCURY",
        "HIGHPRESSURESODIUM",
        "LED",
        "METALHALIDE",
        "OLED",
        "TUNGSTENFILAMENT",
        "USERDEFINED",
        "NOTDEFINED",
    ],
    type_class: Some("IFCLAMPTYPE"),
};

/// `IfcLightFixture`, 5 permitted tokens.
pub const IFCLIGHTFIXTURE: Occurrence = Occurrence {
    type_name: "IFCLIGHTFIXTURE",
    arity: 9,
    predefined_slot: Some(8),
    members: &[
        "DIRECTIONSOURCE",
        "POINTSOURCE",
        "SECURITYLIGHTING",
        "USERDEFINED",
        "NOTDEFINED",
    ],
    type_class: Some("IFCLIGHTFIXTURETYPE"),
};

/// `IfcLiquidTerminal`, 4 permitted tokens.
pub const IFCLIQUIDTERMINAL: Occurrence = Occurrence {
    type_name: "IFCLIQUIDTERMINAL",
    arity: 9,
    predefined_slot: Some(8),
    members: &["HOSEREEL", "LOADINGARM", "USERDEFINED", "NOTDEFINED"],
    type_class: Some("IFCLIQUIDTERMINALTYPE"),
};

/// `IfcMechanicalFastener`, 17 permitted tokens.
pub const IFCMECHANICALFASTENER: Occurrence = Occurrence {
    type_name: "IFCMECHANICALFASTENER",
    arity: 11,
    predefined_slot: Some(10),
    members: &[
        "ANCHORBOLT",
        "BOLT",
        "CHAIN",
        "COUPLER",
        "DOWEL",
        "NAIL",
        "NAILPLATE",
        "RAILFASTENING",
        "RAILJOINT",
        "RIVET",
        "ROPE",
        "SCREW",
        "SHEARCONNECTOR",
        "STAPLE",
        "STUDSHEARCONNECTOR",
        "USERDEFINED",
        "NOTDEFINED",
    ],
    type_class: Some("IFCMECHANICALFASTENERTYPE"),
};

/// `IfcMedicalDevice`, 7 permitted tokens.
pub const IFCMEDICALDEVICE: Occurrence = Occurrence {
    type_name: "IFCMEDICALDEVICE",
    arity: 9,
    predefined_slot: Some(8),
    members: &[
        "AIRSTATION",
        "FEEDAIRUNIT",
        "OXYGENGENERATOR",
        "OXYGENPLANT",
        "VACUUMSTATION",
        "USERDEFINED",
        "NOTDEFINED",
    ],
    type_class: Some("IFCMEDICALDEVICETYPE"),
};

/// `IfcMember`, 21 permitted tokens.
pub const IFCMEMBER: Occurrence = Occurrence {
    type_name: "IFCMEMBER",
    arity: 9,
    predefined_slot: Some(8),
    members: &[
        "ARCH_SEGMENT",
        "BRACE",
        "CHORD",
        "COLLAR",
        "MEMBER",
        "MULLION",
        "PLATE",
        "POST",
        "PURLIN",
        "RAFTER",
        "STAY_CABLE",
        "STIFFENING_RIB",
        "STRINGER",
        "STRUCTURALCABLE",
        "STRUT",
        "STUD",
        "SUSPENDER",
        "SUSPENSION_CABLE",
        "TIEBAR",
        "USERDEFINED",
        "NOTDEFINED",
    ],
    type_class: Some("IFCMEMBERTYPE"),
};

/// `IfcMobileTelecommunicationsAppliance`, 15 permitted tokens.
pub const IFCMOBILETELECOMMUNICATIONSAPPLIANCE: Occurrence = Occurrence {
    type_name: "IFCMOBILETELECOMMUNICATIONSAPPLIANCE",
    arity: 9,
    predefined_slot: Some(8),
    members: &[
        "ACCESSPOINT",
        "BASEBANDUNIT",
        "BASETRANSCEIVERSTATION",
        "E_UTRAN_NODE_B",
        "GATEWAY_GPRS_SUPPORT_NODE",
        "MASTERUNIT",
        "MOBILESWITCHINGCENTER",
        "MSCSERVER",
        "PACKETCONTROLUNIT",
        "REMOTERADIOUNIT",
        "REMOTEUNIT",
        "SERVICE_GPRS_SUPPORT_NODE",
        "SUBSCRIBERSERVER",
        "USERDEFINED",
        "NOTDEFINED",
    ],
    type_class: Some("IFCMOBILETELECOMMUNICATIONSAPPLIANCETYPE"),
};

/// `IfcMooringDevice`, 7 permitted tokens.
pub const IFCMOORINGDEVICE: Occurrence = Occurrence {
    type_name: "IFCMOORINGDEVICE",
    arity: 9,
    predefined_slot: Some(8),
    members: &[
        "BOLLARD",
        "LINETENSIONER",
        "MAGNETICDEVICE",
        "MOORINGHOOKS",
        "VACUUMDEVICE",
        "USERDEFINED",
        "NOTDEFINED",
    ],
    type_class: Some("IFCMOORINGDEVICETYPE"),
};

/// `IfcMotorConnection`, 5 permitted tokens.
pub const IFCMOTORCONNECTION: Occurrence = Occurrence {
    type_name: "IFCMOTORCONNECTION",
    arity: 9,
    predefined_slot: Some(8),
    members: &[
        "BELTDRIVE",
        "COUPLING",
        "DIRECTDRIVE",
        "USERDEFINED",
        "NOTDEFINED",
    ],
    type_class: Some("IFCMOTORCONNECTIONTYPE"),
};
