//! Half-space lowering: infinite cutting tools for boolean trees.
//!
//! # The polarity inversion that silently cuts the wrong half
//!
//! IFC and the neutral kernel disagree about what the boundary flag means.
//!
//! - `IfcHalfSpaceSolid.AgreementFlag = .T.` means the material lies on the
//!   side the base surface normal points **away from**.
//! - `axiolid_primitive::HalfSpace.agreement = true` selects the **normal**
//!   side.
//!
//! They are opposites. Transcribing the flag straight through produces a
//! clipping tool that keeps exactly the half it should have removed. Nothing
//! errors: the boolean still evaluates, the mesh is still watertight, and the
//! wall simply has the wrong end missing. `flip` below is the whole fix, and
//! the reason it is a named constant rather than an inline `!`.
//!
//! # Why a half space is never a root
//!
//! A half space has infinite volume. It is meaningful only as a boolean
//! operand, and the dispatcher reaches this module only through
//! `lower_boolean_result_node`. Lowering one as a representation item's root
//! would hand a consumer a solid it cannot bound, tessellate, or measure.
//!
//! # Bounded subtypes
//!
//! `IfcBoxedHalfSpace` adds an `Enclosure` box and
//! `IfcPolygonalBoundedHalfSpace` adds a prism. Both are *clipping hints* that
//! bound an otherwise infinite tool. The neutral `HalfSpace` node carries no
//! bound, so both currently lower to their underlying half space: the geometry
//! is correct, and the hint is dropped rather than approximated. See the
//! module's `PLAN.md` entry for why that is deliberate.

use axiolid_core::{Plane3, Point3, Vec3};
use axiolid_model::{GeometryNode, NodeId};
use axiolid_primitive::HalfSpace;
use ifc_model::EntityId;

use crate::error::{GeometryError, GeometryResult};
use crate::lower::session::LoweringSession;
use crate::resource::placement::axis_placement_transform;
use crate::solid::halfspace::HalfSpaceSolid;
use crate::surface::elementary::Plane;
use crate::transform::Transform;

/// Family label used for memoization.
const KIND: &str = "half space";

/// IFC's `.T.` is the side the normal points AWAY from; the kernel's `true`
/// is the normal side. Every conversion goes through this.
fn flip(ifc_agreement_flag: bool) -> bool {
    !ifc_agreement_flag
}

/// Lower an `IfcHalfSpaceSolid` (or a bounded subtype) into the session.
pub fn lower_half_space_node(
    session: &mut LoweringSession<'_>,
    id: EntityId,
    frame: Transform,
) -> GeometryResult<NodeId> {
    if let Some(node) = session.memoized(id, KIND, frame) {
        return Ok(node);
    }
    session.enter(id, KIND)?;
    let result = build(session, id, frame);
    session.exit(id);
    let node = result?;
    session.memoize(id, KIND, frame, node);
    Ok(node)
}

fn build(
    session: &mut LoweringSession<'_>,
    id: EntityId,
    frame: Transform,
) -> GeometryResult<NodeId> {
    let entity = session.entity(id, id)?;
    let view = HalfSpaceSolid::new(id, entity);
    let type_name = view.type_name().to_string();

    let surface_ref = view.base_surface()?;
    let agreement = view.agreement_flag()?;

    let boundary = plane_of(session, id, &type_name, surface_ref, frame)?;
    let half_space = HalfSpace {
        boundary,
        agreement: flip(agreement),
    };
    session.node_for(id, GeometryNode::HalfSpace(half_space))
}

/// Resolve the base surface to a placed plane.
///
/// Only `IfcPlane` is accepted. A curved base surface is legal IFC and would
/// need an exact surface node; reporting it as unsupported names the real gap
/// instead of silently substituting the tangent plane, which would cut along
/// the wrong shape.
fn plane_of(
    session: &mut LoweringSession<'_>,
    owner: EntityId,
    owner_type: &str,
    surface_ref: EntityId,
    frame: Transform,
) -> GeometryResult<Plane3> {
    let surface = session.entity(owner, surface_ref)?;
    let surface_type = surface.type_name.to_string();
    if !surface_type.eq_ignore_ascii_case("IFCPLANE") {
        return Err(session.unsupported(
            surface_ref,
            &surface_type,
            "half-space base surfaces other than IfcPlane need exact surface nodes",
        ));
    }

    let plane = Plane::new(surface_ref, surface);
    let position_ref = plane.position_ref()?;
    let position = session.entity(surface_ref, position_ref)?;
    let local = axis_placement_transform(session.model(), position_ref, position)?
        .to_metres(session.units());
    let placed = frame.compose(&local);

    // The placement's origin is a point on the plane; its +Z is the normal.
    // Origin goes through the full affine, the normal through the linear part
    // only -- translating a normal tilts every cut.
    let origin = placed.apply([0.0, 0.0, 0.0]);
    let normal = placed.apply_direction([0.0, 0.0, 1.0]);

    let length = (normal[0] * normal[0] + normal[1] * normal[1] + normal[2] * normal[2]).sqrt();
    if !length.is_finite() || length <= f64::EPSILON {
        return Err(GeometryError::Degenerate {
            entity: owner,
            type_name: owner_type.to_string(),
            detail: "base plane normal is zero-length or non-finite".to_string(),
        });
    }

    Ok(Plane3 {
        origin: Point3::from_array(origin),
        normal: Vec3::from_array([normal[0] / length, normal[1] / length, normal[2] / length]),
    })
}

#[cfg(test)]
mod tests;
