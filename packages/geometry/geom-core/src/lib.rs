//! `geom-core` — geometry **data**, and nothing else.
//!
//! This crate is the root of the graph. It holds the types every other crate
//! passes around: points, transforms, boxes, and the tolerance policy. It
//! contains **no algorithms** and **no backend code** — those live in
//! `geom-kernel` (the trait contract plus its hardware backends). Meshes live
//! in `geom-mesh`, exact topology in `geom-brep`.
//!
//! # Why data is split from algorithms
//!
//! The backends (`scalar`, `simd`, `gpu`) must all speak the same vocabulary or
//! they cannot be swapped for one another. If `Vec3` lived in the scalar
//! backend, every other backend would depend on it just to name its input.
//! Keeping data here makes the backends true siblings.
//!
//! # Invariants (learned from the sibling `../vendor/solibri` kernel)
//!
//! 1. **No rendering types.** No colour, no material, no presentation flag. A
//!    geometry kernel that carries `getColorBuffer()` on its base type cannot be
//!    refactored without touching a renderer.
//! 2. **No serialization derives.** Persisting geometry belongs to a codec
//!    crate. Derived `Serialize` on kernel types freezes the layout forever.
//! 3. **Tolerance is a parameter, never a global.** BIM data arrives in
//!    millimetres *and* metres; a file-scope `1e-9` is wrong in one of them.
//! 4. **`f64` storage.** IFC site coordinates routinely exceed `f32` precision.
//!    Backends may compute in `f32` internally where they prove it is safe.
//! 5. **Dirty geometry is a state, not an error.** Real IFC meshes are
//!    frequently non-manifold; the type system must be able to represent that
//!    without panicking.

pub mod primitives;
pub mod scalar;

pub use primitives::{Aabb, Mat4, Vec3};
pub use scalar::Tolerance;
