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
//! `IfcBoxedHalfSpace.Enclosure` is a search box only; the IFC specification
//! explicitly says it does not alter the Boolean result, so dropping that
//! computational hint preserves geometry exactly. `IfcPolygonalBoundedHalfSpace`
//! is different: its positioned boundary limits the cutting volume, so it
//! lowers to a bounded node carrying `Position` and the boundary (see
//! `boundary`), never widened to an infinite half-space.
//!
//! # The prism axis (#515)
//!
//! IFC bounds the half space by the boundary's infinite prism along
//! `Position`'s Z axis, in both directions, whatever the plane's slope: a
//! sloped roof plane over a vertical prism is the usual gable-wall clip.
//! That is exactly Axiolid's `SolidOperation::BoundedHalfSpaceAlongZ`
//! (axiolid-model 0.3.14, axiolid/kernel#321): the kept side of the plane
//! within the lines through the boundary's region parallel to the
//! placement's Z, the boundary point `(u, v)` at `placement * (u, v, 0)`
//! with the frame read whole (its scale, shear and handedness), only the
//! direction of Z counting. Every `IfcPolygonalBoundedHalfSpace` lowers to
//! that one node, `Position` carried as written, so there is one path and
//! nothing here decides an orientation, mirrors a footprint or splits a
//! prism. Up to ifc-geometry 0.27.2 the clip lowered to
//! `BoundedHalfSpace`, whose prism runs along the plane NORMAL: a plane
//! against `Position`'s Z was read from its other side, and an oblique or
//! rescaled prism lowered to a chain of half spaces for a convex polygon
//! and was refused otherwise.
//!
//! `BoundedHalfSpace` is not emitted for the aligned case either, though it
//! is the same set there and keeps such graphs on wire format 1.3: both of
//! Axiolid's compilers compile the new node for every frame (the exact one
//! certifies it), so a second, frame-dependent lowering would only buy an
//! older wire label at the cost of the case analysis this module dropped.
//!
//! What the kernel refuses of the node it refuses by name when compiled,
//! never here: a Z lying in the plane (the prism never crosses it), a
//! degenerate placement, the node as a root or a union's operand (it is
//! unbounded on its kept side), and in the exact compiler an arc boundary
//! under an uneven scale (its wall is an elliptic cylinder, which the mesh
//! compiler cuts). An IFC `Position` is orthonormal, so the frame is scaled,
//! sheared or mirrored only by a mapped item's operator, whose scaled
//! instance the exact compiler does not take in any case.
//!
//! # The depth (#519)
//!
//! The node carries no depth: it is the half space cut by an infinite
//! prism, and the compilers bound it by the subject they cut
//! (axiolid-mesh-compile 0.3.27, axiolid/kernel#325). Nothing in this
//! bridge sizes the cut (ADR 0004).

use axiolid_core::{Plane3, Point3, Vec3};
use axiolid_model::{GeometryNode, NodeId, SolidOperation};
use axiolid_primitive::HalfSpace;
use ifc_model::EntityId;

use crate::error::{GeometryError, GeometryResult};
use crate::lower::session::LoweringSession;
use crate::resource::placement::axis_placement_transform;
use crate::solid::halfspace::{HalfSpaceSolid, PolygonalBoundedHalfSpace};
use crate::surface::elementary::Plane;
use crate::transform::Transform;

/// Family label used for memoization.
const KIND: &str = "half space";

/// IFC's `.T.` is the side the normal points AWAY from; the kernel's `true`
/// is the normal side. Every conversion goes through this.
fn flip(ifc_agreement_flag: bool) -> bool {
    !ifc_agreement_flag
}

/// Lower an `IfcHalfSpaceSolid` (or the geometry-equivalent boxed subtype).
pub fn lower_half_space_node(
    session: &mut LoweringSession<'_>,
    id: EntityId,
    frame: Transform,
) -> GeometryResult<NodeId> {
    let type_name = session.type_name(id)?;
    if let Some(node) = session.memoized(id, KIND, frame) {
        return Ok(node);
    }
    session.enter(id, KIND)?;
    let result = if type_name.eq_ignore_ascii_case("IFCPOLYGONALBOUNDEDHALFSPACE") {
        build_polygonal(session, id, frame)
    } else {
        build(session, id, frame)
    };
    session.exit(id);
    let node = result?;
    session.memoize(id, KIND, frame, node);
    Ok(node)
}

/// `IfcPolygonalBoundedHalfSpace`: half space clipped by an extruded polygon.
///
/// `Position` is independent of `BaseSurface`, and the boundary is authored in
/// that placement's XY plane, its prism along that placement's Z. The node
/// carries `Position` whole, beside the plane, so the placement is never
/// folded into the clip plane, which would move the clip (see "The prism
/// axis" above).
fn build_polygonal(
    session: &mut LoweringSession<'_>,
    id: EntityId,
    frame: Transform,
) -> GeometryResult<NodeId> {
    let entity = session.entity(id, id)?;
    let view = PolygonalBoundedHalfSpace::new(id, entity);
    let base = view.base();
    let type_name = base.type_name().to_string();

    // The infinite half space, exactly as the unbounded case builds it.
    let plane = plane_of(session, id, &type_name, base.base_surface()?, frame)?;
    let agreement = flip(base.agreement_flag()?);
    let position_ref = view.position()?;
    let boundary = view.polygonal_boundary()?;

    // Position both places the prism and orients the authored boundary
    // within its own plane. It is carried as written: the kernel reads its
    // scale, shear and handedness.
    let position = session.entity(id, position_ref)?;
    let local = axis_placement_transform(session.model(), position_ref, position)?
        .to_metres(session.units());
    let placed = frame.compose(&local);

    let half_space = session.node_for(
        id,
        GeometryNode::HalfSpace(HalfSpace {
            boundary: plane,
            agreement,
        }),
    )?;

    // The 2D boundary is a closed bounded curve, or a line-and-arc contour,
    // in Position's XY plane. It lowers as a curve or a profile, never a 3D
    // curve: that is the kernel's contract, and the 3D curve path produced
    // graphs that lowered but never compiled (#45).
    let boundary = boundary::lower_boundary_curve(session, id, boundary)?;

    session.node_for(
        id,
        GeometryNode::SolidOperation(SolidOperation::BoundedHalfSpaceAlongZ {
            half_space,
            boundary,
            placement: placed.to_geom(),
        }),
    )
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

    // The placement's origin is a point on the plane. A normal is a covector,
    // so non-uniform affine scale requires inverse-transpose transformation.
    let origin = placed.apply([0.0, 0.0, 0.0]);
    let normal =
        placed
            .apply_unit_normal([0.0, 0.0, 1.0])
            .ok_or_else(|| GeometryError::Degenerate {
                entity: owner,
                type_name: owner_type.to_string(),
                detail: "base plane transform is singular or non-finite".to_string(),
            })?;

    Ok(Plane3 {
        origin: Point3::from_array(origin),
        normal: Vec3::from_array(normal),
    })
}

mod boundary;

#[cfg(test)]
mod tests;
