//! Element types L-P. Generated; do not edit.

use super::{ElementType, Family};

/// `IfcLaborResourceType`, 21 permitted tokens.
pub const IFCLABORRESOURCETYPE: ElementType = ElementType {
    type_name: "IFCLABORRESOURCETYPE",
    arity: 12,
    predefined_slot: 11,
    predefined_optional: false,
    fallback_attr: "ResourceType",
    fallback_slot: 8,
    family: Family::ResourceOrProcess,
    members: &[
        "ADMINISTRATION",
        "CARPENTRY",
        "CLEANING",
        "CONCRETE",
        "DRYWALL",
        "ELECTRIC",
        "FINISHING",
        "FLOORING",
        "GENERAL",
        "HVAC",
        "LANDSCAPING",
        "MASONRY",
        "PAINTING",
        "PAVING",
        "PLUMBING",
        "ROOFING",
        "SITEGRADING",
        "STEELWORK",
        "SURVEYING",
        "USERDEFINED",
        "NOTDEFINED",
    ],
};

/// `IfcLampType`, 11 permitted tokens.
pub const IFCLAMPTYPE: ElementType = ElementType {
    type_name: "IFCLAMPTYPE",
    arity: 10,
    predefined_slot: 9,
    predefined_optional: false,
    fallback_attr: "ElementType",
    fallback_slot: 8,
    family: Family::Element,
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
};

/// `IfcLightFixtureType`, 5 permitted tokens.
pub const IFCLIGHTFIXTURETYPE: ElementType = ElementType {
    type_name: "IFCLIGHTFIXTURETYPE",
    arity: 10,
    predefined_slot: 9,
    predefined_optional: false,
    fallback_attr: "ElementType",
    fallback_slot: 8,
    family: Family::Element,
    members: &[
        "DIRECTIONSOURCE",
        "POINTSOURCE",
        "SECURITYLIGHTING",
        "USERDEFINED",
        "NOTDEFINED",
    ],
};

/// `IfcLiquidTerminalType`, 4 permitted tokens.
pub const IFCLIQUIDTERMINALTYPE: ElementType = ElementType {
    type_name: "IFCLIQUIDTERMINALTYPE",
    arity: 10,
    predefined_slot: 9,
    predefined_optional: false,
    fallback_attr: "ElementType",
    fallback_slot: 8,
    family: Family::Element,
    members: &["HOSEREEL", "LOADINGARM", "USERDEFINED", "NOTDEFINED"],
};

/// `IfcMechanicalFastenerType`, 17 permitted tokens.
pub const IFCMECHANICALFASTENERTYPE: ElementType = ElementType {
    type_name: "IFCMECHANICALFASTENERTYPE",
    arity: 12,
    predefined_slot: 9,
    predefined_optional: false,
    fallback_attr: "ElementType",
    fallback_slot: 8,
    family: Family::Element,
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
};

/// `IfcMedicalDeviceType`, 7 permitted tokens.
pub const IFCMEDICALDEVICETYPE: ElementType = ElementType {
    type_name: "IFCMEDICALDEVICETYPE",
    arity: 10,
    predefined_slot: 9,
    predefined_optional: false,
    fallback_attr: "ElementType",
    fallback_slot: 8,
    family: Family::Element,
    members: &[
        "AIRSTATION",
        "FEEDAIRUNIT",
        "OXYGENGENERATOR",
        "OXYGENPLANT",
        "VACUUMSTATION",
        "USERDEFINED",
        "NOTDEFINED",
    ],
};

/// `IfcMemberType`, 21 permitted tokens.
pub const IFCMEMBERTYPE: ElementType = ElementType {
    type_name: "IFCMEMBERTYPE",
    arity: 10,
    predefined_slot: 9,
    predefined_optional: false,
    fallback_attr: "ElementType",
    fallback_slot: 8,
    family: Family::Element,
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
};

/// `IfcMobileTelecommunicationsApplianceType`, 15 permitted tokens.
pub const IFCMOBILETELECOMMUNICATIONSAPPLIANCETYPE: ElementType = ElementType {
    type_name: "IFCMOBILETELECOMMUNICATIONSAPPLIANCETYPE",
    arity: 10,
    predefined_slot: 9,
    predefined_optional: false,
    fallback_attr: "ElementType",
    fallback_slot: 8,
    family: Family::Element,
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
};

/// `IfcMooringDeviceType`, 7 permitted tokens.
pub const IFCMOORINGDEVICETYPE: ElementType = ElementType {
    type_name: "IFCMOORINGDEVICETYPE",
    arity: 10,
    predefined_slot: 9,
    predefined_optional: false,
    fallback_attr: "ElementType",
    fallback_slot: 8,
    family: Family::Element,
    members: &[
        "BOLLARD",
        "LINETENSIONER",
        "MAGNETICDEVICE",
        "MOORINGHOOKS",
        "VACUUMDEVICE",
        "USERDEFINED",
        "NOTDEFINED",
    ],
};

/// `IfcMotorConnectionType`, 5 permitted tokens.
pub const IFCMOTORCONNECTIONTYPE: ElementType = ElementType {
    type_name: "IFCMOTORCONNECTIONTYPE",
    arity: 10,
    predefined_slot: 9,
    predefined_optional: false,
    fallback_attr: "ElementType",
    fallback_slot: 8,
    family: Family::Element,
    members: &[
        "BELTDRIVE",
        "COUPLING",
        "DIRECTDRIVE",
        "USERDEFINED",
        "NOTDEFINED",
    ],
};

/// `IfcNavigationElementType`, 4 permitted tokens.
pub const IFCNAVIGATIONELEMENTTYPE: ElementType = ElementType {
    type_name: "IFCNAVIGATIONELEMENTTYPE",
    arity: 10,
    predefined_slot: 9,
    predefined_optional: false,
    fallback_attr: "ElementType",
    fallback_slot: 8,
    family: Family::Element,
    members: &["BEACON", "BUOY", "USERDEFINED", "NOTDEFINED"],
};

/// `IfcOutletType`, 7 permitted tokens.
pub const IFCOUTLETTYPE: ElementType = ElementType {
    type_name: "IFCOUTLETTYPE",
    arity: 10,
    predefined_slot: 9,
    predefined_optional: false,
    fallback_attr: "ElementType",
    fallback_slot: 8,
    family: Family::Element,
    members: &[
        "AUDIOVISUALOUTLET",
        "COMMUNICATIONSOUTLET",
        "DATAOUTLET",
        "POWEROUTLET",
        "TELEPHONEOUTLET",
        "USERDEFINED",
        "NOTDEFINED",
    ],
};

/// `IfcPavementType`, 4 permitted tokens.
pub const IFCPAVEMENTTYPE: ElementType = ElementType {
    type_name: "IFCPAVEMENTTYPE",
    arity: 10,
    predefined_slot: 9,
    predefined_optional: false,
    fallback_attr: "ElementType",
    fallback_slot: 8,
    family: Family::Element,
    members: &["FLEXIBLE", "RIGID", "USERDEFINED", "NOTDEFINED"],
};

/// `IfcPileType`, 8 permitted tokens.
pub const IFCPILETYPE: ElementType = ElementType {
    type_name: "IFCPILETYPE",
    arity: 10,
    predefined_slot: 9,
    predefined_optional: false,
    fallback_attr: "ElementType",
    fallback_slot: 8,
    family: Family::Element,
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
};

/// `IfcPipeFittingType`, 9 permitted tokens.
pub const IFCPIPEFITTINGTYPE: ElementType = ElementType {
    type_name: "IFCPIPEFITTINGTYPE",
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

/// `IfcPipeSegmentType`, 7 permitted tokens.
pub const IFCPIPESEGMENTTYPE: ElementType = ElementType {
    type_name: "IFCPIPESEGMENTTYPE",
    arity: 10,
    predefined_slot: 9,
    predefined_optional: false,
    fallback_attr: "ElementType",
    fallback_slot: 8,
    family: Family::Element,
    members: &[
        "CULVERT",
        "FLEXIBLESEGMENT",
        "GUTTER",
        "RIGIDSEGMENT",
        "SPOOL",
        "USERDEFINED",
        "NOTDEFINED",
    ],
};

/// `IfcPlateType`, 11 permitted tokens.
pub const IFCPLATETYPE: ElementType = ElementType {
    type_name: "IFCPLATETYPE",
    arity: 10,
    predefined_slot: 9,
    predefined_optional: false,
    fallback_attr: "ElementType",
    fallback_slot: 8,
    family: Family::Element,
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
};

/// `IfcProcedureType`, 9 permitted tokens.
pub const IFCPROCEDURETYPE: ElementType = ElementType {
    type_name: "IFCPROCEDURETYPE",
    arity: 10,
    predefined_slot: 9,
    predefined_optional: false,
    fallback_attr: "ProcessType",
    fallback_slot: 8,
    family: Family::ResourceOrProcess,
    members: &[
        "ADVICE_CAUTION",
        "ADVICE_NOTE",
        "ADVICE_WARNING",
        "CALIBRATION",
        "DIAGNOSTIC",
        "SHUTDOWN",
        "STARTUP",
        "USERDEFINED",
        "NOTDEFINED",
    ],
};

/// `IfcProtectiveDeviceTrippingUnitType`, 6 permitted tokens.
pub const IFCPROTECTIVEDEVICETRIPPINGUNITTYPE: ElementType = ElementType {
    type_name: "IFCPROTECTIVEDEVICETRIPPINGUNITTYPE",
    arity: 10,
    predefined_slot: 9,
    predefined_optional: false,
    fallback_attr: "ElementType",
    fallback_slot: 8,
    family: Family::Element,
    members: &[
        "ELECTROMAGNETIC",
        "ELECTRONIC",
        "RESIDUALCURRENT",
        "THERMAL",
        "USERDEFINED",
        "NOTDEFINED",
    ],
};

/// `IfcProtectiveDeviceType`, 12 permitted tokens.
pub const IFCPROTECTIVEDEVICETYPE: ElementType = ElementType {
    type_name: "IFCPROTECTIVEDEVICETYPE",
    arity: 10,
    predefined_slot: 9,
    predefined_optional: false,
    fallback_attr: "ElementType",
    fallback_slot: 8,
    family: Family::Element,
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
};

/// `IfcPumpType`, 9 permitted tokens.
pub const IFCPUMPTYPE: ElementType = ElementType {
    type_name: "IFCPUMPTYPE",
    arity: 10,
    predefined_slot: 9,
    predefined_optional: false,
    fallback_attr: "ElementType",
    fallback_slot: 8,
    family: Family::Element,
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
};
