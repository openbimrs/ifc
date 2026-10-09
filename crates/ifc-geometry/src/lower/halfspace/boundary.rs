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
//!
//! # Composite and indexed boundaries (#393)
//!
//! `BoundaryType` admits `IfcPolyline` and `IfcCompositeCurve` (IFC2X3, IFC4
//! ADD2 TC1) and also `IfcIndexedPolyCurve` (IFC4X3 ADD2). The two curve
//! families are read by the profile boundary readers (`lower::profile`'s
//! composite walk and #335's indexed reading) under
//! [`BoundaryRole::HalfSpace`], so `SameSense`, nesting, the collinear-arc
//! fallback and the closure rules are the profiles' own, and the resulting
//! straight-edged contour becomes one closed `Polyline2`.
//!
//! Axiolid's `BoundedHalfSpace` takes nothing but a `Polyline2`: the
//! reference compiler (`axiolid-mesh-compile`, both its mesh and its exact
//! clip) refuses every other `Curve2`, and `Curve2` has no composite variant.
//! A circular arc (a trimmed `IfcCircle`, a non-collinear `IfcArcIndex`) is
//! therefore refused as `Unsupported` by name; chording it would move the
//! clip.

use axiolid_core::Point2;
use axiolid_curve::{Curve2, Polyline2};
use axiolid_model::{GeometryNode, NodeId};
use axiolid_profile::Contour;
use ifc_model::EntityId;

use crate::curve::polyline::Polyline;
use crate::error::{GeometryError, GeometryResult};
use crate::lower::profile::{curve_to_contour, BoundaryRole};
use crate::lower::session::LoweringSession;
use crate::resource::point::CartesianPoint;

/// Lower `PolygonalBoundary` to a `Curve2` node in the boundary's own plane.
///
/// `IfcPolyline` is lowered point for point. `IfcCompositeCurve` and
/// `IfcIndexedPolyCurve` are lowered when closed and straight-edged (see the
/// module docs); any other curve family is refused by name.
pub(super) fn lower_boundary_curve(
    session: &mut LoweringSession<'_>,
    owner: EntityId,
    id: EntityId,
) -> GeometryResult<NodeId> {
    let type_name = session.type_name(id)?;
    match type_name.as_str() {
        "IFCPOLYLINE" => {}
        "IFCCOMPOSITECURVE" | "IFCINDEXEDPOLYCURVE" => {
            let contour = curve_to_contour(
                session.model(),
                id,
                session.units(),
                BoundaryRole::HalfSpace,
            )?;
            let points = contour_vertices(id, &type_name, &contour)?;
            return session.node_for(
                id,
                GeometryNode::Curve2(Curve2::Polyline(Polyline2 {
                    points,
                    closed: true,
                })),
            );
        }
        _ => {
            return Err(session.unsupported(
                id,
                &type_name,
                "polygonal half-space boundary family (IfcPolyline, IfcCompositeCurve and \
                 IfcIndexedPolyCurve are lowered)",
            ))
        }
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

/// The vertices of a closed, straight-edged contour, in traversal order.
///
/// Each edge contributes its start point, so the closing edge is carried by
/// `closed`, as for an `IfcPolyline`. Joints already met within the model's
/// `Precision` (the readers refuse a gap), so the start of an edge stands
/// for the end of the one before. The readers emit only straight edges for
/// this role; anything else is refused rather than trusted.
fn contour_vertices(
    id: EntityId,
    type_name: &str,
    contour: &Contour,
) -> GeometryResult<Vec<Point2>> {
    let mut points = Vec::with_capacity(contour.segments.len());
    for segment in &contour.segments {
        let Curve2::Line(line) = &segment.curve else {
            return Err(GeometryError::Unsupported {
                entity: id,
                type_name: type_name.to_owned(),
                detail: crate::lower::profile::HALF_SPACE_ARC,
            });
        };
        let t = if segment.same_sense {
            segment.domain.start
        } else {
            segment.domain.end
        };
        points.push(line.origin + line.direction * t);
    }
    if points.len() < 3 {
        return Err(GeometryError::Degenerate {
            entity: id,
            type_name: type_name.to_owned(),
            detail: format!(
                "a polygonal half-space boundary needs at least 3 edges, this one has {}",
                points.len()
            ),
        });
    }
    Ok(points)
}

/// One boundary vertex in the boundary plane, converted to metres.
///
/// A 3D point is admitted only with `z == 0` exactly. Exporters do write the
/// zero, and it carries no information; any other value puts the vertex off
/// the plane the schema requires, and projecting it would silently move the
/// clip. No tolerance is applied because the plane is exact by definition.
/// The rule is [`BoundaryRole::HalfSpace`]'s, shared with composite and
/// indexed boundaries.
fn planar_point(
    session: &mut LoweringSession<'_>,
    owner: EntityId,
    point_ref: EntityId,
) -> GeometryResult<Point2> {
    let entity = session.entity(owner, point_ref)?;
    let coordinates = CartesianPoint::new(point_ref, entity).coordinates()?;
    let [x, y] = BoundaryRole::HalfSpace.planar(point_ref, "IFCCARTESIANPOINT", &coordinates)?;
    let units = session.units();
    Ok(Point2::new(units.length(x), units.length(y)))
}
