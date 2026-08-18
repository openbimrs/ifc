//! `geom-brep` — boundary representation: **exact** topology.
//!
//! # Why a B-rep layer exists at all
//!
//! [`geom_mesh::TriMesh`] is a triangle soup with indices: cheap, universal,
//! and **lossy**. A cylinder becomes 64 flat facets; the fact that it *was* a
//! cylinder is gone. That loss is acceptable for clash detection and rendering
//! and unacceptable for three things IFC actually requires:
//!
//! 1. **Exact quantities.** An IFC quantity takeoff wants the true lateral area
//!    of a curved wall, not the sum of its facets, which is short by a factor
//!    that depends on tessellation density.
//! 2. **Round-tripping.** Reading an `IfcExtrudedAreaSolid` and writing it back
//!    should not silently convert it to a tessellated blob.
//! 3. **Robust booleans on curved input.** Cutting a circular duct opening from
//!    a wall is exact on the analytic surface and approximate on the facets.
//!
//! This is precisely the capability OpenCascade provides to IfcOpenShell, and
//! the reason IfcOpenShell carries that dependency. Building a lean B-rep is the
//! hard, honest core of being a real alternative rather than a mesh viewer.
//!
//! # Scope discipline (what keeps this lightweight)
//!
//! A full CAD B-rep kernel — NURBS, blends, fillets, surface-surface
//! intersection in the general case — is a multi-year effort and is **not** the
//! goal. IFC's geometry is overwhelmingly: extrusions of 2D profiles, revolves,
//! half-space clips, CSG of those, B-rep with planar faces, and tessellation.
//! Analytic surfaces beyond plane/cylinder/cone/sphere/torus are rare in real
//! files. Supporting *that* set exactly, and lowering everything else to
//! [`geom_mesh::TriMesh`], is the tractable path.
//!
//! # Status
//!
//! Reserved. No topology types are implemented yet — the current pipeline is
//! mesh-only, and [`Tessellate`] is the seam where a future B-rep enters. See
//! `docs/ROADMAP.md` Stage 4. This module documents intent so the capability is
//! scheduled rather than silently dropped.

use geom_mesh::TriMesh;

/// Tessellation tolerance: the maximum permitted deviation between the exact
/// surface and its triangle approximation, in model length units.
///
/// This is the single knob that trades mesh size against fidelity. It is a
/// parameter rather than a constant because BIM models arrive in millimetres
/// *and* metres — a fixed value is wrong in one of them.
#[derive(Debug, Clone, Copy, PartialEq)]
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
