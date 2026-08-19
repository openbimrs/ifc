//! `IfcGeometricConstraintResource`: object placement and grids.
//!
//! This schema answers "where is this element in the world". It is small (11
//! entities) and disproportionately important: nothing renders in the right
//! place without it.
//!
//! # The two placement kinds
//!
//! - [`local::LocalPlacement`] nests. An element sits in a storey, in a
//!   building, on a site, and world position is the composition of that chain.
//! - [`grid::GridPlacement`] positions relative to the intersection of two
//!   construction grid axes, which is how structural drawings are laid out.
//!
//! Both are `IfcObjectPlacement` subtypes, so a product's `ObjectPlacement`
//! may be either and consumers must handle both.

pub mod connection;
pub mod grid;
pub mod local;

pub use connection::ConnectionGeometry;
pub use grid::{GridAxis, GridPlacement, VirtualGridIntersection};
pub use local::LocalPlacement;
