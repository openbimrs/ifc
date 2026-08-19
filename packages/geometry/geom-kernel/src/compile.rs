//! Complete graph compilation capability.

use geom_mesh::TriMesh;
use geom_model::{GeometryGraph, NodeId};

use crate::{Backend, ExecutionOptions, GeomResult};

/// Backend/orchestrator capable of lowering any advertised graph node to mesh.
///
/// Implementations must return [`crate::GeomError::Unsupported`] for a node they
/// do not support. They must not silently omit it or approximate exact geometry
/// unless the execution policy explicitly permits that precision.
pub trait GeometryCompiler: Backend {
    /// Compile one root.
    fn compile(
        &self,
        graph: &GeometryGraph,
        root: NodeId,
        options: &ExecutionOptions,
    ) -> GeomResult<TriMesh>;

    /// Compile roots as a batch. GPU and parallel implementations should
    /// override this rather than forcing callers into a serial loop.
    fn compile_batch(
        &self,
        graph: &GeometryGraph,
        roots: &[NodeId],
        options: &ExecutionOptions,
    ) -> GeomResult<Vec<TriMesh>> {
        roots
            .iter()
            .map(|&root| self.compile(graph, root, options))
            .collect()
    }
}
