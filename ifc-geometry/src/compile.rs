//! Mesh compilation: run the lowered DAG through a geometry backend.
//!
//! Gated behind the opt-in `compile` feature. ADR 0004 keeps this crate a
//! bridge, and compilation is the one narrow exception: it selects an
//! execution provider and a memory budget, which are application policy
//! rather than properties of the file. Nothing here is reachable in a
//! default build.
//!
//! The bridge still implements no geometry. It calls a compiler that
//! already exists and translates the refusal into IFC terms.
//!
//! # Bringing your own kernel
//!
//! Every entry point is generic over [`MeshCompiler`], so a caller may bring
//! CGAL, OCCT, a GPU tessellator, or a research prototype without forking
//! this crate. Two granularities are supported:
//!
//! * **Whole-kernel.** Implement [`MeshCompiler`] over your own engine and
//!   pass it to [`compile_product_mesh_with`]. Enable `compile` alone and
//!   none of the reference execution path is even linked.
//! * **Per-area.** Keep the reference compiler's traversal and swap only the
//!   area a kernel does well, e.g. `ReferenceMeshCompiler::new(MyBoolean)`
//!   to replace boolean evaluation while retaining everything else. That
//!   needs the `compile-reference-backend` feature for the traversal.
//!
//! Selection is the application's, never the file's: two backends given the
//! same model must agree on what the file means, so choosing between them is
//! a question of speed, robustness, or licence -- which is why this module
//! refuses to pick for you beyond the documented default.

mod bounds;

use axiolid_contracts::ExecutionOptions;
use axiolid_core::{Aabb, Tolerance};
use axiolid_mesh::TriMesh;
use axiolid_mesh_compile_contract::{MeshClosure, MeshCompiler};
use ifc_model::{EntityId, Model};

use crate::error::{GeometryError, GeometryResult};
use crate::lower::{
    lower_product_net, lower_product_representation, LoweringSession, NetLowering,
    RepresentationPurpose,
};
use crate::units;

#[cfg(feature = "compile-reference-backend")]
use axiolid_mesh_boolean_boolmesh::BoolmeshBoolean;
#[cfg(feature = "compile-reference-backend")]
use axiolid_mesh_compile::ReferenceMeshCompiler;

/// The backend used when a caller expresses no preference.
///
/// Named and returned rather than constructed inline so a caller can ask
/// what the default *is* -- to log it beside a benchmark, or to compare
/// against it -- without hard-coding the same type this module does.
#[cfg(feature = "compile-reference-backend")]
#[must_use]
pub fn default_backend() -> ReferenceMeshCompiler<BoolmeshBoolean> {
    ReferenceMeshCompiler::new(BoolmeshBoolean)
}

/// Compile one product's body representation into triangles.
///
/// Uses [`default_backend`]. Call [`compile_product_mesh_with`] to supply
/// your own.
///
/// Returns `Ok(None)` when the product has no body representation at all,
/// which is ordinary -- a spatial container or an annotation-only product is
/// not an error.
///
/// # Tolerance
///
/// Lowering converts every length to metres, and the backend reads tolerance
/// in the model's current length unit, so [`Tolerance::MILLIMETRE`] (1e-3) is
/// a millimetre here regardless of what the file declared. Deriving a
/// tolerance from the file's unit scale would double-apply the conversion.
///
/// # Errors
///
/// Returns [`GeometryError::CompilationRefused`] when the backend cannot
/// produce a mesh, and any lowering error the representation raises first.
#[cfg(feature = "compile-reference-backend")]
pub fn compile_product_mesh(
    model: &Model,
    product: EntityId,
    tolerance: Tolerance,
) -> GeometryResult<Option<TriMesh>> {
    compile_product_mesh_with(&default_backend(), model, product, tolerance)
}

/// Compile one product's body representation with a caller-supplied backend.
///
/// The backend is borrowed, so one instance serves a whole run: building a
/// compiler per product would discard whatever caches or device handles an
/// implementation holds.
///
/// # Errors
///
/// Returns [`GeometryError::CompilationRefused`] when the backend cannot
/// produce a mesh, and any lowering error the representation raises first.
///
/// # Examples
///
/// Swapping only boolean evaluation, keeping the reference traversal:
///
/// ```no_run
/// # #[cfg(feature = "compile-reference-backend")] {
/// # use axiolid_core::Tolerance;
/// # use axiolid_mesh_boolean_boolmesh::BoolmeshBoolean;
/// # use axiolid_mesh_compile::ReferenceMeshCompiler;
/// # use ifc_geometry::compile::compile_product_mesh_with;
/// # use ifc_model::{EntityId, Model};
/// # fn run(model: &Model, product: EntityId) -> Result<(), Box<dyn std::error::Error>> {
/// let backend = ReferenceMeshCompiler::new(BoolmeshBoolean);
/// let mesh = compile_product_mesh_with(&backend, model, product, Tolerance::MILLIMETRE)?;
/// # let _ = mesh;
/// # Ok(())
/// # }
/// # }
/// ```
pub fn compile_product_mesh_with<B: MeshCompiler>(
    backend: &B,
    model: &Model,
    product: EntityId,
    tolerance: Tolerance,
) -> GeometryResult<Option<TriMesh>> {
    let scale = units::resolve(model);
    let mut session = LoweringSession::new(model, &scale);
    let Some(root) =
        lower_product_representation(&mut session, product, RepresentationPurpose::Body)?
    else {
        return Ok(None);
    };
    let lowered = session.finish(root)?;

    let options = ExecutionOptions::new(tolerance);
    backend
        .compile_mesh(&lowered.graph, lowered.root, &options)
        .map(Some)
        .map_err(|error| refused(product, error))
}

/// A compiled mesh and whether it bounds a solid.
///
/// IFC distinguishes solids (`IfcFacetedBrep`, `IfcExtrudedAreaSolid`,
/// `IfcPolygonalFaceSet` with `Closed`) from surface models
/// (`IfcShellBasedSurfaceModel`, `IfcFaceBasedSurfaceModel`), and a surface
/// model has an area but no volume -- even when its shells happen to close.
/// A bare `TriMesh` cannot say which one it is, so a caller summing a
/// divergence volume over every compiled product silently reports a volume
/// for surfaces. `closure` carries the backend's answer instead.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct CompiledMesh {
    /// The triangles.
    pub mesh: TriMesh,
    /// [`MeshClosure::Solid`] when the mesh bounds a volume the file
    /// declared, [`MeshClosure::Surface`] for a surface model, and
    /// [`MeshClosure::Unknown`] when the backend does not report it.
    pub closure: MeshClosure,
}

impl CompiledMesh {
    /// The mesh, only when it bounds a declared solid.
    ///
    /// Use this, not `mesh`, before measuring a volume or feeding a boolean.
    ///
    /// # Errors
    ///
    /// [`GeometryError::NotASolid`] naming `product` for a surface model, and
    /// for a backend that does not report closure: an unknown closure is not
    /// evidence of a solid.
    pub fn solid_mesh(&self, product: EntityId) -> GeometryResult<&TriMesh> {
        match self.closure {
            MeshClosure::Solid => Ok(&self.mesh),
            closure => Err(GeometryError::NotASolid {
                entity: product,
                closure,
            }),
        }
    }
}

/// Compile one product's body and report whether it bounds a solid.
///
/// Uses [`default_backend`]; [`compile_product_mesh_reported_with`] takes
/// your own. Same triangles as [`compile_product_mesh`], plus the closure a
/// volume reader needs (axiolid/kernel#161).
///
/// # Errors
///
/// As [`compile_product_mesh`].
#[cfg(feature = "compile-reference-backend")]
pub fn compile_product_mesh_reported(
    model: &Model,
    product: EntityId,
    tolerance: Tolerance,
) -> GeometryResult<Option<CompiledMesh>> {
    compile_product_mesh_reported_with(&default_backend(), model, product, tolerance)
}

/// Compile one product's body with a caller-supplied backend and report
/// whether it bounds a solid.
///
/// A backend that does not override `MeshCompiler::compile_mesh_reported`
/// reports [`MeshClosure::Unknown`], which [`CompiledMesh::solid_mesh`]
/// refuses. That is deliberate: a volume needs evidence, not a default.
///
/// # Errors
///
/// As [`compile_product_mesh_with`].
pub fn compile_product_mesh_reported_with<B: MeshCompiler>(
    backend: &B,
    model: &Model,
    product: EntityId,
    tolerance: Tolerance,
) -> GeometryResult<Option<CompiledMesh>> {
    let scale = units::resolve(model);
    let mut session = LoweringSession::new(model, &scale);
    let Some(root) =
        lower_product_representation(&mut session, product, RepresentationPurpose::Body)?
    else {
        return Ok(None);
    };
    let lowered = session.finish(root)?;

    let options = ExecutionOptions::new(tolerance);
    backend
        .compile_mesh_reported(&lowered.graph, lowered.root, &options)
        .map(|outcome| {
            Some(CompiledMesh {
                closure: outcome.closure,
                mesh: outcome.mesh,
            })
        })
        .map_err(|error| refused(product, error))
}

/// A product's NET mesh and the openings removed to produce it.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct NetMesh {
    /// The Body with every voiding opening subtracted.
    pub mesh: TriMesh,
    /// The openings subtracted, in ascending id. Empty when none void it.
    pub openings: Vec<EntityId>,
    /// Whether `mesh` bounds a solid. A net body is [`MeshClosure::Solid`]
    /// whenever a subtraction ran, since a boolean refuses a surface operand;
    /// a host with no openings keeps whatever its gross body reported.
    pub closure: MeshClosure,
}

/// Compile one product's Body with every voiding opening subtracted (#44).
///
/// Uses [`default_backend`]; [`compile_product_mesh_net_with`] takes your own.
/// [`compile_product_mesh`] stays the gross body: quantity takeoff wants
/// gross, clearance and ratio checks want net, so neither replaces the other.
///
/// # Errors
///
/// [`GeometryError::OpeningNotSubtracted`] naming the opening when one cannot
/// be removed. The gross body is never returned in place of the net one.
#[cfg(feature = "compile-reference-backend")]
pub fn compile_product_mesh_net(
    model: &Model,
    product: EntityId,
    tolerance: Tolerance,
) -> GeometryResult<Option<NetMesh>> {
    compile_product_mesh_net_with(&default_backend(), model, product, tolerance)
}

/// Compile one product's net Body with a caller-supplied backend.
///
/// # Cost
///
/// The success path compiles the net graph once; the backend's own cache
/// shares the host and opening meshes across the chain. Only a refusal pays
/// more: the host is compiled alone, then the chain one subtraction at a
/// time, to name the opening at fault. That is a
/// diagnostic pass on a path that has already failed, so it trades time for a
/// precise error rather than slowing every successful call.
///
/// # Errors
///
/// A refusal the host's gross body alone would raise stays
/// [`GeometryError::CompilationRefused`] on the host. A refusal that only an
/// opening explains is [`GeometryError::OpeningNotSubtracted`] naming it.
pub fn compile_product_mesh_net_with<B: MeshCompiler>(
    backend: &B,
    model: &Model,
    product: EntityId,
    tolerance: Tolerance,
) -> GeometryResult<Option<NetMesh>> {
    let scale = units::resolve(model);
    let mut session = LoweringSession::new(model, &scale);
    let Some(net) = lower_product_net(&mut session, product)? else {
        return Ok(None);
    };
    let openings = net.openings();
    let lowered = session.finish(net.root)?;
    let options = ExecutionOptions::new(tolerance);

    match backend.compile_mesh_reported(&lowered.graph, lowered.root, &options) {
        Ok(outcome) => Ok(Some(NetMesh {
            closure: outcome.closure,
            mesh: outcome.mesh,
            openings,
        })),
        Err(error) => Err(attribute_net_refusal(
            backend,
            &lowered.graph,
            &net,
            &options,
            product,
            error,
        )),
    }
}

/// Find the part of a net graph the backend cannot compile, and name it.
///
/// Order matters: a broken host would fail every step, so it is checked
/// first; then the chain is walked to the first step the kernel refuses. If
/// every step compiles alone, the refusal is reported on the host,
/// unattributed, rather than guessed.
fn attribute_net_refusal<B: MeshCompiler>(
    backend: &B,
    graph: &axiolid_model::GeometryGraph,
    net: &NetLowering,
    options: &ExecutionOptions,
    product: EntityId,
    original: axiolid_contracts::GeomError,
) -> GeometryError {
    if let Err(error) = backend.compile_mesh(graph, net.gross, options) {
        return refused(product, error);
    }
    let blame = |opening: EntityId, error| GeometryError::OpeningNotSubtracted {
        host: product,
        opening,
        cause: Box::new(refused(opening, error)),
    };
    // Step k's graph holds the host, bodies 1..=k and subtractions 1..=k, so
    // the first failing step is the first opening whose body OR whose cut the
    // backend refuses. No separate per-body pass is needed.
    for step in &net.subtractions {
        if let Err(error) = backend.compile_mesh(graph, step.result, options) {
            return blame(step.opening, error);
        }
    }
    refused(product, original)
}

/// How a product's bounds were obtained.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum BoundsSource {
    /// Read off the exact lowered geometry without tessellating: every leaf
    /// was a mesh or an authored bounding box, so the box is the shape's.
    Exact,
    /// Taken from the compiled mesh. Exact for planar geometry; for curved
    /// geometry the mesh's vertices lie on the surface and chords cut
    /// inside it, so the box can fall short of the true surface by up to
    /// the compile tolerance.
    Tessellated,
}

/// A product's axis-aligned bounds in world coordinates, in metres.
#[derive(Debug, Clone, Copy, PartialEq)]
#[non_exhaustive]
pub struct ProductBounds {
    /// The box, after placement.
    pub aabb: Aabb,
    /// Whether it came from the exact graph or a compiled mesh.
    pub source: BoundsSource,
}

/// The world-space axis-aligned bounding box of one product's body.
///
/// Uses [`default_backend`] for the tessellated fallback; see
/// [`product_bounds_with`].
///
/// # Errors
///
/// As [`product_bounds_with`].
#[cfg(feature = "compile-reference-backend")]
pub fn product_bounds(
    model: &Model,
    product: EntityId,
    tolerance: Tolerance,
) -> GeometryResult<Option<ProductBounds>> {
    product_bounds_with(&default_backend(), model, product, tolerance)
}

/// The world-space axis-aligned bounding box of one product's body.
///
/// The body is resolved and placed exactly as [`compile_product_mesh_with`]
/// does, and returns `Ok(None)` in the same case: a product with no body
/// representation. When every exact leaf is a mesh or an authored bounding
/// box, the box is read off the lowered graph without tessellating
/// ([`BoundsSource::Exact`]). Otherwise the body is compiled with `backend`
/// and the mesh's bounds are used ([`BoundsSource::Tessellated`]).
///
/// The result feeds a spatial index such as `axiolid_spatial::Bvh` directly;
/// this crate computes the leaf boxes and leaves the index to Axiolid.
///
/// # Errors
///
/// Any lowering error, [`GeometryError::CompilationRefused`] when the
/// fallback cannot compile, and [`GeometryError::Degenerate`] when the body
/// has no finite extent (an empty or non-finite mesh): an empty box is not
/// a bound.
pub fn product_bounds_with<B: MeshCompiler>(
    backend: &B,
    model: &Model,
    product: EntityId,
    tolerance: Tolerance,
) -> GeometryResult<Option<ProductBounds>> {
    let scale = units::resolve(model);
    let mut session = LoweringSession::new(model, &scale);
    let Some(root) =
        lower_product_representation(&mut session, product, RepresentationPurpose::Body)?
    else {
        return Ok(None);
    };
    let lowered = session.finish(root)?;
    let (aabb, source) = match bounds::exact(&lowered.graph, lowered.root) {
        Some(aabb) => (aabb, BoundsSource::Exact),
        None => {
            let options = ExecutionOptions::new(tolerance);
            let mesh = backend
                .compile_mesh(&lowered.graph, lowered.root, &options)
                .map_err(|error| refused(product, error))?;
            (mesh.bounds(), BoundsSource::Tessellated)
        }
    };
    if aabb.is_empty() || !aabb.is_finite() {
        return Err(GeometryError::Degenerate {
            entity: product,
            type_name: model
                .get(product)
                .map_or_else(String::new, |entity| entity.type_name.to_string()),
            detail: "body has no finite extent to bound".into(),
        });
    }
    Ok(Some(ProductBounds { aabb, source }))
}

fn refused(entity: EntityId, error: axiolid_contracts::GeomError) -> GeometryError {
    GeometryError::CompilationRefused {
        entity,
        reason: format!("{error:?}"),
    }
}
