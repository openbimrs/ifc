//! Swept-area solids placed in world coordinates.
//!
//! Takes the [`LocalSweptSolid`] that `local::describe` read and applies the
//! same frame composition lowering does: the enclosing frame, then the
//! solid's `Position`. `ExtrudedDirection` and the revolution axis are
//! expressed in that `Position` system, so they are carried through the
//! composed frame and normalised once. A frame that is not rigid yields no
//! world description.

use ifc_model::{Entity, EntityId};

use super::{LocalSweepPath, LocalSweptSolid, SweepPath, SweptSolid};
use crate::error::GeometryResult;
use crate::slots::Slots;
use crate::transform::Transform;

/// How far a frame may drift from a proper rotation and still count as rigid.
///
/// Placements and operators are orthonormalised from `f64` ratios, so a rigid
/// frame is exact to a few ulps; an authored scale of even 1.001 is far
/// outside this.
const RIGID_TOLERANCE: f64 = 1e-9;

/// Place a described swept-area solid in `frame`.
///
/// `Ok(None)` when `frame` is not rigid: a mapping that scales or mirrors the
/// item would make the world profile parameters differ from the authored
/// ones, and [`LocalSweptSolid`] already states the authored ones.
pub(super) fn world(
    local: &LocalSweptSolid,
    id: EntityId,
    entity: &Entity,
    frame: Transform,
) -> GeometryResult<Option<SweptSolid>> {
    if !is_rigid(&frame) {
        return Ok(None);
    }
    let placement_world = match &local.position {
        Some(position) => frame.compose(position),
        None => frame,
    };
    let path = match &local.path {
        LocalSweepPath::Extrusion { direction, depth } => SweepPath::Extrusion {
            direction_world: world_direction(id, entity, &placement_world, *direction)?,
            depth: *depth,
        },
        LocalSweepPath::Revolution {
            axis_origin,
            axis_direction,
            angle,
        } => SweepPath::Revolution {
            axis_origin_world: placement_world.apply(*axis_origin),
            axis_direction_world: world_direction(id, entity, &placement_world, *axis_direction)?,
            angle: *angle,
        },
        LocalSweepPath::Directrix { directrix } => SweepPath::Directrix {
            directrix: *directrix,
        },
    };
    Ok(Some(SweptSolid {
        profile: local.profile.clone(),
        end_profile: local.end_profile.clone(),
        placement_world,
        path,
    }))
}

/// Carry a local unit direction into world coordinates, normalised once.
fn world_direction(
    id: EntityId,
    entity: &Entity,
    frame: &Transform,
    local: [f64; 3],
) -> GeometryResult<[f64; 3]> {
    let [x, y, z] = frame.apply_direction(local);
    let length = (x * x + y * y + z * z).sqrt();
    if length == 0.0 || !length.is_finite() {
        return Err(Slots::new(id, entity).degenerate("direction vanishes in world coordinates"));
    }
    Ok([x / length, y / length, z / length])
}

/// Is `frame`'s linear part a proper rotation (orthonormal, determinant +1)?
fn is_rigid(frame: &Transform) -> bool {
    let [x, y, z] = frame.basis;
    let dot = |a: [f64; 3], b: [f64; 3]| a[0] * b[0] + a[1] * b[1] + a[2] * b[2];
    let unit = |a: [f64; 3]| (dot(a, a) - 1.0).abs() <= RIGID_TOLERANCE;
    let cross = [
        x[1] * y[2] - x[2] * y[1],
        x[2] * y[0] - x[0] * y[2],
        x[0] * y[1] - x[1] * y[0],
    ];
    unit(x)
        && unit(y)
        && unit(z)
        && dot(x, y).abs() <= RIGID_TOLERANCE
        && dot(y, z).abs() <= RIGID_TOLERANCE
        && dot(z, x).abs() <= RIGID_TOLERANCE
        && dot(cross, z) > 0.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_rotation_is_rigid_and_a_scale_or_mirror_is_not() {
        let rotated =
            Transform::from_axes([1.0, 2.0, 3.0], Some([0.0, 1.0, 0.0]), None).expect("valid axes");
        assert!(is_rigid(&rotated));
        assert!(is_rigid(&Transform::identity()));
        assert!(!is_rigid(&Transform::identity().scaled(2.0)));
        assert!(!is_rigid(
            &Transform::identity().scaled_nonuniform([1.0, 1.0, 1.5])
        ));
        assert!(!is_rigid(
            &Transform::identity().scaled_nonuniform([-1.0, 1.0, 1.0])
        ));
    }
}
