//! Element types A-C. Generated; do not edit.

use super::{ElementType, Family};

/// `IfcActuatorType`, 7 permitted tokens.
pub const IFCACTUATORTYPE: ElementType = ElementType {
    type_name: "IFCACTUATORTYPE",
    arity: 10,
    predefined_slot: 9,
    predefined_optional: false,
    fallback_attr: "ElementType",
    fallback_slot: 8,
    family: Family::Element,
    members: &[
        "ELECTRICACTUATOR",
        "HANDOPERATEDACTUATOR",
        "HYDRAULICACTUATOR",
        "PNEUMATICACTUATOR",
        "THERMOSTATICACTUATOR",
        "USERDEFINED",
        "NOTDEFINED",
    ],
};

/// `IfcAirTerminalBoxType`, 5 permitted tokens.
pub const IFCAIRTERMINALBOXTYPE: ElementType = ElementType {
    type_name: "IFCAIRTERMINALBOXTYPE",
    arity: 10,
    predefined_slot: 9,
    predefined_optional: false,
    fallback_attr: "ElementType",
    fallback_slot: 8,
    family: Family::Element,
    members: &[
        "CONSTANTFLOW",
        "VARIABLEFLOWPRESSUREDEPENDANT",
        "VARIABLEFLOWPRESSUREINDEPENDANT",
        "USERDEFINED",
        "NOTDEFINED",
    ],
};

/// `IfcAirTerminalType`, 6 permitted tokens.
pub const IFCAIRTERMINALTYPE: ElementType = ElementType {
    type_name: "IFCAIRTERMINALTYPE",
    arity: 10,
    predefined_slot: 9,
    predefined_optional: false,
    fallback_attr: "ElementType",
    fallback_slot: 8,
    family: Family::Element,
    members: &[
        "DIFFUSER",
        "GRILLE",
        "LOUVRE",
        "REGISTER",
        "USERDEFINED",
        "NOTDEFINED",
    ],
};

/// `IfcAirToAirHeatRecoveryType`, 11 permitted tokens.
pub const IFCAIRTOAIRHEATRECOVERYTYPE: ElementType = ElementType {
    type_name: "IFCAIRTOAIRHEATRECOVERYTYPE",
    arity: 10,
    predefined_slot: 9,
    predefined_optional: false,
    fallback_attr: "ElementType",
    fallback_slot: 8,
    family: Family::Element,
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
};

/// `IfcAlarmType`, 10 permitted tokens.
pub const IFCALARMTYPE: ElementType = ElementType {
    type_name: "IFCALARMTYPE",
    arity: 10,
    predefined_slot: 9,
    predefined_optional: false,
    fallback_attr: "ElementType",
    fallback_slot: 8,
    family: Family::Element,
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
};

/// `IfcAudioVisualApplianceType`, 15 permitted tokens.
pub const IFCAUDIOVISUALAPPLIANCETYPE: ElementType = ElementType {
    type_name: "IFCAUDIOVISUALAPPLIANCETYPE",
    arity: 10,
    predefined_slot: 9,
    predefined_optional: false,
    fallback_attr: "ElementType",
    fallback_slot: 8,
    family: Family::Element,
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
};

/// `IfcBeamType`, 14 permitted tokens.
pub const IFCBEAMTYPE: ElementType = ElementType {
    type_name: "IFCBEAMTYPE",
    arity: 10,
    predefined_slot: 9,
    predefined_optional: false,
    fallback_attr: "ElementType",
    fallback_slot: 8,
    family: Family::Element,
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
};

/// `IfcBearingType`, 10 permitted tokens.
pub const IFCBEARINGTYPE: ElementType = ElementType {
    type_name: "IFCBEARINGTYPE",
    arity: 10,
    predefined_slot: 9,
    predefined_optional: false,
    fallback_attr: "ElementType",
    fallback_slot: 8,
    family: Family::Element,
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
};

/// `IfcBoilerType`, 4 permitted tokens.
pub const IFCBOILERTYPE: ElementType = ElementType {
    type_name: "IFCBOILERTYPE",
    arity: 10,
    predefined_slot: 9,
    predefined_optional: false,
    fallback_attr: "ElementType",
    fallback_slot: 8,
    family: Family::Element,
    members: &["STEAM", "WATER", "USERDEFINED", "NOTDEFINED"],
};

/// `IfcBuildingElementPartType`, 7 permitted tokens.
pub const IFCBUILDINGELEMENTPARTTYPE: ElementType = ElementType {
    type_name: "IFCBUILDINGELEMENTPARTTYPE",
    arity: 10,
    predefined_slot: 9,
    predefined_optional: false,
    fallback_attr: "ElementType",
    fallback_slot: 8,
    family: Family::Element,
    members: &[
        "APRON",
        "ARMOURUNIT",
        "INSULATION",
        "PRECASTPANEL",
        "SAFETYCAGE",
        "USERDEFINED",
        "NOTDEFINED",
    ],
};

/// `IfcBuildingElementProxyType`, 7 permitted tokens.
pub const IFCBUILDINGELEMENTPROXYTYPE: ElementType = ElementType {
    type_name: "IFCBUILDINGELEMENTPROXYTYPE",
    arity: 10,
    predefined_slot: 9,
    predefined_optional: false,
    fallback_attr: "ElementType",
    fallback_slot: 8,
    family: Family::Element,
    members: &[
        "COMPLEX",
        "ELEMENT",
        "PARTIAL",
        "PROVISIONFORSPACE",
        "PROVISIONFORVOID",
        "USERDEFINED",
        "NOTDEFINED",
    ],
};

/// `IfcBurnerType`, 2 permitted tokens.
pub const IFCBURNERTYPE: ElementType = ElementType {
    type_name: "IFCBURNERTYPE",
    arity: 10,
    predefined_slot: 9,
    predefined_optional: false,
    fallback_attr: "ElementType",
    fallback_slot: 8,
    family: Family::Element,
    members: &["USERDEFINED", "NOTDEFINED"],
};

/// `IfcCableCarrierFittingType`, 9 permitted tokens.
pub const IFCCABLECARRIERFITTINGTYPE: ElementType = ElementType {
    type_name: "IFCCABLECARRIERFITTINGTYPE",
    arity: 10,
    predefined_slot: 9,
    predefined_optional: false,
    fallback_attr: "ElementType",
    fallback_slot: 8,
    family: Family::Element,
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
};

/// `IfcCableCarrierSegmentType`, 9 permitted tokens.
pub const IFCCABLECARRIERSEGMENTTYPE: ElementType = ElementType {
    type_name: "IFCCABLECARRIERSEGMENTTYPE",
    arity: 10,
    predefined_slot: 9,
    predefined_optional: false,
    fallback_attr: "ElementType",
    fallback_slot: 8,
    family: Family::Element,
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
};

/// `IfcCableFittingType`, 8 permitted tokens.
pub const IFCCABLEFITTINGTYPE: ElementType = ElementType {
    type_name: "IFCCABLEFITTINGTYPE",
    arity: 10,
    predefined_slot: 9,
    predefined_optional: false,
    fallback_attr: "ElementType",
    fallback_slot: 8,
    family: Family::Element,
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
};

/// `IfcCableSegmentType`, 12 permitted tokens.
pub const IFCCABLESEGMENTTYPE: ElementType = ElementType {
    type_name: "IFCCABLESEGMENTTYPE",
    arity: 10,
    predefined_slot: 9,
    predefined_optional: false,
    fallback_attr: "ElementType",
    fallback_slot: 8,
    family: Family::Element,
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
};

/// `IfcCaissonFoundationType`, 4 permitted tokens.
pub const IFCCAISSONFOUNDATIONTYPE: ElementType = ElementType {
    type_name: "IFCCAISSONFOUNDATIONTYPE",
    arity: 10,
    predefined_slot: 9,
    predefined_optional: false,
    fallback_attr: "ElementType",
    fallback_slot: 8,
    family: Family::Element,
    members: &["CAISSON", "WELL", "USERDEFINED", "NOTDEFINED"],
};

/// `IfcChillerType`, 5 permitted tokens.
pub const IFCCHILLERTYPE: ElementType = ElementType {
    type_name: "IFCCHILLERTYPE",
    arity: 10,
    predefined_slot: 9,
    predefined_optional: false,
    fallback_attr: "ElementType",
    fallback_slot: 8,
    family: Family::Element,
    members: &[
        "AIRCOOLED",
        "HEATRECOVERY",
        "WATERCOOLED",
        "USERDEFINED",
        "NOTDEFINED",
    ],
};

/// `IfcChimneyType`, 2 permitted tokens.
pub const IFCCHIMNEYTYPE: ElementType = ElementType {
    type_name: "IFCCHIMNEYTYPE",
    arity: 10,
    predefined_slot: 9,
    predefined_optional: false,
    fallback_attr: "ElementType",
    fallback_slot: 8,
    family: Family::Element,
    members: &["USERDEFINED", "NOTDEFINED"],
};

/// `IfcCoilType`, 9 permitted tokens.
pub const IFCCOILTYPE: ElementType = ElementType {
    type_name: "IFCCOILTYPE",
    arity: 10,
    predefined_slot: 9,
    predefined_optional: false,
    fallback_attr: "ElementType",
    fallback_slot: 8,
    family: Family::Element,
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
};

/// `IfcColumnType`, 7 permitted tokens.
pub const IFCCOLUMNTYPE: ElementType = ElementType {
    type_name: "IFCCOLUMNTYPE",
    arity: 10,
    predefined_slot: 9,
    predefined_optional: false,
    fallback_attr: "ElementType",
    fallback_slot: 8,
    family: Family::Element,
    members: &[
        "COLUMN",
        "PIERSTEM",
        "PIERSTEM_SEGMENT",
        "PILASTER",
        "STANDCOLUMN",
        "USERDEFINED",
        "NOTDEFINED",
    ],
};

/// `IfcCommunicationsApplianceType`, 26 permitted tokens.
pub const IFCCOMMUNICATIONSAPPLIANCETYPE: ElementType = ElementType {
    type_name: "IFCCOMMUNICATIONSAPPLIANCETYPE",
    arity: 10,
    predefined_slot: 9,
    predefined_optional: false,
    fallback_attr: "ElementType",
    fallback_slot: 8,
    family: Family::Element,
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
};

/// `IfcCompressorType`, 17 permitted tokens.
pub const IFCCOMPRESSORTYPE: ElementType = ElementType {
    type_name: "IFCCOMPRESSORTYPE",
    arity: 10,
    predefined_slot: 9,
    predefined_optional: false,
    fallback_attr: "ElementType",
    fallback_slot: 8,
    family: Family::Element,
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
};

/// `IfcCondenserType`, 9 permitted tokens.
pub const IFCCONDENSERTYPE: ElementType = ElementType {
    type_name: "IFCCONDENSERTYPE",
    arity: 10,
    predefined_slot: 9,
    predefined_optional: false,
    fallback_attr: "ElementType",
    fallback_slot: 8,
    family: Family::Element,
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
};

/// `IfcConstructionEquipmentResourceType`, 10 permitted tokens.
pub const IFCCONSTRUCTIONEQUIPMENTRESOURCETYPE: ElementType = ElementType {
    type_name: "IFCCONSTRUCTIONEQUIPMENTRESOURCETYPE",
    arity: 12,
    predefined_slot: 11,
    predefined_optional: false,
    fallback_attr: "ResourceType",
    fallback_slot: 8,
    family: Family::ResourceOrProcess,
    members: &[
        "DEMOLISHING",
        "EARTHMOVING",
        "ERECTING",
        "HEATING",
        "LIGHTING",
        "PAVING",
        "PUMPING",
        "TRANSPORTING",
        "USERDEFINED",
        "NOTDEFINED",
    ],
};

/// `IfcConstructionMaterialResourceType`, 11 permitted tokens.
pub const IFCCONSTRUCTIONMATERIALRESOURCETYPE: ElementType = ElementType {
    type_name: "IFCCONSTRUCTIONMATERIALRESOURCETYPE",
    arity: 12,
    predefined_slot: 11,
    predefined_optional: false,
    fallback_attr: "ResourceType",
    fallback_slot: 8,
    family: Family::ResourceOrProcess,
    members: &[
        "AGGREGATES",
        "CONCRETE",
        "DRYWALL",
        "FUEL",
        "GYPSUM",
        "MASONRY",
        "METAL",
        "PLASTIC",
        "WOOD",
        "USERDEFINED",
        "NOTDEFINED",
    ],
};

/// `IfcConstructionProductResourceType`, 4 permitted tokens.
pub const IFCCONSTRUCTIONPRODUCTRESOURCETYPE: ElementType = ElementType {
    type_name: "IFCCONSTRUCTIONPRODUCTRESOURCETYPE",
    arity: 12,
    predefined_slot: 11,
    predefined_optional: false,
    fallback_attr: "ResourceType",
    fallback_slot: 8,
    family: Family::ResourceOrProcess,
    members: &["ASSEMBLY", "FORMWORK", "USERDEFINED", "NOTDEFINED"],
};

/// `IfcControllerType`, 7 permitted tokens.
pub const IFCCONTROLLERTYPE: ElementType = ElementType {
    type_name: "IFCCONTROLLERTYPE",
    arity: 10,
    predefined_slot: 9,
    predefined_optional: false,
    fallback_attr: "ElementType",
    fallback_slot: 8,
    family: Family::Element,
    members: &[
        "FLOATING",
        "MULTIPOSITION",
        "PROGRAMMABLE",
        "PROPORTIONAL",
        "TWOPOSITION",
        "USERDEFINED",
        "NOTDEFINED",
    ],
};

/// `IfcConveyorSegmentType`, 6 permitted tokens.
pub const IFCCONVEYORSEGMENTTYPE: ElementType = ElementType {
    type_name: "IFCCONVEYORSEGMENTTYPE",
    arity: 10,
    predefined_slot: 9,
    predefined_optional: false,
    fallback_attr: "ElementType",
    fallback_slot: 8,
    family: Family::Element,
    members: &[
        "BELTCONVEYOR",
        "BUCKETCONVEYOR",
        "CHUTECONVEYOR",
        "SCREWCONVEYOR",
        "USERDEFINED",
        "NOTDEFINED",
    ],
};

/// `IfcCooledBeamType`, 4 permitted tokens.
pub const IFCCOOLEDBEAMTYPE: ElementType = ElementType {
    type_name: "IFCCOOLEDBEAMTYPE",
    arity: 10,
    predefined_slot: 9,
    predefined_optional: false,
    fallback_attr: "ElementType",
    fallback_slot: 8,
    family: Family::Element,
    members: &["ACTIVE", "PASSIVE", "USERDEFINED", "NOTDEFINED"],
};

/// `IfcCoolingTowerType`, 5 permitted tokens.
pub const IFCCOOLINGTOWERTYPE: ElementType = ElementType {
    type_name: "IFCCOOLINGTOWERTYPE",
    arity: 10,
    predefined_slot: 9,
    predefined_optional: false,
    fallback_attr: "ElementType",
    fallback_slot: 8,
    family: Family::Element,
    members: &[
        "MECHANICALFORCEDDRAFT",
        "MECHANICALINDUCEDDRAFT",
        "NATURALDRAFT",
        "USERDEFINED",
        "NOTDEFINED",
    ],
};

/// `IfcCourseType`, 8 permitted tokens.
pub const IFCCOURSETYPE: ElementType = ElementType {
    type_name: "IFCCOURSETYPE",
    arity: 10,
    predefined_slot: 9,
    predefined_optional: false,
    fallback_attr: "ElementType",
    fallback_slot: 8,
    family: Family::Element,
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
};

/// `IfcCoveringType`, 14 permitted tokens.
pub const IFCCOVERINGTYPE: ElementType = ElementType {
    type_name: "IFCCOVERINGTYPE",
    arity: 10,
    predefined_slot: 9,
    predefined_optional: false,
    fallback_attr: "ElementType",
    fallback_slot: 8,
    family: Family::Element,
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
};

/// `IfcCrewResourceType`, 4 permitted tokens.
pub const IFCCREWRESOURCETYPE: ElementType = ElementType {
    type_name: "IFCCREWRESOURCETYPE",
    arity: 12,
    predefined_slot: 11,
    predefined_optional: false,
    fallback_attr: "ResourceType",
    fallback_slot: 8,
    family: Family::ResourceOrProcess,
    members: &["OFFICE", "SITE", "USERDEFINED", "NOTDEFINED"],
};

/// `IfcCurtainWallType`, 2 permitted tokens.
pub const IFCCURTAINWALLTYPE: ElementType = ElementType {
    type_name: "IFCCURTAINWALLTYPE",
    arity: 10,
    predefined_slot: 9,
    predefined_optional: false,
    fallback_attr: "ElementType",
    fallback_slot: 8,
    family: Family::Element,
    members: &["USERDEFINED", "NOTDEFINED"],
};
