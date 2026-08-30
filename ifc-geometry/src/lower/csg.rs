//! CSG solids and swept-disk solids.
//!
//! # `IfcCsgSolid` is a wrapper, not a shape
//!
//! Its single `TreeRootExpression` is either a boolean result or a primitive.
//! The solid itself contributes no geometry, so it lowers to whatever its root
//! lowers to. Emitting a distinct node for the wrapper would add a graph level
//! that no consumer can act on.
//!
//! # Primitives are local, placement is separate
//!
//! `IfcCsgPrimitive3D` subtypes define their shape in their OWN coordinate
//! system: an `IfcBlock` always sits with a corner at its local origin and
//! extends along +x/+y/+z. The `Position` places it. `Primitive::Block` in the
//! kernel is likewise local-only, so the placement must ride on an Instance
//! node -- folding it into the extents would be wrong for any non-identity
//! rotation and would silently discard the origin offset.
//!
//! # Swept disks sweep a DISK, not a profile
//!
//! `IfcSweptDiskSolid` has no `IfcProfileDef`: the radii are given directly.
//! `InnerRadius` makes it a pipe. The directrix is a curve, which is why this
//! family waited for curve lowering.

use axiolid_model::{GeometryNode, Instance, NodeId, SolidOperation};
use axiolid_primitive::Primitive;
use ifc_model::EntityId;

use crate::error::GeometryResult;
use crate::lower::curve::{lower_curve_node, scale_parameter};
use crate::lower::profile::lower_profile_node;
use crate::lower::session::LoweringSession;
use crate::lower::surface::lower_surface_node;
use crate::resource::placement::axis_placement_transform;
use crate::slots::Slots;
use crate::solid::csg::{CsgPrimitive3D, CsgSolid};
use crate::solid::swept::directrix::{
    SurfaceCurveSweptAreaSolid, SweptDiskSolid, SweptDiskSolidPolygonal,
};
use crate::transform::Transform;

const CSG: &str = "csg solid";
const DISK: &str = "swept disk";

/// Lower an `IfcCsgSolid` by lowering its tree root.
pub fn lower_csg_solid_node(
    session: &mut LoweringSession<'_>,
    id: EntityId,
    frame: Transform,
) -> GeometryResult<NodeId> {
    if let Some(node) = session.memoized(id, CSG, frame) {
        return Ok(node);
    }
    session.enter(id, CSG)?;
    let result = (|| {
        let entity = session.entity(id, id)?;
        let root = CsgSolid::new(id, entity).tree_root_expression()?;
        // The wrapper contributes nothing; the root IS the solid.
        session.lower_operand(root, frame)
    })();
    session.exit(id);
    let node = result?;
    session.memoize(id, CSG, frame, node);
    Ok(node)
}

/// Lower an `IfcCsgPrimitive3D` subtype into a placed primitive.
pub fn lower_csg_primitive_node(
    session: &mut LoweringSession<'_>,
    id: EntityId,
    frame: Transform,
) -> GeometryResult<NodeId> {
    if let Some(node) = session.memoized(id, CSG, frame) {
        return Ok(node);
    }
    session.enter(id, CSG)?;
    let result = build_primitive(session, id, frame);
    session.exit(id);
    let node = result?;
    session.memoize(id, CSG, frame, node);
    Ok(node)
}

fn build_primitive(
    session: &mut LoweringSession<'_>,
    id: EntityId,
    frame: Transform,
) -> GeometryResult<NodeId> {
    let type_name = session.type_name(id)?;
    let entity = session.entity(id, id)?;
    let slots = Slots::new(id, entity);
    let view = CsgPrimitive3D::new(id, entity);

    // Slot 0 is `Position` on every IfcCsgPrimitive3D subtype.
    let _ = &view;
    let position_ref = slots.req_ref(0, "Position")?;
    let position = session.entity(id, position_ref)?;
    let local = axis_placement_transform(session.model(), position_ref, position)?
        .to_metres(session.units());

    let length = |value: f64| session.units().length(value);
    let primitive = match type_name.as_str() {
        "IFCBLOCK" => Primitive::Block {
            x: length(slots.req_f64(1, "XLength")?),
            y: length(slots.req_f64(2, "YLength")?),
            z: length(slots.req_f64(3, "ZLength")?),
        },
        "IFCSPHERE" => Primitive::Sphere {
            radius: length(slots.req_f64(1, "Radius")?),
        },
        "IFCRIGHTCIRCULARCYLINDER" => Primitive::Cylinder {
            height: length(slots.req_f64(1, "Height")?),
            radius: length(slots.req_f64(2, "Radius")?),
        },
        "IFCRIGHTCIRCULARCONE" => Primitive::Cone {
            height: length(slots.req_f64(1, "Height")?),
            radius: length(slots.req_f64(2, "BottomRadius")?),
        },
        // Slots follow IfcBlock (XLength, YLength, then the third length),
        // NOT IfcRightCircularCone, which puts Height first. Copying the cone
        // arm here would swap height with x and still build a valid pyramid.
        "IFCRECTANGULARPYRAMID" => Primitive::Pyramid {
            x: length(slots.req_f64(1, "XLength")?),
            y: length(slots.req_f64(2, "YLength")?),
            height: length(slots.req_f64(3, "Height")?),
        },
        other => return Err(session.unsupported(id, other, "CSG primitive family")),
    };

    // The primitive is local; the placement rides on an Instance so a
    // non-identity rotation is preserved rather than folded into extents.
    let source = session.node(GeometryNode::Primitive(primitive))?;
    let placed = frame.compose(&local);
    session.node_for(
        id,
        GeometryNode::Instance(Instance {
            source,
            transform: placed.to_geom(),
        }),
    )
}

/// Lower an `IfcSweptDiskSolid` into a `SweptDisk` operation.
///
/// The directrix is lowered first so the operation can reference its node,
/// matching the operand-before-operation ordering the append-only builder
/// requires.
pub fn lower_swept_disk_node(
    session: &mut LoweringSession<'_>,
    id: EntityId,
    frame: Transform,
) -> GeometryResult<NodeId> {
    if let Some(node) = session.memoized(id, DISK, frame) {
        return Ok(node);
    }
    session.enter(id, DISK)?;
    let result = build_disk(session, id, frame);
    session.exit(id);
    let node = result?;
    session.memoize(id, DISK, frame, node);
    Ok(node)
}

fn build_disk(
    session: &mut LoweringSession<'_>,
    id: EntityId,
    frame: Transform,
) -> GeometryResult<NodeId> {
    let entity = session.entity(id, id)?;
    let view = SweptDiskSolid::new(id, entity);

    // IfcSweptDiskSolidPolygonal adds FilletRadius, which rounds the corners
    // where consecutive directrix segments meet. The neutral SweptDisk carries
    // it directly, so the subtype lowers as an ordinary swept disk that
    // happens to know its corner radius.
    let fillet_radius = if session
        .type_name(id)?
        .eq_ignore_ascii_case("IFCSWEPTDISKSOLIDPOLYGONAL")
    {
        SweptDiskSolidPolygonal::new(id, entity)
            .fillet_radius()
            .map(|value| session.units().length(value))
    } else {
        None
    };

    // `checked_radii` enforces the schema's inner < outer rule. A pipe whose
    // inner radius meets or exceeds the outer has no material at all, and the
    // failure downstream is an empty mesh rather than an error.
    let (radius_raw, inner_raw) = view.checked_radii()?;
    let radius = session.units().length(radius_raw);
    let inner_radius = inner_raw.map(|value| session.units().length(value));

    let directrix_ref = view.directrix()?;
    let directrix = lower_curve_node(session, directrix_ref, frame)?;

    // Trim parameters are in the DIRECTRIX's parameterisation, which is not
    // always a length: see scale_parameter for the index-based cases.
    let directrix_kind = session.type_name(directrix_ref)?;
    let convert = |value: f64| scale_parameter(session, directrix_kind.as_str(), value);
    let parameter_range = match (view.start_param(), view.end_param()) {
        (Some(start), Some(end)) => Some((convert(start), convert(end))),
        (None, None) => None,
        _ => {
            return Err(session.degenerate(
                id,
                "IFCSWEPTDISKSOLID",
                "only one of StartParam/EndParam is present; a half-open sweep is undefined",
            ))
        }
    };

    session.node_for(
        id,
        GeometryNode::SolidOperation(SolidOperation::SweptDisk {
            directrix,
            radius,
            inner_radius,
            parameter_range,
            fillet_radius,
        }),
    )
}

#[cfg(test)]
mod tests;

/// Lower an `IfcSurfaceCurveSweptAreaSolid` into a `SurfaceCurveSweep`.
///
/// The directrix of this family is a curve that *lies on* the reference
/// surface. The surface is not decoration: it fixes the sweep's twist. At any
/// point of the directrix the profile is oriented by the surface normal
/// there, so two solids with identical profiles and identical directrices but
/// different reference surfaces are different solids.
///
/// Dropping the surface and treating this as a plain directrix sweep produces
/// a shape with the right footprint and the wrong cross-section rotation --
/// a duct elbow whose flanges twist. That is why the reference surface is
/// lowered and referenced rather than ignored.
pub fn lower_surface_curve_swept_area_solid_node(
    session: &mut LoweringSession<'_>,
    id: EntityId,
    frame: Transform,
) -> GeometryResult<NodeId> {
    if let Some(node) = session.memoized(id, CSG, frame) {
        return Ok(node);
    }
    let entity = session.entity(id, id)?;
    let view = SurfaceCurveSweptAreaSolid::new(id, entity);
    let base = view.base();

    // An optional Position places the swept solid.
    let placed = match base.position() {
        Some(position_id) => {
            let position = session.entity(id, position_id)?;
            let local = axis_placement_transform(session.model(), position_id, position)?
                .to_metres(session.units());
            frame.compose(&local)
        }
        None => frame,
    };

    let profile = lower_profile_node(session, base.swept_area()?)?;
    let directrix_ref = view.directrix()?;
    let directrix = lower_curve_node(session, directrix_ref, placed)?;
    let reference_surface = lower_surface_node(session, view.reference_surface()?, placed)?;

    // Same rule as the swept disk: a parameter on a conic directrix is an
    // angle, on anything else a length.
    let directrix_kind = session.type_name(directrix_ref)?;
    let convert = |value: f64| match directrix_kind.as_str() {
        "IFCCIRCLE" | "IFCELLIPSE" | "IFCTRIMMEDCURVE" => session.units().angle(value),
        _ => session.units().length(value),
    };
    let parameter_range = match (view.start_param(), view.end_param()) {
        (Some(start), Some(end)) => Some((convert(start), convert(end))),
        (None, None) => None,
        _ => {
            return Err(session.degenerate(
                id,
                "IFCSURFACECURVESWEPTAREASOLID",
                "only one of StartParam/EndParam is present; the sweep extent is ambiguous",
            ));
        }
    };

    let node = session.node_for(
        id,
        GeometryNode::SolidOperation(SolidOperation::SurfaceCurveSweep {
            profile,
            directrix,
            reference_surface,
            parameter_range,
        }),
    )?;
    session.memoize(id, CSG, frame, node);
    Ok(node)
}
