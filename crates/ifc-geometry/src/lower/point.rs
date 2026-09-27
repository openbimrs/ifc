//! Point lowering: exact parametric references, never evaluated here.
//!
//! # Why these are not evaluated to Cartesian coordinates
//!
//! `IfcPointOnCurve` and `IfcPointOnSurface` are legal `IfcPoint` subtypes
//! whose position is defined by a parameter on a basis curve or surface, not
//! by stored coordinates. Evaluating them here would require a curve/surface
//! evaluator, which this crate deliberately does not own -- see the crate
//! boundary in `AGENTS.md`. Preserving the reference and the parameter(s)
//! exactly, as `axiolid_model::PointOnCurve`/`PointOnSurface`, keeps the
//! adapter kernel-agnostic: an application chooses when and how to evaluate.
//!
//! # Parameters are not lengths
//!
//! Both parameters are in the *basis*'s own parameter space: an angle on a
//! conic or revolved surface, a dimensionless ordinal on a polyline, a length
//! on most other curves and surfaces. `scale_parameter` (curve) and
//! `trim_converter` (surface) already encode this per-basis-kind distinction
//! for trims; both are reused here rather than re-deriving the unit logic.

use axiolid_model::PointOnSurface as KernelPointOnSurface;
use axiolid_model::{GeometryNode, NodeId, PointOnCurve as KernelPointOnCurve};
use ifc_model::EntityId;

use crate::error::GeometryResult;
use crate::lower::curve::{lower_curve_node, scale_parameter};
use crate::lower::session::LoweringSession;
use crate::lower::surface::lower_surface_node;
use crate::resource::point::{PointOnCurve, PointOnSurface};
use crate::transform::Transform;

/// Family label used for point memoization.
const KIND: &str = "point";

/// Lower an `IfcPointOnCurve` into a `PointOnCurve` node.
///
/// The basis curve is lowered first (through the total curve dispatcher, so
/// any curve family this crate supports may serve as the basis), then the
/// parameter is converted into the basis's own unit convention.
pub fn lower_point_on_curve_node(
    session: &mut LoweringSession<'_>,
    id: EntityId,
    frame: Transform,
) -> GeometryResult<NodeId> {
    if let Some(node) = session.memoized(id, KIND, frame) {
        return Ok(node);
    }
    session.enter(id, KIND)?;
    let result = build_point_on_curve(session, id, frame);
    session.exit(id);
    let node = result?;
    session.memoize(id, KIND, frame, node);
    Ok(node)
}

fn build_point_on_curve(
    session: &mut LoweringSession<'_>,
    id: EntityId,
    frame: Transform,
) -> GeometryResult<NodeId> {
    let entity = session.entity(id, id)?;
    let view = PointOnCurve::new(id, entity);
    let basis_ref = view.basis_curve()?;
    let raw_parameter = view.point_parameter()?;

    let basis_kind = session.type_name(basis_ref)?;
    let curve = lower_curve_node(session, basis_ref, frame)?;
    let parameter = scale_parameter(session, &basis_kind, raw_parameter);

    session.node_for(
        id,
        GeometryNode::PointOnCurve(KernelPointOnCurve { curve, parameter }),
    )
}

/// Lower an `IfcPointOnSurface` into a `PointOnSurface` node.
///
/// Both `(u, v)` parameters are in the basis surface's own parameter space
/// and may use different unit conventions on a mixed analytic surface (for
/// example a cylinder: `u` an angle, `v` a length). `trim_converter` already
/// resolves the single-factor cases this crate supports; the pair is
/// converted with the same per-basis-kind factor since neither IFC nor this
/// adapter distinguishes a surface with different conventions per axis.
pub fn lower_point_on_surface_node(
    session: &mut LoweringSession<'_>,
    id: EntityId,
    frame: Transform,
) -> GeometryResult<NodeId> {
    if let Some(node) = session.memoized(id, KIND, frame) {
        return Ok(node);
    }
    session.enter(id, KIND)?;
    let result = build_point_on_surface(session, id, frame);
    session.exit(id);
    let node = result?;
    session.memoize(id, KIND, frame, node);
    Ok(node)
}

fn build_point_on_surface(
    session: &mut LoweringSession<'_>,
    id: EntityId,
    frame: Transform,
) -> GeometryResult<NodeId> {
    let entity = session.entity(id, id)?;
    let view = PointOnSurface::new(id, entity);
    let basis_ref = view.basis_surface()?;
    let (raw_u, raw_v) = view.parameters()?;

    let basis_kind = session.type_name(basis_ref)?;
    let surface = lower_surface_node(session, basis_ref, frame)?;
    let convert = crate::lower::surface::trim_converter(session, &basis_kind);
    let (u, v) = (convert(raw_u), convert(raw_v));

    session.node_for(
        id,
        GeometryNode::PointOnSurface(KernelPointOnSurface { surface, u, v }),
    )
}

#[cfg(test)]
mod tests;
