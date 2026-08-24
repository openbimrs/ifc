//! Exact swept-solid lowering into the format-neutral geometry DAG.
//!
//! Each family has two entry points. The `_node` form appends into a caller-
//! owned [`LoweringSession`] and returns a [`NodeId`], so a composite parent
//! (boolean, mapped item, CSG) can reference the result. The non-`_node` form
//! is the convenience wrapper that opens a session, lowers one item, and
//! freezes the graph.

use axiolid_core::{Point3, Vec3};
use axiolid_model::{GeometryNode, Instance, NodeId, SolidOperation};
use ifc_model::{EntityId, Model};

use crate::error::{GeometryError, GeometryResult};
use crate::lower::session::LoweringSession;
use crate::lower::{lower_profile_node, LoweredGeometry, Tolerance};
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

/// Family label used for memoization and chain diagnostics.
const EXTRUSION: &str = "extruded area solid";
/// Family label used for memoization and chain diagnostics.
const REVOLUTION: &str = "revolved area solid";

/// Lower one `IfcExtrudedAreaSolid` into an exact profile plus extrusion node.
pub fn lower_extruded_area_solid(
    model: &Model,
    id: EntityId,
    world: Transform,
    units: &UnitScale,
    tol: &Tolerance,
) -> GeometryResult<LoweredGeometry> {
    let mut session = LoweringSession::new(model, units, *tol);
    let root = lower_extruded_area_solid_node(&mut session, id, world)?;
    session.finish(root)
}

/// Lower one `IfcRevolvedAreaSolid` into an exact profile plus revolution node.
pub fn lower_revolved_area_solid(
    model: &Model,
    id: EntityId,
    world: Transform,
    units: &UnitScale,
    tol: &Tolerance,
) -> GeometryResult<LoweredGeometry> {
    let mut session = LoweringSession::new(model, units, *tol);
    let root = lower_revolved_area_solid_node(&mut session, id, world)?;
    session.finish(root)
}

/// Append one `IfcExtrudedAreaSolid` to a shared session.
pub fn lower_extruded_area_solid_node(
    session: &mut LoweringSession<'_>,
    id: EntityId,
    world: Transform,
) -> GeometryResult<NodeId> {
    if let Some(node) = session.memoized(id, EXTRUSION, world) {
        return Ok(node);
    }
    session.enter(id, "swept solid")?;
    let result = extrusion_node(session, id, world);
    session.exit(id);
    let node = result?;
    session.memoize(id, EXTRUSION, world, node);
    Ok(node)
}

/// Append one `IfcRevolvedAreaSolid` to a shared session.
pub fn lower_revolved_area_solid_node(
    session: &mut LoweringSession<'_>,
    id: EntityId,
    world: Transform,
) -> GeometryResult<NodeId> {
    if let Some(node) = session.memoized(id, REVOLUTION, world) {
        return Ok(node);
    }
    session.enter(id, "swept solid")?;
    let result = revolution_node(session, id, world);
    session.exit(id);
    let node = result?;
    session.memoize(id, REVOLUTION, world, node);
    Ok(node)
}

fn extrusion_node(
    session: &mut LoweringSession<'_>,
    id: EntityId,
    world: Transform,
) -> GeometryResult<NodeId> {
    let model = session.model();
    let units = session.units();
    let entity = session.entity(id, id)?;
    let slots = Slots::new(id, entity);

    let profile_ref = slots.req_ref(slot::SWEPT_AREA, "SweptArea")?;
    let depth = units.length(slots.req_f64(slot::DEPTH, "Depth")?);
    if depth <= 0.0 {
        return Err(slots.degenerate("extrusion depth is not positive"));
    }
    let direction = Vec3::from_array(direction_ratios(
        model,
        slots.req_ref(slot::EXTRUDED_DIRECTION, "ExtrudedDirection")?,
    )?);
    let placement = compose_placement(model, &slots, world, units)?.to_geom();

    let profile = lower_profile_node(session, profile_ref)?;
    let operation = session.node_for(
        id,
        GeometryNode::SolidOperation(SolidOperation::Extrusion {
            profile,
            direction,
            depth,
        }),
    )?;
    session.node_for(
        id,
        GeometryNode::Instance(Instance {
            source: operation,
            transform: placement,
        }),
    )
}

fn revolution_node(
    session: &mut LoweringSession<'_>,
    id: EntityId,
    world: Transform,
) -> GeometryResult<NodeId> {
    let model = session.model();
    let units = session.units();
    let entity = session.entity(id, id)?;
    let slots = Slots::new(id, entity);

    let profile_ref = slots.req_ref(slot::SWEPT_AREA, "SweptArea")?;
    let angle = units.angle(slots.req_f64(slot::ANGLE, "Angle")?);
    if angle <= 0.0 {
        return Err(slots.degenerate("revolution angle is not positive"));
    }

    let axis_id = slots.req_ref(slot::AXIS, "Axis")?;
    let axis = session.entity(id, axis_id)?;
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

    let profile = lower_profile_node(session, profile_ref)?;
    let operation = session.node_for(
        id,
        GeometryNode::SolidOperation(SolidOperation::Revolution {
            profile,
            axis_origin,
            axis_direction,
            angle,
        }),
    )?;
    session.node_for(
        id,
        GeometryNode::Instance(Instance {
            source: operation,
            transform: placement,
        }),
    )
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
    transform.to_metres(units)
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
