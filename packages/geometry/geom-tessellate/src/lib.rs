//! `geom-tessellate` — exact geometry → triangles.
//!
//! # Why this is its own crate
//!
//! Tessellation is the bridge from `geom-topology` / `geom-surface` to
//! `geom-mesh`, and it is the one place the question *"how fine?"* should be
//! answered. Scattering chord-tolerance decisions across the sweep, B-rep and
//! viewer paths is how a codebase ends up with three different curved-wall
//! resolutions.
//!
//! # Scope
//!
//! - Curve → polyline, surface → triangle grid, B-rep face → triangles
//! - Chord-height tolerance as the primary control (not segment count)
//! - Watertightness across shared edges: adjacent faces must agree on the
//!   discretisation of the edge between them, or the result leaks
//!
//! # The invariant that matters
//!
//! **Shared edges tessellate identically from both sides.** Tessellating each
//! face independently is the classic source of cracks in a "watertight" solid,
//! and a leaking solid silently corrupts every volume and clash result
//! downstream.

use geom_mesh::TriMesh;

pub struct ChordTolerance(pub f64);

impl Default for ChordTolerance {
    /// 1 mm expressed in metres — a reasonable default for building-scale work.
    fn default() -> Self {
        Self(1e-3)
    }
}

/// The bridge from exact geometry to discrete geometry.
///
/// Anything that can describe itself exactly must be able to lower itself to a
/// mesh, because a mesh is what every backend consumes. Implementing this trait
/// is how a future analytic solid joins the existing pipeline without changing
/// a single call site.
pub trait Tessellate {
    /// Approximate `self` as a triangle mesh within `tol`.
    fn tessellate(&self, tol: ChordTolerance) -> TriMesh;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_chord_tolerance_is_one_millimetre_in_metres() {
        assert_eq!(ChordTolerance::default().0, 1e-3);
    }

    /// The trait must be object-safe: the pipeline stores heterogeneous
    /// tessellable things behind `dyn`, exactly as it stores backends.
    #[test]
    fn tessellate_is_object_safe() {
        struct Unit;
        impl Tessellate for Unit {
            fn tessellate(&self, _tol: ChordTolerance) -> TriMesh {
                TriMesh::default()
            }
        }
        let boxed: Box<dyn Tessellate> = Box::new(Unit);
        assert_eq!(
            boxed.tessellate(ChordTolerance::default()).triangle_count(),
            0
        );
    }
}
