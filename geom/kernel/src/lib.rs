//! `geom-kernel` — the **contract**, not an implementation.
//!
//! This crate is the single most important boundary in the workspace. It
//! defines the traits a geometry kernel must satisfy, in terms of `geom-core`
//! data types. It contains no algorithms.
//!
//! # The two swaps this enables
//!
//! **1. Swap the whole geometry kernel.** `ifc/` depends on *these traits*,
//! never on `geom-cpu`/`geom-simd`/`geom-gpu`. If a better kernel is written
//! later — by us or by someone else — it implements [`MeshBoolean`] and friends
//! and the IFC side gains it with zero call-site change. This is the explicit
//! requirement: *"ifc package should be built in a way that we could swap the
//! geom package for a better one."*
//!
//! **2. Swap the hardware backend.** The same traits are implemented once per
//! execution strategy (scalar CPU, SIMD, GPU). The `geom-dispatch` crate picks
//! one at runtime from detected CPU features. Adding AVX-512 or a GPU path later means
//! adding an impl, not editing callers.
//!
//! # Why traits and not `#[cfg]`
//!
//! Conditional compilation would bake one hardware choice into the binary at
//! build time and make cross-backend differential testing impossible. With
//! traits, every backend can be built simultaneously and checked against the
//! scalar reference on the same input — which is how we will prove a SIMD or
//! GPU path is correct rather than asserting it.
//!
//! # Cost of this design (stated honestly)
//!
//! Dynamic dispatch costs an indirect call per operation. That is irrelevant
//! for coarse operations (a boolean over a mesh, a BVH query over a model) and
//! would be unacceptable per-triangle. **Trait granularity is therefore coarse
//! by design** — a backend receives whole meshes and whole batches, never one
//! triangle, so the indirect call is amortized across thousands of elements.

pub mod boolean;
pub mod capability;
pub mod error;

pub use boolean::{BooleanOp, MeshBoolean};
pub use capability::{Backend, Capabilities};
pub use error::{GeomError, GeomResult};
