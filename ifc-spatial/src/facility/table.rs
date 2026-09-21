//! Generated facility catalogue. Do not edit by hand.
//!
//! Source: `references/ifc-spec/ifc4x3-add2/IFC4X3_ADD2.exp`
//! Regenerate: `python3 scripts/gen-facilities.py`

use super::Facility;

/// `IfcBridge`.
pub const IFCBRIDGE: Facility = Facility {
    type_name: "IFCBRIDGE",
    arity: 10,
    predefined_slot: Some(9),
    usage_slot: None,
    members: &[
        "ARCHED",
        "CABLE_STAYED",
        "CANTILEVER",
        "CULVERT",
        "FRAMEWORK",
        "GIRDER",
        "SUSPENSION",
        "TRUSS",
        "USERDEFINED",
        "NOTDEFINED",
    ],
};

/// `IfcBridgePart`.
pub const IFCBRIDGEPART: Facility = Facility {
    type_name: "IFCBRIDGEPART",
    arity: 11,
    predefined_slot: Some(10),
    usage_slot: Some(9),
    members: &[
        "ABUTMENT",
        "DECK",
        "DECK_SEGMENT",
        "FOUNDATION",
        "PIER",
        "PIER_SEGMENT",
        "PYLON",
        "SUBSTRUCTURE",
        "SUPERSTRUCTURE",
        "SURFACESTRUCTURE",
        "USERDEFINED",
        "NOTDEFINED",
    ],
};

/// `IfcFacility`.
pub const IFCFACILITY: Facility = Facility {
    type_name: "IFCFACILITY",
    arity: 9,
    predefined_slot: None,
    usage_slot: None,
    members: &[],
};

/// `IfcFacilityPartCommon`.
pub const IFCFACILITYPARTCOMMON: Facility = Facility {
    type_name: "IFCFACILITYPARTCOMMON",
    arity: 11,
    predefined_slot: Some(10),
    usage_slot: Some(9),
    members: &[
        "ABOVEGROUND",
        "BELOWGROUND",
        "JUNCTION",
        "LEVELCROSSING",
        "SEGMENT",
        "SUBSTRUCTURE",
        "SUPERSTRUCTURE",
        "TERMINAL",
        "USERDEFINED",
        "NOTDEFINED",
    ],
};

/// `IfcMarineFacility`.
pub const IFCMARINEFACILITY: Facility = Facility {
    type_name: "IFCMARINEFACILITY",
    arity: 10,
    predefined_slot: Some(9),
    usage_slot: None,
    members: &[
        "BARRIERBEACH",
        "BREAKWATER",
        "CANAL",
        "DRYDOCK",
        "FLOATINGDOCK",
        "HYDROLIFT",
        "JETTY",
        "LAUNCHRECOVERY",
        "MARINEDEFENCE",
        "NAVIGATIONALCHANNEL",
        "PORT",
        "QUAY",
        "REVETMENT",
        "SHIPLIFT",
        "SHIPLOCK",
        "SHIPYARD",
        "SLIPWAY",
        "WATERWAY",
        "WATERWAYSHIPLIFT",
        "USERDEFINED",
        "NOTDEFINED",
    ],
};

/// `IfcMarinePart`.
pub const IFCMARINEPART: Facility = Facility {
    type_name: "IFCMARINEPART",
    arity: 11,
    predefined_slot: Some(10),
    usage_slot: Some(9),
    members: &[
        "ABOVEWATERLINE",
        "ANCHORAGE",
        "APPROACHCHANNEL",
        "BELOWWATERLINE",
        "BERTHINGSTRUCTURE",
        "CHAMBER",
        "CILL_LEVEL",
        "COPELEVEL",
        "CORE",
        "CREST",
        "GATEHEAD",
        "GUDINGSTRUCTURE",
        "HIGHWATERLINE",
        "LANDFIELD",
        "LEEWARDSIDE",
        "LOWWATERLINE",
        "MANUFACTURING",
        "NAVIGATIONALAREA",
        "PROTECTION",
        "SHIPTRANSFER",
        "STORAGEAREA",
        "VEHICLESERVICING",
        "WATERFIELD",
        "WEATHERSIDE",
        "USERDEFINED",
        "NOTDEFINED",
    ],
};

/// `IfcRailway`.
pub const IFCRAILWAY: Facility = Facility {
    type_name: "IFCRAILWAY",
    arity: 10,
    predefined_slot: Some(9),
    usage_slot: None,
    members: &["USERDEFINED", "NOTDEFINED"],
};

/// `IfcRailwayPart`.
pub const IFCRAILWAYPART: Facility = Facility {
    type_name: "IFCRAILWAYPART",
    arity: 11,
    predefined_slot: Some(10),
    usage_slot: Some(9),
    members: &[
        "ABOVETRACK",
        "DILATIONTRACK",
        "LINESIDE",
        "LINESIDEPART",
        "PLAINTRACK",
        "SUBSTRUCTURE",
        "TRACK",
        "TRACKPART",
        "TURNOUTTRACK",
        "USERDEFINED",
        "NOTDEFINED",
    ],
};

/// `IfcRoad`.
pub const IFCROAD: Facility = Facility {
    type_name: "IFCROAD",
    arity: 10,
    predefined_slot: Some(9),
    usage_slot: None,
    members: &["USERDEFINED", "NOTDEFINED"],
};

/// `IfcRoadPart`.
pub const IFCROADPART: Facility = Facility {
    type_name: "IFCROADPART",
    arity: 11,
    predefined_slot: Some(10),
    usage_slot: Some(9),
    members: &[
        "BICYCLECROSSING",
        "BUS_STOP",
        "CARRIAGEWAY",
        "CENTRALISLAND",
        "CENTRALRESERVE",
        "HARDSHOULDER",
        "INTERSECTION",
        "LAYBY",
        "PARKINGBAY",
        "PASSINGBAY",
        "PEDESTRIAN_CROSSING",
        "RAILWAYCROSSING",
        "REFUGEISLAND",
        "ROADSEGMENT",
        "ROADSIDE",
        "ROADSIDEPART",
        "ROADWAYPLATEAU",
        "ROUNDABOUT",
        "SHOULDER",
        "SIDEWALK",
        "SOFTSHOULDER",
        "TOLLPLAZA",
        "TRAFFICISLAND",
        "TRAFFICLANE",
        "USERDEFINED",
        "NOTDEFINED",
    ],
};

/// Every facility class this crate can author.
pub const ALL: &[Facility] = &[
    IFCBRIDGE,
    IFCBRIDGEPART,
    IFCFACILITY,
    IFCFACILITYPARTCOMMON,
    IFCMARINEFACILITY,
    IFCMARINEPART,
    IFCRAILWAY,
    IFCRAILWAYPART,
    IFCROAD,
    IFCROADPART,
];
