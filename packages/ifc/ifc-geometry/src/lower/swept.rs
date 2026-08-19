//! Exact swept-solid lowering into the format-neutral geometry DAG.

use geom_core::{Point3, Vec3};
use geom_model::{GeometryGraphBuilder, GeometryNode, Instance, SolidOperation};
use ifc_model::{EntityId, Model};

use crate::error::{GeometryError, GeometryResult};
use crate::lower::{lower_profile, LoweredGeometry, Tolerance};
use crate::resource::placement::axis_placement_transform;
use crate::slots::Slots;
use crate::transform::Transform;
use crate::units::UnitScale;

mod slot {
    pub const SWEPT_AREA: usize = 0;
    pub const POSITION: usize = 1;
    pub const EXTRUDED_DIRECTION: usize = 2;
    pub const DEPTH: usize = 3;
    pub const AXIS: usize = 2;
    pub const ANGLE: usize = 3;
}

/// Lower one `IfcExtrudedAreaSolid` into an exact profile plus extrusion node.
pub fn lower_extruded_area_solid(
    model: &Model,
    id: EntityId,
    world: Transform,
    units: &UnitScale,
    tol: &Tolerance,
) -> GeometryResult<LoweredGeometry> {
    let entity = model.get(id).ok_or(GeometryError::MissingEntity {
        referrer: id,
        missing: id,
    })?;
    let slots = Slots::new(id, entity);
    let profile = lower_profile(
        model,
        slots.req_ref(slot::SWEPT_AREA, "SweptArea")?,
        units,
        tol,
    )?;
    let depth = units.length(slots.req_f64(slot::DEPTH, "Depth")?);
    if depth <= 0.0 {
        return Err(slots.degenerate("extrusion depth is not positive"));
    }
    let direction = Vec3::from_array(direction_ratios(
        model,
        slots.req_ref(slot::EXTRUDED_DIRECTION, "ExtrudedDirection")?,
    )?);
    let placement = compose_placement(model, &slots, world, units)?.to_geom();

    build_sweep_graph(id, profile, direction, depth, placement)
}

/// Lower one `IfcRevolvedAreaSolid` into an exact profile plus revolution node.
pub fn lower_revolved_area_solid(
    model: &Model,
    id: EntityId,
    world: Transform,
    units: &UnitScale,
    tol: &Tolerance,
) -> GeometryResult<LoweredGeometry> {
    let entity = model.get(id).ok_or(GeometryError::MissingEntity {
        referrer: id,
        missing: id,
    })?;
    let slots = Slots::new(id, entity);
    let profile = lower_profile(
        model,
        slots.req_ref(slot::SWEPT_AREA, "SweptArea")?,
        units,
        tol,
    )?;
    let angle = units.angle(slots.req_f64(slot::ANGLE, "Angle")?);
    if angle <= 0.0 {
        return Err(slots.degenerate("revolution angle is not positive"));
    }

    let axis_id = slots.req_ref(slot::AXIS, "Axis")?;
    let axis = model.get(axis_id).ok_or(GeometryError::MissingEntity {
        referrer: id,
        missing: axis_id,
    })?;
    let axis_slots = Slots::new(axis_id, axis);
    let axis_origin = Point3::from_array(point_coords(
        model,
        axis_slots.req_ref(0, "Location")?,
        units,
    )?);
    let axis_direction = Vec3::from_array(match axis_slots.opt_ref(1) {
        Some(direction) => direction_ratios(model, direction)?,
        None => [0.0, 0.0, 1.0],
    });
    let placement = compose_placement(model, &slots, world, units)?.to_geom();

    let mut builder = GeometryGraphBuilder::new();
    let profile_id = builder
        .push(GeometryNode::Profile(profile))
        .map_err(|error| graph_error(id, error))?;
    let operation = builder
        .push(GeometryNode::SolidOperation(SolidOperation::Revolution {
            profile: profile_id,
            axis_origin,
            axis_direction,
            angle,
        }))
        .map_err(|error| graph_error(id, error))?;
    let root = builder
        .push(GeometryNode::Instance(Instance {
            source: operation,
            transform: placement,
        }))
        .map_err(|error| graph_error(id, error))?;
    let graph = builder
        .finish(vec![root])
        .map_err(|error| graph_error(id, error))?;
    Ok(LoweredGeometry { graph, root })
}

fn build_sweep_graph(
    id: EntityId,
    profile: geom_profile::Profile,
    direction: Vec3,
    depth: f64,
    placement: geom_core::Transform3,
) -> GeometryResult<LoweredGeometry> {
    let mut builder = GeometryGraphBuilder::new();
    let profile_id = builder
        .push(GeometryNode::Profile(profile))
        .map_err(|error| graph_error(id, error))?;
    let operation = builder
        .push(GeometryNode::SolidOperation(SolidOperation::Extrusion {
            profile: profile_id,
            direction,
            depth,
        }))
        .map_err(|error| graph_error(id, error))?;
    let root = builder
        .push(GeometryNode::Instance(Instance {
            source: operation,
            transform: placement,
        }))
        .map_err(|error| graph_error(id, error))?;
    let graph = builder
        .finish(vec![root])
        .map_err(|error| graph_error(id, error))?;
    Ok(LoweredGeometry { graph, root })
}

fn graph_error(id: EntityId, error: geom_model::GraphError) -> GeometryError {
    GeometryError::Degenerate {
        entity: id,
        type_name: "geometry graph".to_string(),
        detail: error.to_string(),
    }
}

fn compose_placement(
    model: &Model,
    slots: &Slots<'_>,
    world: Transform,
    units: &UnitScale,
) -> GeometryResult<Transform> {
    match slots.opt_ref(slot::POSITION) {
        Some(position_id) => {
            let position = model.get(position_id).ok_or(GeometryError::MissingEntity {
                referrer: slots.id(),
                missing: position_id,
            })?;
            let local = to_metres(
                axis_placement_transform(model, position_id, position)?,
                units,
            );
            Ok(world.compose(&local))
        }
        None => Ok(world),
    }
}

fn to_metres(transform: Transform, units: &UnitScale) -> Transform {
    Transform {
        basis: transform.basis,
        origin: transform.origin.map(|coordinate| units.length(coordinate)),
    }
}

fn direction_ratios(model: &Model, id: EntityId) -> GeometryResult<[f64; 3]> {
    let entity = model.get(id).ok_or(GeometryError::MissingEntity {
        referrer: id,
        missing: id,
    })?;
    let slots = Slots::new(id, entity);
    let ratios = slots.req_f64_list(0, "DirectionRatios")?;
    match ratios.as_slice() {
        [x, y] => Ok([*x, *y, 0.0]),
        [x, y, z] => Ok([*x, *y, *z]),
        _ => Err(slots.degenerate("direction must have 2 or 3 ratios")),
    }
}

fn point_coords(model: &Model, id: EntityId, units: &UnitScale) -> GeometryResult<[f64; 3]> {
    let entity = model.get(id).ok_or(GeometryError::MissingEntity {
        referrer: id,
        missing: id,
    })?;
    let slots = Slots::new(id, entity);
    let coordinates = slots.req_f64_list(0, "Coordinates")?;
    match coordinates.as_slice() {
        [x, y] => Ok([units.length(*x), units.length(*y), 0.0]),
        [x, y, z] => Ok([units.length(*x), units.length(*y), units.length(*z)]),
        _ => Err(slots.degenerate("point must have 2 or 3 coordinates")),
    }
}
