//! Borrowed mesh interoperability seam.

use geom_core::Point3;

use crate::TriMesh;

/// Read-only triangle mesh view.
///
/// Solibri or another kernel can implement this for its native mesh and call
/// algorithms without first adopting Nehirde's owned container.
pub trait MeshView {
    /// Position buffer.
    fn positions(&self) -> &[Point3];
    /// Triangle corner index buffer.
    fn indices(&self) -> &[u32];
}

impl MeshView for TriMesh {
    fn positions(&self) -> &[Point3] {
        &self.positions
    }

    fn indices(&self) -> &[u32] {
        &self.indices
    }
}
