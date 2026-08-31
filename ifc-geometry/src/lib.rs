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
//! provider.
#![cfg_attr(
    feature = "lowering",
    doc = "The legacy [`kernel`] namespace is retained only as a source-
compatibility shell for the pre-DAG public API. Neutral names that would
otherwise collide are exported explicitly as [`AnalyticPrimitive`],
[`ExactProfile`], and [`GeometryBooleanOperator`]."
)]
//!
//! **Feature `lowering`** (default on) carries the neutral geometry crates.
//! Without it this crate is representation selection only -- contexts,
//! plan/body choice, profiles, curves, surfaces, solids, units and
//! placements -- and links no geometry code at all.

pub mod constraint;
pub mod curve;
pub mod error;
#[cfg(feature = "lowering")]
pub mod kernel;
#[cfg(feature = "lowering")]
pub mod lower;
pub mod resource;
pub mod rules;
pub mod select;
pub mod slots;
pub mod solid;
pub mod surface;
pub mod transform;
pub mod units;

// Neutral geometry vocabulary, re-exported so a lowering consumer needs only
// this crate in scope. Gated with the lowering it exists to serve.
#[cfg(feature = "lowering")]
pub use axiolid_model::BooleanOperator as GeometryBooleanOperator;
#[cfg(feature = "lowering")]
pub use axiolid_model::{GeometryGraph, GeometryNode, NodeId, SolidOperation};
#[cfg(feature = "lowering")]
pub use axiolid_primitive::Primitive as AnalyticPrimitive;
#[cfg(feature = "lowering")]
pub use axiolid_profile::Profile as ExactProfile;
// Placement resolution is the most-reused operation in any IFC consumer
// and the one most often reimplemented wrongly, so it is reachable from
// the crate root and does not require the `lowering` feature: a 2D drawing
// needs world coordinates without compiling a solid kernel.
pub use constraint::{product_world_transform, products_world_transforms};
pub use error::{GeometryError, GeometryResult};
#[cfg(feature = "lowering")]
pub use kernel::{BooleanOp, CsgShape, Primitive, Profile};
pub use slots::Slots;
pub use transform::Transform;
pub use units::UnitScale;
mod input;

// Representation contexts and selection policy. Public because drawing
// production is a first-class consumer: choosing the geometry a plan is drawn
// from is a question about contexts, not about lowering.
pub use input::context::{
    all_contexts, context_of, plan_contexts, RepresentationContext, TargetView,
};
pub use input::representation::{
    select_plan_representation, select_product_representation, select_shape_representation,
    ProductShape, Representation, RepresentationPurpose, PLAN_IDENTIFIERS, SOLID_IDENTIFIERS,
};

// Which entities carry a shape at all. Kernel-free: a slot read, not a lowering
// question, so a 2D or auditing consumer reaches it without linking a kernel.
pub use input::product::geometric_products;
