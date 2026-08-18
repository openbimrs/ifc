//! Triangle meshes — the universal exchange currency between backends.
//!
//! Every representation the IFC side can produce (extrusion, B-rep, CSG,
//! tessellation, swept disk) is lowered to a [`TriMesh`] before it reaches a
//! geometry backend. That is deliberate: it is the ONE input type a backend
//! must understand, which is what keeps backends swappable.

use crate::primitives::{Aabb, Vec3};

/// An indexed triangle mesh.
///
/// Positions are in the mesh's own local space; placement is applied by the
/// caller. Indices are triplets — `indices.len()` is always a multiple of 3.
///
/// **Dirtiness is representable, not an error.** Real IFC meshes are frequently
/// non-manifold, have duplicate vertices, or contain degenerate triangles. This
/// type accepts all of that; validation is a separate, explicit step so callers
/// choose when to pay for it.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct TriMesh {
    /// Vertex positions, local space.
    pub positions: Vec<Vec3>,
    /// Triangle corner indices into [`TriMesh::positions`], 3 per triangle.
    pub indices: Vec<u32>,
}

impl TriMesh {
    /// Construct from positions and indices. Does not validate — call
    /// [`TriMesh::validate_structure`] when the source is untrusted.
    pub fn new(positions: Vec<Vec3>, indices: Vec<u32>) -> Self {
        Self { positions, indices }
    }

    /// Number of triangles.
    #[inline]
    pub fn triangle_count(&self) -> usize {
        self.indices.len() / 3
    }

    /// Bounds of all vertices. Empty mesh yields [`Aabb::empty`].
    pub fn bounds(&self) -> Aabb {
        let mut b = Aabb::empty();
        for &p in &self.positions {
            b.extend(p);
        }
        b
    }

    /// Is every index in range and is the index buffer a whole number of
    /// triangles? This is the cheap structural check, NOT a manifold check.
    pub fn is_structurally_valid(&self) -> bool {
        self.validate_structure().is_ok()
    }

    /// Structural check that reports **why** it failed.
    ///
    /// Backends call this before doing work: a caller passing a malformed index
    /// buffer is a bug worth naming precisely, not a silent wrong answer.
    pub fn validate_structure(&self) -> Result<(), String> {
        if self.indices.len() % 3 != 0 {
            return Err(format!(
                "index buffer length {} is not a multiple of 3",
                self.indices.len()
            ));
        }
        if let Some(&bad) = self
            .indices
            .iter()
            .find(|&&i| (i as usize) >= self.positions.len())
        {
            return Err(format!(
                "index {bad} out of range for {} vertices",
                self.positions.len()
            ));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn unit_triangle() -> TriMesh {
        TriMesh {
            positions: vec![
                Vec3::ZERO,
                Vec3::new(1.0, 0.0, 0.0),
                Vec3::new(0.0, 1.0, 0.0),
            ],
            indices: vec![0, 1, 2],
        }
    }

    #[test]
    fn triangle_count_and_bounds() {
        let m = unit_triangle();
        assert_eq!(m.triangle_count(), 1);
        assert!(m.is_structurally_valid());
        let b = m.bounds();
        assert_eq!(b.min, Vec3::ZERO);
        assert_eq!(b.max, Vec3::new(1.0, 1.0, 0.0));
    }

    #[test]
    fn out_of_range_index_is_structurally_invalid() {
        let mut m = unit_triangle();
        m.indices = vec![0, 1, 99];
        assert!(!m.is_structurally_valid());
    }

    #[test]
    fn empty_mesh_has_empty_bounds() {
        assert!(TriMesh::default().bounds().is_empty());
    }
}
