//! Points, directions and axis placements.
//!
//! These are the leaves every other geometry entity references, so the
//! validation here is the cheapest place to stop a malformed body.

use ifc_model::{Entity, EntityId, Transaction, Value};

use crate::error::GeometryError;
use crate::resource::direction::slot as direction_slot;
use crate::resource::placement::slot as placement_slot;
use crate::resource::point::slot as point_slot;

use super::{invalid, reals, require_finite};

/// Stage an `IfcCartesianPoint`.
///
/// Accepts 1-3 coordinates: IFC implies a point's dimension from the list
/// length rather than declaring it.
///
/// # Errors
///
/// Refuses a non-finite coordinate, or a count outside 1..=3. A NaN here
/// would otherwise propagate into every placement referencing the point.
pub fn cartesian_point(tx: &mut Transaction, coords: &[f64]) -> Result<EntityId, GeometryError> {
    if coords.is_empty() || coords.len() > 3 {
        return Err(invalid(
            "IFCCARTESIANPOINT",
            "Coordinates",
            format!("expected 1..=3 coordinates, got {}", coords.len()),
        ));
    }
    require_finite("IFCCARTESIANPOINT", "Coordinates", coords)?;
    let mut attrs = vec![Value::Null];
    attrs[point_slot::cartesian_point::COORDINATES] = reals(coords);
    Ok(tx.create(Entity::new("IFCCARTESIANPOINT", attrs)))
}

/// Stage an `IfcDirection`.
///
/// Ratios are written exactly as given. IFC direction ratios are *not*
/// required to be unit length, and the reader documents that it returns them
/// unnormalized, so normalizing here would silently disagree with the read
/// direction and discard a magnitude the file was entitled to express.
///
/// # Errors
///
/// Refuses a non-finite ratio, a count outside 2..=3, and an all-zero vector,
/// which denotes no direction at all and cannot be normalized downstream.
pub fn direction(tx: &mut Transaction, ratios: &[f64]) -> Result<EntityId, GeometryError> {
    if ratios.len() < 2 || ratios.len() > 3 {
        return Err(invalid(
            "IFCDIRECTION",
            "DirectionRatios",
            format!("expected 2..=3 ratios, got {}", ratios.len()),
        ));
    }
    require_finite("IFCDIRECTION", "DirectionRatios", ratios)?;
    if ratios.iter().all(|r| *r == 0.0) {
        return Err(invalid(
            "IFCDIRECTION",
            "DirectionRatios",
            "all ratios are zero, which denotes no direction",
        ));
    }
    let mut attrs = vec![Value::Null];
    attrs[direction_slot::direction::DIRECTION_RATIOS] = reals(ratios);
    Ok(tx.create(Entity::new("IFCDIRECTION", attrs)))
}

/// Stage an `IfcAxis2Placement3D`.
///
/// `axis` is the local Z, `ref_direction` the local X. Both are optional in
/// IFC; omitting them means the implicit global axes, which is a meaningful
/// default rather than missing data, so `None` is written as `$`.
pub fn axis2_placement_3d(
    tx: &mut Transaction,
    location: EntityId,
    axis: Option<EntityId>,
    ref_direction: Option<EntityId>,
) -> EntityId {
    let mut attrs = vec![Value::Null, Value::Null, Value::Null];
    attrs[placement_slot::LOCATION] = Value::Ref(location);
    attrs[placement_slot::axis2_3d::AXIS] = axis.map_or(Value::Null, Value::Ref);
    attrs[placement_slot::axis2_3d::REF_DIRECTION] = ref_direction.map_or(Value::Null, Value::Ref);
    tx.create(Entity::new("IFCAXIS2PLACEMENT3D", attrs))
}

/// Stage an `IfcAxis2Placement2D`.
///
/// Carries no `Axis`: a 2D placement has only a location and a reference
/// direction, so this is a shorter record than its 3D counterpart rather
/// than the same one with a hole in it.
pub fn axis2_placement_2d(
    tx: &mut Transaction,
    location: EntityId,
    ref_direction: Option<EntityId>,
) -> EntityId {
    let mut attrs = vec![Value::Null, Value::Null];
    attrs[placement_slot::LOCATION] = Value::Ref(location);
    attrs[placement_slot::axis2_2d::REF_DIRECTION] = ref_direction.map_or(Value::Null, Value::Ref);
    tx.create(Entity::new("IFCAXIS2PLACEMENT2D", attrs))
}
