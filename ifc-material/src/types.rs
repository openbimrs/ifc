//! MaterialResource defined types and selects.

use ifc_model::EntityId;

/// Complete IFC4 ADD2 TC1 MaterialResource entity inventory, including abstracts.
pub const IFC4_MATERIAL_RESOURCE_ENTITIES: &[&str] = &[
    "IFCMATERIAL",
    "IFCMATERIALCLASSIFICATIONRELATIONSHIP",
    "IFCMATERIALCONSTITUENT",
    "IFCMATERIALCONSTITUENTSET",
    "IFCMATERIALDEFINITION",
    "IFCMATERIALDEFINITIONREPRESENTATION",
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
    /// `POSITIVE`: the layer/profile direction runs along the reference axis.
    Positive,
    /// `NEGATIVE`: the layer/profile direction runs opposite the reference axis.
    Negative,
}

impl DirectionSense {
    /// Parses an `IfcDirectionSenseEnum` token (case-insensitively). Returns
    /// `None` for any string that is not `POSITIVE` or `NEGATIVE`.
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
    /// `AXIS1`: layer set thickness is measured along the first local axis.
    Axis1,
    /// `AXIS2`: layer set thickness is measured along the second local axis.
    Axis2,
    /// `AXIS3`: layer set thickness is measured along the third local axis.
    Axis3,
}

impl LayerSetDirection {
    /// Parses an `IfcLayerSetDirectionEnum` token (case-insensitively).
    /// Returns `None` for any string that is not `AXIS1`, `AXIS2`, or `AXIS3`.
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
    /// `.F.`: the logical value is definitely false.
    False,
    /// `.T.`: the logical value is definitely true.
    True,
    /// `.U.`: the logical value is unknown or not applicable.
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
    /// Builds a `CardinalPointReference` from a raw `IfcCardinalPointReference`
    /// integer. Returns `None` if `value` is not strictly positive, per the
    /// IFC4 WHERE rule on this defined type.
    pub fn new(value: i64) -> Option<Self> {
        u64::try_from(value)
            .ok()
            .filter(|value| *value > 0)
            .map(Self)
    }

    /// Returns the underlying positive integer value.
    pub fn get(self) -> u64 {
        self.0
    }

    /// Resolves this value to its standardized placement meaning, if it
    /// falls in the reserved 1-19 range; otherwise `None`.
    pub fn standard(self) -> Option<StandardCardinalPoint> {
        StandardCardinalPoint::from_number(self.0)
    }
}

/// Standard placement meanings assigned to cardinal values 1-19.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum StandardCardinalPoint {
    /// Value 1: bottom-left corner.
    BottomLeft = 1,
    /// Value 2: bottom-center.
    BottomCenter = 2,
    /// Value 3: bottom-right corner.
    BottomRight = 3,
    /// Value 4: mid-depth, left side.
    MidDepthLeft = 4,
    /// Value 5: mid-depth, center.
    MidDepthCenter = 5,
    /// Value 6: mid-depth, right side.
    MidDepthRight = 6,
    /// Value 7: top-left corner.
    TopLeft = 7,
    /// Value 8: top-center.
    TopCenter = 8,
    /// Value 9: top-right corner.
    TopRight = 9,
    /// Value 10: geometric centroid of the profile.
    GeometricCentroid = 10,
    /// Value 11: bottom edge, at the geometric centroid's horizontal position.
    BottomAtGeometricCentroid = 11,
    /// Value 12: left edge, at the geometric centroid's vertical position.
    LeftAtGeometricCentroid = 12,
    /// Value 13: right edge, at the geometric centroid's vertical position.
    RightAtGeometricCentroid = 13,
    /// Value 14: top edge, at the geometric centroid's horizontal position.
    TopAtGeometricCentroid = 14,
    /// Value 15: the profile's shear center.
    ShearCenter = 15,
    /// Value 16: bottom edge, at the shear center's horizontal position.
    BottomAtShearCenter = 16,
    /// Value 17: left edge, at the shear center's vertical position.
    LeftAtShearCenter = 17,
    /// Value 18: right edge, at the shear center's vertical position.
    RightAtShearCenter = 18,
    /// Value 19: top edge, at the shear center's horizontal position.
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
    /// Resolves to an `IfcMaterialDefinition` (material, layer set, profile
    /// set, or constituent set) by its entity id.
    Definition(EntityId),
    /// Resolves to an `IfcMaterialList` by its entity id.
    List(EntityId),
    /// Resolves to an `IfcMaterialUsageDefinition` (a layer/profile set
    /// usage) by its entity id.
    Usage(EntityId),
}
