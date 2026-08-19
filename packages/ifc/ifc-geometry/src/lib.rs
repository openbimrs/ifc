//! `ifc-geometry` — the IFC side of geometry.
//!
//! # What this crate is
//!
//! It answers *"what does this IFC entity mean geometrically"* and emits
//! kernel-neutral work orders. It does not triangulate, does not evaluate
//! NURBS, and does not perform booleans. Those belong to a geometry kernel,
//! which this crate only ever depends on as a **trait**.
//!
//! ```text
//!   ifc-model            this crate                     geometry kernel
//!   (untyped graph) -->  typed views  -->  requests -->  (implemented
//!                        + resolution                     elsewhere)
//! ```
//!
//! # Scope
//!
//! The three IFC geometry resource schemas, counted from IFC4 ADD2 TC1:
//!
//! | Schema | Entities | Types | Functions |
//! | --- | ---: | ---: | ---: |
//! | `IfcGeometryResource` | 59 | 14 | 25 |
//! | `IfcGeometricModelResource` | 42 | 4 | 2 |
//! | `IfcGeometricConstraintResource` | 11 | 5 | 1 |
//!
//! # Design
//!
//! **Typed views over borrowed entities.** Every geometry entity gets a
//! newtype wrapping `(EntityId, &Entity)` with named accessors and `mod slot`
//! constants citing the EXPRESS declaration. Views own nothing, so
//! constructing one is free and the model stays the single source of truth.
//!
//! **Total lowering.** Every unhandled case returns a typed
//! [`GeometryError::Unsupported`] naming the entity. Nothing panics and
//! nothing silently substitutes wrong geometry — a missing shape is
//! recoverable, a wrong one is not.
//!
//! **The kernel is demanded, not provided.** This crate states the capability
//! surface a geometry backend must satisfy. See [`kernel::Primitive`].

pub mod error;
pub mod kernel;
pub mod slots;
pub mod transform;
pub mod units;

pub use error::{GeometryError, GeometryResult};
pub use kernel::{BooleanOp, CsgShape, Primitive, Profile};
pub use slots::Slots;
pub use transform::Transform;
pub use units::UnitScale;
