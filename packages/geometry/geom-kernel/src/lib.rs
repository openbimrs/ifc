//! `geom-kernel` — the geometry **contract**, plus the backends implementing it.
//!
//! This crate is the single most important boundary in the workspace. The
//! traits ([`MeshBoolean`], [`Capabilities`]) are defined in terms of
//! `geom-core`/`geom-mesh` data types and contain no algorithms; the
//! [`backend`] module holds the implementations, each behind a feature.
//!
//! # The two swaps this enables
//!
//! **1. Swap the whole geometry kernel.** `packages/ifc/` depends on these
//! traits with `default-features = false` — the contract with *no*
//! implementation compiled in. If a better kernel is written later, by us or by
//! someone else, it implements [`MeshBoolean`] and the IFC side gains it with
//! zero call-site change. This is the explicit project requirement: *"ifc
//! package should be built in a way that we could swap the geom package for a
//! better one."*
//!
//! **2. Swap the hardware backend.** The same traits are implemented once per
//! execution strategy — `backend::scalar`, `backend::simd`, `backend::gpu` —
//! and [`backend::Dispatcher`] picks one at runtime from detected CPU
//! features. Adding an AVX-512 or GPU path later means adding an
//! impl, not editing callers.
//!
//! # Why traits and not `#[cfg]`
//!
//! Conditional compilation would bake one hardware choice into the binary at
//! build time and make cross-backend differential testing impossible. With
//! traits, every available backend can be built simultaneously and checked
//! against the scalar reference on the same input — which is how we will prove
//! a SIMD or GPU path is correct rather than asserting it.
//!
//! # Cost of this design (stated honestly)
//!
//! Dynamic dispatch costs an indirect call per operation. That is irrelevant
//! for coarse operations (a boolean over a mesh, a BVH query over a model) and
//! would be unacceptable per-triangle. **Trait granularity is therefore coarse
//! by design** — a backend receives whole meshes and whole batches, never one
//! triangle, so the indirect call is amortized across thousands of elements.

pub mod backend;
pub mod boolean;
pub mod capability;
pub mod error;

pub use boolean::{BooleanOp, MeshBoolean};
pub use capability::{Backend, Capabilities};
pub use error::{GeomError, GeomResult};
