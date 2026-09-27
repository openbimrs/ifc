//! Swept-area solids described in world coordinates.
//!
//! Reads the same slots `lower::swept` lowers, through the `solid::swept`
//! views, and applies the same frame composition: the enclosing frame, then
//! the solid's `Position`. `ExtrudedDirection` and the revolution axis are
//! expressed in that `Position` system, so they are carried through the
//! composed frame and normalised once.

use ifc_model::{Entity, EntityId, Model};

use super::{SweepPath, SweptSolid};
use crate::error::{GeometryError, GeometryResult};
use crate::input::profile::{describe_profile, ProfileDescription};
use crate::resource::placement::axis_placement_transform;
use crate::resource::resolve;
use crate::slots::Slots;
use crate::solid::swept::{
    ExtrudedAreaSolid, ExtrudedAreaSolidTapered, FixedReferenceSweptAreaSolid, RevolvedAreaSolid,
    RevolvedAreaSolidTapered, SurfaceCurveSweptAreaSolid, SweptAreaSolid,
};
use crate::transform::Transform;
use crate::units::UnitScale;

/// How far a frame may drift from a proper rotation and still count as rigid.
///
/// Placements and operators are orthonormalised from `f64` ratios, so a rigid
/// frame is exact to a few ulps; an authored scale of even 1.001 is far
/// outside this.
const RIGID_TOLERANCE: f64 = 1e-9;

/// Describe one swept-area solid placed in `frame`.
///
/// `culprit` is the entity blamed when `frame` is not rigid: the innermost
/// mapped item, since only a mapping operator can scale or mirror.
pub(super) fn describe(
    model: &Model,
    units: &UnitScale,
    id: EntityId,
    entity: &Entity,
    frame: Transform,
    culprit: EntityId,
) -> GeometryResult<SweptSolid> {
    if !is_rigid(&frame) {
        let culprit_type = model
            .get(culprit)
            .map_or_else(String::new, |e| e.type_name.to_ascii_uppercase());
        return Err(GeometryError::Unsupported {
            entity: culprit,
            type_name: culprit_type,
            detail: "the mapping scales or mirrors a swept solid, so its profile \
                     parameters would not be the authored ones",
        });
    }
    let base = SweptAreaSolid::new(id, entity);
    let profile = area_profile(model, units, id, base.swept_area()?)?;
    let placement_world = match base.position() {
        Some(position) => {
            let placement = Slots::new(id, entity).resolve(model, position)?;
            let local = axis_placement_transform(model, position, placement)?.to_metres(units);
            frame.compose(&local)
        }
        None => frame,
    };

    let type_name = entity.type_name.to_ascii_uppercase();
    let (end_profile, path) = match type_name.as_str() {
        "IFCEXTRUDEDAREASOLID" => (None, extrusion(model, units, id, entity, &placement_world)?),
        "IFCEXTRUDEDAREASOLIDTAPERED" => {
            let end = ExtrudedAreaSolidTapered::new(id, entity).end_swept_area()?;
            (
                Some(area_profile(model, units, id, end)?),
                extrusion(model, units, id, entity, &placement_world)?,
            )
        }
        "IFCREVOLVEDAREASOLID" => (
            None,
            revolution(model, units, id, entity, &placement_world)?,
        ),
        "IFCREVOLVEDAREASOLIDTAPERED" => {
            let end = RevolvedAreaSolidTapered::new(id, entity).end_swept_area()?;
            (
                Some(area_profile(model, units, id, end)?),
                revolution(model, units, id, entity, &placement_world)?,
            )
        }
        "IFCFIXEDREFERENCESWEPTAREASOLID" => {
            let directrix = FixedReferenceSweptAreaSolid::new(id, entity).directrix()?;
            (None, directrix_path(model, id, entity, directrix)?)
        }
        "IFCSURFACECURVESWEPTAREASOLID" => {
            let directrix = SurfaceCurveSweptAreaSolid::new(id, entity).directrix()?;
            (None, directrix_path(model, id, entity, directrix)?)
        }
        _ => return Err(Slots::new(id, entity).unsupported("not a swept-area solid family")),
    };
    Ok(SweptSolid {
        profile,
        end_profile,
        placement_world,
        path,
    })
}

/// A swept area's profile, refused when it bounds no area.
///
/// `IfcSweptAreaSolid` requires `SweptArea.ProfileType = AREA`; an open
/// curve swept into a "solid" has no cross section to report.
fn area_profile(
    model: &Model,
    units: &UnitScale,
    solid: EntityId,
    profile: EntityId,
) -> GeometryResult<ProfileDescription> {
    // Resolve against the solid first, so a dangling SweptArea names the
    // solid that holds it rather than the missing id alone.
    if model.get(profile).is_none() {
        return Err(GeometryError::MissingEntity {
            referrer: solid,
            missing: profile,
        });
    }
    let description = describe_profile(model, units, profile)?;
    if !description.bounds_area() {
        return Err(GeometryError::Unsupported {
            entity: profile,
            type_name: description.type_name,
            detail: "an open profile bounds no area, so it cannot be a swept solid's section",
        });
    }
    Ok(description)
}

/// `ExtrudedDirection` and `Depth`, in world coordinates and metres.
fn extrusion(
    model: &Model,
    units: &UnitScale,
    id: EntityId,
    entity: &Entity,
    placement_world: &Transform,
) -> GeometryResult<SweepPath> {
    let view = ExtrudedAreaSolid::new(id, entity);
    // A non-positive depth is refused, as lowering refuses it.
    let depth = units.length(view.checked_depth()?);
    let local = resolve::direction(model, id, view.extruded_direction()?)?.unit()?;
    Ok(SweepPath::Extrusion {
        direction_world: world_direction(id, entity, placement_world, local)?,
        depth,
    })
}

/// `Axis` and `Angle`, in world coordinates and radians.
fn revolution(
    model: &Model,
    units: &UnitScale,
    id: EntityId,
    entity: &Entity,
    placement_world: &Transform,
) -> GeometryResult<SweepPath> {
    let view = RevolvedAreaSolid::new(id, entity);
    // Angle is an IfcPlaneAngleMeasure in the file's unit, very often degrees.
    let angle = units.angle(view.angle_raw()?);
    if angle <= 0.0 {
        return Err(Slots::new(id, entity).degenerate("revolution angle is not positive"));
    }
    let axis = resolve::axis1_placement(model, id, view.axis()?)?;
    let origin = axis
        .location(model)?
        .map(|coordinate| units.length(coordinate));
    let direction = axis.axis(model)?;
    Ok(SweepPath::Revolution {
        axis_origin_world: placement_world.apply(origin),
        axis_direction_world: world_direction(id, entity, placement_world, direction)?,
        angle,
    })
}

/// A directrix sweep's curve, which must resolve.
fn directrix_path(
    model: &Model,
    id: EntityId,
    entity: &Entity,
    directrix: EntityId,
) -> GeometryResult<SweepPath> {
    Slots::new(id, entity).resolve(model, directrix)?;
    Ok(SweepPath::Directrix { directrix })
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
