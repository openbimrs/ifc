//! Swept disks described in their own item frame.
//!
//! Reads the slots `lower::csg` lowers a pipe or a bar from, through the
//! `solid::swept` views, and reports them as authored: radii and fillet in
//! metres, the directrix by reference and `StartParam`/`EndParam` unconverted.
//! A mapping that scales or mirrors the item is not refused: the parameters
//! are the authored ones and [`BodyItem::item_world`](super::BodyItem) places
//! them.

use ifc_model::{Entity, EntityId, Model};

use super::SweptDisk;
use crate::error::GeometryResult;
use crate::slots::Slots;
use crate::solid::swept::{SweptDiskSolid, SweptDiskSolidPolygonal};
use crate::units::UnitScale;

/// Describe one `IfcSweptDiskSolid` or `IfcSweptDiskSolidPolygonal`.
pub(super) fn describe(
    model: &Model,
    units: &UnitScale,
    id: EntityId,
    entity: &Entity,
    type_name: &str,
) -> GeometryResult<SweptDisk> {
    let slots = Slots::new(id, entity);
    let view = SweptDiskSolid::new(id, entity);
    let (radius, inner_radius) = view.checked_radii()?;
    let directrix = view.directrix()?;
    slots.resolve(model, directrix)?;

    let parameter_range = match (view.start_param(), view.end_param()) {
        (Some(start), Some(end)) => Some((start, end)),
        (None, None) => None,
        _ => {
            return Err(slots.degenerate(
                "only one of StartParam/EndParam is present; a half-open sweep is undefined",
            ))
        }
    };

    let fillet_radius = if type_name == "IFCSWEPTDISKSOLIDPOLYGONAL" {
        match SweptDiskSolidPolygonal::new(id, entity).fillet_radius() {
            Some(fillet) if !fillet.is_finite() || fillet <= 0.0 => {
                return Err(slots.degenerate(format!(
                    "FilletRadius must be positive and finite, found {fillet}"
                )))
            }
            fillet => fillet.map(|value| units.length(value)),
        }
    } else {
        None
    };

    Ok(SweptDisk {
        radius: units.length(radius),
        inner_radius: inner_radius.map(|value| units.length(value)),
        fillet_radius,
        directrix,
        parameter_range,
    })
}
