//! Mesh boolean — the operation that decides whether we can replace OpenCascade.
//!
//! # Why this trait is the project's linchpin
//!
//! IfcOpenShell depends on OpenCascade largely because IFC needs robust
//! constructive solid geometry: `IfcRelVoidsElement` cuts door and window
//! openings out of walls, `IfcBooleanClippingResult` clips solids with
//! half-spaces, and `IfcBooleanResult` is a general union/difference tree.
//! OpenCascade solves that, and costs a very heavy C++ dependency.
//!
//! Our position: a robust mesh boolean is achievable in pure Rust. Two existing
//! pure-Rust implementations demonstrate it (`boolmesh`, a from-scratch
//! Manifold-inspired kernel whose only hard dependency is `glam`; and
//! `manifold-rust`, a port targeting numerical parity with Manifold v3.5.0).
//! We therefore express boolean as a trait and can either implement it or adopt
//! one of those behind it — without the IFC layer knowing which.
//!
//! # Contract
//!
//! Implementors must guarantee: **manifold input yields manifold output.** An
//! implementation that cannot uphold that for a given input returns
//! [`crate::GeomError::NotManifold`] rather than silently emitting a broken
//! mesh — a corrupt solid propagates into every downstream area, volume, and clash
//! result, which is far worse than an explicit failure.

use crate::error::GeomResult;
use geom_core::TriMesh;

/// Which boolean to perform.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BooleanOp {
    /// `a ∪ b`
    Union,
    /// `a ∩ b`
    Intersection,
    /// `a \ b` — the IFC opening cut (`IfcRelVoidsElement`).
    Difference,
}

/// A backend that can compute mesh booleans.
///
/// # Granularity
///
/// [`MeshBoolean::batch_difference`] exists because IFC's dominant pattern is
/// one wall minus *many* openings. Handing the backend the whole tool set at
/// once lets a parallel or GPU implementation schedule all cuts together;
/// looping [`MeshBoolean::boolean`] on the caller side would serialize it and
/// pay the dynamic-dispatch cost per opening.
pub trait MeshBoolean: Send + Sync {
    /// Stable name for diagnostics and differential test output.
    fn name(&self) -> &'static str;

    /// Compute `op` between two meshes.
    fn boolean(&self, a: &TriMesh, b: &TriMesh, op: BooleanOp) -> GeomResult<TriMesh>;

    /// Subtract many tools from one subject.
    ///
    /// The default implementation folds [`MeshBoolean::boolean`], which is
    /// always correct. Backends that can do better (parallel, GPU, or a CSG
    /// tree that batches) should override it.
    fn batch_difference(&self, subject: &TriMesh, tools: &[TriMesh]) -> GeomResult<TriMesh> {
        let mut acc = subject.clone();
        for tool in tools {
            acc = self.boolean(&acc, tool, BooleanOp::Difference)?;
        }
        Ok(acc)
    }
}
