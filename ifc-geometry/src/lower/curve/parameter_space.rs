//! Parameter-space (p-curve) reference curve lowering.
//!
//! # The invariant: nothing here is a length
//!
//! Every value read in this module is a coordinate in a surface's own (u, v)
//! parameter domain, never a model length. That domain is dimensionless or
//! mixed: on a cylinder u is an angle and v is a length, so there is no single
//! factor that could correctly convert it. Applying the project length factor
//! would rescale the u axis of every millimetre file by 1000; applying the
//! plane-angle factor would corrupt v. This module therefore reads defining
//! values verbatim and performs no unit conversion at all.
//!
//! That is exactly the opposite of `super`, which lowers the same entity types
//! as world-space geometry *with* unit conversion applied. `IfcCircle` reached
//! through `lower_curve_node` scales its radius; the same `IfcCircle` reached
//! as an `IfcPCurve` reference curve must not. Keeping the two paths in
//! separate modules keeps that distinction visible rather than relying on a
//! reader noticing which helper was called.

use axiolid_core::{Frame2, Point2, Vec2};
use axiolid_curve::{BSplineCurve2, Circle2, Curve2, Ellipse2, Line2, Polyline2};
use axiolid_model::{
    CurveRelation, CurveSegment, GeometryNode, NodeId, Transition, TrimSelector, TrimmingPreference,
};
use ifc_model::EntityId;

use crate::curve::bspline::BSplineCurve;
use crate::curve::conic::{Circle, Ellipse};
use crate::curve::line::Line;
use crate::curve::polyline::{IndexedPolyCurve, PolySegment, Polyline};
use crate::error::GeometryResult;
use crate::lower::curve::{bspline_knot_spec, finite_values};
use crate::lower::session::LoweringSession;
use crate::resource::direction::resolve_unit;
use crate::resource::placement::Axis2Placement2D;
use crate::resource::point::{CartesianPoint, CartesianPointList2D};

/// Parameter coordinates are dimensionless/mixed-domain values, not model
/// lengths. Supports the exact polyline form, the `IfcIndexedPolyCurve` form
/// including explicit `IfcArcIndex` segments, and the analytic
/// `IfcLine`/`IfcCircle`/`IfcEllipse` and explicit-knot B-spline families, all
/// of which read their defining values directly with no unit conversion.
/// Remaining forms (convention-only base splines, trimmed and composite
/// curves) stay typed unsupported rather than receiving a wrong uniform unit
/// scale or an unimplemented parameter-space contract.
///
/// # No unit conversion in parameter space
///
/// Every value read here is a surface parameter, not a length. `IfcCircle`
/// lowered as a 3D curve multiplies its radius by the project length factor;
/// a p-curve circle must not, because its "radius" is a displacement in the
/// surface's own (u, v) domain, which for a cylinder mixes an angle with a
/// length. Applying the length factor would rescale the u axis of every
/// millimetre file by 1000. The same reasoning bars the angle factor.
pub(super) fn parameter_reference_curve(
    session: &mut LoweringSession<'_>,
    owner: EntityId,
    id: EntityId,
) -> GeometryResult<NodeId> {
    let type_name = session.type_name(id)?;
    match type_name.as_str() {
        "IFCPOLYLINE" => parameter_space_polyline(session, owner, id),
        "IFCINDEXEDPOLYCURVE" => parameter_space_indexed_polycurve(session, owner, id),
        "IFCLINE" => parameter_space_line(session, owner, id),
        "IFCCIRCLE" => parameter_space_circle(session, owner, id),
        "IFCELLIPSE" => parameter_space_ellipse(session, owner, id),
        "IFCBSPLINECURVEWITHKNOTS" | "IFCRATIONALBSPLINECURVEWITHKNOTS" => {
            parameter_space_bspline(session, owner, id)
        }
        _ => Err(session.unsupported(
            id,
            &type_name,
            "parameter-space curve family (only exact IfcPolyline, line-only \
             IfcIndexedPolyCurve, IfcLine, IfcCircle, IfcEllipse and \
             explicit-knot B-splines are supported)",
        )),
    }
}

fn parameter_space_polyline(
    session: &mut LoweringSession<'_>,
    owner: EntityId,
    id: EntityId,
) -> GeometryResult<NodeId> {
    let entity = session.entity(owner, id)?;
    let view = Polyline::new(id, entity);
    let point_refs = view.point_refs()?;
    let closed = point_refs.first() == point_refs.last();
    let points = parameter_space_points(session, owner, &point_refs)?;
    session.node_for(
        id,
        GeometryNode::Curve2(Curve2::Polyline(Polyline2 { points, closed })),
    )
}

/// `IfcIndexedPolyCurve` in parameter space, with or without explicit
/// `Segments`. Line runs read as ordered point sequences; an `IfcArcIndex`
/// composes exactly through [`parameter_space_arc`] rather than being
/// flattened to a chord.
fn parameter_space_indexed_polycurve(
    session: &mut LoweringSession<'_>,
    owner: EntityId,
    id: EntityId,
) -> GeometryResult<NodeId> {
    let entity = session.entity(owner, id)?;
    let view = IndexedPolyCurve::new(id, entity);
    let explicit = view.has_explicit_segments();
    let point_list_ref = view.points_ref()?;
    let list_entity = session.entity(owner, point_list_ref)?;
    if !list_entity
        .type_name
        .eq_ignore_ascii_case("IFCCARTESIANPOINTLIST2D")
    {
        return Err(session.unsupported(
            point_list_ref,
            &list_entity.type_name,
            "parameter-space indexed polycurve needs a 2D point list",
        ));
    }
    let coordinates = CartesianPointList2D::new(point_list_ref, list_entity).coordinates()?;
    let mut points = Vec::with_capacity(coordinates.len());
    for xy in coordinates {
        if !xy.iter().all(|value| value.is_finite()) {
            return Err(session.degenerate(
                point_list_ref,
                "IFCCARTESIANPOINTLIST2D",
                "parameter-space point must contain two finite coordinates",
            ));
        }
        points.push(Point2::from_array(xy));
    }
    let segments = view.segments(points.len())?;
    let closed = points.first() == points.last() && points.len() > 1;
    if !explicit {
        return session.node_for(
            id,
            GeometryNode::Curve2(Curve2::Polyline(Polyline2 { points, closed })),
        );
    }

    // Explicit segments: mirror the 3D path exactly, composing line runs and
    // three-point arcs into one composite curve.
    let mut children = Vec::with_capacity(segments.len());
    for segment in segments {
        let curve = match segment {
            PolySegment::Line(indices) => {
                let run_closed = indices.first() == indices.last();
                let mut selected: Vec<_> = indices.into_iter().map(|i| points[i]).collect();
                if run_closed && selected.len() > 1 {
                    selected.pop();
                }
                session.node_for(
                    id,
                    GeometryNode::Curve2(Curve2::Polyline(Polyline2 {
                        points: selected,
                        closed: run_closed,
                    })),
                )?
            }
            PolySegment::Arc { start, mid, end } => {
                parameter_space_arc(session, id, points[start], points[mid], points[end])?
            }
        };
        children.push(CurveSegment {
            curve,
            same_sense: true,
            transition: Transition::Continuous,
        });
    }
    session.node_for(
        id,
        GeometryNode::CurveRelation(CurveRelation::Composite { segments: children }),
    )
}

/// A single parameter-space coordinate pair, read without unit conversion.
fn parameter_space_point(
    session: &mut LoweringSession<'_>,
    owner: EntityId,
    point_ref: EntityId,
) -> GeometryResult<Point2> {
    let entity = session.entity(owner, point_ref)?;
    let coordinates = CartesianPoint::new(point_ref, entity).coordinates()?;
    if coordinates.len() != 2 || !coordinates.iter().all(|value| value.is_finite()) {
        return Err(session.degenerate(
            point_ref,
            "IFCCARTESIANPOINT",
            "parameter-space point must contain two finite coordinates",
        ));
    }
    Ok(Point2::from_array([coordinates[0], coordinates[1]]))
}

/// The parameter-space frame of a conic's `Position`.
///
/// Only `IfcAxis2Placement2D` is admitted: a parameter-space conic lives in
/// the surface's two-dimensional (u, v) domain, so a 3D placement would carry
/// an axis that has no meaning there. Y is the `IfcOrthogonalComplement` of X
/// (X rotated a quarter turn counter-clockwise), matching the 2D placement
/// view rather than being read from the file.
fn parameter_space_frame(
    session: &mut LoweringSession<'_>,
    owner: EntityId,
    position_ref: EntityId,
) -> GeometryResult<Frame2> {
    let position = session.entity(owner, position_ref)?;
    if !position
        .type_name
        .eq_ignore_ascii_case("IFCAXIS2PLACEMENT2D")
    {
        return Err(session.unsupported(
            position_ref,
            &position.type_name,
            "parameter-space conic needs an IfcAxis2Placement2D; a 3D placement \
             has no meaning in a surface parameter domain",
        ));
    }
    let view = Axis2Placement2D::new(position_ref, position);
    let origin = view.location(session.model())?;
    let x = view.ref_direction(session.model())?;
    if !origin.iter().chain(x.iter()).all(|value| value.is_finite()) {
        return Err(session.degenerate(
            position_ref,
            "IFCAXIS2PLACEMENT2D",
            "parameter-space placement must be finite",
        ));
    }
    if x[0].hypot(x[1]) == 0.0 {
        return Err(session.degenerate(
            position_ref,
            "IFCAXIS2PLACEMENT2D",
            "parameter-space placement RefDirection is zero-length",
        ));
    }
    Ok(Frame2 {
        origin: Point2::from_array([origin[0], origin[1]]),
        x: Vec2::new(x[0], x[1]),
        y: Vec2::new(-x[1], x[0]),
    })
}

/// `IfcLine` in parameter space: origin plus the `Dir` vector, unscaled.
///
/// `Dir` is an `IfcVector`, so it carries a magnitude that sets the parameter
/// scale. Neither the magnitude nor the origin receives the length factor
/// here, and the vector is deliberately left un-normalized so a trim taken on
/// this line keeps its authored parameterisation.
fn parameter_space_line(
    session: &mut LoweringSession<'_>,
    owner: EntityId,
    id: EntityId,
) -> GeometryResult<NodeId> {
    let entity = session.entity(owner, id)?;
    let view = Line::new(id, entity);
    let point_ref = view.point_ref()?;
    let vector_ref = view.direction_vector_ref()?;
    let origin = parameter_space_point(session, owner, point_ref)?;

    let vector_entity = session.entity(owner, vector_ref)?;
    let slots = crate::slots::Slots::new(vector_ref, vector_entity);
    let direction_ref = slots.req_ref(0, "Orientation")?;
    let magnitude = slots.req_f64(1, "Magnitude")?;
    let unit = resolve_unit(session.model(), owner, direction_ref)?;
    let direction = Vec2::new(unit[0] * magnitude, unit[1] * magnitude);
    if !magnitude.is_finite() || !direction.x.is_finite() || !direction.y.is_finite() {
        return Err(session.degenerate(
            vector_ref,
            "IFCVECTOR",
            "parameter-space line direction must be finite",
        ));
    }
    if direction.x == 0.0 && direction.y == 0.0 {
        return Err(session.degenerate(
            vector_ref,
            "IFCVECTOR",
            "parameter-space line direction is zero-length",
        ));
    }
    session.node_for(
        id,
        GeometryNode::Curve2(Curve2::Line(Line2 { origin, direction })),
    )
}

/// `IfcCircle` in parameter space: an unscaled radius in the placement frame.
fn parameter_space_circle(
    session: &mut LoweringSession<'_>,
    owner: EntityId,
    id: EntityId,
) -> GeometryResult<NodeId> {
    let entity = session.entity(owner, id)?;
    let view = Circle::new(id, entity);
    let radius = view.radius()?;
    let position_ref = view.position_ref()?;
    if !radius.is_finite() {
        return Err(session.degenerate(
            id,
            "IFCCIRCLE",
            "parameter-space circle radius must be finite",
        ));
    }
    let frame = parameter_space_frame(session, owner, position_ref)?;
    session.node_for(
        id,
        GeometryNode::Curve2(Curve2::Circle(Circle2 { frame, radius })),
    )
}

/// `IfcEllipse` in parameter space: unscaled semi-axes in the placement frame.
fn parameter_space_ellipse(
    session: &mut LoweringSession<'_>,
    owner: EntityId,
    id: EntityId,
) -> GeometryResult<NodeId> {
    let entity = session.entity(owner, id)?;
    let view = Ellipse::new(id, entity);
    let (semi_axis_x, semi_axis_y) = view.semi_axes()?;
    if !semi_axis_x.is_finite() || !semi_axis_y.is_finite() {
        return Err(session.degenerate(
            id,
            "IFCELLIPSE",
            "parameter-space ellipse semi-axes must be finite",
        ));
    }
    let position_ref = view.position_ref()?;
    let frame = parameter_space_frame(session, owner, position_ref)?;
    session.node_for(
        id,
        GeometryNode::Curve2(Curve2::Ellipse(Ellipse2 {
            frame,
            semi_axis_x,
            semi_axis_y,
        })),
    )
}

/// Read an ordered list of `IfcCartesianPoint` references as 2D parameter
/// coordinates, unscaled: parameter-space values are dimensionless/mixed and
/// must never receive the project length factor.
fn parameter_space_points(
    session: &mut LoweringSession<'_>,
    owner: EntityId,
    point_refs: &[EntityId],
) -> GeometryResult<Vec<Point2>> {
    let mut points = Vec::with_capacity(point_refs.len());
    for &point_ref in point_refs {
        points.push(parameter_space_point(session, owner, point_ref)?);
    }
    Ok(points)
}

/// Explicit-knot B-spline in parameter space.
fn parameter_space_bspline(
    session: &mut LoweringSession<'_>,
    owner: EntityId,
    id: EntityId,
) -> GeometryResult<NodeId> {
    let type_name = session.type_name(id)?;
    let entity = session.entity(owner, id)?;
    let view = BSplineCurve::new(id, entity);
    let degree = u16::try_from(view.degree()?)
        .map_err(|_| session.degenerate(id, &type_name, "Degree exceeds u16"))?;
    let knots = view.knots()?.ok_or_else(|| {
        session.unsupported(id, &type_name, "explicit knots required in parameter space")
    })?;
    finite_values(session, id, &type_name, "Knots", &knots.values)?;
    let declared: u128 = knots.multiplicities.iter().map(|m| *m as u128).sum();
    session.check_aggregate(id, &type_name, "knot multiplicities", declared)?;
    let multiplicities = knots
        .multiplicities
        .into_iter()
        .map(|value| {
            u32::try_from(value)
                .map_err(|_| session.degenerate(id, &type_name, "multiplicity exceeds u32"))
        })
        .collect::<GeometryResult<Vec<_>>>()?;

    let refs = view.control_point_refs()?;
    let control_points = parameter_space_points(session, owner, &refs)?;
    let weights = view.weights()?;
    if let Some(values) = weights.as_deref() {
        finite_values(session, id, &type_name, "WeightsData", values)?;
    }
    let closed = view.closed_curve().ok_or_else(|| {
        session.unsupported(
            id,
            &type_name,
            "unknown ClosedCurve is not lossless in bool",
        )
    })?;
    session.node_for(
        id,
        GeometryNode::Curve2(Curve2::BSpline(BSplineCurve2 {
            degree,
            control_points,
            knots: knots.values,
            multiplicities,
            weights,
            closed,
            self_intersect: view.self_intersect(),
            knot_spec: bspline_knot_spec(view.knot_spec()),
        })),
    )
}

#[cfg(test)]
mod tests;

/// A three-point arc in parameter space.
///
/// Mirrors the 3D indexed-arc path: circumcenter, then a trimmed circle.
/// 2D is the simpler case -- there is no plane normal to derive, and the
/// sweep sense is the sign of the 2D cross product. No unit conversion:
/// these are surface parameters, not lengths.
fn parameter_space_arc(
    session: &mut LoweringSession<'_>,
    owner: EntityId,
    start: Point2,
    mid: Point2,
    end: Point2,
) -> GeometryResult<NodeId> {
    let u = mid - start;
    let v = end - start;
    // 2D cross product: zero means the three points are collinear, so no
    // circle exists. Refuse rather than emit a degenerate arc.
    let cross = u.x * v.y - u.y * v.x;
    if !cross.is_finite() || cross == 0.0 {
        return Err(session.degenerate(
            owner,
            "IFCINDEXEDPOLYCURVE",
            "parameter-space arc points are collinear or non-finite",
        ));
    }
    let uu = u.length_squared();
    let vv = v.length_squared();
    let center = start
        + Vec2::new(
            (uu * v.y - vv * u.y) / (2.0 * cross),
            (vv * u.x - uu * v.x) / (2.0 * cross),
        );
    let radial = start - center;
    let radius = radial.length();
    if !radius.is_finite() || radius <= 0.0 {
        return Err(session.degenerate(
            owner,
            "IFCINDEXEDPOLYCURVE",
            "parameter-space arc circumcenter arithmetic overflowed",
        ));
    }
    let x = radial / radius;
    // Frame2 Y is X rotated a quarter turn counter-clockwise, matching the
    // conic helpers in this module. The cross sign then gives the sweep.
    let frame = Frame2 {
        origin: center,
        x,
        y: Vec2::new(-x.y, x.x),
    };
    let basis = session.node_for(
        owner,
        GeometryNode::Curve2(Curve2::Circle(Circle2 { frame, radius })),
    )?;
    session.node_for(
        owner,
        GeometryNode::CurveRelation(CurveRelation::Trimmed {
            basis,
            start: vec![TrimSelector::Point2(start)],
            end: vec![TrimSelector::Point2(end)],
            sense_agreement: cross > 0.0,
            preference: TrimmingPreference::Cartesian,
        }),
    )
}
