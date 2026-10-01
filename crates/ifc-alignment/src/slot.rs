//! Attribute slots for the alignment entities, shared by readers and
//! authoring.
//!
//! Every index below was read from the IFC4X3 ADD2 schema with a throwaway
//! probe over `ifc_schema::ifc4x3().attributes(..)`, not inferred. The
//! layouts and segments inherit differently, so the two groups start at
//! different offsets.
//!
//! Readers indexed these positions with bare literals before authoring
//! existed. Both directions now index this module, so a slot cannot be
//! corrected on one side only (ADR 0011).
//!
//! `tests.rs` pins every slot, arity, the IFC4X3 ADD2 profile name, the
//! declarations the crate reads, the subtype and SELECT memberships the
//! traversal relies on, and the absence of the layouts from earlier
//! releases, against the table bundled in `ifc-schema` (#16).

/// `IfcAlignmentHorizontalSegment`: `StartTag`/`EndTag` are inherited from
/// `IfcAlignmentParameterSegment` at slots 0..1.
pub(crate) mod horizontal {
    /// `StartPoint : IfcCartesianPoint`.
    pub const START_POINT: usize = 2;
    /// `StartDirection : IfcPlaneAngleMeasure`.
    pub const START_DIRECTION: usize = 3;
    /// `StartRadiusOfCurvature : IfcLengthMeasure`.
    pub const START_RADIUS: usize = 4;
    /// `EndRadiusOfCurvature : IfcLengthMeasure`.
    pub const END_RADIUS: usize = 5;
    /// `SegmentLength : IfcNonNegativeLengthMeasure`.
    pub const SEGMENT_LENGTH: usize = 6;
    /// `GravityCenterLineHeight : OPTIONAL IfcPositiveLengthMeasure`.
    pub const GRAVITY_CENTER_LINE_HEIGHT: usize = 7;
    /// `PredefinedType : IfcAlignmentHorizontalSegmentTypeEnum`.
    pub const PREDEFINED_TYPE: usize = 8;
}

/// `IfcAlignmentVerticalSegment`: same inherited pair at slots 0..1.
pub(crate) mod vertical {
    /// `StartDistAlong : IfcLengthMeasure`.
    pub const START_DIST_ALONG: usize = 2;
    /// `HorizontalLength : IfcNonNegativeLengthMeasure`.
    pub const HORIZONTAL_LENGTH: usize = 3;
    /// `StartHeight : IfcLengthMeasure`.
    pub const START_HEIGHT: usize = 4;
    /// `StartGradient : IfcRatioMeasure`.
    pub const START_GRADIENT: usize = 5;
    /// `EndGradient : IfcRatioMeasure`.
    pub const END_GRADIENT: usize = 6;
    /// `RadiusOfCurvature : OPTIONAL IfcLengthMeasure`.
    pub const RADIUS_OF_CURVATURE: usize = 7;
    /// `PredefinedType : IfcAlignmentVerticalSegmentTypeEnum`.
    pub const PREDEFINED_TYPE: usize = 8;
}

/// `IfcAlignmentCantSegment`: same inherited pair at slots 0..1.
pub(crate) mod cant {
    /// `StartDistAlong : IfcLengthMeasure`.
    pub const START_DIST_ALONG: usize = 2;
    /// `HorizontalLength : IfcPositiveLengthMeasure`.
    pub const HORIZONTAL_LENGTH: usize = 3;
    /// `StartCantLeft : IfcLengthMeasure`.
    pub const START_CANT_LEFT: usize = 4;
    /// `EndCantLeft : OPTIONAL IfcLengthMeasure`.
    pub const END_CANT_LEFT: usize = 5;
    /// `StartCantRight : IfcLengthMeasure`.
    pub const START_CANT_RIGHT: usize = 6;
    /// `EndCantRight : OPTIONAL IfcLengthMeasure`.
    pub const END_CANT_RIGHT: usize = 7;
    /// `PredefinedType : IfcAlignmentCantSegmentTypeEnum`.
    pub const PREDEFINED_TYPE: usize = 8;
}

/// `IfcAlignment` and its layout children are `IfcProduct` subtypes, so
/// `GlobalId`, `OwnerHistory`, `Name`, `Description`, `ObjectType`,
/// `ObjectPlacement` and `Representation` occupy slots 0..6.
///
/// Only the positions this crate writes are named. The rest are left to
/// `ifc-author`, which owns generic product attributes.
pub(crate) mod product {
    /// `GlobalId : IfcGloballyUniqueId`.
    pub const GLOBAL_ID: usize = 0;
    /// `Name : OPTIONAL IfcLabel`.
    pub const NAME: usize = 2;
    /// `ObjectPlacement : OPTIONAL IfcObjectPlacement`.
    pub const OBJECT_PLACEMENT: usize = 5;
    /// Attribute count shared by `IfcAlignmentHorizontal` and
    /// `IfcAlignmentVertical`, which add nothing of their own.
    pub const ARITY: usize = 7;
}

/// `IfcAlignment` adds `PredefinedType` after the product attributes.
pub(crate) mod alignment {
    /// `PredefinedType : OPTIONAL IfcAlignmentTypeEnum`.
    pub const PREDEFINED_TYPE: usize = 7;
    /// Total attribute count.
    pub const ARITY: usize = 8;
}

/// `IfcAlignmentCant` adds `RailHeadDistance`.
pub(crate) mod cant_layout {
    /// `RailHeadDistance : IfcPositiveLengthMeasure`.
    pub const RAIL_HEAD_DISTANCE: usize = 7;
    /// Total attribute count.
    pub const ARITY: usize = 8;
}

/// `IfcAlignmentSegment` adds `DesignParameters`.
pub(crate) mod segment {
    /// `DesignParameters : IfcAlignmentParameterSegment`.
    pub const DESIGN_PARAMETERS: usize = 7;
    /// Total attribute count.
    pub const ARITY: usize = 8;
}

/// `IfcReferent` slots. An `IfcProduct` with a predefined type.
pub mod referent {
    /// Attribute count.
    pub const ARITY: usize = 8;
    /// `PredefinedType`.
    pub const PREDEFINED_TYPE: usize = 7;
}

/// `IfcLinearPlacement` slots.
pub mod linear_placement {
    /// Attribute count.
    pub const ARITY: usize = 3;
    /// `PlacementRelTo`.
    pub const PLACEMENT_REL_TO: usize = 0;
    /// `RelativePlacement`. Required.
    pub const RELATIVE_PLACEMENT: usize = 1;
    /// `CartesianPosition`.
    pub const CARTESIAN_POSITION: usize = 2;
}

/// `IfcPointByDistanceExpression` slots.
pub mod point_by_distance {
    /// Attribute count.
    pub const ARITY: usize = 5;
    /// `DistanceAlong`. Required.
    pub const DISTANCE_ALONG: usize = 0;
    /// `OffsetLateral`.
    pub const OFFSET_LATERAL: usize = 1;
    /// `OffsetVertical`.
    pub const OFFSET_VERTICAL: usize = 2;
    /// `OffsetLongitudinal`.
    pub const OFFSET_LONGITUDINAL: usize = 3;
    /// `BasisCurve`. Required.
    pub const BASIS_CURVE: usize = 4;
}

/// `IfcAxis2PlacementLinear` slots.
pub mod axis2_placement_linear {
    /// Attribute count.
    pub const ARITY: usize = 3;
    /// `Location`. Required.
    pub const LOCATION: usize = 0;
    /// `Axis`.
    pub const AXIS: usize = 1;
    /// `RefDirection`.
    pub const REF_DIRECTION: usize = 2;
}

/// `IfcRelNests` and `IfcRelAggregates`: both are `IfcRelDecomposes`
/// subtypes, so the `IfcRoot` quartet occupies slots 0..3 and each adds
/// its own `RelatingObject`/`RelatedObjects` pair (a `LIST` for nesting,
/// a `SET` for aggregation).
pub(crate) mod decomposes {
    /// `RelatingObject : IfcObjectDefinition`.
    pub const RELATING_OBJECT: usize = 4;
    /// `RelatedObjects : LIST|SET [1:?] OF IfcObjectDefinition`.
    pub const RELATED_OBJECTS: usize = 5;
    /// Attribute count.
    #[cfg(test)]
    pub const ARITY: usize = 6;
}

/// `IfcRelPositions`, an `IfcRelConnects` subtype.
pub(crate) mod rel_positions {
    /// `RelatingPositioningElement : IfcPositioningElement`.
    pub const RELATING_POSITIONING_ELEMENT: usize = 4;
    /// `RelatedProducts : SET [1:?] OF IfcProduct`.
    pub const RELATED_PRODUCTS: usize = 5;
    /// Attribute count.
    #[cfg(test)]
    pub const ARITY: usize = 6;
}

/// `IfcRelDefinesByProperties`, an `IfcRelDefines` subtype.
pub(crate) mod rel_defines_by_properties {
    /// `RelatedObjects : SET [1:?] OF IfcObjectDefinition`.
    pub const RELATED_OBJECTS: usize = 4;
    /// `RelatingPropertyDefinition : IfcPropertySetDefinitionSelect`.
    pub const RELATING_PROPERTY_DEFINITION: usize = 5;
    /// Attribute count.
    #[cfg(test)]
    pub const ARITY: usize = 6;
}

/// `IfcPropertySet`: `Name` is inherited from `IfcRoot`.
pub(crate) mod property_set {
    /// `Name : OPTIONAL IfcLabel` (inherited, required by `ExistsName`).
    pub const NAME: usize = 2;
    /// `HasProperties : SET [1:?] OF IfcProperty`.
    pub const HAS_PROPERTIES: usize = 4;
    /// Attribute count.
    #[cfg(test)]
    pub const ARITY: usize = 5;
}

/// `IfcPropertySingleValue`: `Name`/`Specification` come from
/// `IfcProperty`; `IfcPropertyAbstraction` contributes none.
pub(crate) mod property_single_value {
    /// `Name : IfcIdentifier`.
    pub const NAME: usize = 0;
    /// `NominalValue : OPTIONAL IfcValue`.
    pub const NOMINAL_VALUE: usize = 2;
    /// Attribute count.
    #[cfg(test)]
    pub const ARITY: usize = 4;
}

#[cfg(test)]
mod tests;
