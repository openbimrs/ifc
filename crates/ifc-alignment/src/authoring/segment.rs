//! Alignment parameter segments: horizontal, vertical and cant.
//!
//! Each writes one `IfcAlignment*Segment`. `StartTag`/`EndTag` are the
//! inherited optional labels at slots 0..1 and are left null unless named.

use ifc_model::{Entity, EntityId, Transaction, Value};

use super::{finite, invalid};
use crate::error::AlignmentError;
use crate::slot;

/// Authored fields for `IfcAlignmentHorizontalSegment`.
///
/// Radii follow the IFC convention the reader already applies: zero means
/// straight, and a signed value carries the turn direction.
#[derive(Debug, Clone, Copy)]
pub struct HorizontalSegmentDraft {
    /// `StartPoint`, an existing `IfcCartesianPoint`.
    pub start_point: EntityId,
    /// `StartDirection`, the bearing in the file's angle unit.
    pub start_direction: f64,
    /// `StartRadiusOfCurvature`; zero for a straight.
    pub start_radius: f64,
    /// `EndRadiusOfCurvature`; zero for a straight.
    pub end_radius: f64,
    /// `SegmentLength`; must be non-negative.
    pub segment_length: f64,
    /// `GravityCenterLineHeight`, optional.
    pub gravity_center_line_height: Option<f64>,
    /// `PredefinedType`, e.g. `LINE` or `CIRCULARARC`.
    pub predefined_type: &'static str,
}

const HSEG: &str = "IFCALIGNMENTHORIZONTALSEGMENT";

/// Stage an `IfcAlignmentHorizontalSegment`.
///
/// # Errors
///
/// Refuses a non-finite measure, a negative `SegmentLength`, and a `LINE`
/// carrying a non-zero radius -- the reader rejects that combination when
/// lowering, so writing it would produce a file this crate cannot read.
pub fn horizontal_segment(
    tx: &mut Transaction,
    draft: &HorizontalSegmentDraft,
) -> Result<EntityId, AlignmentError> {
    finite(HSEG, "StartDirection", draft.start_direction)?;
    finite(HSEG, "StartRadiusOfCurvature", draft.start_radius)?;
    finite(HSEG, "EndRadiusOfCurvature", draft.end_radius)?;
    finite(HSEG, "SegmentLength", draft.segment_length)?;
    if let Some(height) = draft.gravity_center_line_height {
        finite(HSEG, "GravityCenterLineHeight", height)?;
    }
    if draft.segment_length < 0.0 {
        return Err(invalid(
            HSEG,
            "SegmentLength",
            "a segment cannot be shorter than nothing",
        ));
    }
    if draft.predefined_type == "LINE" && (draft.start_radius != 0.0 || draft.end_radius != 0.0) {
        return Err(invalid(
            HSEG,
            "PredefinedType",
            "LINE requires zero start and end radii",
        ));
    }
    let mut attrs = vec![Value::Null; slot::horizontal::PREDEFINED_TYPE + 1];
    attrs[slot::horizontal::START_POINT] = Value::Ref(draft.start_point);
    attrs[slot::horizontal::START_DIRECTION] = Value::Real(draft.start_direction);
    attrs[slot::horizontal::START_RADIUS] = Value::Real(draft.start_radius);
    attrs[slot::horizontal::END_RADIUS] = Value::Real(draft.end_radius);
    attrs[slot::horizontal::SEGMENT_LENGTH] = Value::Real(draft.segment_length);
    if let Some(height) = draft.gravity_center_line_height {
        attrs[slot::horizontal::GRAVITY_CENTER_LINE_HEIGHT] = Value::Real(height);
    }
    attrs[slot::horizontal::PREDEFINED_TYPE] = Value::Enum(draft.predefined_type.into());
    Ok(tx.create(Entity::new(HSEG, attrs)))
}

/// Authored fields for `IfcAlignmentVerticalSegment`.
#[derive(Debug, Clone, Copy)]
pub struct VerticalSegmentDraft {
    /// `StartDistAlong`, measured along the horizontal layout.
    pub start_dist_along: f64,
    /// `HorizontalLength`; must be non-negative.
    pub horizontal_length: f64,
    /// `StartHeight`.
    pub start_height: f64,
    /// `StartGradient`, a ratio, not an angle.
    pub start_gradient: f64,
    /// `EndGradient`, a ratio.
    pub end_gradient: f64,
    /// `RadiusOfCurvature`; required by arc families.
    pub radius_of_curvature: Option<f64>,
    /// `PredefinedType`, e.g. `CONSTANTGRADIENT` or `CIRCULARARC`.
    pub predefined_type: &'static str,
}

const VSEG: &str = "IFCALIGNMENTVERTICALSEGMENT";

/// Stage an `IfcAlignmentVerticalSegment`.
///
/// # Errors
///
/// Refuses a non-finite measure, a negative `HorizontalLength`, and a
/// radius that disagrees with the family: the reader requires one exactly
/// for `CIRCULARARC` and `PARABOLICARC`, so both a missing radius there and
/// a stray radius elsewhere are refused here.
pub fn vertical_segment(
    tx: &mut Transaction,
    draft: &VerticalSegmentDraft,
) -> Result<EntityId, AlignmentError> {
    finite(VSEG, "StartDistAlong", draft.start_dist_along)?;
    finite(VSEG, "HorizontalLength", draft.horizontal_length)?;
    finite(VSEG, "StartHeight", draft.start_height)?;
    finite(VSEG, "StartGradient", draft.start_gradient)?;
    finite(VSEG, "EndGradient", draft.end_gradient)?;
    if let Some(radius) = draft.radius_of_curvature {
        finite(VSEG, "RadiusOfCurvature", radius)?;
    }
    if draft.horizontal_length < 0.0 {
        return Err(invalid(
            VSEG,
            "HorizontalLength",
            "a segment cannot be shorter than nothing",
        ));
    }
    let needs_radius = matches!(draft.predefined_type, "CIRCULARARC" | "PARABOLICARC");
    if needs_radius != draft.radius_of_curvature.is_some() {
        return Err(invalid(
            VSEG,
            "RadiusOfCurvature",
            "radius is required exactly for circular and parabolic vertical segments",
        ));
    }
    let mut attrs = vec![Value::Null; slot::vertical::PREDEFINED_TYPE + 1];
    attrs[slot::vertical::START_DIST_ALONG] = Value::Real(draft.start_dist_along);
    attrs[slot::vertical::HORIZONTAL_LENGTH] = Value::Real(draft.horizontal_length);
    attrs[slot::vertical::START_HEIGHT] = Value::Real(draft.start_height);
    attrs[slot::vertical::START_GRADIENT] = Value::Real(draft.start_gradient);
    attrs[slot::vertical::END_GRADIENT] = Value::Real(draft.end_gradient);
    if let Some(radius) = draft.radius_of_curvature {
        attrs[slot::vertical::RADIUS_OF_CURVATURE] = Value::Real(radius);
    }
    attrs[slot::vertical::PREDEFINED_TYPE] = Value::Enum(draft.predefined_type.into());
    Ok(tx.create(Entity::new(VSEG, attrs)))
}

/// Authored fields for `IfcAlignmentCantSegment`.
#[derive(Debug, Clone, Copy)]
pub struct CantSegmentDraft {
    /// `StartDistAlong`.
    pub start_dist_along: f64,
    /// `HorizontalLength`; must be non-negative.
    pub horizontal_length: f64,
    /// `StartCantLeft`.
    pub start_cant_left: f64,
    /// `EndCantLeft`; paired with `end_cant_right`.
    pub end_cant_left: Option<f64>,
    /// `StartCantRight`.
    pub start_cant_right: f64,
    /// `EndCantRight`; paired with `end_cant_left`.
    pub end_cant_right: Option<f64>,
    /// `PredefinedType`, e.g. `CONSTANTCANT` or `LINEARTRANSITION`.
    pub predefined_type: &'static str,
}

const CSEG: &str = "IFCALIGNMENTCANTSEGMENT";

/// Stage an `IfcAlignmentCantSegment`.
///
/// # Errors
///
/// Refuses a non-finite measure, a negative `HorizontalLength`, and an
/// unpaired end cant: the reader requires left and right to be both present
/// or both absent, because one alone does not describe a rail pair.
pub fn cant_segment(
    tx: &mut Transaction,
    draft: &CantSegmentDraft,
) -> Result<EntityId, AlignmentError> {
    finite(CSEG, "StartDistAlong", draft.start_dist_along)?;
    finite(CSEG, "HorizontalLength", draft.horizontal_length)?;
    finite(CSEG, "StartCantLeft", draft.start_cant_left)?;
    finite(CSEG, "StartCantRight", draft.start_cant_right)?;
    if let Some(value) = draft.end_cant_left {
        finite(CSEG, "EndCantLeft", value)?;
    }
    if let Some(value) = draft.end_cant_right {
        finite(CSEG, "EndCantRight", value)?;
    }
    if draft.horizontal_length < 0.0 {
        return Err(invalid(
            CSEG,
            "HorizontalLength",
            "a segment cannot be shorter than nothing",
        ));
    }
    if draft.end_cant_left.is_some() != draft.end_cant_right.is_some() {
        return Err(invalid(
            CSEG,
            "EndCantLeft",
            "left and right end cant must both be supplied or both omitted",
        ));
    }
    let mut attrs = vec![Value::Null; slot::cant::PREDEFINED_TYPE + 1];
    attrs[slot::cant::START_DIST_ALONG] = Value::Real(draft.start_dist_along);
    attrs[slot::cant::HORIZONTAL_LENGTH] = Value::Real(draft.horizontal_length);
    attrs[slot::cant::START_CANT_LEFT] = Value::Real(draft.start_cant_left);
    if let Some(value) = draft.end_cant_left {
        attrs[slot::cant::END_CANT_LEFT] = Value::Real(value);
    }
    attrs[slot::cant::START_CANT_RIGHT] = Value::Real(draft.start_cant_right);
    if let Some(value) = draft.end_cant_right {
        attrs[slot::cant::END_CANT_RIGHT] = Value::Real(value);
    }
    attrs[slot::cant::PREDEFINED_TYPE] = Value::Enum(draft.predefined_type.into());
    Ok(tx.create(Entity::new(CSEG, attrs)))
}
