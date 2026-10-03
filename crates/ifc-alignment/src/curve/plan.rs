//! A whole horizontal layout as ONE exact plan curve.
//!
//! A plane curve is fixed by its start frame and its curvature as a
//! function of arc length. Every horizontal segment with a curvature law
//! -- zero for `LINE`, `1/R` for `CIRCULARARC`, the family law for a
//! transition spiral -- is a piece of that law in its own arc length, so a
//! run of such segments is one `Curve2::Intrinsic` carrying a
//! `CurvatureLaw::Piecewise` with a seam at every segment boundary, anchored
//! at the FIRST segment's `StartPoint` and `StartDirection`.
//!
//! A `CUBIC` has no curvature law in arc length (see `cubic.rs`). A layout
//! holding one is a `Curve2::Chain` instead: the same start frame, one
//! `ChainPiece2::Intrinsic` per segment with a law and one
//! `ChainPiece2::Parametric` cubic parabola, read by arc length, per
//! `CUBIC`. Each piece starts where the previous one ends, in the direction
//! it ends in. A layout without a `CUBIC` keeps the single intrinsic curve.
//!
//! This is what `Elevated3.plan` needs: a single `Curve2` whose parameter is
//! distance along the plan. A separate curve per segment would need an
//! absolute start frame at each seam, and after a spiral or a cubic that
//! frame is a non-elementary integral. Anchoring the interior by arc length
//! alone needs no interior position.
//!
//! # What the single curve implies
//!
//! Position and heading are continuous by construction. Later segments'
//! authored `StartPoint`s and `StartDirection`s are not stored; they are
//! checked against the curve by the rule in `seam.rs`:
//!
//! - a heading mismatch is refused at every seam after a segment with a
//!   curvature law, because one plan curve cannot carry a kink and storing
//!   it anyway would move the road;
//! - a position mismatch after a `LINE` or `CIRCULARARC` is refused;
//! - a position after a transition spiral, and a position and heading after
//!   a `CUBIC`, are recorded as
//!   [`SeamCheck::Authored`](super::SeamCheck::Authored), not verified.

use axiolid_core::{Frame2, Vec2};
use axiolid_curve::{Chain2, ChainPiece2, CurvatureLaw, Curve2, Intrinsic2};
use ifc_model::{EntityId, Model};

use super::cubic::{cubic_curve, local_frame, CUBIC};
use super::seam::{check_direction, check_position, HorizontalSeam};
use super::spiral::{curvature_of, is_exactly_lowerable, refuse_unlowerable, transition_curvature};
use super::terminal::split_closing;
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
    /// The whole layout as one `Curve2::Intrinsic`, or a `Curve2::Chain`
    /// when it holds a `CUBIC`, parameterised by distance along the plan
    /// from the first segment's start.
    pub curve: Curve2,
    /// The segments it was lowered from, in authored order.
    pub sources: Vec<EntityId>,
    /// Every seam between consecutive segments and how it was checked,
    /// including the seam before a closing zero-length segment.
    pub seams: Vec<HorizontalSeam>,
}

/// Lower an `IfcAlignmentHorizontal` to one exact plan curve.
///
/// The curve is a `Curve2::Intrinsic` anchored at the first segment and
/// carrying one curvature piece per segment, so it is parameterised by
/// distance along the plan and needs no interior position. A single-segment
/// layout yields that segment's law without a piecewise wrapper. A layout
/// with a `CUBIC` is a `Curve2::Chain` of the same pieces, the cubic as a
/// parametric piece read by arc length.
///
/// `cant` is needed only by a `VIENNESEBEND`, whose law depends on the cant
/// swing across it.
///
/// # Errors
///
/// Refuses a layout with no segments, any segment without an exact law (as
/// [`lower_horizontal_layout`](super::lower_horizontal_layout) does), a
/// heading kink at any seam whose predecessor has a curvature law, and a
/// position gap at a seam whose predecessor has a closed-form end point. A
/// seam after a transition spiral or a `CUBIC` is not refused; it is
/// reported in [`HorizontalPlan::seams`] as unverified.
///
/// The zero-length segment IFC4.3 requires at the end of a layout adds no
/// piece; its start is checked against the curve's end like any seam, and
/// that seam is reported last. A zero-length segment anywhere else, or as
/// the only segment, is refused ([`AlignmentError::SemanticViolation`]).
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
    // The closing zero-length segment adds no curvature piece; its seam is
    // checked below like any other.
    let (body, closing) = split_closing(&segments, |s| s.segment_length, |s| s.entity)?;
    let Some(first) = body.first() else {
        return Err(AlignmentError::SemanticViolation {
            entity: Some(entity),
            rule: "IfcAlignmentHorizontal must nest at least one IfcAlignmentSegment",
        });
    };

    // Each kept piece with the station it starts at.
    let mut pieces: Vec<(f64, Piece)> = Vec::with_capacity(segments.len());
    let mut seams = Vec::with_capacity(segments.len().saturating_sub(1));
    let mut station = 0.0_f64;
    let mut previous: Option<(&HorizontalSegment, Piece)> = None;
    for segment in body {
        let piece = segment_piece(segment, cant, station)?;
        if let Some((before, before_piece)) = &previous {
            seams.push(check_seam(before, before_piece, segment, station)?);
        }
        let end = station + segment.segment_length;
        // A piece too short to advance the station in f64 would put two
        // seams at one distance, which the kernel reads as a malformed law.
        if end > station {
            pieces.push((station, piece.clone()));
        }
        station = end;
        previous = Some((segment, piece));
    }
    if let (Some(closing), Some((before, before_piece))) = (closing, &previous) {
        seams.push(check_seam(before, before_piece, closing, station)?);
    }

    let direction = Vec2::new(first.start_direction.cos(), first.start_direction.sin());
    let start = Frame2 {
        origin: first.start_point,
        x: direction,
        y: Vec2::new(-direction.y, direction.x),
    };
    let malformed = || AlignmentError::Graph {
        detail: "the horizontal layout did not assemble into a well-formed plan curve".to_owned(),
    };
    let curve = if pieces
        .iter()
        .all(|(_, piece)| matches!(piece, Piece::Law(_)))
    {
        let mut breaks = Vec::with_capacity(pieces.len());
        let mut laws = Vec::with_capacity(pieces.len());
        for (at, piece) in pieces {
            if let Piece::Law(law) = piece {
                if !laws.is_empty() {
                    breaks.push(at);
                }
                laws.push(law);
            }
        }
        let curvature = if laws.len() == 1 {
            laws.remove(0)
        } else {
            CurvatureLaw::piecewise(breaks, laws)
        };
        let curve = Intrinsic2::new(start, curvature, station);
        if !curve.curvature.is_well_formed()
            || curve.total_turning().is_none_or(|turn| !turn.is_finite())
        {
            return Err(malformed());
        }
        Curve2::Intrinsic(curve)
    } else {
        // Each piece's length is the gap to the next kept start, so the
        // chain's joins fall exactly where the intrinsic law's breaks would.
        let ends: Vec<f64> = pieces
            .iter()
            .skip(1)
            .map(|(at, _)| *at)
            .chain([station])
            .collect();
        let chain = Chain2::new(
            start,
            pieces
                .into_iter()
                .zip(ends)
                .map(|((at, piece), end)| {
                    let length = end - at;
                    match piece {
                        Piece::Law(curvature) => ChainPiece2::Intrinsic { curvature, length },
                        Piece::Cubic(curve) => ChainPiece2::Parametric {
                            curve,
                            start: 0.0,
                            length,
                        },
                    }
                })
                .collect(),
        );
        if !chain.is_well_formed() {
            return Err(malformed());
        }
        Curve2::Chain(chain)
    };
    Ok(HorizontalPlan {
        curve,
        sources: ids,
        seams,
    })
}

/// One segment of the plan: a curvature law in its own arc length, or a
/// `CUBIC`'s cubic parabola in its local frame, read by arc length.
#[derive(Debug, Clone)]
enum Piece {
    Law(CurvatureLaw),
    Cubic(Curve2),
}

/// The seam where `next` starts after `before`: heading checked in closed
/// form after a curvature law, position by the rule in `seam.rs`.
fn check_seam(
    before: &HorizontalSegment,
    before_piece: &Piece,
    next: &HorizontalSegment,
    station: f64,
) -> AlignmentResult<HorizontalSeam> {
    // A cubic's end heading is not closed form; its seam reports the
    // position, and with it the heading, as authored.
    if let Piece::Law(law) = before_piece {
        check_direction(before, law, next)?;
    }
    check_position(before, next, station)
}

/// One segment's piece of the plan.
fn segment_piece(
    segment: &HorizontalSegment,
    cant: Option<&CantLayout>,
    station: f64,
) -> AlignmentResult<Piece> {
    match &segment.segment_type {
        HorizontalSegmentType::Transition(name) if name == CUBIC => {
            cubic_curve(segment, local_frame()).map(Piece::Cubic)
        }
        _ => segment_law(segment, cant, station).map(Piece::Law),
    }
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
