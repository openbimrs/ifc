//! A whole horizontal layout as ONE exact plan curve.
//!
//! A plane curve is fixed by its start frame and its curvature as a
//! function of arc length. Every horizontal segment this crate lowers has an
//! exact law in its own arc length -- zero for `LINE`, `1/R` for
//! `CIRCULARARC`, the family law for a transition spiral -- so a run of
//! segments is one `Curve2::Intrinsic` carrying a `CurvatureLaw::Piecewise`
//! with a seam at every segment boundary, anchored at the FIRST segment's
//! `StartPoint` and `StartDirection`.
//!
//! This is what `Elevated3.plan` needs: a single `Curve2` whose parameter is
//! distance along the plan. It is also why a piecewise law exists upstream.
//! A separate curve per segment would need an absolute start frame at each
//! seam, and after a spiral that frame's origin is a Fresnel-type integral.
//! Anchoring the interior by arc length alone needs no interior position.
//!
//! # What the single curve implies
//!
//! Position and heading are continuous by construction. Later segments'
//! authored `StartPoint`s and `StartDirection`s are not stored; they are
//! checked against the curve by the rule in `seam.rs`:
//!
//! - a heading mismatch is refused at every seam, because one intrinsic
//!   curve cannot carry a kink and storing it anyway would move the road;
//! - a position mismatch after a `LINE` or `CIRCULARARC` is refused;
//! - a position after a transition spiral is recorded as
//!   [`SeamCheck::Authored`](super::SeamCheck::Authored), not verified.

use axiolid_core::{Frame2, Vec2};
use axiolid_curve::{CurvatureLaw, Curve2, Intrinsic2};
use ifc_model::{EntityId, Model};

use super::seam::{check_direction, check_position, HorizontalSeam};
use super::spiral::{curvature_of, is_exactly_lowerable, refuse_unlowerable, transition_curvature};
use crate::cant::CantLayout;
use crate::error::{AlignmentError, AlignmentResult};
use crate::horizontal::{
    read_horizontal_segment, AlignmentUnits, HorizontalSegment, HorizontalSegmentType,
};
use crate::view::AlignmentView;

/// A horizontal layout lowered to one exact plan curve.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct HorizontalPlan {
    /// The whole layout as one `Curve2::Intrinsic`, parameterised by
    /// distance along the plan from the first segment's start.
    pub curve: Curve2,
    /// The segments it was lowered from, in authored order.
    pub sources: Vec<EntityId>,
    /// Every seam between consecutive segments and how it was checked.
    pub seams: Vec<HorizontalSeam>,
}

/// Lower an `IfcAlignmentHorizontal` to one exact plan curve.
///
/// The curve is a `Curve2::Intrinsic` anchored at the first segment and
/// carrying one curvature piece per segment, so it is parameterised by
/// distance along the plan and needs no interior position. A single-segment
/// layout yields that segment's law without a piecewise wrapper.
///
/// `cant` is needed only by a `VIENNESEBEND`, whose law depends on the cant
/// swing across it.
///
/// # Errors
///
/// Refuses a layout with no segments, any segment without an exact law (as
/// [`lower_horizontal_layout`](super::lower_horizontal_layout) does), a
/// heading kink at any seam, and a position gap at a seam whose predecessor
/// has a closed-form end point. A seam after a transition spiral is not
/// refused; it is reported in [`HorizontalPlan::seams`] as unverified.
pub fn lower_horizontal_plan(
    model: &Model,
    entity: EntityId,
    units: AlignmentUnits,
    cant: Option<&CantLayout>,
) -> AlignmentResult<HorizontalPlan> {
    let view = AlignmentView::for_model(model)?;
    let horizontal = model
        .get(entity)
        .ok_or(AlignmentError::MissingEntity { entity })?;
    if !view
        .schema
        .is_a(&horizontal.type_name, "IfcAlignmentHorizontal")
    {
        return Err(AlignmentError::WrongType {
            entity,
            expected: "IfcAlignmentHorizontal",
            actual: horizontal.type_name.to_string(),
        });
    }
    let ids = view.segment_chain(entity, "IfcAlignmentHorizontalSegment")?;
    let segments = ids
        .iter()
        .map(|id| read_horizontal_segment(model, *id, units))
        .collect::<AlignmentResult<Vec<_>>>()?;
    let Some(first) = segments.first() else {
        return Err(AlignmentError::SemanticViolation {
            entity: Some(entity),
            rule: "IfcAlignmentHorizontal must nest at least one IfcAlignmentSegment",
        });
    };

    let mut laws = Vec::with_capacity(segments.len());
    let mut breaks = Vec::with_capacity(segments.len());
    let mut seams = Vec::with_capacity(segments.len().saturating_sub(1));
    let mut station = 0.0_f64;
    let mut previous: Option<(&HorizontalSegment, CurvatureLaw)> = None;
    for segment in &segments {
        let law = segment_law(segment, cant, station)?;
        if let Some((before, before_law)) = &previous {
            check_direction(before, before_law, segment)?;
            seams.push(check_position(before, segment, station)?);
        }
        let end = station + segment.segment_length;
        // A piece too short to advance the station in f64 would put two
        // seams at one distance, which the kernel reads as a malformed law.
        if end > station {
            if !laws.is_empty() {
                breaks.push(station);
            }
            laws.push(law.clone());
        }
        station = end;
        previous = Some((segment, law));
    }

    let curvature = if laws.len() == 1 {
        laws.remove(0)
    } else {
        CurvatureLaw::piecewise(breaks, laws)
    };
    let direction = Vec2::new(first.start_direction.cos(), first.start_direction.sin());
    let curve = Intrinsic2::new(
        Frame2 {
            origin: first.start_point,
            x: direction,
            y: Vec2::new(-direction.y, direction.x),
        },
        curvature,
        station,
    );
    if !curve.curvature.is_well_formed()
        || curve.total_turning().is_none_or(|turn| !turn.is_finite())
    {
        return Err(AlignmentError::Graph {
            detail: "the horizontal layout did not assemble into a well-formed curvature law"
                .to_owned(),
        });
    }
    Ok(HorizontalPlan {
        curve: Curve2::Intrinsic(curve),
        sources: ids,
        seams,
    })
}

/// One segment's exact curvature law in its own arc length.
///
/// Validates `LINE` and `CIRCULARARC` exactly as their single-segment
/// lowerings do, so the plan and the composite accept the same files.
fn segment_law(
    segment: &HorizontalSegment,
    cant: Option<&CantLayout>,
    station: f64,
) -> AlignmentResult<CurvatureLaw> {
    match &segment.segment_type {
        HorizontalSegmentType::Line => {
            if segment.start_radius != 0.0 || segment.end_radius != 0.0 {
                return Err(AlignmentError::InvalidSegment {
                    entity: segment.entity,
                    detail: "LINE requires zero start and end radii",
                });
            }
            Ok(CurvatureLaw::straight())
        }
        HorizontalSegmentType::CircularArc => {
            if segment.start_radius == 0.0
                || segment.start_radius != segment.end_radius
                || !segment.start_radius.is_finite()
            {
                return Err(AlignmentError::InvalidSegment {
                    entity: segment.entity,
                    detail: "CIRCULARARC requires equal, finite, non-zero start and end radii",
                });
            }
            // Signed: a positive radius turns counter-clockwise, as IFC
            // states and as the kernel's curvature sign convention reads it.
            Ok(CurvatureLaw::circular(curvature_of(segment.start_radius)))
        }
        HorizontalSegmentType::Transition(name) if is_exactly_lowerable(name, cant.is_some()) => {
            transition_curvature(segment, name, cant, station)
        }
        _ => Err(refuse_unlowerable(segment)),
    }
}
