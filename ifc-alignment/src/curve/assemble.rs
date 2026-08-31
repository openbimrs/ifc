//! Exact neutral graph assembly for horizontal alignment segments.

use axiolid_core::{Frame2, Point2, Vec2};
use axiolid_curve::{Circle2, Curve2, Line2};
use axiolid_model::{
    CurveRelation, GeometryGraph, GeometryGraphBuilder, GeometryNode, NodeId, TrimSelector,
    TrimmingPreference,
};
use ifc_model::{EntityId, Model};

use crate::error::{AlignmentError, AlignmentResult};
use crate::horizontal::{
    read_horizontal_segment, AlignmentUnits, HorizontalSegment, HorizontalSegmentType,
};
use crate::vertical::{read_vertical_segment, VerticalSegmentType};

/// Exact neutral curve graph for one IFC alignment segment.
#[derive(Debug, Clone, PartialEq)]
pub struct LoweredAlignmentCurve {
    pub graph: GeometryGraph,
    pub root: NodeId,
    pub source: EntityId,
}

pub fn lower_horizontal_segment(
    model: &Model,
    id: EntityId,
    units: AlignmentUnits,
) -> AlignmentResult<LoweredAlignmentCurve> {
    let segment = read_horizontal_segment(model, id, units)?;
    match &segment.segment_type {
        HorizontalSegmentType::Line => lower_line(segment),
        HorizontalSegmentType::CircularArc => lower_arc(segment),
        kind => Err(AlignmentError::Unsupported {
            entity: id,
            type_name: kind.source_name().to_owned(),
            detail: "the pinned neutral curve vocabulary has no exact transition-curve primitive",
        }),
    }
}

pub fn lower_vertical_segment(
    model: &Model,
    id: EntityId,
    units: AlignmentUnits,
) -> AlignmentResult<LoweredAlignmentCurve> {
    let segment = read_vertical_segment(model, id, units)?;
    if !matches!(
        segment.predefined_type,
        VerticalSegmentType::ConstantGradient
    ) {
        return Err(AlignmentError::Unsupported {
            entity: id,
            type_name: segment.predefined_type.source_name().to_owned(),
            detail: "exact neutral vertical lowering is currently limited to constant gradient",
        });
    }
    if segment.radius_of_curvature.is_some() || segment.start_gradient != segment.end_gradient {
        return Err(AlignmentError::InvalidSegment {
            entity: id,
            detail: "CONSTANTGRADIENT requires equal gradients and no curvature radius",
        });
    }
    let mut builder = GeometryGraphBuilder::new();
    let basis = push(
        &mut builder,
        GeometryNode::Curve2(Curve2::Line(Line2 {
            origin: Point2::new(segment.start_dist_along, segment.start_height),
            direction: Vec2::new(1.0, segment.start_gradient),
        })),
    )?;
    let root = push(
        &mut builder,
        GeometryNode::CurveRelation(CurveRelation::Trimmed {
            basis,
            start: vec![TrimSelector::Parameter(0.0)],
            end: vec![TrimSelector::Parameter(segment.horizontal_length)],
            sense_agreement: true,
            preference: TrimmingPreference::Parameter,
        }),
    )?;
    finish(builder, root, id)
}

fn lower_line(segment: HorizontalSegment) -> AlignmentResult<LoweredAlignmentCurve> {
    if segment.start_radius != 0.0 || segment.end_radius != 0.0 {
        return Err(AlignmentError::InvalidSegment {
            entity: segment.entity,
            detail: "LINE requires zero start and end radii",
        });
    }
    let direction = Vec2::new(segment.start_direction.cos(), segment.start_direction.sin());
    let mut builder = GeometryGraphBuilder::new();
    let basis = push(
        &mut builder,
        GeometryNode::Curve2(Curve2::Line(Line2 {
            origin: segment.start_point,
            direction,
        })),
    )?;
    let root = push(
        &mut builder,
        GeometryNode::CurveRelation(CurveRelation::Trimmed {
            basis,
            start: vec![TrimSelector::Parameter(0.0)],
            end: vec![TrimSelector::Parameter(segment.segment_length)],
            sense_agreement: true,
            preference: TrimmingPreference::Parameter,
        }),
    )?;
    finish(builder, root, segment.entity)
}

fn lower_arc(segment: HorizontalSegment) -> AlignmentResult<LoweredAlignmentCurve> {
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

    let mut builder = GeometryGraphBuilder::new();
    let basis = push(
        &mut builder,
        GeometryNode::Curve2(Curve2::Circle(Circle2 {
            frame: Frame2 {
                origin: centre,
                x,
                y,
            },
            radius,
        })),
    )?;
    let root = push(
        &mut builder,
        GeometryNode::CurveRelation(CurveRelation::Trimmed {
            basis,
            start: vec![TrimSelector::Parameter(0.0)],
            end: vec![TrimSelector::Parameter(sweep)],
            sense_agreement: true,
            preference: TrimmingPreference::Parameter,
        }),
    )?;
    finish(builder, root, segment.entity)
}

fn push(builder: &mut GeometryGraphBuilder, node: GeometryNode) -> AlignmentResult<NodeId> {
    builder.push(node).map_err(|error| AlignmentError::Graph {
        detail: error.to_string(),
    })
}

fn finish(
    builder: GeometryGraphBuilder,
    root: NodeId,
    source: EntityId,
) -> AlignmentResult<LoweredAlignmentCurve> {
    let graph = builder
        .finish(vec![root])
        .map_err(|error| AlignmentError::Graph {
            detail: error.to_string(),
        })?;
    Ok(LoweredAlignmentCurve {
        graph,
        root,
        source,
    })
}
