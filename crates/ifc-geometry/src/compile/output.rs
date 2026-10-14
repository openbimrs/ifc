//! Curves and surfaces as output primitives (#458).
//!
//! A product's selected representation may hold curves (an `Axis`, a
//! `FootPrint`, the curves of an `IfcAnnotation`), surfaces and solids, and
//! a curve has no area, so it has no triangles. The representation is
//! therefore split into leaves, and each leaf goes to the operation that
//! compiles its dimensionality:
//!
//! - a **curve** leaf -- a curve, an open profile, or a placement of one --
//!   to [`PolylineCompiler::compile_polyline`], one polyline per leaf, with
//!   the backend's certified chord bound;
//! - every **other** leaf to [`MeshCompiler::compile_mesh_reported`], as
//!   one mesh whose closure says whether it bounds a solid
//!   ([`MeshClosure::Solid`]) or is a surface ([`MeshClosure::Surface`]).
//!
//! Collections are opened, and a placement (a mapped item) only when it
//! holds a curve: the polyline compiler takes one curve, so each curve
//! under a placement is placed on its own by the same transform, composed.
//! That regroups the lowered nodes exactly and evaluates nothing. A
//! representation with no curve is compiled from its own root, as the Body
//! path compiles it, so its mesh is unchanged. Nothing is tessellated,
//! flattened or substituted here (ADR 0004).
//!
//! # Refused by name
//!
//! A backend that cannot compile a curve to a polyline refuses it. When the
//! curve is a trimmed, composite or offset curve, or an intrinsic one, the
//! refusal is [`GeometryError::PolylineUnsupported`] naming that family:
//! the reference compiler refuses them until axiolid/kernel#317, tracked
//! here in #506. Anything else the backend refuses is
//! [`GeometryError::CompilationRefused`].

use axiolid_contracts::ExecutionOptions;
use axiolid_core::Transform3;
use axiolid_curve::{Curve2, Curve3};
use axiolid_mesh_compile_contract::MeshCompiler;
use axiolid_model::{CurveRelation, GeometryGraph, GeometryNode, Instance, NodeId};
use ifc_model::{EntityId, Model};

pub use axiolid_core::Point3;
pub use axiolid_mesh_compile_contract::{ChordDeviation, MeshClosure, PolylineCompiler};

use super::{refused, CompiledMesh, Tolerance};
use crate::error::{GeometryError, GeometryResult};
use crate::lower::session::NodeShape;
use crate::lower::{
    lower_product_representation, LoweredGeometry, LoweringSession, RepresentationPurpose,
};
use crate::units;

#[cfg(feature = "compile-reference-backend")]
use super::default_backend;

/// Nesting of collections and placements followed before refusing.
const MAX_DEPTH: usize = 256;

/// The curve families the reference polyline compiler refuses by name
/// (axiolid/kernel#317; #506).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum UnpolylinedCurve {
    /// An `IfcTrimmedCurve`: a trim of a basis curve.
    Trimmed,
    /// An `IfcCompositeCurve` and its subtypes, or an `IfcIndexedPolyCurve`
    /// with arcs: a chain of segments.
    Composite,
    /// An `IfcOffsetCurve2D`, `IfcOffsetCurve3D` or
    /// `IfcOffsetCurveByDistances`.
    Offset,
    /// A curve integrated from its curvature, such as an IFC4X3 clothoid
    /// segment.
    Intrinsic,
}

impl std::fmt::Display for UnpolylinedCurve {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Trimmed => "a trimmed curve",
            Self::Composite => "a composite curve",
            Self::Offset => "an offset curve",
            Self::Intrinsic => "an intrinsic curve",
        })
    }
}

/// One curve of a representation as a 3D polyline.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct CompiledPolyline {
    /// The IFC curve it was compiled from, when the lowering recorded one.
    pub curve: Option<EntityId>,
    /// The vertices in the curve's direction, world coordinates in metres.
    /// A closed curve ends on the vertex it starts on.
    pub points: Vec<Point3>,
    /// Whether the curve is closed: the last vertex is the first.
    pub closed: bool,
    /// The chord budget asked for and the backend's certified bound on the
    /// distance from every point of the exact curve to the polyline, in
    /// metres. A polyline the file authored is its own curve: bound `0`.
    pub deviation: ChordDeviation,
}

/// The curves of one product's representation for `purpose` as
/// polylines, with [`default_backend`].
///
/// # Errors
///
/// As [`compile_product_polylines_with`].
#[cfg(feature = "compile-reference-backend")]
pub fn compile_product_polylines(
    model: &Model,
    product: EntityId,
    purpose: RepresentationPurpose,
    tolerance: Tolerance,
) -> GeometryResult<Option<Vec<CompiledPolyline>>> {
    compile_product_polylines_with(&default_backend(), model, product, purpose, tolerance)
}

/// The curves of one product's representation for `purpose` as
/// polylines, one per curve in authored order, with a caller-supplied
/// backend.
///
/// `Ok(None)` when the product has no representation for `purpose`; an
/// empty list when it has one with no curves (a solid Body). The chord
/// budget is `tolerance`, in metres as for the meshes.
///
/// # Errors
///
/// Any lowering error; [`GeometryError::PolylineUnsupported`] for a
/// trimmed, composite, offset or intrinsic curve the backend refuses
/// (#506), and [`GeometryError::CompilationRefused`] for any other
/// refusal. One refused curve refuses the product: a partial outline is
/// not returned as the whole.
pub fn compile_product_polylines_with<B: PolylineCompiler>(
    backend: &B,
    model: &Model,
    product: EntityId,
    purpose: RepresentationPurpose,
    tolerance: Tolerance,
) -> GeometryResult<Option<Vec<CompiledPolyline>>> {
    let Some(split) = lower_split(model, product, purpose)? else {
        return Ok(None);
    };
    let lowered = &split.lowered;
    let options = ExecutionOptions::new(tolerance);
    split
        .curves
        .iter()
        .map(|&(leaf, curve)| {
            backend
                .compile_polyline(&lowered.graph, leaf, &options)
                .map(|outcome| CompiledPolyline {
                    curve,
                    closed: outcome.closed,
                    deviation: outcome.deviation,
                    points: outcome.points,
                })
                .map_err(|error| polyline_refused(lowered, leaf, curve, product, error))
        })
        .collect::<GeometryResult<Vec<_>>>()
        .map(Some)
}

/// The surfaces and solids of one product's representation for `purpose`
/// as one mesh, with [`default_backend`].
///
/// # Errors
///
/// As [`compile_product_mesh_for_with`].
#[cfg(feature = "compile-reference-backend")]
pub fn compile_product_mesh_for(
    model: &Model,
    product: EntityId,
    purpose: RepresentationPurpose,
    tolerance: Tolerance,
) -> GeometryResult<Option<CompiledMesh>> {
    compile_product_mesh_for_with(&default_backend(), model, product, purpose, tolerance)
}

/// The surfaces and solids of one product's representation for `purpose`
/// as one mesh, reporting its closure, with a caller-supplied backend.
///
/// Every leaf that is not a curve is compiled together, so a `Body` with
/// no curves compiles from the same root, to the same mesh, as
/// [`super::compile_product_mesh_reported_with`]. A surface -- a surface
/// model, a face, a rectangular trimmed surface, a B-rep of loose faces --
/// is [`MeshClosure::Surface`]. `Ok(None)` when the product has no
/// representation for `purpose`, or one with curves only (an `Axis`):
/// [`compile_product_polylines_with`] returns those.
///
/// # Errors
///
/// Any lowering error, and [`GeometryError::CompilationRefused`] when the
/// backend refuses the mesh.
pub fn compile_product_mesh_for_with<B: MeshCompiler>(
    backend: &B,
    model: &Model,
    product: EntityId,
    purpose: RepresentationPurpose,
    tolerance: Tolerance,
) -> GeometryResult<Option<CompiledMesh>> {
    let Some(split) = lower_split(model, product, purpose)? else {
        return Ok(None);
    };
    let Some(root) = split.mesh else {
        return Ok(None);
    };
    let lowered = &split.lowered;
    let options = ExecutionOptions::new(tolerance);
    backend
        .compile_mesh_reported(&lowered.graph, root, &options)
        .map(|outcome| {
            Some(CompiledMesh {
                closure: outcome.closure,
                mesh: outcome.mesh,
            })
        })
        .map_err(|error| refused(product, error, &lowered.provenance))
}

/// A representation lowered and split by dimension.
struct Split {
    /// The lowered graph; the nodes below are among its nodes.
    lowered: LoweredGeometry,
    /// Each curve, placed, with the IFC curve it came from.
    curves: Vec<(NodeId, Option<EntityId>)>,
    /// The node holding every other leaf, `None` when there is none: the
    /// representation's own root when it has no curve.
    mesh: Option<NodeId>,
}

/// One leaf of a representation and the placement composed above it
/// inside the representation, `None` when nothing there places it.
type Leaf = (NodeId, Option<Transform3>);

/// The product's representation for `purpose`, lowered and placed, and
/// split into its curves and the rest.
fn lower_split(
    model: &Model,
    product: EntityId,
    purpose: RepresentationPurpose,
) -> GeometryResult<Option<Split>> {
    let scale = units::resolve(model);
    let mut session = LoweringSession::new(model, &scale);
    let Some(root) = lower_product_representation(&mut session, product, purpose)? else {
        return Ok(None);
    };
    let (mut curves, mut others) = (Vec::new(), Vec::new());
    walk(&session, root, None, product, 0, &mut curves, &mut others)?;
    let (curves, mesh) = if curves.is_empty() {
        (Vec::new(), Some(root))
    } else {
        let curves = curves
            .into_iter()
            .map(|leaf| {
                let source = session.provenance().source(leaf.0);
                place(&mut session, leaf, product).map(|node| (node, source))
            })
            .collect::<GeometryResult<Vec<_>>>()?;
        let others = others
            .into_iter()
            .map(|leaf| place(&mut session, leaf, product))
            .collect::<GeometryResult<Vec<_>>>()?;
        let mesh = match others.as_slice() {
            [] => None,
            [only] => Some(*only),
            _ => Some(session.node_for(product, GeometryNode::Collection(others))?),
        };
        (curves, mesh)
    };
    let lowered = session.finish(root)?;
    Ok(Some(Split {
        lowered,
        curves,
        mesh,
    }))
}

/// Sort the leaves under `node`, placed by `outer`, into `curves` and
/// `others`, in authored order.
fn walk(
    session: &LoweringSession<'_>,
    node: NodeId,
    outer: Option<Transform3>,
    product: EntityId,
    depth: usize,
    curves: &mut Vec<Leaf>,
    others: &mut Vec<Leaf>,
) -> GeometryResult<()> {
    if depth > MAX_DEPTH {
        return Err(GeometryError::ChainTooDeep {
            entity: product,
            kind: "representation leaf",
            limit: MAX_DEPTH,
        });
    }
    match session.shape(node) {
        Some(NodeShape::Collection(members)) => {
            for &member in members {
                walk(session, member, outer, product, depth + 1, curves, others)?;
            }
        }
        Some(&NodeShape::Instance { source, transform }) if holds_curve(session, source, 0) => {
            // `outer` is applied after this instance's own transform.
            let composed = outer.map_or(transform, |outer| outer * transform);
            walk(
                session,
                source,
                Some(composed),
                product,
                depth + 1,
                curves,
                others,
            )?;
        }
        _ if is_curve(session, node) => curves.push((node, outer)),
        _ => others.push((node, outer)),
    }
    Ok(())
}

/// Whether `node` is a curve, an open profile, or a curve placed at a
/// station: each one curve to the polyline compiler.
fn is_curve(session: &LoweringSession<'_>, node: NodeId) -> bool {
    match session.shape(node) {
        Some(NodeShape::Curve) => true,
        _ => matches!(
            session.relation(node),
            Some(GeometryNode::InstanceAtStation(placed))
                if matches!(session.shape(placed.source), Some(NodeShape::Curve))
        ),
    }
}

/// Whether a curve lies anywhere under `node`.
fn holds_curve(session: &LoweringSession<'_>, node: NodeId, depth: usize) -> bool {
    if depth > MAX_DEPTH {
        return false;
    }
    match session.shape(node) {
        Some(NodeShape::Collection(members)) => members
            .iter()
            .any(|&member| holds_curve(session, member, depth + 1)),
        Some(&NodeShape::Instance { source, .. }) => holds_curve(session, source, depth + 1),
        _ => is_curve(session, node),
    }
}

/// `leaf` under the placement composed above it, as a new instance node,
/// or the leaf itself when nothing places it.
fn place(
    session: &mut LoweringSession<'_>,
    leaf: Leaf,
    product: EntityId,
) -> GeometryResult<NodeId> {
    match leaf {
        (node, None) => Ok(node),
        (source, Some(transform)) => session.node_for(
            product,
            GeometryNode::Instance(Instance { source, transform }),
        ),
    }
}

/// The family of the curve under `node` the reference compiler refuses by
/// name, through placements and open profiles.
//
// TODO(#506): trimmed, composite, offset and intrinsic curves compile to
// polylines once the reference compiler does (axiolid/kernel#317). The
// support belongs in the kernel, not here: this only names the refusal.
fn unpolylined(graph: &GeometryGraph, mut node: NodeId) -> Option<UnpolylinedCurve> {
    for _ in 0..=MAX_DEPTH {
        match graph.get(node)? {
            GeometryNode::CurveRelation(relation) => {
                return match relation {
                    CurveRelation::Trimmed { .. } => Some(UnpolylinedCurve::Trimmed),
                    CurveRelation::Composite { .. } => Some(UnpolylinedCurve::Composite),
                    CurveRelation::Offset { .. }
                    | CurveRelation::OffsetByStations { .. }
                    | CurveRelation::OffsetsAtStations { .. } => Some(UnpolylinedCurve::Offset),
                    // A curve on a surface or in its parameter space is
                    // none of the families #506 names.
                    _ => None,
                };
            }
            GeometryNode::Curve2(Curve2::Intrinsic(_))
            | GeometryNode::Curve3(Curve3::Intrinsic(_)) => {
                return Some(UnpolylinedCurve::Intrinsic);
            }
            GeometryNode::Instance(instance) => node = instance.source,
            GeometryNode::InstanceAtStation(placed) => node = placed.source,
            GeometryNode::OpenProfile(profile) => node = profile.path,
            _ => return None,
        }
    }
    None
}

/// The backend's refusal of curve `leaf`, named by its family when it is
/// one #506 tracks.
fn polyline_refused(
    lowered: &LoweredGeometry,
    leaf: NodeId,
    curve: Option<EntityId>,
    product: EntityId,
    error: axiolid_contracts::GeomError,
) -> GeometryError {
    match unpolylined(&lowered.graph, leaf) {
        Some(family) => GeometryError::PolylineUnsupported {
            product,
            curve: curve.unwrap_or(product),
            family,
            reason: format!("{error:?}"),
        },
        None => refused(product, error, &lowered.provenance),
    }
}
