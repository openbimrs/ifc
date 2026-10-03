//! Exact neutral graph assembly for alignment segments and their layouts.
//!
//! # Stations
//!
//! An `IfcAlignmentHorizontalSegment` does not state its own distance along;
//! it follows from chaining, and a Viennese bend needs it to read the cant
//! swing across itself. Both chain walks, [`lower_horizontal_layout`] and
//! [`lower_horizontal_layout_partial`], therefore keep their own station
//! accumulator. They are two accumulators, so a test that exercises only one
//! walk leaves the other unguarded; test both. A single-segment lowering has
//! no chain context and refuses a Viennese bend rather than assuming station
//! zero.

use axiolid_core::{Frame2, Point2, Vec2};
use axiolid_curve::{BSplineCurve2, Circle2, Curve2, ElevationLaw, KnotSpec, Line2};
use axiolid_model::{
    CurveRelation, CurveSegment, GeometryGraph, GeometryGraphBuilder, GeometryNode, NodeId,
    Transition, TrimSelector, TrimmingPreference,
};
use ifc_model::{EntityId, Model};

use crate::cant::CantLayout;
use crate::curve::cubic::{cubic_curve, start_frame, CUBIC};
use crate::curve::elevation::elevation_law;
use crate::curve::seam::{check_position, HorizontalSeam, SeamCheck};
use crate::curve::spiral::{is_exactly_lowerable, refuse_unlowerable, spiral_curve};
use crate::curve::terminal::split_closing;
use crate::error::{AlignmentError, AlignmentResult};
use crate::horizontal::{
    read_horizontal_segment, AlignmentUnits, HorizontalSegment, HorizontalSegmentType,
};
use crate::vertical::{read_vertical_segment, VerticalSegment};
use crate::view::AlignmentView;

/// Exact neutral curve graph for one or more IFC alignment segments.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct LoweredAlignmentCurve {
    /// The neutral geometry graph holding the lowered curve and its
    /// supporting nodes (trims, composites).
    pub graph: GeometryGraph,
    /// The graph node the lowered curve is rooted at.
    pub root: NodeId,
    /// The segment(s) this graph was lowered from, in authored order.
    pub sources: Vec<EntityId>,
    /// Seams between consecutive horizontal segments and how each was
    /// checked, in authored order. Empty for a single segment and for a
    /// vertical lowering.
    pub seams: Vec<HorizontalSeam>,
}

/// The composite transition a checked seam supports.
///
/// IFC alignment segments carry no explicit continuity attribute (unlike
/// `IfcCompositeCurveSegment.Transition`); it is a geometric fact about the
/// lowered curves. Only a seam whose position was verified in closed form
/// claims `Continuous`. A seam after a transition spiral claims nothing:
/// `Discontinuous` is this crate's no-claim value, the same one the first
/// segment carries, and the seam itself is reported in
/// [`LoweredAlignmentCurve::seams`] as [`SeamCheck::Authored`].
fn seam_transition(seam: &HorizontalSeam) -> Transition {
    match seam.position {
        SeamCheck::Verified => Transition::Continuous,
        _ => Transition::Discontinuous,
    }
}

/// One segment this crate declined to lower, and why.
///
/// Carries the authored type name verbatim so a caller can report
/// `CLOTHOID` rather than "some unsupported segment", and the entity id so
/// it can point at the offending line of the source file.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct RefusedSegment {
    /// The `IfcAlignmentHorizontalSegment` entity that was refused.
    pub entity: EntityId,
    /// The segment's authored `PredefinedType`, preserved exactly.
    pub type_name: String,
    /// Why this segment could not be lowered exactly.
    pub reason: AlignmentError,
}

/// A horizontal layout lowered as far as exactness allows.
///
/// Real railway and highway alignments interleave transition spirals between
/// their lines and arcs, so an all-or-nothing lowering refuses essentially
/// every production file. This result keeps what is exactly lowerable and
/// names what is not, instead of collapsing both into one opaque error.
///
/// `runs` holds maximal stretches of consecutive lowerable segments, each
/// assembled into a single composite exactly as [`lower_horizontal_layout`]
/// would, seams included. A run ends only where a segment is refused:
/// continuity across a segment this crate did not lower is not a fact it is
/// entitled to assert. A seam after a transition spiral does not end a run;
/// it is recorded in that run's `seams` as unverified.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct PartialHorizontalLayout {
    /// Maximal runs of consecutive exactly-lowered segments, in authored
    /// order.
    pub runs: Vec<LoweredAlignmentCurve>,
    /// Refused segments in authored order.
    pub refused: Vec<RefusedSegment>,
    /// Total segments nested by the layout, lowered or not.
    pub segment_count: usize,
}

impl PartialHorizontalLayout {
    /// Whether every segment lowered exactly.
    ///
    /// When true the layout is covered by exactly one run, matching what
    /// [`lower_horizontal_layout`] returns: runs split only at refused
    /// segments, never at a seam after a spiral.
    #[must_use]
    pub fn is_complete(&self) -> bool {
        self.refused.is_empty()
    }

    /// Number of segments lowered exactly.
    #[must_use]
    pub fn lowered_count(&self) -> usize {
        self.segment_count - self.refused.len()
    }
}

/// Lower one `IfcAlignmentHorizontalSegment` to an exact neutral curve.
///
/// A `CUBIC` lowers as its exact cubic parabola trimmed at
/// `TrimSelector::ArcLength(SegmentLength)` (see `cubic.rs`).
///
/// Fails if `id` is missing or malformed, or the segment is a family with
/// no exact law here (`USERDEFINED`, `NOTDEFINED`, an unknown token, a
/// `VIENNESEBEND` without its cant layout, a `CUBIC` that does not leave a
/// straight), or `CIRCULARARC` with unequal/zero/non-finite start and end
/// radii, or `LINE` with a non-zero radius. A zero-length segment, which IFC4.3 allows
/// only as a layout's closing segment, has no geometry to lower and is
/// refused as [`AlignmentError::InvalidSegment`].
pub fn lower_horizontal_segment(
    model: &Model,
    id: EntityId,
    units: AlignmentUnits,
) -> AlignmentResult<LoweredAlignmentCurve> {
    let segment = read_horizontal_segment(model, id, units)?;
    if segment.segment_length == 0.0 {
        return Err(no_geometry(id));
    }
    let mut builder = GeometryGraphBuilder::new();
    let root = match &segment.segment_type {
        HorizontalSegmentType::Line => push_line(&mut builder, &segment)?,
        HorizontalSegmentType::CircularArc => push_arc(&mut builder, &segment)?,
        HorizontalSegmentType::Transition(name) if name == CUBIC => {
            push_cubic(&mut builder, &segment)?
        }
        HorizontalSegmentType::Transition(name) if is_exactly_lowerable(name, false) => {
            push_spiral(&mut builder, &segment, name, None, 0.0)?
        }
        _ => return Err(refuse_unlowerable(&segment)),
    };
    finish(builder, root, vec![id])
}

/// Lower one `IfcAlignmentVerticalSegment` to an exact neutral curve in the
/// (distance along, height) plane.
///
/// The segment goes through [`elevation_law`], the same law the composed
/// gradient curve uses, so the per-segment and composed paths accept,
/// refuse and place exactly the same segments. The law is then written as a
/// `Curve2` whose parameter is plan distance from `StartDistAlong`, trimmed
/// to `0..HorizontalLength`:
///
/// - degree 1 (`CONSTANTGRADIENT`): a line through
///   `(StartDistAlong, StartHeight)` with direction `(1, grade)`;
/// - degree 2 (`PARABOLICARC`): a quadratic Bezier over the knot span
///   `0..L` with control points `z0`, `z0 + g0 L / 2`, `z(L)` at stations
///   `d0`, `d0 + L / 2`, `d0 + L`. Equally spaced stations make the station
///   linear in the parameter, so this is the IFC parabola exactly, not a fit.
///
/// A `CIRCULARARC` is the circle itself, `Curve2::Circle`, trimmed by angle
/// from the start to where the plan distance `HorizontalLength` ends; its
/// parameter is that angle, not plan distance.
///
/// # Errors
///
/// Fails if `id` is missing or malformed, and with everything
/// [`elevation_law`] refuses: `CLOTHOID` (its curvature is not stated), and
/// parameters that contradict their family. A zero-length segment (a
/// layout's closing segment) has no geometry to lower and is refused as
/// [`AlignmentError::InvalidSegment`].
pub fn lower_vertical_segment(
    model: &Model,
    id: EntityId,
    units: AlignmentUnits,
) -> AlignmentResult<LoweredAlignmentCurve> {
    let segment = read_vertical_segment(model, id, units)?;
    if segment.horizontal_length == 0.0 {
        return Err(no_geometry(id));
    }
    let law = elevation_law(&segment)?;
    let mut builder = GeometryGraphBuilder::new();
    let root = push_vertical_law(&mut builder, &segment, &law)?;
    finish(builder, root, vec![id])
}

/// Push a `IfcAlignmentHorizontalSegment.LINE` as a trimmed neutral line.
fn push_line(
    builder: &mut GeometryGraphBuilder,
    segment: &HorizontalSegment,
) -> AlignmentResult<NodeId> {
    if segment.start_radius != 0.0 || segment.end_radius != 0.0 {
        return Err(AlignmentError::InvalidSegment {
            entity: segment.entity,
            detail: "LINE requires zero start and end radii",
        });
    }
    let direction = Vec2::new(segment.start_direction.cos(), segment.start_direction.sin());
    let basis = push(
        builder,
        GeometryNode::Curve2(Curve2::Line(Line2 {
            origin: segment.start_point,
            direction,
        })),
    )?;
    push(
        builder,
        GeometryNode::CurveRelation(CurveRelation::Trimmed {
            basis,
            start: vec![TrimSelector::Parameter(0.0)],
            end: vec![TrimSelector::Parameter(segment.segment_length)],
            sense_agreement: true,
            preference: TrimmingPreference::Parameter,
        }),
    )
}

/// Push a `IfcAlignmentHorizontalSegment.CIRCULARARC` as a trimmed neutral
/// circle.
fn push_arc(
    builder: &mut GeometryGraphBuilder,
    segment: &HorizontalSegment,
) -> AlignmentResult<NodeId> {
    if segment.start_radius == 0.0
        || segment.start_radius != segment.end_radius
        || !segment.start_radius.is_finite()
    {
        return Err(AlignmentError::InvalidSegment {
            entity: segment.entity,
            detail: "CIRCULARARC requires equal, finite, non-zero start and end radii",
        });
    }
    let direction = Vec2::new(segment.start_direction.cos(), segment.start_direction.sin());
    let left = Vec2::new(-direction.y, direction.x);
    let signed_radius = segment.start_radius;
    let radius = signed_radius.abs();
    let centre = segment.start_point + left * signed_radius;
    // Derive the radial frame from the source tangent and curvature sign. Using
    // `(start - centre) / radius` loses the direction when a tiny radius is
    // added to a large global coordinate.
    let x = left * -signed_radius.signum();
    // Keep the frame right-handed. A negative radius then has a negative end
    // parameter, which preserves the source traversal direction exactly.
    let y = Vec2::new(-x.y, x.x);
    let sweep = segment.segment_length / signed_radius;
    if [centre.x, centre.y, x.x, x.y, y.x, y.y, radius, sweep]
        .iter()
        .any(|value| !value.is_finite())
    {
        return Err(AlignmentError::InvalidSegment {
            entity: segment.entity,
            detail: "CIRCULARARC derived frame and trim parameters must be finite",
        });
    }
    let basis = push(
        builder,
        GeometryNode::Curve2(Curve2::Circle(Circle2 {
            frame: Frame2 {
                origin: centre,
                x,
                y,
            },
            radius,
        })),
    )?;
    push(
        builder,
        GeometryNode::CurveRelation(CurveRelation::Trimmed {
            basis,
            start: vec![TrimSelector::Parameter(0.0)],
            end: vec![TrimSelector::Parameter(sweep)],
            sense_agreement: true,
            preference: TrimmingPreference::Parameter,
        }),
    )
}

/// Push a vertical segment's exact elevation law as a trimmed `Curve2` in
/// the (distance along, height) plane, parameterised by plan distance.
fn push_vertical_law(
    builder: &mut GeometryGraphBuilder,
    segment: &VerticalSegment,
    law: &ElevationLaw,
) -> AlignmentResult<NodeId> {
    let unsupported = |detail| AlignmentError::Unsupported {
        entity: segment.entity,
        type_name: segment.predefined_type.source_name().to_owned(),
        detail,
    };
    let coefficients = match law {
        ElevationLaw::Polynomial { coefficients } => coefficients,
        ElevationLaw::CircularArc { grade, radius, .. } => {
            return push_vertical_arc(builder, segment, *grade, *radius)
        }
        _ => {
            return Err(unsupported(
                "a single vertical segment lowers from one polynomial or circular piece",
            ))
        }
    };
    let start = segment.start_dist_along;
    let length = segment.horizontal_length;
    let coefficient = |power: usize| coefficients.get(power).copied().unwrap_or(0.0);
    let curve = match coefficients.len() {
        0..=2 => Curve2::Line(Line2 {
            origin: Point2::new(start, coefficient(0)),
            direction: Vec2::new(1.0, coefficient(1)),
        }),
        3 => {
            let (z0, g0, c2) = (coefficient(0), coefficient(1), coefficient(2));
            let control_points = vec![
                Point2::new(start, z0),
                Point2::new(start + 0.5 * length, z0 + 0.5 * g0 * length),
                Point2::new(start + length, z0 + g0 * length + c2 * length * length),
            ];
            if control_points
                .iter()
                .any(|p| !p.x.is_finite() || !p.y.is_finite())
            {
                return Err(AlignmentError::InvalidSegment {
                    entity: segment.entity,
                    detail: "vertical control points must be finite",
                });
            }
            Curve2::BSpline(BSplineCurve2 {
                degree: 2,
                control_points,
                knots: vec![0.0, length],
                multiplicities: vec![3, 3],
                weights: None,
                closed: false,
                self_intersect: Some(false),
                knot_spec: KnotSpec::PiecewiseBezier,
            })
        }
        _ => {
            return Err(unsupported(
                "the vertical law has no exact neutral curve of its degree",
            ))
        }
    };
    let basis = push(builder, GeometryNode::Curve2(curve))?;
    push(
        builder,
        GeometryNode::CurveRelation(CurveRelation::Trimmed {
            basis,
            start: vec![TrimSelector::Parameter(0.0)],
            end: vec![TrimSelector::Parameter(length)],
            sense_agreement: true,
            preference: TrimmingPreference::Parameter,
        }),
    )
}

/// Push a vertical `CIRCULARARC` as the circle itself in the
/// (distance along, height) plane, trimmed by angle.
///
/// The circle starts at `(StartDistAlong, StartHeight)` with its tangent at
/// `atan(grade)`, centre `R` along the left normal (above for a sag), in the
/// same right-handed frame as a horizontal arc. It sweeps `t1 - t0`, where
/// `sin t1 = sin t0 + L / R` is where the plan distance `L` ends: closed-form
/// trigonometry on the stated values, nothing evaluated.
fn push_vertical_arc(
    builder: &mut GeometryGraphBuilder,
    segment: &VerticalSegment,
    grade: f64,
    radius: f64,
) -> AlignmentResult<NodeId> {
    // sin and cos of atan(grade) without forming the angle.
    let norm = grade.hypot(1.0);
    let (sin0, cos0) = (grade / norm, 1.0 / norm);
    let left = Vec2::new(-sin0, cos0);
    let start = Point2::new(segment.start_dist_along, segment.start_height);
    let centre = start + left * radius;
    let x = left * -radius.signum();
    let y = Vec2::new(-x.y, x.x);
    let sin1 = sin0 + segment.horizontal_length / radius;
    let sweep = sin1.asin() - sin0.atan2(cos0);
    if [centre.x, centre.y, sweep].iter().any(|v| !v.is_finite()) || sin1.abs() >= 1.0 {
        return Err(AlignmentError::InvalidSegment {
            entity: segment.entity,
            detail: "CIRCULARARC derived frame and trim angle must be finite",
        });
    }
    let basis = push(
        builder,
        GeometryNode::Curve2(Curve2::Circle(Circle2 {
            frame: Frame2 {
                origin: centre,
                x,
                y,
            },
            radius: radius.abs(),
        })),
    )?;
    push(
        builder,
        GeometryNode::CurveRelation(CurveRelation::Trimmed {
            basis,
            start: vec![TrimSelector::Parameter(0.0)],
            end: vec![TrimSelector::Parameter(sweep)],
            sense_agreement: true,
            preference: TrimmingPreference::Parameter,
        }),
    )
}

/// Push a transition spiral as an exact intrinsic curve, trimmed to its
/// authored length.
fn push_spiral(
    builder: &mut GeometryGraphBuilder,
    segment: &HorizontalSegment,
    name: &str,
    cant: Option<&CantLayout>,
    start_distance: f64,
) -> AlignmentResult<NodeId> {
    let curve = spiral_curve(segment, name, cant, start_distance)?;
    let basis = push(builder, GeometryNode::Curve2(curve))?;
    push(
        builder,
        GeometryNode::CurveRelation(CurveRelation::Trimmed {
            basis,
            start: vec![TrimSelector::Parameter(0.0)],
            end: vec![TrimSelector::Parameter(segment.segment_length)],
            sense_agreement: true,
            preference: TrimmingPreference::Parameter,
        }),
    )
}

/// Push a `CUBIC` as its exact cubic parabola, trimmed where its arc length
/// reaches `SegmentLength` (see `cubic.rs`).
///
/// The trim end is `TrimSelector::ArcLength`: the parameter there inverts an
/// elliptic integral, which an evaluator resolves to a stated tolerance and
/// this crate never computes.
fn push_cubic(
    builder: &mut GeometryGraphBuilder,
    segment: &HorizontalSegment,
) -> AlignmentResult<NodeId> {
    let curve = cubic_curve(segment, start_frame(segment))?;
    let basis = push(builder, GeometryNode::Curve2(curve))?;
    push(
        builder,
        GeometryNode::CurveRelation(CurveRelation::Trimmed {
            basis,
            start: vec![TrimSelector::Parameter(0.0)],
            end: vec![TrimSelector::ArcLength(segment.segment_length)],
            sense_agreement: true,
            preference: TrimmingPreference::Parameter,
        }),
    )
}

/// A zero-length segment lowered on its own: IFC4.3 allows one only to
/// close a layout, where it contributes no geometry.
fn no_geometry(entity: EntityId) -> AlignmentError {
    AlignmentError::InvalidSegment {
        entity,
        detail: "a zero-length segment has no geometry to lower; IFC4.3 allows it only as the \
                 closing segment of a layout",
    }
}

fn push(builder: &mut GeometryGraphBuilder, node: GeometryNode) -> AlignmentResult<NodeId> {
    builder.push(node).map_err(|error| AlignmentError::Graph {
        detail: error.to_string(),
    })
}

pub(super) fn finish(
    builder: GeometryGraphBuilder,
    root: NodeId,
    sources: Vec<EntityId>,
) -> AlignmentResult<LoweredAlignmentCurve> {
    let graph = builder
        .finish(vec![root])
        .map_err(|error| AlignmentError::Graph {
            detail: error.to_string(),
        })?;
    Ok(LoweredAlignmentCurve {
        graph,
        root,
        sources,
        seams: Vec::new(),
    })
}

/// Lower an `IfcAlignmentHorizontal`'s nested segment chain to one exact
/// neutral curve, refusing (rather than approximating) any segment that
/// `lower_horizontal_segment` cannot lower exactly.
///
/// Each segment keeps its own authored start frame inside the composite.
/// Seams are checked by the rule in [`SeamCheck`]: a position gap after a
/// `LINE` or `CIRCULARARC` is refused, and a seam after a transition spiral,
/// whose end point has no closed form, is accepted as authored and reported
/// in [`LoweredAlignmentCurve::seams`].
///
/// The zero-length segment IFC4.3 requires at the end of a layout adds no
/// composite segment; its seam is checked and reported last. A zero-length
/// segment anywhere else is refused ([`AlignmentError::SemanticViolation`]).
pub fn lower_horizontal_layout(
    model: &Model,
    entity: EntityId,
    units: AlignmentUnits,
    cant: Option<&CantLayout>,
) -> AlignmentResult<LoweredAlignmentCurve> {
    let view = AlignmentView::for_model(model)?;
    let horizontal_entity = model
        .get(entity)
        .ok_or(AlignmentError::MissingEntity { entity })?;
    if !view
        .schema
        .is_a(&horizontal_entity.type_name, "IfcAlignmentHorizontal")
    {
        return Err(AlignmentError::WrongType {
            entity,
            expected: "IfcAlignmentHorizontal",
            actual: horizontal_entity.type_name.to_string(),
        });
    }
    let ids = view.segment_chain(entity, "IfcAlignmentHorizontalSegment")?;
    if ids.is_empty() {
        return Err(AlignmentError::SemanticViolation {
            entity: Some(entity),
            rule: "IfcAlignmentHorizontal must nest at least one IfcAlignmentSegment",
        });
    }

    let mut segments = Vec::with_capacity(ids.len());
    for id in &ids {
        segments.push(read_horizontal_segment(model, *id, units)?);
    }

    let (body, closing) = split_closing(&segments, |s| s.segment_length, |s| s.entity)?;

    let mut builder = GeometryGraphBuilder::new();
    let mut composite_segments = Vec::with_capacity(body.len());
    let mut seams = Vec::with_capacity(segments.len().saturating_sub(1));
    let mut station = 0.0_f64;
    for (index, segment) in body.iter().enumerate() {
        let curve = match &segment.segment_type {
            HorizontalSegmentType::Line => push_line(&mut builder, segment)?,
            HorizontalSegmentType::CircularArc => push_arc(&mut builder, segment)?,
            HorizontalSegmentType::Transition(name) if name == CUBIC => {
                push_cubic(&mut builder, segment)?
            }
            HorizontalSegmentType::Transition(name)
                if is_exactly_lowerable(name, cant.is_some()) =>
            {
                push_spiral(&mut builder, segment, name, cant, station)?
            }
            _ => return Err(refuse_unlowerable(segment)),
        };
        let transition = if index == 0 {
            Transition::Discontinuous
        } else {
            let seam = check_position(&body[index - 1], segment, station)?;
            let transition = seam_transition(&seam);
            seams.push(seam);
            transition
        };
        composite_segments.push(CurveSegment {
            curve,
            same_sense: true,
            transition,
        });
        station += segment.segment_length;
    }
    // The closing segment adds no composite segment; its start is still a
    // seam against the last piece.
    if let (Some(closing), Some(last)) = (closing, body.last()) {
        seams.push(check_position(last, closing, station)?);
    }

    let root = push(
        &mut builder,
        GeometryNode::CurveRelation(CurveRelation::Composite {
            segments: composite_segments,
        }),
    )?;
    let mut lowered = finish(builder, root, ids)?;
    lowered.seams = seams;
    Ok(lowered)
}

/// Lower a horizontal layout as far as exactness allows, reporting refusals.
///
/// Unlike [`lower_horizontal_layout`], a segment without an exact law (such
/// as a `USERDEFINED` one) does not abort the whole layout. The segments around it still
/// lower exactly; the refused one is recorded in
/// [`PartialHorizontalLayout::refused`] with its authored type name and
/// entity id. Seams are checked by the same rule as the strict path.
///
/// Nothing here is approximated. A refused segment stays refused -- this
/// reports the boundary per segment instead of collapsing an entire
/// production alignment into one opaque error.
///
/// Errors that are not a lowering refusal (a wrong entity type, a malformed
/// attribute, an empty layout, a zero-length segment before the end) still
/// fail the whole call, because they mean the layout could not be read at
/// all rather than that one segment resisted exact lowering.
///
/// The closing zero-length segment IFC4.3 requires is neither lowered nor
/// refused: it adds no geometry, counts in
/// [`PartialHorizontalLayout::segment_count`] and
/// [`PartialHorizontalLayout::lowered_count`], and its seam is reported on
/// the last run when the segment before it was lowered.
pub fn lower_horizontal_layout_partial(
    model: &Model,
    entity: EntityId,
    units: AlignmentUnits,
    cant: Option<&CantLayout>,
) -> AlignmentResult<PartialHorizontalLayout> {
    let view = AlignmentView::for_model(model)?;
    let horizontal_entity = model
        .get(entity)
        .ok_or(AlignmentError::MissingEntity { entity })?;
    if !view
        .schema
        .is_a(&horizontal_entity.type_name, "IfcAlignmentHorizontal")
    {
        return Err(AlignmentError::WrongType {
            entity,
            expected: "IfcAlignmentHorizontal",
            actual: horizontal_entity.type_name.to_string(),
        });
    }
    let ids = view.segment_chain(entity, "IfcAlignmentHorizontalSegment")?;
    if ids.is_empty() {
        return Err(AlignmentError::SemanticViolation {
            entity: Some(entity),
            rule: "IfcAlignmentHorizontal must nest at least one IfcAlignmentSegment",
        });
    }

    let mut segments = Vec::with_capacity(ids.len());
    for id in &ids {
        segments.push(read_horizontal_segment(model, *id, units)?);
    }

    let (body, closing) = split_closing(&segments, |s| s.segment_length, |s| s.entity)?;

    let mut runs = Vec::new();
    let mut refused = Vec::new();
    // Segments accumulated since the last refusal, as (index, node) pairs in a
    // builder that is discarded and restarted whenever a run ends.
    let mut builder = GeometryGraphBuilder::new();
    let mut pending: Vec<(usize, CurveSegment)> = Vec::new();
    let mut pending_ids: Vec<EntityId> = Vec::new();
    let mut pending_seams: Vec<HorizontalSeam> = Vec::new();
    let mut station = 0.0_f64;

    for (index, segment) in body.iter().enumerate() {
        let lowered = match &segment.segment_type {
            HorizontalSegmentType::Line => push_line(&mut builder, segment),
            HorizontalSegmentType::CircularArc => push_arc(&mut builder, segment),
            HorizontalSegmentType::Transition(name) if name == CUBIC => {
                push_cubic(&mut builder, segment)
            }
            HorizontalSegmentType::Transition(name)
                if is_exactly_lowerable(name, cant.is_some()) =>
            {
                push_spiral(&mut builder, segment, name, cant, station)
            }
            _ => Err(refuse_unlowerable(segment)),
        };
        let start_station = station;
        // Advance before the refusal branch below: that path `continue`s, and
        // advancing at the loop tail would mis-station every later segment.
        station += segment.segment_length;
        let curve = match lowered {
            Ok(curve) => curve,
            Err(reason) => {
                // The run ends here: continuity across a segment this crate
                // did not lower is not a fact it can assert.
                flush_run(
                    &mut runs,
                    &mut builder,
                    &mut pending,
                    &mut pending_ids,
                    &mut pending_seams,
                )?;
                refused.push(RefusedSegment {
                    entity: segment.entity,
                    type_name: segment.segment_type.source_name().to_owned(),
                    reason,
                });
                continue;
            }
        };
        // Continuity is only claimed against the immediately preceding
        // segment when that segment is in the same run.
        let transition = match pending.last() {
            None => Transition::Discontinuous,
            Some((previous_index, _)) => {
                let seam = check_position(&body[*previous_index], segment, start_station)?;
                let transition = seam_transition(&seam);
                pending_seams.push(seam);
                transition
            }
        };
        pending.push((
            index,
            CurveSegment {
                curve,
                same_sense: true,
                transition,
            },
        ));
        pending_ids.push(segment.entity);
    }
    // The closing segment joins the last run only when the piece before it
    // was lowered: continuity across a refused segment is not asserted.
    if let (Some(closing), Some((previous_index, _))) = (closing, pending.last()) {
        pending_seams.push(check_position(&body[*previous_index], closing, station)?);
    }
    flush_run(
        &mut runs,
        &mut builder,
        &mut pending,
        &mut pending_ids,
        &mut pending_seams,
    )?;

    Ok(PartialHorizontalLayout {
        runs,
        refused,
        segment_count: segments.len(),
    })
}

/// Close the current run, if any, into its own composite curve.
///
/// Takes the builder by mutable reference and replaces it, so each run owns a
/// self-contained graph whose node ids are meaningful within that graph.
fn flush_run(
    runs: &mut Vec<LoweredAlignmentCurve>,
    builder: &mut GeometryGraphBuilder,
    pending: &mut Vec<(usize, CurveSegment)>,
    pending_ids: &mut Vec<EntityId>,
    pending_seams: &mut Vec<HorizontalSeam>,
) -> AlignmentResult<()> {
    if pending.is_empty() {
        // Nothing accumulated; drop whatever partial nodes exist so a refused
        // segment does not leak orphan nodes into the next run.
        *builder = GeometryGraphBuilder::new();
        pending_ids.clear();
        pending_seams.clear();
        return Ok(());
    }
    let mut finished = GeometryGraphBuilder::new();
    core::mem::swap(builder, &mut finished);
    let composite_segments: Vec<CurveSegment> =
        pending.drain(..).map(|(_, segment)| segment).collect();
    let root = push(
        &mut finished,
        GeometryNode::CurveRelation(CurveRelation::Composite {
            segments: composite_segments,
        }),
    )?;
    let sources = core::mem::take(pending_ids);
    let mut run = finish(finished, root, sources)?;
    run.seams = core::mem::take(pending_seams);
    runs.push(run);
    Ok(())
}
