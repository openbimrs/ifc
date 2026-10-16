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
//! `prism_orientation` decides which case holds. *Against* has an exact
//! answer here: the plane with its normal reversed and its side flipped is
//! the same half space, now framed by `(X, Y)`. *Oblique* and a scaled or
//! sheared frame have no bounded node in the neutral model. As a boolean's
//! tool they lower to half spaces (below); anywhere else they are refused
//! by name.
//!
//! # The bounded node's depth (#519)
//!
//! The bounded node carries no depth: it is the half space cut by an
//! infinite prism, and the compiler bounds it. Axiolid's exact compiler
//! bounds it by the subject's envelope. Its reference mesh compiler
//! (axiolid-mesh-compile 0.3.26, axiolid-construct 0.3.18) builds a slab
//! twice the boundary's largest coordinate deep instead, so a small
//! boundary over a tall subject (a 238 x 350 mm strip at a wall's end, the
//! wall reaching 4.8 m above the plane) is cut only 0.7 m deep in the mesh.
//! The deviation report, measured against the exact body, then misses the
//! requested tolerance; nothing in this bridge sizes the cut (ADR 0004),
//! and the fix is the kernel's (axiolid/kernel#325).
//!
//! The half-space chain below would mesh the full cut, but it is no
//! mitigation: it loses the exact result for every aligned difference that
//! is certified today, and a prism that only grazes its subject (its corner
//! on the subject's edge, as such a strip meets a sloped clip before it)
//! meets tangentially in the chain's intersection, which the mesh compiler
//! refuses.
//!
//! # The oblique prism as half spaces
//!
//! The infinite prism of a CONVEX polygon is the intersection of one half
//! space per edge: the plane through the edge and parallel to `Position`'s
//! Z, on the polygon's side. So the IFC tool is `T = H ∩ W1 ∩ ... ∩ Wn`,
//! all unbounded half spaces, each exact in `Position`'s frame and placed
//! as any IFC plane is. Neither of Axiolid's compilers takes an unbounded
//! half space alone, but both take one as the TOOL of an intersection or a
//! difference, bounded there by the subject (axiolid-mesh-compile 0.3.26:
//! the mesh compiler's `build_boolean` with `half_space::for_subject`, the
//! exact compiler's `exact::clip`). `lower_prism_clip` therefore lowers a
//! boolean whose tool is `T`, given its subject `S`, as
//!
//! - `S ∩ T` = `(((S ∩ H) ∩ W1) ... ∩ Wn)`, and
//! - `S - T` = `S - (S ∩ T)`, the same chain as the tool,
//!
//! identities of sets: nothing is tessellated, cut, bounded or measured
//! here. What the kernel does with them today: the mesh compiler builds
//! both, and the gable wall of #515 meets IfcOpenShell's volume to 1e-12
//! m³. The exact compiler builds `S ∩ T` but takes no boolean as a
//! difference's tool, so a difference's mesh carries no certified deviation
//! bound (it names the missing exact result instead), and a chain whose
//! subject empties before its last tool (a prism that misses the subject)
//! is refused by the mesh compiler rather than kept as the subject.
//!
//! What is not an intersection of half spaces is refused by name: a
//! non-convex polygon (its prism is a union; a convex decomposition would
//! be a construction this bridge does not make, ADR 0004), an arc (its wall
//! is a cylinder), a union with the tool (unbounded), and the tool lowered
//! anywhere but as a boolean's second operand. A kernel node whose prism
//! runs along its own placement's Z (axiolid/kernel#321) would make each of
//! these one node again.

use axiolid_core::{BooleanOperator, Plane3, Point2, Point3, Vec3};
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

/// The parts of an `IfcPolygonalBoundedHalfSpace` every lowering reads.
struct Polygonal {
    type_name: String,
    /// The unbounded half space, in the kernel's polarity.
    half_space: HalfSpace,
    /// `Position` in the frame of the plane (metres).
    placed: Transform,
    /// `PolygonalBoundary`.
    boundary: EntityId,
}

fn read_polygonal(
    session: &mut LoweringSession<'_>,
    id: EntityId,
    frame: Transform,
) -> GeometryResult<Polygonal> {
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
    Ok(Polygonal {
        type_name,
        half_space: HalfSpace {
            boundary: plane,
            agreement,
        },
        placed: frame.compose(&local),
        boundary: view.polygonal_boundary()?,
    })
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
    let Polygonal {
        type_name,
        half_space,
        placed,
        boundary,
    } = read_polygonal(session, id, frame)?;
    let HalfSpace {
        boundary: plane,
        agreement,
    } = half_space;

    // The neutral node holds the IFC prism only for some frames; orient the
    // plane so it does, or refuse (#515, see "The prism axis" above). The
    // others lower only as a clip (`lower_prism_clip`), where the subject
    // bounds the cut.
    let half_space = match prism_orientation(&placed, plane.normal) {
        PrismOrientation::Along => half_space,
        PrismOrientation::Against => HalfSpace {
            boundary: Plane3 {
                origin: plane.origin,
                normal: -plane.normal,
            },
            agreement: !agreement,
        },
        PrismOrientation::Oblique => return Err(session.unsupported(id, &type_name, OBLIQUE)),
        PrismOrientation::Distorted => return Err(session.unsupported(id, &type_name, DISTORTED)),
    };
    let half_space = session.node_for(id, GeometryNode::HalfSpace(half_space))?;

    // The 2D boundary is a closed bounded curve in Position's XY plane. It
    // lowers as a curve, not a profile: the kernel extrudes it along the plane
    // normal itself, which is Position's Z up to sign once checked above,
    // and a profile would assert a filled region this entity does not author.
    // It must be a `Curve2`: that is the kernel's contract, and the 3D curve
    // path produced graphs that lowered but never compiled (#45).
    let boundary = boundary::lower_boundary_curve(session, id, boundary)?;

    session.node_for(
        id,
        GeometryNode::SolidOperation(SolidOperation::BoundedHalfSpace {
            half_space,
            boundary,
            placement: placed.to_geom(),
        }),
    )
}

/// `subject` intersected with the polygonal bounded half-space `id`, as an
/// intersection of unbounded half spaces, when the neutral bounded node
/// cannot hold its prism; `None` when it can (#515).
///
/// The caller is a boolean whose second operand is `id` and whose first is
/// `subject`, under a difference or an intersection. For a convex straight
/// boundary the IFC tool is `H ∩ W1 ∩ ... ∩ Wn`: `H` the base half space
/// and `Wi` the side of edge `i`'s plane holding the polygon, the plane
/// through the edge and parallel to `Position`'s Z. So `subject ∩ tool` is a
/// chain of intersections whose every tool is an unbounded half space, the
/// one form both of Axiolid's compilers bound by their subject, and
/// `subject - tool = subject - (subject ∩ tool)`. See "The oblique prism as
/// half spaces" in the module docs.
pub(crate) fn lower_prism_clip(
    session: &mut LoweringSession<'_>,
    id: EntityId,
    frame: Transform,
    subject: NodeId,
) -> GeometryResult<Option<NodeId>> {
    if !session
        .type_name(id)?
        .eq_ignore_ascii_case("IFCPOLYGONALBOUNDEDHALFSPACE")
    {
        return Ok(None);
    }
    session.enter(id, KIND)?;
    let result = prism_clip(session, id, frame, subject);
    session.exit(id);
    result
}

fn prism_clip(
    session: &mut LoweringSession<'_>,
    id: EntityId,
    frame: Transform,
    subject: NodeId,
) -> GeometryResult<Option<NodeId>> {
    let Polygonal {
        type_name,
        half_space,
        placed,
        boundary,
    } = read_polygonal(session, id, frame)?;
    // TODO(#515): once Axiolid's BoundedHalfSpace takes a prism along its
    // placement's Z with the full frame (axiolid/kernel#321, #322), emit
    // that node here for every orientation and boundary, and drop this
    // chain of half spaces.
    match prism_orientation(&placed, half_space.boundary.normal) {
        PrismOrientation::Along | PrismOrientation::Against => return Ok(None),
        PrismOrientation::Oblique | PrismOrientation::Distorted => {}
    }
    let points = match boundary::read_boundary(session, id, boundary)? {
        boundary::Boundary::Polygon {
            points,
            closed: true,
        } => points,
        boundary::Boundary::Polygon { closed: false, .. } => {
            return Err(session.degenerate(
                boundary,
                "IFCPOLYLINE",
                "a polygonal half-space boundary must close",
            ))
        }
        boundary::Boundary::Contour(_) => {
            return Err(session.unsupported(id, &type_name, OBLIQUE_ARC))
        }
    };
    let walls = prism_walls(&points, &placed)
        .map_err(|refusal| session.unsupported(id, &type_name, refusal))?;

    let mut cut = subject;
    for tool in std::iter::once(half_space).chain(walls) {
        let tool = session.node_for(id, GeometryNode::HalfSpace(tool))?;
        cut = session.node_for(
            id,
            GeometryNode::SolidOperation(SolidOperation::Boolean {
                left: cut,
                right: tool,
                operator: BooleanOperator::Intersection,
            }),
        )?;
    }
    Ok(Some(cut))
}

/// Refusal for an oblique or rescaled prism around a circular arc.
const OBLIQUE_ARC: &str = "a boundary with a circular arc whose prism is oblique to the base \
                           plane or rescaled: an arc's prism wall is a cylinder, not a half \
                           space, and the neutral bounded half-space sweeps along the normal";

/// Refusal for an oblique or rescaled prism around a non-convex polygon.
const NON_CONVEX: &str = "a non-convex boundary whose prism is oblique to the base plane or \
                          rescaled: only a convex prism is an intersection of half spaces, and \
                          the neutral bounded half-space sweeps along the normal";

/// The half spaces whose intersection is the infinite prism of the convex
/// polygon `points` (in `Position`'s XY plane) along `Position`'s Z, placed
/// by `placed`; refused when the polygon is not convex.
///
/// Each wall is built in `Position`'s own frame, where it is exact: the edge
/// line in the XY plane, its in-plane normal toward the polygon, and Z in
/// the plane. It is then placed as any IFC plane is (`plane_of`): a point
/// through the transform, the normal as a covector, so a scaled, sheared or
/// reflected `Position` maps the prism and its sides alike.
///
/// Convexity is decided on the authored vertices: every vertex must lie on
/// the inner side of every edge, or on it within the rounding of the
/// orientation product. That also refuses a polygon that winds twice (a
/// star), whose edge half-planes enclose less than it does.
fn prism_walls(points: &[Point2], placed: &Transform) -> Result<Vec<HalfSpace>, &'static str> {
    // A repeated vertex adds no edge.
    let mut vertices: Vec<Point2> = Vec::with_capacity(points.len());
    for &point in points {
        if vertices.last() != Some(&point) {
            vertices.push(point);
        }
    }
    while vertices.len() > 1 && vertices.first() == vertices.last() {
        vertices.pop();
    }
    if vertices.len() < 3 {
        return Err(NON_CONVEX);
    }
    let n = vertices.len();
    let edges = || (0..n).map(|i| (vertices[i], vertices[(i + 1) % n]));
    // Twice the signed area; its sign says which side of each edge is inside.
    let area: f64 = edges().map(|(p, q)| p.perp_dot(q)).sum();
    let inside = if area > 0.0 {
        1.0
    } else if area < 0.0 {
        -1.0
    } else {
        return Err(NON_CONVEX);
    };
    let mut walls = Vec::with_capacity(n);
    for (p, q) in edges() {
        let edge = q - p;
        for &v in &vertices {
            let offset = v - p;
            let turn = inside * edge.perp_dot(offset);
            let rounding = ORIENTATION_ROUNDING * edge.length() * offset.length();
            if turn < -rounding {
                return Err(NON_CONVEX);
            }
        }
        // The left normal of a counter-clockwise polygon points inside.
        let normal = [-edge.y * inside, edge.x * inside, 0.0];
        let origin = placed.apply([p.x, p.y, 0.0]);
        let normal = placed.apply_unit_normal(normal).ok_or(DISTORTED)?;
        walls.push(HalfSpace {
            boundary: Plane3 {
                origin: Point3::from_array(origin),
                normal: Vec3::from_array(normal),
            },
            // The kernel's `true` keeps the normal side: the polygon's.
            agreement: true,
        });
    }
    Ok(walls)
}

/// The relative rounding of a 2D orientation product `a x b`: its sign is
/// certain where it exceeds this times `|a| |b|`. A few ulps, with room
/// for the subtractions that formed `a` and `b`.
const ORIENTATION_ROUNDING: f64 = 16.0 * f64::EPSILON;

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

/// Refusal for [`PrismOrientation::Oblique`].
const OBLIQUE: &str = "a boundary prism oblique to the base plane: Position's Z axis is not \
                       along the plane normal, and the neutral bounded half-space sweeps its \
                       boundary along the normal";

/// Refusal for [`PrismOrientation::Distorted`].
const DISTORTED: &str = "a scaled or sheared boundary frame: the neutral bounded half-space \
                         reads only the direction of Position's X axis, so the boundary would \
                         keep its authored size";

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
