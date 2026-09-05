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
use axiolid_curve::{Circle2, Curve2, Ellipse2, Line2, Polyline2};
use axiolid_model::{GeometryNode, NodeId};
use ifc_model::EntityId;

use crate::curve::conic::{Circle, Ellipse};
use crate::curve::line::Line;
use crate::curve::polyline::{IndexedPolyCurve, Polyline};
use crate::error::GeometryResult;
use crate::lower::session::LoweringSession;
use crate::resource::direction::resolve_unit;
use crate::resource::placement::Axis2Placement2D;
use crate::resource::point::{CartesianPoint, CartesianPointList2D};

/// Parameter coordinates are dimensionless/mixed-domain values, not model
/// lengths. Supports the exact polyline form, the implicit-order (no explicit
/// `Segments`, or all-line `IfcLineIndex`) `IfcIndexedPolyCurve` form, and the
/// analytic `IfcLine`/`IfcCircle`/`IfcEllipse` families, all of which read
/// their defining values directly with no evaluation and no unit conversion.
/// Remaining forms (explicit-arc indexed polycurves, B-splines, trimmed and
/// composite curves) stay typed unsupported rather than receiving a wrong
/// uniform unit scale or an unimplemented parameter-space contract.
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
        _ => Err(session.unsupported(
            id,
            &type_name,
            "parameter-space curve family (only exact IfcPolyline, line-only \
             IfcIndexedPolyCurve, IfcLine, IfcCircle and IfcEllipse are currently supported)",
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

/// `IfcIndexedPolyCurve` with no explicit `Segments`, or with `Segments`
/// consisting only of `IfcLineIndex` entries, reads identically to a plain
/// ordered point sequence: no arc evaluation is needed. An arc segment would
/// require an exact parameter-space arc contract this crate does not carry,
/// so that case stays a named typed refusal rather than being flattened to a
/// straight line.
fn parameter_space_indexed_polycurve(
    session: &mut LoweringSession<'_>,
    owner: EntityId,
    id: EntityId,
) -> GeometryResult<NodeId> {
    let entity = session.entity(owner, id)?;
    let view = IndexedPolyCurve::new(id, entity);
    if view.has_explicit_segments() {
        return Err(session.unsupported(
            id,
            "IFCINDEXEDPOLYCURVE",
            "parameter-space indexed polycurve with explicit (non-line) segments is not yet represented",
        ));
    }
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
    let closed = points.first() == points.last() && points.len() > 1;
    session.node_for(
        id,
        GeometryNode::Curve2(Curve2::Polyline(Polyline2 { points, closed })),
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

#[cfg(test)]
mod tests;
