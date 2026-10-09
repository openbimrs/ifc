//! `IfcIndexedPolyCurve`, and its three-point arcs: the exact arc, or the
//! polyline IFC prescribes.
//!
//! An `IfcArcIndex` names "the start point of the circular arc, ... a point
//! on arc, ... the end point". Its three points are classified once, by
//! [`arc_points`], the helper the profile and half-space boundary reader
//! shares (#335, #393, #396), within the model's `Precision`, in the curve's
//! own coordinates (before the caller's frame):
//!
//! - coincident points define neither a circle nor a polyline segment and
//!   are refused as `Degenerate`;
//! - three distinct collinear points are, in IFC4 ADD2 TC1's and IFC4X3
//!   ADD2's words, "treated as a polyline segment": a `Polyline3` from start
//!   to end, through the middle point when it does not lie between them;
//! - otherwise the exact circle through the points, trimmed at the start
//!   and end points, with every derived value checked finite.

use axiolid_core::{Frame3, Point3, Vec3};
use axiolid_curve::{Circle3, Curve3, Polyline3};
use axiolid_model::{
    CurveRelation, CurveSegment, GeometryNode, NodeId, Transition, TrimSelector,
    TrimmingPreference as KernelPreference,
};
use ifc_model::EntityId;

use crate::constraint::tolerance::{arc_points, model_precision_metres, ArcPoints};
use crate::curve::polyline::{IndexedPolyCurve, PolySegment};
use crate::error::GeometryResult;
use crate::lower::session::LoweringSession;
use crate::resource::point::{CartesianPointList2D, CartesianPointList3D};
use crate::transform::Transform;

fn vec3_is_finite(value: Vec3) -> bool {
    value
        .to_array()
        .into_iter()
        .all(|component| component.is_finite())
}

/// Lower one `IfcArcIndex` of `owner`.
///
/// `local` are its points in metres in the curve's own coordinates, where
/// `precision` (metres) applies; `world` the same points under the frame.
fn indexed_arc(
    session: &mut LoweringSession<'_>,
    owner: EntityId,
    precision: f64,
    local: [[f64; 3]; 3],
    world: [Point3; 3],
) -> GeometryResult<NodeId> {
    let [start, mid, end] = world;
    let points = match arc_points(precision, local[0], local[1], local[2]) {
        ArcPoints::Coincident => {
            return Err(session.degenerate(
                owner,
                "IFCINDEXEDPOLYCURVE",
                "an IfcArcIndex has two points that coincide within the model's Precision, \
                 so no circle is defined",
            ))
        }
        ArcPoints::NonFinite => {
            return Err(session.degenerate(
                owner,
                "IFCINDEXEDPOLYCURVE",
                "arc point arithmetic is non-finite",
            ))
        }
        // "In case that this informal proposition is not maintained, the arc
        // segment shall be treated as a polyline segment."
        ArcPoints::Collinear { through_mid: false } => vec![start, end],
        ArcPoints::Collinear { through_mid: true } => vec![start, mid, end],
        ArcPoints::Circular => return circular_arc(session, owner, start, mid, end),
    };
    session.node_for(
        owner,
        GeometryNode::Curve3(Curve3::Polyline(Polyline3 {
            points,
            closed: false,
        })),
    )
}

/// The circle through three points already classified as an arc.
fn circular_arc(
    session: &mut LoweringSession<'_>,
    owner: EntityId,
    start: Point3,
    mid: Point3,
    end: Point3,
) -> GeometryResult<NodeId> {
    let u = mid - start;
    let v = end - start;
    let normal_raw = u.cross(v);
    let normal_sq = normal_raw.length_squared();
    // Collinearity was judged by `arc_points`; this guards the arithmetic
    // only (an overflowing or vanishing product of finite coordinates).
    if !(normal_sq.is_finite() && normal_sq > 0.0) {
        return Err(session.degenerate(
            owner,
            "IFCINDEXEDPOLYCURVE",
            "arc plane normal arithmetic overflowed or vanished",
        ));
    }
    let center = start
        + (u.length_squared() * v.cross(normal_raw) + v.length_squared() * normal_raw.cross(u))
            / (2.0 * normal_sq);
    if !vec3_is_finite(center) {
        return Err(session.degenerate(
            owner,
            "IFCINDEXEDPOLYCURVE",
            "arc circumcenter arithmetic overflowed",
        ));
    }
    let radial = start - center;
    let radius = radial.length();
    let z = normal_raw.normalize();
    let x = radial / radius;
    let y = z.cross(x);
    let mid_radial = mid - center;
    let end_radial = end - center;
    if !radius.is_finite()
        || radius <= 0.0
        || !vec3_is_finite(radial)
        || !vec3_is_finite(z)
        || !vec3_is_finite(x)
        || !vec3_is_finite(y)
        || !vec3_is_finite(mid_radial)
        || !vec3_is_finite(end_radial)
    {
        return Err(session.degenerate(
            owner,
            "IFCINDEXEDPOLYCURVE",
            "arc derived frame or radius is non-finite or degenerate",
        ));
    }
    let mid_angle = mid_radial
        .dot(y)
        .atan2(mid_radial.dot(x))
        .rem_euclid(std::f64::consts::TAU);
    let end_angle = end_radial
        .dot(y)
        .atan2(end_radial.dot(x))
        .rem_euclid(std::f64::consts::TAU);
    if !mid_angle.is_finite() || !end_angle.is_finite() {
        return Err(session.degenerate(
            owner,
            "IFCINDEXEDPOLYCURVE",
            "arc trim-angle arithmetic is non-finite",
        ));
    }
    let sense_agreement = mid_angle <= end_angle;

    let basis = session.node_for(
        owner,
        GeometryNode::Curve3(Curve3::Circle(Circle3 {
            frame: Frame3 {
                origin: center,
                x,
                y,
                z,
            },
            radius,
        })),
    )?;
    session.node_for(
        owner,
        GeometryNode::CurveRelation(CurveRelation::Trimmed {
            basis,
            start: vec![TrimSelector::Point3(start)],
            end: vec![TrimSelector::Point3(end)],
            sense_agreement,
            preference: KernelPreference::Cartesian,
        }),
    )
}

/// Lower indexed line and arc segments against one shared point list.
///
/// An `IfcArcIndex` is read by [`indexed_arc`]: the exact arc through its
/// three points, or the polyline segment IFC prescribes when they are
/// collinear within the model's `Precision` (#396).
pub(super) fn indexed_polycurve(
    session: &mut LoweringSession<'_>,
    id: EntityId,
    frame: Transform,
) -> GeometryResult<NodeId> {
    let entity = session.entity(id, id)?;
    let view = IndexedPolyCurve::new(id, entity);
    let point_list_ref = view.points_ref()?;
    let local = indexed_points(session, id, point_list_ref)?;
    let points: Vec<Point3> = local
        .iter()
        .map(|p| Point3::from_array(frame.apply(*p)))
        .collect();
    let explicit = view.has_explicit_segments();
    let segments = view.segments(points.len())?;

    if !explicit {
        return session.node_for(
            id,
            GeometryNode::Curve3(Curve3::Polyline(Polyline3 {
                points,
                closed: false,
            })),
        );
    }

    // Read only when the curve has an arc: a line-only curve lowers
    // whatever the contexts declare.
    let mut precision = None;
    let mut children = Vec::with_capacity(segments.len());
    for segment in segments {
        let curve = match segment {
            PolySegment::Line(indices) => {
                let closed = indices.first() == indices.last();
                let mut selected: Vec<_> = indices.into_iter().map(|i| points[i]).collect();
                if closed && selected.len() > 1 {
                    selected.pop();
                }
                session.node_for(
                    id,
                    GeometryNode::Curve3(Curve3::Polyline(Polyline3 {
                        points: selected,
                        closed,
                    })),
                )?
            }
            PolySegment::Arc { start, mid, end } => {
                let precision = match precision {
                    Some(precision) => precision,
                    None => {
                        *precision.insert(model_precision_metres(session.model(), session.units())?)
                    }
                };
                indexed_arc(
                    session,
                    id,
                    precision,
                    [local[start], local[mid], local[end]],
                    [points[start], points[mid], points[end]],
                )?
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

/// The point list in metres, in the curve's own coordinates: 2D points get
/// `z = 0`. The caller applies the frame.
fn indexed_points(
    session: &LoweringSession<'_>,
    owner: EntityId,
    list_id: EntityId,
) -> GeometryResult<Vec<[f64; 3]>> {
    let entity = session.entity(owner, list_id)?;
    let raw: Vec<[f64; 3]> = match entity.type_name.to_ascii_uppercase().as_str() {
        "IFCCARTESIANPOINTLIST2D" => CartesianPointList2D::new(list_id, entity)
            .coordinates()?
            .into_iter()
            .map(|p| [p[0], p[1], 0.0])
            .collect(),
        "IFCCARTESIANPOINTLIST3D" => CartesianPointList3D::new(list_id, entity).coordinates()?,
        other => {
            return Err(session.unsupported(
                list_id,
                other,
                "indexed curve requires IfcCartesianPointList2D or IfcCartesianPointList3D",
            ));
        }
    };
    if raw.len() < 2 {
        return Err(session.degenerate(
            list_id,
            &entity.type_name,
            "point list needs at least two points",
        ));
    }
    raw.into_iter()
        .map(|p| {
            let metres = p.map(|value| session.units().length(value));
            if !metres.iter().all(|value| value.is_finite()) {
                return Err(session.degenerate(
                    list_id,
                    &entity.type_name,
                    "coordinates must be finite",
                ));
            }
            Ok(metres)
        })
        .collect()
}
