//! Surface lowering: the LOW-EXACT surface half.
//!
//! # Scope
//!
//! Covers the surface families the corpus exercises: `IfcPlane` (already
//! needed by half spaces, now reachable as a surface in its own right) and
//! `IfcSurfaceOfLinearExtrusion`. The curved elementary families
//! (`IfcCylindricalSurface`, `IfcSphericalSurface`, `IfcToroidalSurface`,
//! `IfcSurfaceOfRevolution`) and the B-spline families have complete readers
//! in `crate::surface` but no licensed fixture to prove a lowering against,
//! so they stay in `dispatch::PLANNED` with a stated reason rather than
//! shipping untested code paths.
//!
//! # `Depth` is a hint, not a bound
//!
//! `IfcSurfaceOfLinearExtrusion` carries a `Depth`, but the surface it
//! defines is **unbounded** in the extrusion parameter: the schema's own
//! definition sweeps the curve infinitely and `Depth` exists so a viewer can
//! draw something finite. The neutral `SurfaceRelation::LinearExtrusion`
//! therefore has no depth field.
//!
//! Scaling the direction by `Depth` to "keep" the information would change
//! the surface's parameterisation: a point at parameter `v` would move to
//! `v * depth`, silently reparameterising every trim taken against this
//! surface. The direction is lowered as a unit-magnitude direction and the
//! depth is deliberately dropped, which is lossy in exactly the way the
//! schema intends.

use axiolid_core::Vec3;
use axiolid_model::{GeometryNode, NodeId, SurfaceRelation};
use axiolid_surface::{Plane as KernelPlane, Surface};
use ifc_model::EntityId;

use crate::error::GeometryResult;
use crate::lower::session::LoweringSession;
use crate::resource::direction::resolve_unit;
use crate::resource::placement::axis_placement_transform;
use crate::surface::elementary::Plane;
use crate::surface::swept::SurfaceOfLinearExtrusion;
use crate::transform::Transform;

/// Family label used for surface memoization.
const SURFACE: &str = "surface";

/// Lower any supported `IfcSurface` into a node.
///
/// Kept as one entry point so a caller holding only an `IfcSurface` reference
/// (a half space's `BaseSurface`, a swept solid's `ReferenceSurface`) does not
/// have to re-dispatch on the concrete subtype itself.
pub fn lower_surface_node(
    session: &mut LoweringSession<'_>,
    id: EntityId,
    frame: Transform,
) -> GeometryResult<NodeId> {
    if let Some(existing) = session.memoized(id, SURFACE, frame) {
        return Ok(existing);
    }
    let type_name = session.type_name(id)?.to_ascii_uppercase();
    let node = match type_name.as_str() {
        "IFCPLANE" => lower_plane(session, id, frame)?,
        "IFCSURFACEOFLINEAREXTRUSION" => lower_linear_extrusion(session, id, frame)?,
        other => {
            return Err(session.unsupported(id, other, "curved and B-spline surfaces"));
        }
    };
    session.memoize(id, SURFACE, frame, node);
    Ok(node)
}

/// Lower an `IfcPlane` into a kernel plane.
///
/// The placement's Z axis is the normal and its origin the reference point;
/// both are placed by `frame` and the origin converted to metres. The normal
/// takes the linear part only, so an off-origin plane keeps its orientation.
pub fn lower_plane(
    session: &mut LoweringSession<'_>,
    id: EntityId,
    frame: Transform,
) -> GeometryResult<NodeId> {
    let entity = session.entity(id, id)?;
    let view = Plane::new(id, entity);
    let position_id = view.position_ref()?;
    let position = session.entity(id, position_id)?;
    let local = axis_placement_transform(session.model(), position_id, position)?
        .to_metres(session.units());
    let placed = frame.compose(&local);

    let normal = placed.apply_direction([0.0, 0.0, 1.0]);
    let length = (normal[0] * normal[0] + normal[1] * normal[1] + normal[2] * normal[2]).sqrt();
    if !length.is_finite() || length <= f64::EPSILON {
        return Err(session.degenerate(
            id,
            "IFCPLANE",
            "plane normal is zero-length or non-finite",
        ));
    }

    let plane = KernelPlane {
        frame: placed.to_geom_frame(),
    };
    session.node_for(id, GeometryNode::Surface(Surface::Plane(plane)))
}

/// Lower an `IfcSurfaceOfLinearExtrusion` into a linear-extrusion relation.
///
/// `Depth` is intentionally not carried: see the module docs. The swept curve
/// is lowered first so the relation can reference its node.
pub fn lower_linear_extrusion(
    session: &mut LoweringSession<'_>,
    id: EntityId,
    frame: Transform,
) -> GeometryResult<NodeId> {
    let entity = session.entity(id, id)?;
    let view = SurfaceOfLinearExtrusion::new(id, entity);

    // An optional Position places the whole swept surface.
    let placed = match view.position_ref() {
        Some(position_id) => {
            let position = session.entity(id, position_id)?;
            let local = axis_placement_transform(session.model(), position_id, position)?
                .to_metres(session.units());
            frame.compose(&local)
        }
        None => frame,
    };

    let curve_id = view.swept_curve_ref()?;
    let swept_curve = crate::lower::curve::lower_curve_node(session, curve_id, placed)?;

    // ExtrudedDirection is already unit: `resolve_unit` normalizes at the IFC
    // boundary, which is where this crate's contract says directions are
    // normalized exactly once. Under a rigid placement `apply_direction`
    // preserves that, so re-normalizing here would be dead code. Only the
    // degenerate case still needs rejecting.
    let raw = resolve_unit(session.model(), id, view.extruded_direction_ref()?)?;
    let direction = placed.apply_direction(raw);
    let length =
        (direction[0] * direction[0] + direction[1] * direction[1] + direction[2] * direction[2])
            .sqrt();
    if !length.is_finite() || length <= f64::EPSILON {
        return Err(session.degenerate(
            id,
            "IFCSURFACEOFLINEAREXTRUSION",
            "extruded direction is zero-length or non-finite",
        ));
    }

    session.node_for(
        id,
        GeometryNode::SurfaceRelation(SurfaceRelation::LinearExtrusion {
            swept_curve,
            direction: Vec3::from_array(direction),
        }),
    )
}

#[cfg(test)]
mod tests;
