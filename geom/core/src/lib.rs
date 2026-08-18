//! `geom-core` — geometry **data**, and nothing else.
//!
//! This crate is the root of the graph. It holds the types every other crate
//! passes around: points, transforms, boxes, triangle meshes, and the
//! tolerance policy. It contains **no algorithms** and **no backend code** —
//! those live in the `geom-kernel` crate (the trait contract) and the backend
//! crates that implement it.
//!
//! # Why data is split from algorithms
//!
//! The backends (`geom-cpu`, `geom-simd`, `geom-gpu`) must all speak the same
//! vocabulary or they cannot be swapped for one another. If `TriMesh` lived in
//! the CPU backend, every other backend would depend on the CPU backend just to
//! name its input. Keeping data here makes the backends true siblings.
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

pub mod mesh;
pub mod primitives;
pub mod scalar;

pub use mesh::TriMesh;
pub use primitives::{Aabb, Mat4, Vec3};
pub use scalar::Tolerance;
