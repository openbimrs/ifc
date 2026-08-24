//! Legacy pre-DAG geometry request vocabulary.
//!
//! This module remains solely for source compatibility with the public API that
//! preceded the neutral `axiolid-model` graph. New code must lower IFC directly to
//! `axiolid_model::GeometryGraph` through [`crate::lower`]. These values are not
//! consumed by the new execution contracts and will not receive new geometry
//! families. They intentionally do not carry Rust's `#[deprecated]` attribute:
//! existing clients that deny warnings must keep compiling unchanged.
//!
//! The compatibility types preserve their original source shape, including
//! metre/radian units and absolute transforms. They intentionally stay isolated
//! from active lowering so early polygon approximation and recursive request
//! trees cannot leak back into the canonical representation.

use crate::transform::Transform;

/// A point in 3D, in metres.
pub type Point3 = [f64; 3];

/// A direction in 3D. Not guaranteed normalized; kernels should normalize.
pub type Vector3 = [f64; 3];

/// A closed 2D contour in a profile's own coordinate system.
///
/// Curved edges are already approximated into line segments by the caller when
/// a tolerance is supplied, so a kernel receives polygons rather than needing
/// its own curve evaluator for the common case.
#[derive(Debug, Clone, PartialEq)]
pub struct Contour {
    /// Ordered vertices. Not repeated at the end; closure is implicit.
    pub points: Vec<[f64; 2]>,
}

/// A profile: one outer boundary and any number of holes.
///
/// Holes are a separate field rather than a convention about winding order,
/// because IFC states them explicitly and inferring them is a common source of
/// wrong solids.
#[derive(Debug, Clone, PartialEq)]
pub struct Profile {
    /// The outer boundary.
    pub outer: Contour,
    /// Inner boundaries to be subtracted.
    pub inner: Vec<Contour>,
}

/// Boolean operators, matching `IfcBooleanOperator`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BooleanOp {
    /// Set union.
    Union,
    /// Set intersection.
    Intersection,
    /// First operand minus second.
    Difference,
}

/// A half space: everything on one side of a plane.
///
/// # The pitfall this type exists to make explicit
///
/// A half space is **infinite**. It is only meaningful as an operand to a
/// boolean, and a kernel cannot tessellate one on its own. When `bounded_by`
/// is present the region is additionally clipped, which is what makes it
/// usable in practice.
#[derive(Debug, Clone, PartialEq)]
pub struct HalfSpace {
    /// A point on the dividing plane.
    pub origin: Point3,
    /// The plane normal. Material lies on the side this points away from when
    /// `agreement` is false.
    pub normal: Vector3,
    /// Whether the solid is on the normal's side.
    pub agreement: bool,
    /// Optional bounding polygon, extruded perpendicular to its own plane.
    ///
    /// From `IfcPolygonalBoundedHalfSpace`: the boundary lies in the XY plane
    /// of the placement and the region extends along +Z.
    pub bounded_by: Option<Box<BoundedRegion>>,
}

/// A finite clipping region for an otherwise unbounded half space.
#[derive(Debug, Clone, PartialEq)]
pub struct BoundedRegion {
    /// The boundary polygon in the local XY plane.
    pub boundary: Contour,
    /// Placement of that plane in world space.
    pub placement: Transform,
}

/// A deprecated pre-DAG description of a solid request.
///
/// This closed enum preserves the old API shape only. It is not the complete IFC
/// geometry vocabulary and no current geometry provider consumes it.
#[derive(Debug, Clone, PartialEq)]
pub enum Primitive {
    /// Sweep a profile linearly. `IfcExtrudedAreaSolid`.
    Extrusion {
        /// Cross-section to sweep.
        profile: Profile,
        /// Sweep direction; need not be the local Z axis.
        direction: Vector3,
        /// Sweep distance in metres.
        depth: f64,
        /// Where the result sits in the world.
        placement: Transform,
    },

    /// Sweep a profile about an axis. `IfcRevolvedAreaSolid`.
    Revolution {
        /// Cross-section to sweep.
        profile: Profile,
        /// A point on the axis.
        axis_origin: Point3,
        /// The axis direction.
        axis_direction: Vector3,
        /// Sweep angle in radians.
        angle: f64,
        /// Where the result sits in the world.
        placement: Transform,
    },

    /// Sweep a circular disk along a curve. `IfcSweptDiskSolid`.
    DiskSweep {
        /// Polyline approximation of the spine.
        path: Vec<Point3>,
        /// Outer radius in metres.
        radius: f64,
        /// Optional inner radius, producing a pipe.
        inner_radius: Option<f64>,
        /// Where the result sits in the world.
        placement: Transform,
    },

    /// An explicit triangle mesh. `IfcTriangulatedFaceSet`, `IfcPolygonalFaceSet`.
    Mesh {
        /// Vertex positions in metres.
        positions: Vec<Point3>,
        /// Triangle indices, three per face.
        indices: Vec<u32>,
        /// Where the result sits in the world.
        placement: Transform,
    },

    /// A boundary representation from planar faces. `IfcFacetedBrep`.
    ///
    /// Faces may be non-planar or non-convex in malformed files; a kernel is
    /// expected to triangulate defensively rather than assume.
    Brep {
        /// Shells, each a list of faces, each a list of contours.
        ///
        /// The first contour of a face is its outer bound, the rest are holes.
        shells: Vec<Vec<Vec<Vec<Point3>>>>,
        /// Where the result sits in the world.
        placement: Transform,
    },

    /// An analytic primitive. `IfcCsgPrimitive3D` subtypes.
    Csg {
        /// Which primitive.
        shape: CsgShape,
        /// Where it sits in the world.
        placement: Transform,
    },

    /// A half space, only valid as a boolean operand.
    HalfSpace(HalfSpace),

    /// A boolean combination. `IfcBooleanResult`, `IfcBooleanClippingResult`.
    Boolean {
        /// The operator.
        op: BooleanOp,
        /// Left operand.
        first: Box<Primitive>,
        /// Right operand.
        second: Box<Primitive>,
    },

    /// A group of primitives treated as one. `IfcGeometricSet` etc.
    Group(Vec<Primitive>),
}

/// Analytic CSG primitives from `IfcGeometricModelResource`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CsgShape {
    /// `IfcBlock`: axis-aligned box from the placement origin.
    Block {
        /// Extent along local X.
        x: f64,
        /// Extent along local Y.
        y: f64,
        /// Extent along local Z.
        z: f64,
    },
    /// `IfcRightCircularCylinder`.
    Cylinder {
        /// Height along local Z.
        height: f64,
        /// Radius.
        radius: f64,
    },
    /// `IfcRightCircularCone`.
    Cone {
        /// Height along local Z.
        height: f64,
        /// Base radius.
        bottom_radius: f64,
    },
    /// `IfcSphere`.
    Sphere {
        /// Radius.
        radius: f64,
    },
    /// `IfcRectangularPyramid`.
    Pyramid {
        /// Base extent along local X.
        x: f64,
        /// Base extent along local Y.
        y: f64,
        /// Height along local Z.
        height: f64,
    },
}

impl Primitive {
    /// Does evaluating this require boolean support?
    ///
    /// Lets an application detect up front that a file needs a capability the
    /// compiled kernel lacks, rather than failing deep inside a walk.
    pub fn requires_boolean(&self) -> bool {
        match self {
            Self::Boolean { .. } | Self::HalfSpace(_) => true,
            Self::Group(items) => items.iter().any(Self::requires_boolean),
            _ => false,
        }
    }

    /// A short name for diagnostics.
    pub fn kind(&self) -> &'static str {
        match self {
            Self::Extrusion { .. } => "extrusion",
            Self::Revolution { .. } => "revolution",
            Self::DiskSweep { .. } => "disk sweep",
            Self::Mesh { .. } => "mesh",
            Self::Brep { .. } => "brep",
            Self::Csg { .. } => "csg primitive",
            Self::HalfSpace(_) => "half space",
            Self::Boolean { .. } => "boolean",
            Self::Group(_) => "group",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn square() -> Profile {
        Profile {
            outer: Contour {
                points: vec![[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]],
            },
            inner: vec![],
        }
    }

    fn extrusion() -> Primitive {
        Primitive::Extrusion {
            profile: square(),
            direction: [0.0, 0.0, 1.0],
            depth: 2.0,
            placement: Transform::identity(),
        }
    }

    #[test]
    fn plain_solids_need_no_boolean_kernel() {
        assert!(!extrusion().requires_boolean());
    }

    /// The capability check that lets an application fail early and clearly.
    #[test]
    fn booleans_and_half_spaces_declare_their_requirement() {
        let cut = Primitive::Boolean {
            op: BooleanOp::Difference,
            first: Box::new(extrusion()),
            second: Box::new(Primitive::HalfSpace(HalfSpace {
                origin: [0.0, 0.0, 0.0],
                normal: [0.0, 0.0, 1.0],
                agreement: true,
                bounded_by: None,
            })),
        };
        assert!(cut.requires_boolean());
        assert!(Primitive::Group(vec![cut]).requires_boolean());
    }

    /// Structural equality is what makes kernel-side caching possible.
    #[test]
    fn identical_requests_compare_equal() {
        assert_eq!(extrusion(), extrusion());
    }
}
