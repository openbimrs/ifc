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
use axiolid_curve::KnotSpec;
use axiolid_model::{GeometryNode, NodeId, SurfaceRelation};
use axiolid_surface::{
    BSplineSurface as KernelBSpline, Cylinder, Plane as KernelPlane, Sphere, Surface, Torus,
};
use ifc_model::EntityId;

use crate::curve::bspline::KnotType;
use crate::error::GeometryResult;
use crate::lower::curve::lower_curve_node;
use crate::lower::session::LoweringSession;
use crate::resource::direction::resolve_unit;
use crate::resource::placement::axis_placement_transform;
use crate::resource::placement::Axis1Placement;
use crate::resource::point::CartesianPoint;
use crate::surface::bounded::{CurveBoundedPlane, RectangularTrimmedSurface};
use crate::surface::bspline::BSplineSurface;
use crate::surface::elementary::{CylindricalSurface, Plane, SphericalSurface, ToroidalSurface};
use crate::surface::swept::SurfaceOfLinearExtrusion;
use crate::surface::swept::SurfaceOfRevolution;
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
        "IFCCYLINDRICALSURFACE" => lower_cylinder(session, id, frame)?,
        "IFCSPHERICALSURFACE" => lower_sphere(session, id, frame)?,
        "IFCTOROIDALSURFACE" => lower_torus(session, id, frame)?,
        "IFCBSPLINESURFACEWITHKNOTS" | "IFCRATIONALBSPLINESURFACEWITHKNOTS" => {
            lower_bspline(session, id, frame)?
        }
        "IFCSURFACEOFREVOLUTION" => lower_revolution(session, id, frame)?,
        "IFCRECTANGULARTRIMMEDSURFACE" => lower_rectangular_trimmed(session, id, frame)?,
        "IFCCURVEBOUNDEDPLANE" => lower_curve_bounded(session, id, frame)?,
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
    let swept_curve = swept_generatrix(session, id, curve_id, placed)?;

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

/// Compose an entity's `Position` with the caller's frame, in metres.
///
/// Every elementary surface places itself the same way; sharing this keeps the
/// unit conversion in exactly one place. A radius converted with a different
/// factor than its own frame origin puts the surface somewhere the file never
/// described.
fn placed_frame(
    session: &mut LoweringSession<'_>,
    owner: EntityId,
    position_id: EntityId,
    frame: Transform,
) -> GeometryResult<axiolid_core::Frame3> {
    let position = session.entity(owner, position_id)?;
    let local = axis_placement_transform(session.model(), position_id, position)?
        .to_metres(session.units());
    Ok(frame.compose(&local).to_geom_frame())
}

/// Convert one length-valued scalar into metres.
fn to_metres(session: &LoweringSession<'_>, value: f64) -> f64 {
    value * session.units().length_to_metres
}

/// Choose the unit factor for a trim parameter by basis-surface family.
///
/// A revolved or conic direction is parameterised by angle; a plane by length.
/// Returning a closure keeps the decision at the one place that knows the
/// basis type, instead of leaking a bool to every caller.
fn trim_converter(session: &LoweringSession<'_>, basis_kind: &str) -> impl Fn(f64) -> f64 {
    let angular = matches!(
        basis_kind,
        "IFCCYLINDRICALSURFACE"
            | "IFCSPHERICALSURFACE"
            | "IFCTOROIDALSURFACE"
            | "IFCSURFACEOFREVOLUTION"
    );
    let factor = if angular {
        session.units().angle_to_radians
    } else {
        session.units().length_to_metres
    };
    move |value: f64| value * factor
}

/// Read an `IfcCartesianPoint`'s three coordinates, as written.
fn cartesian_point_3d(
    session: &LoweringSession<'_>,
    referrer: EntityId,
    id: EntityId,
) -> GeometryResult<[f64; 3]> {
    let entity = session.entity(referrer, id)?;
    let point = CartesianPoint::new(id, entity);
    point.coordinates_3d()
}

/// Lower an `IfcCylindricalSurface` into a kernel cylinder.
///
/// The radius is a length and converts to metres; the frame axes do not.
/// Scaling the axes as well would turn a unit basis into a millimetre-long
/// one and every parameter measured against it would be off by that factor.
pub fn lower_cylinder(
    session: &mut LoweringSession<'_>,
    id: EntityId,
    frame: Transform,
) -> GeometryResult<NodeId> {
    let entity = session.entity(id, id)?;
    let view = CylindricalSurface::new(id, entity);
    let placed = placed_frame(session, id, view.position_ref()?, frame)?;
    let radius = to_metres(session, view.radius()?);
    session.node_for(
        id,
        GeometryNode::Surface(Surface::Cylinder(Cylinder {
            frame: placed,
            radius,
        })),
    )
}

/// Lower an `IfcSphericalSurface` into a kernel sphere.
pub fn lower_sphere(
    session: &mut LoweringSession<'_>,
    id: EntityId,
    frame: Transform,
) -> GeometryResult<NodeId> {
    let entity = session.entity(id, id)?;
    let view = SphericalSurface::new(id, entity);
    let placed = placed_frame(session, id, view.position_ref()?, frame)?;
    let radius = to_metres(session, view.radius()?);
    session.node_for(
        id,
        GeometryNode::Surface(Surface::Sphere(Sphere {
            frame: placed,
            radius,
        })),
    )
}

/// Lower an `IfcToroidalSurface` into a kernel torus.
///
/// Both radii are lengths. A torus with `minor >= major` self-intersects into
/// a spindle; that is legal IFC and the reader exposes it, so it is preserved
/// rather than rejected -- discarding it would silently change the shape.
pub fn lower_torus(
    session: &mut LoweringSession<'_>,
    id: EntityId,
    frame: Transform,
) -> GeometryResult<NodeId> {
    let entity = session.entity(id, id)?;
    let view = ToroidalSurface::new(id, entity);
    let placed = placed_frame(session, id, view.position_ref()?, frame)?;
    let major_radius = to_metres(session, view.major_radius()?);
    let minor_radius = to_metres(session, view.minor_radius()?);
    session.node_for(
        id,
        GeometryNode::Surface(Surface::Torus(Torus {
            frame: placed,
            major_radius,
            minor_radius,
        })),
    )
}

/// Map the IFC knot enumeration onto the kernel's.
///
/// The variants correspond one to one, so this is a rename, not a decision.
fn knot_spec(source: KnotType) -> KnotSpec {
    match source {
        KnotType::Uniform => KnotSpec::Uniform,
        KnotType::QuasiUniform => KnotSpec::QuasiUniform,
        KnotType::PiecewiseBezier => KnotSpec::PiecewiseBezier,
        KnotType::Unspecified => KnotSpec::Unspecified,
    }
}

/// Lower an `IfcBSplineSurfaceWithKnots` into a kernel B-spline patch.
///
/// The control net stays row-major in `u` then `v`, matching both the IFC
/// `ControlPointsList` and the kernel field. A transposed net still evaluates
/// and still looks like a surface, so the per-direction degrees, knots and
/// multiplicities are carried through verbatim rather than inferred from the
/// net's shape.
///
/// `WeightsData` is present only on the rational subtype. Defaulting absent
/// weights to 1.0 would turn a polynomial patch into a rational one that
/// happens to agree, so `None` is preserved as `None`.
pub fn lower_bspline(
    session: &mut LoweringSession<'_>,
    id: EntityId,
    frame: Transform,
) -> GeometryResult<NodeId> {
    let entity = session.entity(id, id)?;
    let view = BSplineSurface::new(id, entity);
    let type_name = session.type_name(id)?;

    let grid = view.control_points()?;
    let mut control_points = Vec::with_capacity(grid.u_count());
    for row in grid.rows() {
        let mut out_row = Vec::with_capacity(row.len());
        for point_id in row {
            let raw = cartesian_point_3d(session, id, *point_id)?;
            let placed = frame.apply([
                to_metres(session, raw[0]),
                to_metres(session, raw[1]),
                to_metres(session, raw[2]),
            ]);
            out_row.push(axiolid_core::Point3::from_array(placed));
        }
        control_points.push(out_row);
    }

    let u = view.u_knots()?.ok_or_else(|| {
        session.unsupported(id, &type_name, "B-spline surface without explicit u knots")
    })?;
    let v = view.v_knots()?.ok_or_else(|| {
        session.unsupported(id, &type_name, "B-spline surface without explicit v knots")
    })?;

    let surface = KernelBSpline {
        u_degree: u16::try_from(view.u_degree()?).unwrap_or(u16::MAX),
        v_degree: u16::try_from(view.v_degree()?).unwrap_or(u16::MAX),
        control_points,
        u_knots: u.values,
        u_multiplicities: u.multiplicities.iter().map(|m| *m as u32).collect(),
        v_knots: v.values,
        v_multiplicities: v.multiplicities.iter().map(|m| *m as u32).collect(),
        weights: view.weights()?,
        u_closed: view.u_closed().unwrap_or(false),
        v_closed: view.v_closed().unwrap_or(false),
        self_intersect: view.self_intersect(),
        knot_spec: knot_spec(view.knot_spec()),
    };
    session.node_for(id, GeometryNode::Surface(Surface::BSpline(surface)))
}

/// Resolve a swept surface's `SweptCurve` to the curve it actually names.
///
/// IFC types this slot as `IfcProfileDef`, so a plain curve arrives wrapped in
/// an `IfcArbitraryOpenProfileDef`. The wrapper carries no geometry of its own
/// here -- for a swept SURFACE the profile is a generatrix, not an area -- so
/// it is unwrapped rather than lowered as a profile. Lowering it as one would
/// demand a closed contour and reject a legitimately open generatrix.
fn swept_generatrix(
    session: &mut LoweringSession<'_>,
    _owner: EntityId,
    swept_curve: EntityId,
    frame: Transform,
) -> GeometryResult<NodeId> {
    let kind = session.type_name(swept_curve)?.to_ascii_uppercase();
    let curve_id = if kind == "IFCARBITRARYOPENPROFILEDEF" || kind == "IFCARBITRARYCLOSEDPROFILEDEF"
    {
        // Slot 2 is Curve on IfcArbitraryOpenProfileDef and OuterCurve on
        // the closed form; both name the generatrix.
        session.slots(swept_curve)?.req_ref(2, "Curve")?
    } else {
        swept_curve
    };
    lower_curve_node(session, curve_id, frame)
}

/// Lower an `IfcSurfaceOfRevolution` into a revolution relation.
///
/// `AxisPosition` is an `IfcAxis1Placement`, not the `IfcAxis2Placement3D`
/// every other surface family uses. Both carry a Location, so reading the
/// wrong one still yields a surface -- just one revolved about the wrong line.
/// The axis origin is a length and converts; the direction is placed by the
/// linear part only so an off-origin frame does not translate it.
pub fn lower_revolution(
    session: &mut LoweringSession<'_>,
    id: EntityId,
    frame: Transform,
) -> GeometryResult<NodeId> {
    let entity = session.entity(id, id)?;
    let view = SurfaceOfRevolution::new(id, entity);

    let swept_curve = swept_generatrix(session, id, view.swept_curve_ref()?, frame)?;

    let axis_id = view.axis_position_ref()?;
    let axis_entity = session.entity(id, axis_id)?;
    let axis = Axis1Placement::new(axis_id, axis_entity);
    let raw_origin = axis.location(session.model())?;
    let origin = frame.apply([
        to_metres(session, raw_origin[0]),
        to_metres(session, raw_origin[1]),
        to_metres(session, raw_origin[2]),
    ]);
    let direction = frame.apply_direction(axis.axis(session.model())?);

    session.node_for(
        id,
        GeometryNode::SurfaceRelation(SurfaceRelation::Revolution {
            swept_curve,
            axis_origin: axiolid_core::Point3::from_array(origin),
            axis_direction: axiolid_core::Vec3::from_array(direction),
        }),
    )
}

/// Lower an `IfcRectangularTrimmedSurface` into a parameter-space trim.
///
/// The trim parameters are NOT lengths. On a conic or revolved direction they
/// are angles in the model's plane-angle unit, which is degrees in many real
/// exports; on a plane they are lengths. Scaling everything by the length
/// factor turns a 90-degree patch into a sliver and still renders.
pub fn lower_rectangular_trimmed(
    session: &mut LoweringSession<'_>,
    id: EntityId,
    frame: Transform,
) -> GeometryResult<NodeId> {
    let entity = session.entity(id, id)?;
    let view = RectangularTrimmedSurface::new(id, entity);
    let basis_id = view.basis_surface_ref()?;
    let basis = lower_surface_node(session, basis_id, frame)?;
    let rect = view.rectangle()?;

    let basis_kind = session.type_name(basis_id)?.to_ascii_uppercase();
    let convert = trim_converter(session, &basis_kind);

    session.node_for(
        id,
        GeometryNode::SurfaceRelation(SurfaceRelation::RectangularTrimmed {
            basis,
            u: (convert(rect.u1), convert(rect.u2)),
            v: (convert(rect.v1), convert(rect.v2)),
            u_sense: rect.usense,
            v_sense: rect.vsense,
        }),
    )
}

/// Lower an `IfcCurveBoundedPlane` into a curve-bounded relation.
///
/// The outer boundary is explicit here, unlike `IfcCurveBoundedSurface` where
/// it may be implicit, so `implicit_outer` is false and the outer curve leads
/// the boundary list.
pub fn lower_curve_bounded(
    session: &mut LoweringSession<'_>,
    id: EntityId,
    frame: Transform,
) -> GeometryResult<NodeId> {
    let entity = session.entity(id, id)?;
    let view = CurveBoundedPlane::new(id, entity);
    let basis = lower_surface_node(session, view.basis_surface_ref()?, frame)?;

    let mut boundaries = vec![lower_curve_node(
        session,
        view.outer_boundary_ref()?,
        frame,
    )?];
    for inner in view.inner_boundary_refs() {
        boundaries.push(lower_curve_node(session, inner, frame)?);
    }

    session.node_for(
        id,
        GeometryNode::SurfaceRelation(SurfaceRelation::CurveBounded {
            basis,
            boundaries,
            implicit_outer: false,
        }),
    )
}
