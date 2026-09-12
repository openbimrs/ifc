//! Exact neutral graph assembly for alignment segments and their layouts.

use axiolid_core::{Frame2, Point2, Vec2};
use axiolid_curve::{Circle2, Curve2, Line2};
use axiolid_model::{
    CurveRelation, CurveSegment, GeometryGraph, GeometryGraphBuilder, GeometryNode, NodeId,
    Transition, TrimSelector, TrimmingPreference,
};
use ifc_model::{EntityId, Model};

use crate::curve::spiral::{is_exactly_lowerable, spiral_curve};
use crate::error::{AlignmentError, AlignmentResult};
use crate::horizontal::{
    read_horizontal_segment, AlignmentUnits, HorizontalSegment, HorizontalSegmentType,
};
use crate::vertical::{read_vertical_segment, VerticalSegment, VerticalSegmentType};
use crate::view::AlignmentView;

/// Exact neutral curve graph for one or more IFC alignment segments.
#[derive(Debug, Clone, PartialEq)]
pub struct LoweredAlignmentCurve {
    /// The neutral geometry graph holding the lowered curve and its
    /// supporting nodes (trims, composites).
    pub graph: GeometryGraph,
    /// The graph node the lowered curve is rooted at.
    pub root: NodeId,
    /// The segment(s) this graph was lowered from, in authored order.
    pub sources: Vec<EntityId>,
}

/// How tightly two consecutive lowered segments actually connect.
///
/// IFC alignment segments carry no explicit continuity attribute (unlike
/// `IfcCompositeCurveSegment.Transition`); it is a geometric fact about the
/// lowered curves, not something the source states. Only exact equality
/// counts here -- floating error from unrelated upstream authoring is not
/// this crate's problem to paper over, so the tolerance is a parameter the
/// caller controls rather than a silent default.
fn observed_transition(end: Point2, next_start: Point2, tolerance: f64) -> Option<Transition> {
    if end.distance(next_start) <= tolerance {
        Some(Transition::Continuous)
    } else {
        None
    }
}

/// Exact end point of a segment this crate can lower, in closed form.
///
/// `None` for every transition-spiral family: their end point is a
/// Fresnel-type integral, so there is no closed form to return and this
/// crate will not quadrature one into existence. That is the same boundary
/// [`lower_horizontal_segment`] enforces, expressed as data so a caller can
/// see where a run has to stop rather than only that it failed.
fn closed_form_end_point(segment: &HorizontalSegment) -> Option<Point2> {
    match segment.segment_type {
        HorizontalSegmentType::Line => {
            let direction = Vec2::new(segment.start_direction.cos(), segment.start_direction.sin());
            Some(segment.start_point + direction * segment.segment_length)
        }
        HorizontalSegmentType::CircularArc if segment.start_radius != 0.0 => {
            let direction = Vec2::new(segment.start_direction.cos(), segment.start_direction.sin());
            let left = Vec2::new(-direction.y, direction.x);
            let centre = segment.start_point + left * segment.start_radius;
            let sweep = segment.segment_length / segment.start_radius;
            let radial = segment.start_point - centre;
            let (sin_s, cos_s) = sweep.sin_cos();
            Some(
                centre
                    + Vec2::new(
                        radial.x * cos_s - radial.y * sin_s,
                        radial.x * sin_s + radial.y * cos_s,
                    ),
            )
        }
        _ => None,
    }
}

/// One segment this crate declined to lower, and why.
///
/// Carries the authored type name verbatim so a caller can report
/// `CLOTHOID` rather than "some unsupported segment", and the entity id so
/// it can point at the offending line of the source file.
#[derive(Debug, Clone, PartialEq)]
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
/// would. A run ends wherever a segment is refused: continuity across a
/// segment this crate did not lower is not a fact it is entitled to assert.
#[derive(Debug, Clone, PartialEq)]
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
    /// [`lower_horizontal_layout`] returns.
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
/// Fails if `id` is missing or malformed, or the segment is a transition
/// spiral family not in `[is_exactly_lowerable]`'s set, or `CIRCULARARC`
/// with unequal/zero/non-finite start and end radii, or `LINE` with a
/// non-zero radius -- the pinned neutral curve vocabulary has no exact
/// primitive for those cases.
pub fn lower_horizontal_segment(
    model: &Model,
    id: EntityId,
    units: AlignmentUnits,
) -> AlignmentResult<LoweredAlignmentCurve> {
    let segment = read_horizontal_segment(model, id, units)?;
    let mut builder = GeometryGraphBuilder::new();
    let root =
        match &segment.segment_type {
            HorizontalSegmentType::Line => push_line(&mut builder, &segment)?,
            HorizontalSegmentType::CircularArc => push_arc(&mut builder, &segment)?,
            HorizontalSegmentType::Transition(name) if is_exactly_lowerable(name) => {
                push_spiral(&mut builder, &segment, name)?
            }
            kind => return Err(AlignmentError::Unsupported {
                entity: id,
                type_name: kind.source_name().to_owned(),
                detail:
                    "the pinned neutral curve vocabulary has no exact transition-curve primitive",
            }),
        };
    finish(builder, root, vec![id])
}

/// Lower one `IfcAlignmentVerticalSegment` to an exact neutral curve.
///
/// Fails if `id` is missing or malformed, or the segment is not
/// `CONSTANTGRADIENT`, or has a curvature radius, or unequal start/end
/// gradients -- exact neutral vertical lowering does not yet cover arcs or
/// parabolas.
pub fn lower_vertical_segment(
    model: &Model,
    id: EntityId,
    units: AlignmentUnits,
) -> AlignmentResult<LoweredAlignmentCurve> {
    let segment = read_vertical_segment(model, id, units)?;
    let mut builder = GeometryGraphBuilder::new();
    let root = push_constant_gradient(&mut builder, &segment)?;
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

/// Push a `IfcAlignmentVerticalSegment.CONSTANTGRADIENT` as a trimmed
/// neutral line in the (distance-along, height) plane.
fn push_constant_gradient(
    builder: &mut GeometryGraphBuilder,
    segment: &VerticalSegment,
) -> AlignmentResult<NodeId> {
    if !matches!(
        segment.predefined_type,
        VerticalSegmentType::ConstantGradient
    ) {
        return Err(AlignmentError::Unsupported {
            entity: segment.entity,
            type_name: segment.predefined_type.source_name().to_owned(),
            detail: "exact neutral vertical lowering is currently limited to constant gradient",
        });
    }
    if segment.radius_of_curvature.is_some() || segment.start_gradient != segment.end_gradient {
        return Err(AlignmentError::InvalidSegment {
            entity: segment.entity,
            detail: "CONSTANTGRADIENT requires equal gradients and no curvature radius",
        });
    }
    let basis = push(
        builder,
        GeometryNode::Curve2(Curve2::Line(Line2 {
            origin: Point2::new(segment.start_dist_along, segment.start_height),
            direction: Vec2::new(1.0, segment.start_gradient),
        })),
    )?;
    push(
        builder,
        GeometryNode::CurveRelation(CurveRelation::Trimmed {
            basis,
            start: vec![TrimSelector::Parameter(0.0)],
            end: vec![TrimSelector::Parameter(segment.horizontal_length)],
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
) -> AlignmentResult<NodeId> {
    let curve = spiral_curve(segment, name)?;
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

fn push(builder: &mut GeometryGraphBuilder, node: GeometryNode) -> AlignmentResult<NodeId> {
    builder.push(node).map_err(|error| AlignmentError::Graph {
        detail: error.to_string(),
    })
}

fn finish(
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
    })
}

/// Lower an `IfcAlignmentHorizontal`'s nested segment chain to one exact
/// neutral curve, refusing (rather than approximating) any segment that
/// `lower_horizontal_segment` cannot lower exactly.
pub fn lower_horizontal_layout(
    model: &Model,
    entity: EntityId,
    units: AlignmentUnits,
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

    let mut builder = GeometryGraphBuilder::new();
    let mut composite_segments = Vec::with_capacity(segments.len());
    for (index, segment) in segments.iter().enumerate() {
        let curve = match &segment.segment_type {
            HorizontalSegmentType::Line => push_line(&mut builder, segment)?,
            HorizontalSegmentType::CircularArc => push_arc(&mut builder, segment)?,
            HorizontalSegmentType::Transition(name) if is_exactly_lowerable(name) => {
                push_spiral(&mut builder, segment, name)?
            }
            kind => return Err(AlignmentError::Unsupported {
                entity: segment.entity,
                type_name: kind.source_name().to_owned(),
                detail:
                    "the pinned neutral curve vocabulary has no exact transition-curve primitive",
            }),
        };
        let transition = if index == 0 {
            Transition::Discontinuous
        } else {
            let previous = &segments[index - 1];
            // A transition spiral's end point is a Fresnel-type integral, so
            // continuity across it is not provable in closed form. The strict
            // entry point promises a fully continuity-checked composite, so it
            // refuses here rather than asserting a transition it cannot verify.
            let previous_end =
                closed_form_end_point(previous).ok_or(AlignmentError::Unsupported {
                    entity: previous.entity,
                    type_name: previous.segment_type.source_name().to_owned(),
                    detail: "continuity across a transition spiral is not provable in closed form",
                })?;
            observed_transition(previous_end, segment.start_point, 1e-6).ok_or(
                AlignmentError::SemanticViolation {
                    entity: Some(segment.entity),
                    rule: "consecutive horizontal segments must share an endpoint exactly",
                },
            )?
        };
        composite_segments.push(CurveSegment {
            curve,
            same_sense: true,
            transition,
        });
    }

    let root = push(
        &mut builder,
        GeometryNode::CurveRelation(CurveRelation::Composite {
            segments: composite_segments,
        }),
    )?;
    finish(builder, root, ids)
}

/// Lower a horizontal layout as far as exactness allows, reporting refusals.
///
/// Unlike [`lower_horizontal_layout`], a transition spiral does not abort the
/// whole layout. Lines and circular arcs around it still lower exactly; the
/// spiral is recorded in [`PartialHorizontalLayout::refused`] with its
/// authored type name and entity id.
///
/// Nothing here is approximated. A refused segment stays refused -- this
/// reports the boundary per segment instead of collapsing an entire
/// production alignment into one opaque error.
///
/// Errors that are not a lowering refusal (a wrong entity type, a malformed
/// attribute, an empty layout) still fail the whole call, because they mean
/// the layout could not be read at all rather than that one segment resisted
/// exact lowering.
pub fn lower_horizontal_layout_partial(
    model: &Model,
    entity: EntityId,
    units: AlignmentUnits,
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

    let mut runs = Vec::new();
    let mut refused = Vec::new();
    // Segments accumulated since the last refusal, as (index, node) pairs in a
    // builder that is discarded and restarted whenever a run ends.
    let mut builder = GeometryGraphBuilder::new();
    let mut pending: Vec<(usize, CurveSegment)> = Vec::new();
    let mut pending_ids: Vec<EntityId> = Vec::new();

    for (index, segment) in segments.iter().enumerate() {
        // Decide the run boundary BEFORE lowering: `flush_run` swaps in a new
        // builder, so a node pushed beforehand would dangle in a finished
        // graph. A predecessor whose end point is not closed form cannot
        // support any continuity claim, so the run ends here.
        if let Some((previous_index, _)) = pending.last() {
            if closed_form_end_point(&segments[*previous_index]).is_none() {
                flush_run(&mut runs, &mut builder, &mut pending, &mut pending_ids)?;
            }
        }
        let lowered = match &segment.segment_type {
            HorizontalSegmentType::Line => push_line(&mut builder, segment),
            HorizontalSegmentType::CircularArc => push_arc(&mut builder, segment),
            HorizontalSegmentType::Transition(name) if is_exactly_lowerable(name) => {
                push_spiral(&mut builder, segment, name)
            }
            kind => Err(AlignmentError::Unsupported {
                entity: segment.entity,
                type_name: kind.source_name().to_owned(),
                detail:
                    "the pinned neutral curve vocabulary has no exact transition-curve primitive",
            }),
        };
        let curve = match lowered {
            Ok(curve) => curve,
            Err(reason) => {
                // The run ends here: continuity across a segment this crate
                // did not lower is not a fact it can assert.
                flush_run(&mut runs, &mut builder, &mut pending, &mut pending_ids)?;
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
                let previous = &segments[*previous_index];
                // Within a run the predecessor always has a closed-form end
                // point: the boundary check above ended the run otherwise.
                let previous_end = closed_form_end_point(previous)
                    .expect("run boundary guarantees a closed-form predecessor end point");
                match observed_transition(previous_end, segment.start_point, 1e-6) {
                    Some(transition) => transition,
                    None => {
                        return Err(AlignmentError::SemanticViolation {
                            entity: Some(segment.entity),
                            rule: "consecutive horizontal segments must share an endpoint exactly",
                        })
                    }
                }
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
    flush_run(&mut runs, &mut builder, &mut pending, &mut pending_ids)?;

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
) -> AlignmentResult<()> {
    if pending.is_empty() {
        // Nothing accumulated; drop whatever partial nodes exist so a refused
        // segment does not leak orphan nodes into the next run.
        *builder = GeometryGraphBuilder::new();
        pending_ids.clear();
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
    runs.push(finish(finished, root, sources)?);
    Ok(())
}
