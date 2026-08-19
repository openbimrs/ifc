//! Swept solids become extrusion and revolution requests.
//!
//! # The composition that matters
//!
//! An extrusion carries THREE independent placements and confusing them is
//! the classic source of geometry that is subtly in the wrong place:
//!
//! 1. the profile's own `Position` (2D, inside the profile plane),
//! 2. the solid's `Position` (`IfcSweptAreaSolid.Position`, 3D),
//! 3. the product's `ObjectPlacement` chain (`IfcLocalPlacement`).
//!
//! This module composes 2 and 3 into `Primitive::placement`. Number 1 stays
//! baked into the contour by `lower::profile`, because it belongs to the
//! profile's own coordinate system.

use crate::error::{GeometryError, GeometryResult};
use crate::kernel::Primitive;
use crate::lower::{lower_profile, Tolerance};
use crate::resource::placement::axis_placement_transform;
use crate::slots::Slots;
use crate::transform::Transform;
use crate::units::UnitScale;
use ifc_model::{EntityId, Model};

/// Absolute slot indices for swept solids.
mod slot {
    /// `IfcSweptAreaSolid.SweptArea`.
    pub const SWEPT_AREA: usize = 0;
    /// `IfcSweptAreaSolid.Position`.
    pub const POSITION: usize = 1;
    /// `IfcExtrudedAreaSolid.ExtrudedDirection`.
    pub const EXTRUDED_DIRECTION: usize = 2;
    /// `IfcExtrudedAreaSolid.Depth`.
    pub const DEPTH: usize = 3;
    /// `IfcRevolvedAreaSolid.Axis`.
    pub const AXIS: usize = 2;
    /// `IfcRevolvedAreaSolid.Angle`.
    pub const ANGLE: usize = 3;
}

/// Lower an `IfcExtrudedAreaSolid` into a kernel extrusion request.
///
/// `world` is the product placement this solid hangs under, already composed.
/// Pass `Transform::identity()` for a solid being examined in isolation.
pub fn lower_extruded_area_solid(
    model: &Model,
    id: EntityId,
    world: Transform,
    units: &UnitScale,
    tol: &Tolerance,
) -> GeometryResult<Primitive> {
    let entity = model.get(id).ok_or(GeometryError::MissingEntity {
        referrer: id,
        missing: id,
    })?;
    let slots = Slots::new(id, entity);

    let profile_id = slots.req_ref(slot::SWEPT_AREA, "SweptArea")?;
    let profile = lower_profile(model, profile_id, units, tol)?;

    // Depth is a length: it MUST be scaled. A millimetre file whose depth is
    // not converted produces a solid a thousand times too tall.
    let depth = units.length(slots.req_f64(slot::DEPTH, "Depth")?);
    if depth <= 0.0 {
        return Err(slots.degenerate("extrusion depth is not positive"));
    }

    // The direction is a ratio, NOT a length: scaling it would be wrong.
    let dir_id = slots.req_ref(slot::EXTRUDED_DIRECTION, "ExtrudedDirection")?;
    let direction = direction_ratios(model, dir_id)?;

    let placement = compose_placement(model, &slots, world, units)?;

    Ok(Primitive::Extrusion {
        profile,
        direction,
        depth,
        placement,
    })
}

/// Lower an `IfcRevolvedAreaSolid` into a revolution request.
///
/// The angle is an ANGLE and gets the angle conversion, not the length one.
/// A file in degrees whose 90 is passed through as radians sweeps 5156
/// degrees, which wraps and looks almost right.
pub fn lower_revolved_area_solid(
    model: &Model,
    id: EntityId,
    world: Transform,
    units: &UnitScale,
    tol: &Tolerance,
) -> GeometryResult<Primitive> {
    let entity = model.get(id).ok_or(GeometryError::MissingEntity {
        referrer: id,
        missing: id,
    })?;
    let slots = Slots::new(id, entity);

    let profile_id = slots.req_ref(slot::SWEPT_AREA, "SweptArea")?;
    let profile = lower_profile(model, profile_id, units, tol)?;

    let angle = units.angle(slots.req_f64(slot::ANGLE, "Angle")?);
    if angle <= 0.0 {
        return Err(slots.degenerate("revolution angle is not positive"));
    }

    // IfcAxis1Placement: Location at slot 0, Axis at slot 1.
    let axis_id = slots.req_ref(slot::AXIS, "Axis")?;
    let axis = model.get(axis_id).ok_or(GeometryError::MissingEntity {
        referrer: id,
        missing: axis_id,
    })?;
    let axis_slots = Slots::new(axis_id, axis);
    let origin_id = axis_slots.req_ref(0, "Location")?;
    let axis_origin = point_coords(model, origin_id, units)?;
    let axis_direction = match axis_slots.opt_ref(1) {
        Some(d) => direction_ratios(model, d)?,
        // IfcAxis1Placement.Axis defaults to +Z when absent.
        None => [0.0, 0.0, 1.0],
    };

    let placement = compose_placement(model, &slots, world, units)?;

    Ok(Primitive::Revolution {
        profile,
        axis_origin,
        axis_direction,
        angle,
        placement,
    })
}

/// Compose the solid's own `Position` under the product placement.
///
/// `IfcSweptAreaSolid.Position` is OPTIONAL. When absent the solid sits at
/// the product's origin, so the world transform passes through unchanged.
fn compose_placement(
    model: &Model,
    slots: &Slots<'_>,
    world: Transform,
    units: &UnitScale,
) -> GeometryResult<Transform> {
    match slots.opt_ref(slot::POSITION) {
        Some(pos_id) => {
            let pos = model.get(pos_id).ok_or(GeometryError::MissingEntity {
                referrer: slots.id(),
                missing: pos_id,
            })?;
            // The view reports the file's own numbers, so the translation is
            // still in file units here. Scaling the translation but NOT the
            // basis is the whole point: the basis holds direction ratios,
            // which are dimensionless. Scaling both would shear nothing but
            // would multiply every rotation by 1000 in a millimetre file.
            let local = to_metres(axis_placement_transform(model, pos_id, pos)?, units);
            Ok(world.compose(&local))
        }
        None => Ok(world),
    }
}

/// Convert a placement's translation from file units into metres.
///
/// Only the origin is touched. The basis holds direction ratios, which are
/// dimensionless: scaling them would magnify the geometry rather than
/// reposition it. `Transform::scaled` does the basis instead and exists for
/// the transformation operator's scale factor, so it is deliberately NOT
/// reused here.
fn to_metres(t: Transform, units: &UnitScale) -> Transform {
    Transform {
        basis: t.basis,
        origin: [
            units.length(t.origin[0]),
            units.length(t.origin[1]),
            units.length(t.origin[2]),
        ],
    }
}

/// Read an `IfcDirection`'s ratios, padding 2D to 3D with a zero Z.
///
/// Direction ratios are dimensionless, so no unit conversion is applied.
fn direction_ratios(model: &Model, id: EntityId) -> GeometryResult<[f64; 3]> {
    let entity = model.get(id).ok_or(GeometryError::MissingEntity {
        referrer: id,
        missing: id,
    })?;
    let slots = Slots::new(id, entity);
    let r = slots.req_f64_list(0, "DirectionRatios")?;
    match r.len() {
        2 => Ok([r[0], r[1], 0.0]),
        3 => Ok([r[0], r[1], r[2]]),
        _ => Err(slots.degenerate("direction must have 2 or 3 ratios")),
    }
}

/// Read an `IfcCartesianPoint` in metres, padding 2D to 3D.
fn point_coords(model: &Model, id: EntityId, units: &UnitScale) -> GeometryResult<[f64; 3]> {
    let entity = model.get(id).ok_or(GeometryError::MissingEntity {
        referrer: id,
        missing: id,
    })?;
    let slots = Slots::new(id, entity);
    let c = slots.req_f64_list(0, "Coordinates")?;
    match c.len() {
        2 => Ok([units.length(c[0]), units.length(c[1]), 0.0]),
        3 => Ok([units.length(c[0]), units.length(c[1]), units.length(c[2])]),
        _ => Err(slots.degenerate("point must have 2 or 3 coordinates")),
    }
}
