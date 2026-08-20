//! `IfcGeometryResource`: the primitives everything else is built from.
//!
//! Points, directions, placements and transformation operators are the leaves
//! of every geometry graph in an IFC file -- a swept solid, a B-rep face and a
//! grid axis all bottom out here. They are grouped in one module because they
//! share the two traps that make IFC geometry subtle:
//!
//! - **Dimension is per-instance**, not per-type: `IfcCartesianPoint` and
//!   `IfcDirection` are 2D or 3D depending on how many values the record
//!   holds, so nothing may pad silently.
//! - **Inherited attributes come first** in a STEP record, so a subtype's own
//!   attributes start after its supertype's. `Location` is slot 0 of every
//!   `IfcPlacement` subtype for that reason.
//!
//! Curves, surfaces and solids build on these and live in sibling modules.

pub mod axes;
pub mod direction;
pub mod functions;
pub mod mapped;
pub mod operator;
pub mod placement;
pub mod point;
pub mod topology;

pub use direction::{Direction, Vector};
pub use mapped::{MappedInstance, MappedItem, MappingWalker, RepresentationMap};
pub use operator::{
    CartesianTransformationOperator, CartesianTransformationOperator2D,
    CartesianTransformationOperator2DnonUniform, CartesianTransformationOperator3D,
    CartesianTransformationOperator3DnonUniform,
};
pub use placement::{
    axis_placement_transform, Axis1Placement, Axis2Placement2D, Axis2Placement3D, Placement,
};
pub use point::{
    cartesian_point_3d, CartesianPoint, CartesianPointList2D, CartesianPointList3D, PointOnCurve,
    PointOnSurface,
};
