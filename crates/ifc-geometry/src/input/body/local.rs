//! Swept-area solids described in their own item frame.
//!
//! Reads the same slots `lower::swept` lowers, through the `solid::swept`
//! views, and reports them as authored: the profile, the solid's `Position`
//! and the path, with `ExtrudedDirection` and the revolution axis expressed in
//! that `Position` system. No enclosing frame is applied, so a mapping that
//! scales or mirrors the item cannot change an authored parameter.

use ifc_model::{Entity, EntityId, Model};

use super::{LocalSweepPath, LocalSweptSolid};
use crate::error::{GeometryError, GeometryResult};
use crate::input::profile::{describe_profile, ProfileDescription};
use crate::resource::placement::axis_placement_transform;
use crate::resource::resolve;
use crate::slots::Slots;
use crate::solid::swept::{
    ExtrudedAreaSolid, ExtrudedAreaSolidTapered, FixedReferenceSweptAreaSolid, RevolvedAreaSolid,
    RevolvedAreaSolidTapered, SurfaceCurveSweptAreaSolid, SweptAreaSolid,
};
use crate::units::UnitScale;

/// Describe one swept-area solid in its item frame.
pub(super) fn describe(
    model: &Model,
    units: &UnitScale,
    id: EntityId,
    entity: &Entity,
) -> GeometryResult<LocalSweptSolid> {
    let base = SweptAreaSolid::new(id, entity);
    let profile = area_profile(model, units, id, base.swept_area()?)?;
    let position = match base.position() {
        Some(position) => {
            let placement = Slots::new(id, entity).resolve(model, position)?;
            Some(axis_placement_transform(model, position, placement)?.to_metres(units))
        }
        None => None,
    };

    let type_name = entity.type_name.to_ascii_uppercase();
    let (end_profile, path) = match type_name.as_str() {
        "IFCEXTRUDEDAREASOLID" => (None, extrusion(model, units, id, entity)?),
        "IFCEXTRUDEDAREASOLIDTAPERED" => {
            let end = ExtrudedAreaSolidTapered::new(id, entity).end_swept_area()?;
            (
                Some(area_profile(model, units, id, end)?),
                extrusion(model, units, id, entity)?,
            )
        }
        "IFCREVOLVEDAREASOLID" => (None, revolution(model, units, id, entity)?),
        "IFCREVOLVEDAREASOLIDTAPERED" => {
            let end = RevolvedAreaSolidTapered::new(id, entity).end_swept_area()?;
            (
                Some(area_profile(model, units, id, end)?),
                revolution(model, units, id, entity)?,
            )
        }
        // The IFC4X3 subtype declares no slot of its own.
        "IFCFIXEDREFERENCESWEPTAREASOLID" | "IFCDIRECTRIXDERIVEDREFERENCESWEPTAREASOLID" => {
            let directrix = FixedReferenceSweptAreaSolid::new(id, entity).directrix()?;
            (None, directrix_path(model, id, entity, directrix)?)
        }
        "IFCSURFACECURVESWEPTAREASOLID" => {
            let directrix = SurfaceCurveSweptAreaSolid::new(id, entity).directrix()?;
            (None, directrix_path(model, id, entity, directrix)?)
        }
        _ => return Err(Slots::new(id, entity).unsupported("not a swept-area solid family")),
    };
    Ok(LocalSweptSolid {
        profile,
        end_profile,
        position,
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

/// `ExtrudedDirection` and `Depth`, in the `Position` frame and metres.
fn extrusion(
    model: &Model,
    units: &UnitScale,
    id: EntityId,
    entity: &Entity,
) -> GeometryResult<LocalSweepPath> {
    let view = ExtrudedAreaSolid::new(id, entity);
    // A non-positive depth is refused, as lowering refuses it.
    let depth = units.length(view.checked_depth()?);
    let direction = resolve::direction(model, id, view.extruded_direction()?)?.unit()?;
    Ok(LocalSweepPath::Extrusion { direction, depth })
}

/// `Axis` and `Angle`, in the `Position` frame, metres and radians.
fn revolution(
    model: &Model,
    units: &UnitScale,
    id: EntityId,
    entity: &Entity,
) -> GeometryResult<LocalSweepPath> {
    let view = RevolvedAreaSolid::new(id, entity);
    // Angle is an IfcPlaneAngleMeasure in the file's unit, very often degrees.
    let angle = units.angle(id, view.angle_raw()?)?;
    if angle <= 0.0 {
        return Err(Slots::new(id, entity).degenerate("revolution angle is not positive"));
    }
    let axis = resolve::axis1_placement(model, id, view.axis()?)?;
    let axis_origin = axis
        .location(model)?
        .map(|coordinate| units.length(coordinate));
    Ok(LocalSweepPath::Revolution {
        axis_origin,
        axis_direction: axis.axis(model)?,
        angle,
    })
}

/// A directrix sweep's curve, which must resolve.
fn directrix_path(
    model: &Model,
    id: EntityId,
    entity: &Entity,
    directrix: EntityId,
) -> GeometryResult<LocalSweepPath> {
    Slots::new(id, entity).resolve(model, directrix)?;
    Ok(LocalSweepPath::Directrix { directrix })
}
