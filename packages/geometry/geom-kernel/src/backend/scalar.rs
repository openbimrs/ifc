//! Portable scalar backend, and the **correctness oracle**.
//!
//! No SIMD intrinsics, no GPU, no `target_feature`. This crate must compile and
//! produce identical results on any machine that runs Rust. Every other backend
//! is validated by differential test against this one, so it is the only
//! backend whose correctness cannot be established by comparison — it has to be
//! right on its own merits, which is why it stays simple.
//!
//! # Status
//!
//! The boolean implementation here is a **placeholder that fails honestly**.
//! It reports its capability truthfully and returns [`GeomError::Unsupported`]
//! rather than returning a wrong mesh. Wiring a real algorithm (ours, or
//! `boolmesh` behind this trait) is Stage 2 in `docs/ROADMAP.md`.

use crate::{Backend, BooleanOp, Capabilities, GeomError, GeomResult, MeshBoolean};
use geom_mesh::TriMesh;

/// Scalar CPU geometry backend.
#[derive(Debug, Default, Clone, Copy)]
pub struct ScalarBackend;

impl ScalarBackend {
    /// Construct the backend.
    pub fn new() -> Self {
        Self
    }

    /// What this backend supports here. Boolean is reported as NOT implemented
    /// until a real algorithm lands — the capability struct is the honest
    /// signal the dispatcher reads.
    pub fn capabilities(&self) -> Capabilities {
        Capabilities {
            backend: Backend::Scalar,
            available: true,
            mesh_boolean: false,
            gpu_threshold_triangles: None,
        }
    }
}

impl MeshBoolean for ScalarBackend {
    fn name(&self) -> &'static str {
        "scalar"
    }

    fn boolean(&self, a: &TriMesh, b: &TriMesh, _op: BooleanOp) -> GeomResult<TriMesh> {
        // Validate inputs even though the algorithm is absent: structural
        // checks are backend-independent and catch caller bugs early.
        a.validate_structure()
            .map_err(GeomError::StructurallyInvalid)?;
        b.validate_structure()
            .map_err(GeomError::StructurallyInvalid)?;
        Err(GeomError::Unsupported {
            backend: "scalar",
            operation: "mesh boolean (not yet implemented — see docs/ROADMAP.md Stage 2)",
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reports_boolean_unimplemented_rather_than_lying() {
        let m = TriMesh::new(vec![], vec![]);
        let err = ScalarBackend::new()
            .boolean(&m, &m, BooleanOp::Difference)
            .unwrap_err();
        assert!(matches!(err, GeomError::Unsupported { .. }));
        assert!(!ScalarBackend::new().capabilities().mesh_boolean);
    }

    #[test]
    fn rejects_structurally_invalid_input_before_anything_else() {
        let bad = TriMesh::new(vec![geom_core::Vec3::ZERO], vec![0, 1, 2]);
        let good = TriMesh::new(vec![], vec![]);
        let err = ScalarBackend::new()
            .boolean(&bad, &good, BooleanOp::Union)
            .unwrap_err();
        assert!(matches!(err, GeomError::StructurallyInvalid(_)));
    }
}
