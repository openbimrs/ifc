//! `geom-primitive` - parametric primitive solids.
//!
//! # Why this is its own crate
//!
//! CSG trees bottom out in primitives. IFC has six that appear as explicit
//! entities - `IfcBlock`, `IfcSphere`, `IfcRightCircularCylinder`,
//! `IfcRightCircularCone`, `IfcRectangularPyramid`, and the half-space that
//! `IfcHalfSpaceSolid` needs - and every one of them is a closed-form shape
//! defined by three or four numbers.
//!
//! OpenCascade puts this in `BRepPrimAPI` (3,076 lines of the 420k an IFC
//! pipeline touches). It is small, self-contained, and has nothing to do with
//! sweeping a profile, so it is not `geom-sweep`'s job.
//!
//! # Why primitives deserve exact treatment
//!
//! A cylinder tessellated at the wrong resolution is the single most common
//! source of "the volume is slightly wrong" bugs in BIM. The primitive is exact
//! by construction; only the *tessellation* is approximate, and that decision
//! belongs to the caller through a chord tolerance rather than being baked in
//! here at an arbitrary segment count.
//!
//! # The half-space problem
//!
//! `IfcHalfSpaceSolid` is unbounded, and a mesh kernel cannot represent an
//! unbounded solid. The standard trick is to clip the half-space to the
//! bounding box of whatever it is cutting, inflated by a margin. That margin is
//! a correctness parameter, not a detail: too small and the cut misses,
//! too large and precision is wasted. It is therefore explicit in the API.
//!
//! Not yet implemented - see `docs/ROADMAP.md`.

/// The primitive solids a CSG tree can bottom out in.
///
/// Deliberately a closed enum: the set is fixed by the IFC standard and by
/// every other CAD format we might read. A format needing something else is
/// telling us it needs a sweep or a B-rep, not a new primitive.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Primitive {
    /// Axis-aligned box at the local origin, extending along +X, +Y, +Z.
    Block {
        /// Extent along local X.
        x_length: f64,
        /// Extent along local Y.
        y_length: f64,
        /// Extent along local Z.
        z_length: f64,
    },
    /// Sphere centred on the local origin.
    Sphere {
        /// Distance from centre to surface.
        radius: f64,
    },
    /// Cylinder rising along local +Z from the origin.
    Cylinder {
        /// Cross-section radius.
        radius: f64,
        /// Extent along local Z.
        height: f64,
    },
    /// Cone with its base on the local XY plane, apex on +Z.
    Cone {
        /// Base radius. The apex is a point, so there is no top radius.
        bottom_radius: f64,
        /// Extent along local Z.
        height: f64,
    },
    /// Rectangular pyramid, base on the local XY plane, apex on +Z.
    Pyramid {
        /// Base extent along local X.
        x_length: f64,
        /// Base extent along local Y.
        y_length: f64,
        /// Extent along local Z.
        height: f64,
    },
}

/// How far past a subject's bounds an unbounded half-space is extended before
/// it can participate in a mesh boolean.
///
/// Wrapping this in a type rather than passing a bare `f64` is deliberate: a
/// caller that forgets the parameter gets a compile error instead of a silently
/// wrong cut.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ClipMargin(pub f64);

impl ClipMargin {
    /// Extend by this multiple of the subject's diagonal.
    ///
    /// Relative rather than absolute because BIM models arrive in millimetres
    /// and in metres; a fixed 1000.0 is enormous in one and invisible in the
    /// other.
    pub fn relative(factor: f64) -> Self {
        Self(factor)
    }
}

impl Default for ClipMargin {
    /// 10% of the subject diagonal - comfortably clear of the subject without
    /// throwing away float precision.
    fn default() -> Self {
        Self(0.1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_clip_margin_is_relative_and_positive() {
        assert!(ClipMargin::default().0 > 0.0);
        assert_eq!(ClipMargin::relative(0.25).0, 0.25);
    }

    #[test]
    fn primitives_are_copy_so_csg_trees_can_share_them_freely() {
        let a = Primitive::Sphere { radius: 2.0 };
        let b = a;
        assert_eq!(a, b);
    }
}
