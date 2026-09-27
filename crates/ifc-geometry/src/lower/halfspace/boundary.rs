//! `IfcPolygonalBoundedHalfSpace.PolygonalBoundary` as a 2D curve.
//!
//! # Why this is not `lower_curve_node`
//!
//! The boundary is authored in `Position`'s XY plane: the schema's
//! `BoundaryDim` rule fixes `PolygonalBoundary.Dim = 2`. Axiolid's
//! `SolidOperation::BoundedHalfSpace` contract matches that -- the reference
//! compiler resolves `boundary` as a `Curve2` node and refuses anything else.
//! The general curve lowering emits `Curve3`, so routing the boundary through
//! it produced a graph that validated and lowered cleanly but could never
//! compile (openbimrs/ifc#45). A dedicated 2D path keeps the dimension in the
//! type instead of in a comment.
//!
//! # Units
//!
//! Unlike parameter-space curves, these coordinates are real lengths in the
//! boundary's own frame, so the project length factor applies. `Position`
//! itself is converted separately by the caller and carried on the node.

use axiolid_core::Point2;
use axiolid_curve::{Curve2, Polyline2};
use axiolid_model::{GeometryNode, NodeId};
use ifc_model::EntityId;

use crate::curve::polyline::Polyline;
use crate::error::GeometryResult;
use crate::lower::session::LoweringSession;
use crate::resource::point::CartesianPoint;

/// Lower `PolygonalBoundary` to a `Curve2` node in the boundary's own plane.
///
/// Only `IfcPolyline` is lowered. `IfcCompositeCurve` (all schemas) and
/// `IfcIndexedPolyCurve` (IFC4X3) are legal boundaries and are refused by name
/// rather than approximated; composite profile boundaries are tracked in
/// openbimrs/ifc#43.
pub(super) fn lower_boundary_curve(
    session: &mut LoweringSession<'_>,
    owner: EntityId,
    id: EntityId,
) -> GeometryResult<NodeId> {
    let type_name = session.type_name(id)?;
    if type_name != "IFCPOLYLINE" {
        return Err(session.unsupported(
            id,
            &type_name,
            "polygonal half-space boundary family (only IfcPolyline is lowered; \
             IfcCompositeCurve and IfcIndexedPolyCurve boundaries are not yet)",
        ));
    }

    let entity = session.entity(owner, id)?;
    let view = Polyline::new(id, entity);
    let closed = view.closes_by_repeating_first_point()?;
    let refs = view.point_refs()?;

    let mut points = Vec::with_capacity(refs.len());
    for point_ref in &refs {
        points.push(planar_point(session, id, *point_ref)?);
    }
    // Same convention as the 3D polyline: a closing duplicate is carried by
    // `closed`, not by a repeated vertex that would add a zero-length edge.
    if closed && points.len() > 1 {
        points.pop();
    }
    session.node_for(
        id,
        GeometryNode::Curve2(Curve2::Polyline(Polyline2 { points, closed })),
    )
}

/// One boundary vertex in the boundary plane, converted to metres.
///
/// A 3D point is admitted only with `z == 0` exactly. Exporters do write the
/// zero, and it carries no information; any other value puts the vertex off
/// the plane the schema requires, and projecting it would silently move the
/// clip. No tolerance is applied because the plane is exact by definition.
fn planar_point(
    session: &mut LoweringSession<'_>,
    owner: EntityId,
    point_ref: EntityId,
) -> GeometryResult<Point2> {
    let entity = session.entity(owner, point_ref)?;
    let coordinates = CartesianPoint::new(point_ref, entity).coordinates()?;
    let (x, y) = match coordinates.as_slice() {
        [x, y] => (*x, *y),
        [x, y, z] if *z == 0.0 => (*x, *y),
        [_, _, z] => {
            return Err(session.degenerate(
                point_ref,
                "IFCCARTESIANPOINT",
                format!(
                    "polygonal half-space boundary point has z = {z}; the boundary \
                     must lie in Position's XY plane (BoundaryDim)"
                ),
            ));
        }
        other => {
            return Err(session.degenerate(
                point_ref,
                "IFCCARTESIANPOINT",
                format!("Coordinates has {} entries, expected 2 or 3", other.len()),
            ));
        }
    };
    if !x.is_finite() || !y.is_finite() {
        return Err(session.degenerate(
            point_ref,
            "IFCCARTESIANPOINT",
            "polygonal half-space boundary point must be finite",
        ));
    }
    let units = session.units();
    Ok(Point2::new(units.length(x), units.length(y)))
}
