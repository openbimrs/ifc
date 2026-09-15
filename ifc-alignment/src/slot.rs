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
