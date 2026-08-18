//! The `ShapeLowerer` façade — holds the injected kernel, delegates the work.
//!
//! Representation-specific lowering lives in the sibling modules
//! ([`crate::swept`], [`crate::brep`], [`crate::csg`], …); this file stays the
//! thin entry point so it does not accumulate every geometry case.

use geom_kernel::{GeomResult, MeshBoolean};
use geom_mesh::TriMesh;

/// Lowers IFC representation items into meshes using an injected geometry
/// kernel.
///
/// Generic over `K` so the backend is a **compile-time** choice at the
/// application edge with no dynamic dispatch cost, while remaining swappable —
/// `K` is only ever bounded by kernel traits. Use `&dyn MeshBoolean` instead if
/// runtime selection is wanted; both work because the bound is a trait.
pub struct ShapeLowerer<K: MeshBoolean> {
    kernel: K,
}

impl<K: MeshBoolean> ShapeLowerer<K> {
    /// Inject a geometry kernel.
    pub fn new(kernel: K) -> Self {
        Self { kernel }
    }

    /// Which kernel is in use (diagnostics).
    pub fn kernel_name(&self) -> &'static str {
        self.kernel.name()
    }

    /// Apply `IfcRelVoidsElement` openings: subject minus every opening solid.
    ///
    /// Delegates to [`MeshBoolean::batch_difference`] so a backend that can cut
    /// all openings at once does so, rather than being forced into a serial
    /// fold by this call site.
    pub fn apply_openings(&self, body: &TriMesh, openings: &[TriMesh]) -> GeomResult<TriMesh> {
        if openings.is_empty() {
            return Ok(body.clone());
        }
        self.kernel.batch_difference(body, openings)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use geom_kernel::BooleanOp;

    /// A stub kernel proving the seam is genuinely backend-agnostic: this test
    /// injects a kernel that lives in neither a backend feature nor any other
    /// crate, and `ifc-geometry` accepts it without modification.
    struct StubKernel;

    impl MeshBoolean for StubKernel {
        fn name(&self) -> &'static str {
            "stub"
        }
        fn boolean(&self, a: &TriMesh, _b: &TriMesh, _op: BooleanOp) -> GeomResult<TriMesh> {
            Ok(a.clone())
        }
    }

    #[test]
    fn accepts_any_kernel_implementing_the_trait() {
        let lowerer = ShapeLowerer::new(StubKernel);
        assert_eq!(lowerer.kernel_name(), "stub");
    }

    #[test]
    fn no_openings_is_a_passthrough_and_does_not_call_the_kernel() {
        let body = TriMesh::new(vec![], vec![]);
        let out = ShapeLowerer::new(StubKernel)
            .apply_openings(&body, &[])
            .unwrap();
        assert_eq!(out.triangle_count(), 0);
    }
}
