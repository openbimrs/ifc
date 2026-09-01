//! Exact swept-solid lowering into the format-neutral geometry DAG.
//!
//! Each family has two entry points. The `_node` form appends into a caller-
//! owned [`LoweringSession`] and returns a [`NodeId`], so a composite parent
//! (boolean, mapped item, CSG) can reference the result. The non-`_node` form
//! is the convenience wrapper that opens a session, lowers one item, and
//! freezes the graph.

use axiolid_core::{Point3, Vec3};
use axiolid_model::{GeometryNode, Instance, NodeId, Section, SolidOperation};
use ifc_model::{EntityId, Model};

use crate::error::{GeometryError, GeometryResult};
use crate::lower::curve::{lower_curve_node, scale_parameter};
use crate::lower::session::LoweringSession;
use crate::lower::{lower_profile_node, LoweredGeometry};
use crate::resource::placement::axis_placement_transform;
use crate::slots::Slots;
use crate::solid::swept::area::{ExtrudedAreaSolidTapered, RevolvedAreaSolidTapered};
use crate::solid::swept::directrix::{FixedReferenceSweptAreaSolid, SectionedSpine};
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
/// Memo/chain kind for `IfcExtrudedAreaSolidTapered`.
const TAPERED_EXTRUSION: &str = "tapered extruded area solid";
/// Memo/chain kind for `IfcRevolvedAreaSolidTapered`.
const TAPERED_REVOLUTION: &str = "tapered revolved area solid";
/// Memo/chain kind for `IfcFixedReferenceSweptAreaSolid`.
const FIXED_REFERENCE: &str = "fixed reference swept area solid";
/// Memo/chain kind for `IfcSectionedSpine`.
const SECTIONED_SPINE: &str = "sectioned spine";

/// Lower one `IfcExtrudedAreaSolid` into an exact profile plus extrusion node.
pub fn lower_extruded_area_solid(
    model: &Model,
    id: EntityId,
    world: Transform,
    units: &UnitScale,
) -> GeometryResult<LoweredGeometry> {
    let mut session = LoweringSession::new(model, units);
    let root = lower_extruded_area_solid_node(&mut session, id, world)?;
    session.finish(root)
}

/// Lower one `IfcRevolvedAreaSolid` into an exact profile plus revolution node.
pub fn lower_revolved_area_solid(
    model: &Model,
    id: EntityId,
    world: Transform,
    units: &UnitScale,
) -> GeometryResult<LoweredGeometry> {
    let mut session = LoweringSession::new(model, units);
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

/// Append one `IfcExtrudedAreaSolidTapered` to a shared session.
pub fn lower_tapered_extrusion_node(
    session: &mut LoweringSession<'_>,
    id: EntityId,
    world: Transform,
) -> GeometryResult<NodeId> {
    if let Some(node) = session.memoized(id, TAPERED_EXTRUSION, world) {
        return Ok(node);
    }
    session.enter(id, "swept solid")?;
    let result = tapered_extrusion_node(session, id, world);
    session.exit(id);
    let node = result?;
    session.memoize(id, TAPERED_EXTRUSION, world, node);
    Ok(node)
}

/// Append one `IfcRevolvedAreaSolidTapered` to a shared session.
pub fn lower_tapered_revolution_node(
    session: &mut LoweringSession<'_>,
    id: EntityId,
    world: Transform,
) -> GeometryResult<NodeId> {
    if let Some(node) = session.memoized(id, TAPERED_REVOLUTION, world) {
        return Ok(node);
    }
    session.enter(id, "swept solid")?;
    let result = tapered_revolution_node(session, id, world);
    session.exit(id);
    let node = result?;
    session.memoize(id, TAPERED_REVOLUTION, world, node);
    Ok(node)
}

/// Append one `IfcFixedReferenceSweptAreaSolid` to a shared session.
pub fn lower_fixed_reference_sweep_node(
    session: &mut LoweringSession<'_>,
    id: EntityId,
    world: Transform,
) -> GeometryResult<NodeId> {
    if let Some(node) = session.memoized(id, FIXED_REFERENCE, world) {
        return Ok(node);
    }
    session.enter(id, "swept solid")?;
    let result = fixed_reference_sweep_node(session, id, world);
    session.exit(id);
    let node = result?;
    session.memoize(id, FIXED_REFERENCE, world, node);
    Ok(node)
}

/// Append one `IfcSectionedSpine` to a shared session.
pub fn lower_sectioned_spine_node(
    session: &mut LoweringSession<'_>,
    id: EntityId,
    world: Transform,
) -> GeometryResult<NodeId> {
    if let Some(node) = session.memoized(id, SECTIONED_SPINE, world) {
        return Ok(node);
    }
    session.enter(id, "sectioned spine")?;
    let result = sectioned_spine_node(session, id, world);
    session.exit(id);
    let node = result?;
    session.memoize(id, SECTIONED_SPINE, world, node);
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

/// Lower an `IfcExtrudedAreaSolidTapered` into a `TaperedExtrusion`.
///
/// The end profile is the whole point: reusing `SweptArea` for both ends
/// yields a prism that builds, renders, and silently discards the taper.
fn tapered_extrusion_node(
    session: &mut LoweringSession<'_>,
    id: EntityId,
    world: Transform,
) -> GeometryResult<NodeId> {
    let model = session.model();
    let units = session.units();
    let entity = session.entity(id, id)?;
    let slots = Slots::new(id, entity);
    let view = ExtrudedAreaSolidTapered::new(id, entity);

    let start_ref = slots.req_ref(slot::SWEPT_AREA, "SweptArea")?;
    let end_ref = view.end_swept_area()?;
    let depth = units.length(slots.req_f64(slot::DEPTH, "Depth")?);
    if depth <= 0.0 {
        return Err(slots.degenerate("extrusion depth is not positive"));
    }
    let direction = Vec3::from_array(direction_ratios(
        model,
        slots.req_ref(slot::EXTRUDED_DIRECTION, "ExtrudedDirection")?,
    )?);
    let placement = compose_placement(model, &slots, world, units)?.to_geom();

    let start_profile = lower_profile_node(session, start_ref)?;
    let end_profile = lower_profile_node(session, end_ref)?;
    let operation = session.node_for(
        id,
        GeometryNode::SolidOperation(SolidOperation::TaperedExtrusion {
            start_profile,
            end_profile,
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

/// Lower an `IfcRevolvedAreaSolidTapered` into a `TaperedRevolution`.
fn tapered_revolution_node(
    session: &mut LoweringSession<'_>,
    id: EntityId,
    world: Transform,
) -> GeometryResult<NodeId> {
    let model = session.model();
    let units = session.units();
    let entity = session.entity(id, id)?;
    let slots = Slots::new(id, entity);
    let view = RevolvedAreaSolidTapered::new(id, entity);

    let start_ref = slots.req_ref(slot::SWEPT_AREA, "SweptArea")?;
    let end_ref = view.end_swept_area()?;
    // Angle is an IfcPlaneAngleMeasure in the file's declared unit, very often
    // degrees. units.angle applies that conversion exactly once.
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

    let start_profile = lower_profile_node(session, start_ref)?;
    let end_profile = lower_profile_node(session, end_ref)?;
    let operation = session.node_for(
        id,
        GeometryNode::SolidOperation(SolidOperation::TaperedRevolution {
            start_profile,
            end_profile,
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

/// Lower an `IfcFixedReferenceSweptAreaSolid` into a `FixedReferenceSweep`.
///
/// The fixed reference is what distinguishes this from an ordinary directrix
/// sweep: the section keeps a constant orientation relative to that direction
/// instead of rotating with the curve's Frenet frame. Dropping it produces a
/// sweep that twists through bends.
fn fixed_reference_sweep_node(
    session: &mut LoweringSession<'_>,
    id: EntityId,
    world: Transform,
) -> GeometryResult<NodeId> {
    let model = session.model();
    let units = session.units();
    let entity = session.entity(id, id)?;
    let slots = Slots::new(id, entity);
    let view = FixedReferenceSweptAreaSolid::new(id, entity);

    let profile_ref = slots.req_ref(slot::SWEPT_AREA, "SweptArea")?;
    let reference_direction = Vec3::from_array(direction_ratios(model, view.fixed_reference()?)?);
    let placement = compose_placement(model, &slots, world, units)?.to_geom();

    let directrix_ref = view.directrix()?;
    let directrix = lower_curve_node(session, directrix_ref, world)?;

    // Trim parameters live in the DIRECTRIX's parameterisation, so a conic
    // directrix measures in angle while a polyline or composite measures in
    // length. Same split as IfcTrimmedCurve and IfcSweptDiskSolid.
    let directrix_kind = session.type_name(directrix_ref)?;
    let convert = |value: f64| scale_parameter(session, directrix_kind.as_str(), value);
    let parameter_range = match (view.start_param(), view.end_param()) {
        (Some(start), Some(end)) => Some((convert(start), convert(end))),
        (None, None) => None,
        _ => {
            return Err(session.degenerate(
                id,
                "IFCFIXEDREFERENCESWEPTAREASOLID",
                "only one of StartParam/EndParam is present; a half-open sweep is undefined",
            ))
        }
    };

    let profile = lower_profile_node(session, profile_ref)?;
    let operation = session.node_for(
        id,
        GeometryNode::SolidOperation(SolidOperation::FixedReferenceSweep {
            profile,
            directrix,
            reference_direction,
            parameter_range,
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

/// Lower an `IfcSectionedSpine` into a `SectionedSpine`.
///
/// # No Position to compose
///
/// Unlike the swept-area family this subtypes `IfcGeometricRepresentationItem`
/// directly: there is no inherited `Position`, so the world frame applies to
/// the spine curve and each section placement rather than to a wrapper
/// `Instance`.
fn sectioned_spine_node(
    session: &mut LoweringSession<'_>,
    id: EntityId,
    world: Transform,
) -> GeometryResult<NodeId> {
    let entity = session.entity(id, id)?;
    let view = SectionedSpine::new(id, entity);

    let spine = lower_curve_node(session, view.spine_curve()?, world)?;

    // checked_sections rejects a length mismatch between CrossSections and
    // CrossSectionPositions. A plain zip would truncate to the shorter list,
    // producing a solid quietly missing its tail.
    let paired = view.checked_sections()?;
    let mut sections = Vec::with_capacity(paired.len());
    for (profile_ref, position_ref) in paired {
        let position = session.entity(id, position_ref)?;
        // to_metres converts the placement ORIGIN once; the rotation basis is
        // dimensionless and must not be scaled.
        let placement = axis_placement_transform(session.model(), position_ref, position)?
            .to_metres(session.units());
        let profile = lower_profile_node(session, profile_ref)?;
        sections.push(Section {
            profile,
            placement: world.compose(&placement).to_geom(),
        });
    }

    session.node_for(
        id,
        GeometryNode::SolidOperation(SolidOperation::SectionedSpine { spine, sections }),
    )
}
