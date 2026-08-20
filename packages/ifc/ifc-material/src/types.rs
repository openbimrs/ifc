//! MaterialResource defined types and selects.

use ifc_model::EntityId;

/// Complete IFC4 ADD2 TC1 MaterialResource entity inventory, including abstracts.
pub const IFC4_MATERIAL_RESOURCE_ENTITIES: &[&str] = &[
    "IFCMATERIAL",
    "IFCMATERIALCLASSIFICATIONRELATIONSHIP",
    "IFCMATERIALCONSTITUENT",
    "IFCMATERIALCONSTITUENTSET",
    "IFCMATERIALDEFINITION",
    "IFCMATERIALLAYER",
    "IFCMATERIALLAYERSET",
    "IFCMATERIALLAYERSETUSAGE",
    "IFCMATERIALLAYERWITHOFFSETS",
    "IFCMATERIALLIST",
    "IFCMATERIALPROFILE",
    "IFCMATERIALPROFILESET",
    "IFCMATERIALPROFILESETUSAGE",
    "IFCMATERIALPROFILESETUSAGETAPERING",
    "IFCMATERIALPROFILEWITHOFFSETS",
    "IFCMATERIALPROPERTIES",
    "IFCMATERIALRELATIONSHIP",
    "IFCMATERIALUSAGEDEFINITION",
];

/// Complete IFC4 ADD2 TC1 MaterialResource defined/select type inventory.
pub const IFC4_MATERIAL_RESOURCE_TYPES: &[&str] = &[
    "IFCCARDINALPOINTREFERENCE",
    "IFCDIRECTIONSENSEENUM",
    "IFCLAYERSETDIRECTIONENUM",
    "IFCMATERIALSELECT",
];

/// IFC direction along or opposite an axis.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DirectionSense {
    Positive,
    Negative,
}

impl DirectionSense {
    pub fn parse(token: &str) -> Option<Self> {
        match token {
            token if token.eq_ignore_ascii_case("POSITIVE") => Some(Self::Positive),
            token if token.eq_ignore_ascii_case("NEGATIVE") => Some(Self::Negative),
            _ => None,
        }
    }
}

/// Axis used to measure a material layer set.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LayerSetDirection {
    Axis1,
    Axis2,
    Axis3,
}

impl LayerSetDirection {
    pub fn parse(token: &str) -> Option<Self> {
        match token {
            token if token.eq_ignore_ascii_case("AXIS1") => Some(Self::Axis1),
            token if token.eq_ignore_ascii_case("AXIS2") => Some(Self::Axis2),
            token if token.eq_ignore_ascii_case("AXIS3") => Some(Self::Axis3),
            _ => None,
        }
    }
}

/// IFC's three-state logical value.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LogicalValue {
    False,
    True,
    Unknown,
}

/// Positive `IfcCardinalPointReference` value.
///
/// IFC4 constrains this defined type to values greater than zero. Values 1-19
/// have standardized placement meanings; larger positive values remain valid
/// schema values and are preserved.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct CardinalPointReference(u64);

impl CardinalPointReference {
    pub fn new(value: i64) -> Option<Self> {
        u64::try_from(value)
            .ok()
            .filter(|value| *value > 0)
            .map(Self)
    }

    pub fn get(self) -> u64 {
        self.0
    }

    pub fn standard(self) -> Option<StandardCardinalPoint> {
        StandardCardinalPoint::from_number(self.0)
    }
}

/// Standard placement meanings assigned to cardinal values 1-19.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum StandardCardinalPoint {
    BottomLeft = 1,
    BottomCenter = 2,
    BottomRight = 3,
    MidDepthLeft = 4,
    MidDepthCenter = 5,
    MidDepthRight = 6,
    TopLeft = 7,
    TopCenter = 8,
    TopRight = 9,
    GeometricCentroid = 10,
    BottomAtGeometricCentroid = 11,
    LeftAtGeometricCentroid = 12,
    RightAtGeometricCentroid = 13,
    TopAtGeometricCentroid = 14,
    ShearCenter = 15,
    BottomAtShearCenter = 16,
    LeftAtShearCenter = 17,
    RightAtShearCenter = 18,
    TopAtShearCenter = 19,
}

impl StandardCardinalPoint {
    fn from_number(value: u64) -> Option<Self> {
        Some(match value {
            1 => Self::BottomLeft,
            2 => Self::BottomCenter,
            3 => Self::BottomRight,
            4 => Self::MidDepthLeft,
            5 => Self::MidDepthCenter,
            6 => Self::MidDepthRight,
            7 => Self::TopLeft,
            8 => Self::TopCenter,
            9 => Self::TopRight,
            10 => Self::GeometricCentroid,
            11 => Self::BottomAtGeometricCentroid,
            12 => Self::LeftAtGeometricCentroid,
            13 => Self::RightAtGeometricCentroid,
            14 => Self::TopAtGeometricCentroid,
            15 => Self::ShearCenter,
            16 => Self::BottomAtShearCenter,
            17 => Self::LeftAtShearCenter,
            18 => Self::RightAtShearCenter,
            19 => Self::TopAtShearCenter,
            _ => return None,
        })
    }
}

/// Resolved branch of `IfcMaterialSelect`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MaterialSelect {
    Definition(EntityId),
    List(EntityId),
    Usage(EntityId),
}
