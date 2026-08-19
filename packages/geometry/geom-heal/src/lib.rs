//! `geom-heal` - diagnose and repair dirty geometry.
//!
//! # Why this is its own crate, and why it is bigger than you expect
//!
//! In OpenCascade, shape healing (`ShapeFix`, `ShapeAnalysis`, `ShapeUpgrade`,
//! `ShapeExtend`, `ShapeBuild`) is **70,671 lines** - the single largest
//! category of the 420k an IFC pipeline touches, larger than boolean and
//! intersection combined. That is not incidental complexity. It is what happens
//! when a kernel must ingest thirty years of files produced by dozens of
//! authoring tools with incompatible tolerance conventions.
//!
//! Any project claiming to replace OpenCascade must say what it does about
//! healing. Ours: **isolate it, scope it to observed failures, and never let it
//! run implicitly.**
//!
//! # Why healing is separated from the kernel
//!
//! A boolean operation that silently repairs its input is untestable - you
//! cannot tell a robust algorithm from a lucky repair. Keeping healing in its
//! own crate means:
//!
//! - the kernel's contract stays honest: manifold in, manifold out, explicit
//!   error otherwise;
//! - a caller chooses whether to repair, and pays only if it does;
//! - a repair is a visible, logged, testable transformation of the data rather
//!   than a hidden side effect.
//!
//! # Scope discipline - the anti-70k rule
//!
//! We implement a repair **only when a fixture demonstrates the failure**. The
//! OCCT healing suite is large because it is general; ours stays small because
//! it is evidence-driven. Every [`Repair`] variant below must trace to a real
//! broken file, and the crate documents which one.
//!
//! # Diagnosis before repair
//!
//! [`Diagnosis`] is separate from repair on purpose. Reporting *why* a shape is
//! dirty is independently valuable - it is what lets a user fix the source
//! model rather than shipping a silently patched derivative - and it is how we
//! decide, from real data, which repairs are worth writing.
//!
//! Not yet implemented - see `docs/ROADMAP.md`.

/// A specific defect found in a shape.
///
/// Ordered roughly by how often it appears in real BIM data.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Defect {
    /// An edge shared by other than exactly two faces. The classic
    /// boolean-breaking defect.
    NonManifoldEdge,
    /// Adjacent faces wind in opposite directions, so the normal flips across
    /// the shared edge.
    InconsistentOrientation,
    /// Vertices closer together than the working tolerance, usually from
    /// unit conversion or a lossy export.
    DuplicateVertex,
    /// A face whose area is zero within tolerance.
    DegenerateFace,
    /// A shell that does not close, leaving the solid with a hole.
    OpenShell,
    /// Two faces of the same solid that pass through each other.
    SelfIntersection,
}

/// What was found in a shape, without changing it.
///
/// A diagnosis is a *report*, so it is cheap, side-effect free, and safe to run
/// on everything during a validation pass.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Diagnosis {
    /// Every defect found, in discovery order.
    pub defects: Vec<Defect>,
}

impl Diagnosis {
    /// True when no defect was found.
    ///
    /// Note this says nothing about whether the shape is *useful* - an empty
    /// mesh is clean and worthless. Emptiness is the caller's business.
    pub fn is_clean(&self) -> bool {
        self.defects.is_empty()
    }

    /// True when a defect is present that will break a mesh boolean.
    ///
    /// Not every defect is fatal: a duplicate vertex is cosmetic, a
    /// non-manifold edge is not. This is the predicate that decides whether
    /// healing is *required* rather than merely nice.
    pub fn blocks_boolean(&self) -> bool {
        self.defects.iter().any(|d| {
            matches!(
                d,
                Defect::NonManifoldEdge
                    | Defect::InconsistentOrientation
                    | Defect::OpenShell
                    | Defect::SelfIntersection
            )
        })
    }
}

/// A repair the caller explicitly opts into.
///
/// There is deliberately no `Repair::All`. "Fix everything" is how a healing
/// suite grows to 70,000 lines and how a pipeline starts silently changing
/// geometry the user never asked it to change.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Repair {
    /// Merge vertices within tolerance and re-index the triangles.
    WeldVertices,
    /// Flip faces so all normals agree with the dominant orientation.
    UnifyOrientation,
    /// Drop faces whose area is zero within tolerance.
    DropDegenerateFaces,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_clean_diagnosis_blocks_nothing() {
        let d = Diagnosis::default();
        assert!(d.is_clean());
        assert!(!d.blocks_boolean());
    }

    #[test]
    fn cosmetic_defects_do_not_block_a_boolean() {
        let d = Diagnosis {
            defects: vec![Defect::DuplicateVertex, Defect::DegenerateFace],
        };
        assert!(!d.is_clean());
        assert!(!d.blocks_boolean());
    }

    #[test]
    fn non_manifold_edges_block_a_boolean() {
        let d = Diagnosis {
            defects: vec![Defect::NonManifoldEdge],
        };
        assert!(d.blocks_boolean());
    }
}
