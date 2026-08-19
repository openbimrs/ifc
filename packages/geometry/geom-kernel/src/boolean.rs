//! Mesh boolean capability with batch-first IFC-opening semantics.

use geom_mesh::TriMesh;

use crate::{Backend, ExecutionOptions, GeomResult};

/// Set operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BooleanOp {
    /// Union.
    Union,
    /// Intersection.
    Intersection,
    /// Left minus right.
    Difference,
}

/// Backend capable of robust mesh booleans.
pub trait MeshBoolean: Backend {
    /// Compute one boolean under explicit execution policy.
    fn boolean(
        &self,
        left: &TriMesh,
        right: &TriMesh,
        operation: BooleanOp,
        options: &ExecutionOptions,
    ) -> GeomResult<TriMesh>;

    /// Subtract many tools from one subject.
    ///
    /// The default is deterministic and correct. Parallel/GPU implementations
    /// override this to amortize dispatch while preserving the same contract.
    fn batch_difference(
        &self,
        subject: &TriMesh,
        tools: &[TriMesh],
        options: &ExecutionOptions,
    ) -> GeomResult<TriMesh> {
        let mut result = subject.clone();
        for tool in tools {
            result = self.boolean(&result, tool, BooleanOp::Difference, options)?;
        }
        Ok(result)
    }
}
