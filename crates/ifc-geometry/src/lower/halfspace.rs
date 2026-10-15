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
//! lowers to `SolidOperation::BoundedHalfSpace` carrying `Position` and a
//! `Curve2` boundary (see `boundary`), never widened to an infinite half-space.
//!
//! # The prism axis, and the clips that used to vanish (#515)
//!
//! IFC bounds the half space by the boundary's infinite prism along
//! `Position`'s Z axis, in both directions, whatever the plane's slope.
//! Axiolid's `BoundedHalfSpace` is a different solid: its prism runs along
//! the clip plane's NORMAL, and its in-plane frame is `Position`'s X axis
//! projected into the plane (direction only) and `normal x X`
//! (axiolid-model 0.3.13 `SolidOperation::BoundedHalfSpace`,
//! axiolid-construct 0.3.18 `half_space::bounded_half_space_in_frame`). The
//! two are the same set only when `Position`'s Z is along the normal and its
//! X and Y project to an unscaled frame about it. Otherwise the node lowered
//! cleanly and compiled to another tool, with no error:
//!
//! - **Oblique** (a sloped roof plane over a vertical prism, the usual
//!   gable-wall clip): the boundary was laid into the slope and, the normal
//!   pointing down, mirrored off the wall, so the wall meshed uncut.
//! - **Against** (a normal opposite `Position`'s Z, the usual way to say
//!   "remove what is above"): `normal x X` is `-Y`, so the footprint was
//!   mirrored across `Position`'s X axis.
//!
//! [`prism_orientation`] decides which case holds. *Against* has an exact
//! answer here: the plane with its normal reversed and its side flipped is
//! the same half space, now framed by `(X, Y)`. *Oblique* and a scaled or
//! sheared frame have none in the neutral model and are refused by name;
//! they wait on a kernel node whose prism runs along its own placement's Z.

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
/// that placement's XY plane. Axiolid's `BoundedHalfSpace` carries the boundary
/// frame separately for exactly this reason, so the placement is preserved
/// rather than folded into the clip plane, which would move the clip.
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

    // Position is independent of BaseSurface and must survive: it both places
    // the prism and orients the authored profile within its own plane.
    let position_ref = view.position()?;
    let position = session.entity(id, position_ref)?;
    let local = axis_placement_transform(session.model(), position_ref, position)?
        .to_metres(session.units());
    let placed = frame.compose(&local);

    // The neutral node holds the IFC prism only for some frames; orient the
    // plane so it does, or refuse (#515, see "The prism axis" above).
    let half_space =
        match prism_orientation(&placed, plane.normal) {
            PrismOrientation::Along => HalfSpace {
                boundary: plane,
                agreement,
            },
            PrismOrientation::Against => HalfSpace {
                boundary: Plane3 {
                    origin: plane.origin,
                    normal: -plane.normal,
                },
                agreement: !agreement,
            },
            PrismOrientation::Oblique => {
                return Err(session.unsupported(
                    id,
                    &type_name,
                    "a boundary prism oblique to the base plane: Position's Z axis is not along \
                 the plane normal, and the neutral bounded half-space sweeps its boundary \
                 along the normal",
                ))
            }
            PrismOrientation::Distorted => return Err(session.unsupported(
                id,
                &type_name,
                "a scaled or sheared boundary frame: the neutral bounded half-space reads only \
                 the direction of Position's X axis, so the boundary would keep its authored size",
            )),
        };
    let half_space = session.node_for(id, GeometryNode::HalfSpace(half_space))?;

    // The 2D boundary is a closed bounded curve in Position's XY plane. It
    // lowers as a curve, not a profile: the kernel extrudes it along +Z itself,
    // and a profile would assert a filled region this entity does not author.
    // It must be a `Curve2`: that is the kernel's contract, and the 3D curve
    // path produced graphs that lowered but never compiled (#45).
    let boundary = boundary::lower_boundary_curve(session, id, view.polygonal_boundary()?)?;

    session.node_for(
        id,
        GeometryNode::SolidOperation(SolidOperation::BoundedHalfSpace {
            half_space,
            boundary,
            placement: placed.to_geom(),
        }),
    )
}

/// How a bounded half-space's placed `Position` relates to its plane normal.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PrismOrientation {
    /// The neutral node holds the IFC prism as written.
    Along,
    /// It does once the plane is described from its other side.
    Against,
    /// `Position`'s Z is not along the normal: no neutral equivalent.
    Oblique,
    /// `Position`'s XY frame is scaled or sheared in the plane, or singular.
    Distorted,
}

/// Parallelism, unit length and orthogonality are decided to this bound (a
/// sine, or a relative length). It is above the rounding of directions
/// written to eight or more significant digits and far below any authored
/// tilt: at 100 m from the plane it moves the prism by 10 µm.
const PRISM_FRAME_TOLERANCE: f64 = 1e-7;

/// Classify `placed` (`Position` in the plane's world frame, metres) against
/// the plane's unit `normal`.
///
/// The neutral node frames the boundary by `X` projected into the plane,
/// normalised, and `normal x X`, and sweeps it along `normal`. The IFC prism
/// is along `Z`, framed by `X` and `Y`. They agree when `Z` is along the
/// normal (either sign: the prism is infinite both ways), the projections of
/// `X` and `Y` are orthonormal, and `normal x X = Y`, which holds for exactly
/// one sign of the normal.
fn prism_orientation(placed: &Transform, normal: Vec3) -> PrismOrientation {
    let axis = |v: [f64; 3]| Vec3::from_array(placed.apply_direction(v));
    let [x, y, z] = [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]].map(axis);
    let n = normal.normalize_or_zero();
    let z_unit = z.normalize_or_zero();
    if n == Vec3::ZERO || z_unit == Vec3::ZERO || !z_unit.is_finite() {
        return PrismOrientation::Distorted;
    }
    if z_unit.cross(n).length() > PRISM_FRAME_TOLERANCE {
        return PrismOrientation::Oblique;
    }
    let [xp, yp] = [x, y].map(|v| v - n * v.dot(n));
    let unit = |v: Vec3| (v.length() - 1.0).abs() <= PRISM_FRAME_TOLERANCE;
    if !unit(xp) || !unit(yp) || xp.dot(yp).abs() > PRISM_FRAME_TOLERANCE {
        return PrismOrientation::Distorted;
    }
    if n.dot(xp.cross(yp)) > 0.0 {
        PrismOrientation::Along
    } else {
        PrismOrientation::Against
    }
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
