//! `clash` — clash detection between IFC elements.
//!
//! # Kernel-agnostic by construction
//!
//! Clash is the heaviest geometric consumer in the workspace, which makes it the
//! best proof that the kernel boundary is real: it depends on `geom-kernel` with
//! `default-features = false` and receives its backend by injection. Swap the
//! kernel and clash follows for free.
//!
//! # Design notes
//!
//! Broad phase (AABB/BVH) then narrow phase (triangle-triangle, or exact
//! surface intersection once `geom-brep` lands). The broad phase is where the
//! parallelism and the hardware backends pay off — it is a large batch of
//! independent overlap tests, which is exactly the shape SIMD and GPU want.
//!
//! Tolerance is a parameter, never a constant: "clash" versus "touching" versus
//! "clearance violation" differ only by tolerance, and BIM models arrive in both
//! millimetres and metres.
//!
//! # Status
//!
//! Reserved. See `docs/ROADMAP.md` Stage 5.
