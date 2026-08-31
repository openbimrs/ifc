//! Numerically guarded lowering for three-point indexed arcs.

use axiolid_core::{Frame3, Point3, Vec3};
use axiolid_curve::{Circle3, Curve3};
use axiolid_model::{
    CurveRelation, GeometryNode, NodeId, TrimSelector, TrimmingPreference as KernelPreference,
};
use ifc_model::EntityId;

use crate::error::GeometryResult;
use crate::lower::session::LoweringSession;

fn vec3_is_finite(value: Vec3) -> bool {
    value
        .to_array()
        .into_iter()
        .all(|component| component.is_finite())
}

pub(super) fn indexed_arc(
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
    if !normal_sq.is_finite() || normal_sq <= f64::EPSILON {
        return Err(session.degenerate(
            owner,
            "IFCINDEXEDPOLYCURVE",
            "arc points are collinear or non-finite",
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
