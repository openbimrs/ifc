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
//! # Text and fill areas beside curves (#512)
//!
//! [`compile_product_curves_with`] reads the curves of a representation
//! that also holds items with no curve to give: an `IfcTextLiteral` or
//! `IfcTextLiteralWithExtent` (the value of a dimension) and an
//! `IfcAnnotationFillArea`. Each such item of the representation's own
//! `Items` is not lowered: it is listed, by id and [`NonCurveItem`] kind,
//! in [`ProductPolylines::skipped`], and its reader is
//! `ifc_style::StyleView` (`text_literal`, `text_literal_with_extent`,
//! `annotation_fill_area`). Nothing is drawn in its place: a fill area's
//! boundaries are not returned as polylines. Every other item lowers as
//! before, and its refusal still refuses the product; a text literal nested
//! in a mapped representation is refused, not skipped.
//! [`compile_product_polylines_with`] keeps the strict rule and refuses
//! such a representation as a whole.
//!
//! # Refused by name
//!
//! A backend that cannot compile a curve to a polyline refuses it, and
//! the refusal is [`GeometryError::CompilationRefused`] carrying the
//! backend's own reason. The reference compiler polylines trimmed,
//! composite, offset and intrinsic curves since `axiolid-mesh-compile`
//! 0.3.28 (axiolid/kernel#317; #506), and still refuses by name the forms
//! it has no certified bound for: implicit and banked curves, a 3D
//! intrinsic curve with torsion, a trim of a relation or of a placed
//! curve, an offset of an offset, of a placed curve or of a chain, a 3D
//! offset without a reference direction, an offset across a corner of its
//! basis, and an offset by stations.

use axiolid_contracts::ExecutionOptions;
use axiolid_core::Transform3;
use axiolid_mesh_compile_contract::MeshCompiler;
use axiolid_model::{GeometryNode, Instance, NodeId};
use ifc_model::{EntityId, Model};

pub use axiolid_core::Point3;
pub use axiolid_mesh_compile_contract::{ChordDeviation, MeshClosure, PolylineCompiler};

use super::{refused, CompiledMesh, Tolerance};
use crate::error::{GeometryError, GeometryResult};
use crate::lower::context::lower_product_representation_leaving_out;
use crate::lower::session::NodeShape;
use crate::lower::{
    lower_product_representation, LoweredGeometry, LoweringSession, RepresentationPurpose,
};
use crate::units;

#[cfg(feature = "compile-reference-backend")]
use super::default_backend;

/// Nesting of collections and placements followed before refusing.
const MAX_DEPTH: usize = 256;

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

/// A representation item with no curve to give, which the curves reader
/// leaves to its own reader (#512).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum NonCurveItem {
    /// An `IfcTextLiteral` or `IfcTextLiteralWithExtent`: text, read by
    /// `ifc_style::StyleView::text_literal` and `text_literal_with_extent`.
    TextLiteral,
    /// An `IfcAnnotationFillArea`: an area, read by
    /// `ifc_style::StyleView::annotation_fill_area`.
    AnnotationFillArea,
}

impl NonCurveItem {
    /// The kind of a representation item, by STEP type name (any case),
    /// `None` for every item that is lowered.
    #[must_use]
    pub fn of(type_name: &str) -> Option<Self> {
        if type_name.eq_ignore_ascii_case("IFCTEXTLITERAL")
            || type_name.eq_ignore_ascii_case("IFCTEXTLITERALWITHEXTENT")
        {
            Some(Self::TextLiteral)
        } else if type_name.eq_ignore_ascii_case("IFCANNOTATIONFILLAREA") {
            Some(Self::AnnotationFillArea)
        } else {
            None
        }
    }
}

/// A representation item the curves reader did not lower.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub struct SkippedItem {
    /// The item, one of the representation's own `Items`.
    pub item: EntityId,
    /// Why it has no polyline.
    pub kind: NonCurveItem,
}

/// The curves of one product's representation, and the items with no
/// curve to give that were left out.
#[derive(Debug, Clone, PartialEq, Default)]
#[non_exhaustive]
pub struct ProductPolylines {
    /// One polyline per curve, in authored order.
    pub polylines: Vec<CompiledPolyline>,
    /// The text literals and fill areas of the representation, in
    /// authored order, not lowered and not drawn.
    pub skipped: Vec<SkippedItem>,
}

/// The curves of one product's representation for `purpose`, leaving its
/// text literals and fill areas to their readers, with [`default_backend`].
///
/// # Errors
///
/// As [`compile_product_curves_with`].
#[cfg(feature = "compile-reference-backend")]
pub fn compile_product_curves(
    model: &Model,
    product: EntityId,
    purpose: RepresentationPurpose,
    tolerance: Tolerance,
) -> GeometryResult<Option<ProductPolylines>> {
    compile_product_curves_with(&default_backend(), model, product, purpose, tolerance)
}

/// The curves of one product's representation for `purpose` as
/// polylines, leaving its text literals and fill areas to their readers
/// (#512), with a caller-supplied backend.
///
/// As [`compile_product_polylines_with`], except that each
/// `IfcTextLiteral`, `IfcTextLiteralWithExtent` and
/// `IfcAnnotationFillArea` among the representation's own `Items` is not
/// lowered but listed in [`ProductPolylines::skipped`]: a dimension
/// (polylines and its value as text) gives its polylines. `Ok(None)` when
/// the product has no representation for `purpose`.
///
/// # Errors
///
/// Any lowering error of an item not skipped, and
/// [`GeometryError::CompilationRefused`] for a curve the backend refuses.
/// One refused curve refuses the product.
pub fn compile_product_curves_with<B: PolylineCompiler>(
    backend: &B,
    model: &Model,
    product: EntityId,
    purpose: RepresentationPurpose,
    tolerance: Tolerance,
) -> GeometryResult<Option<ProductPolylines>> {
    let Some((split, left_out)) = lower_split(model, product, purpose, true)? else {
        return Ok(None);
    };
    let skipped = left_out
        .into_iter()
        .filter_map(|(item, type_name)| {
            NonCurveItem::of(&type_name).map(|kind| SkippedItem { item, kind })
        })
        .collect();
    let polylines = match split {
        Some(split) => compile_split(backend, &split, product, tolerance)?,
        None => Vec::new(),
    };
    Ok(Some(ProductPolylines { polylines, skipped }))
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
/// Any lowering error, and [`GeometryError::CompilationRefused`], with the
/// backend's reason, for a curve the backend refuses. One refused curve
/// refuses the product: a partial outline is not returned as the whole.
pub fn compile_product_polylines_with<B: PolylineCompiler>(
    backend: &B,
    model: &Model,
    product: EntityId,
    purpose: RepresentationPurpose,
    tolerance: Tolerance,
) -> GeometryResult<Option<Vec<CompiledPolyline>>> {
    let Some((Some(split), _)) = lower_split(model, product, purpose, false)? else {
        return Ok(None);
    };
    compile_split(backend, &split, product, tolerance).map(Some)
}

/// Each curve of `split` as a polyline, in order; one refusal refuses all.
fn compile_split<B: PolylineCompiler>(
    backend: &B,
    split: &Split,
    product: EntityId,
    tolerance: Tolerance,
) -> GeometryResult<Vec<CompiledPolyline>> {
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
                .map_err(|error| refused(product, error, &lowered.provenance))
        })
        .collect()
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
    let Some((Some(split), _)) = lower_split(model, product, purpose, false)? else {
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

/// Items the curves reader left out: id and STEP type name.
type LeftOut = Vec<(EntityId, std::sync::Arc<str>)>;

/// The product's representation for `purpose`, lowered and placed, and
/// split into its curves and the rest.
///
/// With `leave_out_non_curves`, the representation's own text literals
/// and fill areas ([`NonCurveItem`]) are not lowered but returned; the
/// split is `None` when nothing else remains. Without it, nothing is left
/// out, and a representation of no items is `Ok(None)`.
fn lower_split(
    model: &Model,
    product: EntityId,
    purpose: RepresentationPurpose,
    leave_out_non_curves: bool,
) -> GeometryResult<Option<(Option<Split>, LeftOut)>> {
    let scale = units::resolve(model)?;
    let mut session = LoweringSession::new(model, &scale);
    let (root, left_out) = if leave_out_non_curves {
        let Some((root, left_out)) =
            lower_product_representation_leaving_out(&mut session, product, purpose, |name| {
                NonCurveItem::of(name).is_some()
            })?
        else {
            return Ok(None);
        };
        let left_out = left_out
            .into_iter()
            .filter_map(|item| {
                model
                    .get(item)
                    .map(|entity| (item, entity.type_name.clone()))
            })
            .collect();
        (root, left_out)
    } else {
        let Some(root) = lower_product_representation(&mut session, product, purpose)? else {
            return Ok(None);
        };
        (Some(root), Vec::new())
    };
    let Some(root) = root else {
        return Ok(Some((None, left_out)));
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
    Ok(Some((
        Some(Split {
            lowered,
            curves,
            mesh,
        }),
        left_out,
    )))
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
