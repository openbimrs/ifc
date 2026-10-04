//! `ifc-geometry` — the IFC side of geometry.
//!
//! # What this crate is
//!
//! It answers *"what does this IFC entity mean geometrically"* and lowers
//! implemented slices into the format-neutral `axiolid-model` DAG. It does not
//! triangulate, evaluate NURBS, or perform booleans itself.
//!
//! The opt-in `compile` feature is the one narrow exception: it hands the
//! lowered DAG to an Axiolid provider and returns triangles. It is off by
//! default because selecting a provider and a memory budget is application
//! policy, not something the IFC file determines. See the `compile` module
//! and ADR 0004.
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
//! **Honest lowering.** The dispatcher's `IMPLEMENTED` list (in
//! `lower::dispatch`) names every representation-item type it lowers: swept,
//! CSG, boolean and half-space solids, faceted and advanced B-reps,
//! tessellated face sets, surfaces, curves, surface models, collections and
//! mapped items. Every other concrete IFC4 representation item has a
//! recorded disposition in `data/ifc4-representation-item-dispositions.tsv`
//! (nested input, non-shape, or typed refusal). IFC4X3 ADD2 adds 17
//! representation items: `IfcCurveSegment`, `IfcGradientCurve`,
//! `IfcDirectrixDerivedReferenceSweptAreaSolid` and
//! `IfcTriangulatedIrregularNetwork` lower (each with named refused forms in
//! `PARTIAL`), and the rest are in `PLANNED` with a named reason. Input that
//! cannot be lowered
//! exactly returns a typed [`crate::GeometryError`], such as
//! [`crate::GeometryError::Unsupported`], rather than panicking or
//! substituting approximate geometry.
//!
//! **Neutral DAG output.** Implemented lowerers resolve IFC units, placements,
//! profiles, and representation relationships into `axiolid-model` nodes. Active
//! lowering owns no duplicate geometry types and never selects a CPU/GPU
//! provider.
//!
//! **Feature `lowering`** (default on) carries the neutral geometry crates.
//! Without it this crate is representation selection only -- contexts,
//! plan/body choice, profiles, curves, surfaces, solids, units and
//! placements -- and links no geometry code at all.

pub mod authoring;
#[cfg(feature = "compile")]
pub mod compile;
pub mod constraint;
pub mod curve;
pub mod error;
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
pub use slots::Slots;
pub use transform::Transform;
pub use units::UnitScale;
mod input;

// Representation contexts and selection policy. Public because drawing
// production is a first-class consumer: choosing the geometry a plan is drawn
// from is a question about contexts, not about lowering.
pub use input::context::{
    all_contexts, context_of, plan_contexts, product_representation_frame, RepresentationContext,
    TargetView,
};
// The evaluator-taking forms (#353): opt-in with the other providers.
#[cfg(feature = "compile")]
pub use constraint::placement::{product_world_transform_with_evaluator, CachedPositionPolicy};
#[cfg(feature = "compile")]
pub use input::context::product_representation_frame_with_evaluator;
// Geometry-shaping material inputs only. Material identity, quantities, and
// association policy remain owned by `ifc-material`.
pub use input::material_usage::{
    CardinalPoint, DirectionSense, LayerSetDirection, MaterialLayerSetUsageGeometry,
    MaterialProfileGeometry, MaterialProfileSetUsageGeometry,
    MaterialProfileSetUsageTaperingGeometry,
};
pub use input::representation::{
    select_plan_representation, select_product_representation, select_shape_representation,
    ProductShape, Representation, RepresentationPurpose, PLAN_IDENTIFIERS, SOLID_IDENTIFIERS,
};

// Which entities carry a shape at all. Kernel-free: a slot read, not a lowering
// question, so a 2D or auditing consumer reaches it without linking a kernel.
pub use input::product::geometric_products;

// Which openings void a host (`IfcRelVoidsElement`). Kernel-free for the same
// reason; `lower::lower_product_net` turns the answer into subtractions.
pub use input::openings::{openings_of, voiding_conflicts, VoidingConflict};

// How a body is modelled -- kind, swept-solid profile parameters, direction
// and depth -- in SI and world coordinates. Kernel-free: a rule check asking
// "is this beam an HEA300" must not link a solid kernel for the answer.
pub use input::body::{
    body_description, BodyDescription, BodyItem, BodyKind, SweepPath, SweptSolid,
};
// Profile families read into SI parameters. The same reader feeds
// `lower::profile`, so a description and a lowering cannot disagree.
pub use input::profile::{
    describe_profile, profile_outline, ProfileDescription, ProfileOperator, ProfileOutline,
    ProfileParameters, ProfilePosition,
};
