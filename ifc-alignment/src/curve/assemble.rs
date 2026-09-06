//! Exact neutral graph assembly for alignment segments and their layouts.

use axiolid_core::{Frame2, Point2, Vec2};
use axiolid_curve::{Circle2, Curve2, Line2};
use axiolid_model::{
    CurveRelation, CurveSegment, GeometryGraph, GeometryGraphBuilder, GeometryNode, NodeId,
    Transition, TrimSelector, TrimmingPreference,
};
use ifc_model::{EntityId, Model};

use crate::error::{AlignmentError, AlignmentResult};
use crate::horizontal::{
    read_horizontal_segment, AlignmentUnits, HorizontalSegment, HorizontalSegmentType,
};
use crate::vertical::{read_vertical_segment, VerticalSegment, VerticalSegmentType};
use crate::view::AlignmentView;

/// Exact neutral curve graph for one or more IFC alignment segments.
#[derive(Debug, Clone, PartialEq)]
pub struct LoweredAlignmentCurve {
    pub graph: GeometryGraph,
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
            kind => return Err(AlignmentError::Unsupported {
                entity: id,
                type_name: kind.source_name().to_owned(),
                detail:
                    "the pinned neutral curve vocabulary has no exact transition-curve primitive",
            }),
        };
    finish(builder, root, vec![id])
}

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
            let previous_end_angle = previous.start_direction
                + previous.segment_length
                    / if previous.start_radius != 0.0 {
                        previous.start_radius
                    } else {
                        f64::INFINITY
                    };
            let previous_end = if previous.start_radius == 0.0 {
                previous.start_point
                    + Vec2::new(previous_end_angle.cos(), previous_end_angle.sin())
                        * previous.segment_length
            } else {
                let direction = Vec2::new(
                    previous.start_direction.cos(),
                    previous.start_direction.sin(),
                );
                let left = Vec2::new(-direction.y, direction.x);
                let centre = previous.start_point + left * previous.start_radius;
                let sweep = previous.segment_length / previous.start_radius;
                let radial = previous.start_point - centre;
                let cos_s = sweep.cos();
                let sin_s = sweep.sin();
                centre
                    + Vec2::new(
                        radial.x * cos_s - radial.y * sin_s,
                        radial.x * sin_s + radial.y * cos_s,
                    )
            };
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
