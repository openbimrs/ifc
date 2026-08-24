//! `ifc-geometry` — the IFC side of geometry.
//!
//! # What this crate is
//!
//! It answers *"what does this IFC entity mean geometrically"* and lowers
//! implemented slices into the format-neutral `axiolid-model` DAG. It does not
//! triangulate, evaluate NURBS, perform booleans, or select execution providers.
//!
//! ```text
//!   ifc-model            this crate                    geometry package
//!   (untyped graph) -->  typed/family views  -->  GeometryGraph
//!                        + IFC resolution       (implemented elsewhere)
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
//! **Views and explicit inventory.** The 89 concrete entities are represented by
//! dedicated or shared subtype-aware borrowed views. The 23 abstract entities
//! are inheritance/inventory entries, not falsely presented as constructible
//! views. All 23 schema types are modeled.
//!
//! **Honest partial lowering.** Exact profiles and extrusion/revolution are
//! implemented vertical slices. Every other assigned declaration is tracked in
//! the support ledger; attempting an unimplemented lowering returns typed
//! [`crate::GeometryError::Unsupported`] rather than panicking or substituting
//! approximate geometry.
//!
//! **Neutral DAG output.** Implemented lowerers resolve IFC units, placements,
//! profiles, and representation relationships into `axiolid-model` nodes. Active
//! lowering owns no duplicate geometry types and never selects a CPU/GPU
//! provider. The legacy [`kernel`] namespace is retained only as a source-
//! compatibility shell for the pre-DAG public API. Neutral names that would
//! otherwise collide are exported explicitly as [`AnalyticPrimitive`],
//! [`ExactProfile`], and [`GeometryBooleanOperator`].

pub mod constraint;
pub mod curve;
pub mod error;
pub mod kernel;
pub mod lower;
pub mod resource;
pub mod rules;
pub mod select;
pub mod slots;
pub mod solid;
pub mod surface;
pub mod transform;
pub mod units;

pub use axiolid_model::BooleanOperator as GeometryBooleanOperator;
pub use axiolid_model::{GeometryGraph, GeometryNode, NodeId, SolidOperation};
pub use axiolid_primitive::Primitive as AnalyticPrimitive;
pub use axiolid_profile::Profile as ExactProfile;
pub use error::{GeometryError, GeometryResult};
pub use kernel::{BooleanOp, CsgShape, Primitive, Profile};
pub use slots::Slots;
pub use transform::Transform;
pub use units::UnitScale;
mod input;
