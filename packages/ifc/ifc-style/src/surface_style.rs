//! `IfcSurfaceStyle` shading, rendering, lighting, refraction.
//!
//!
//! Implementation is tracked in the adjacent `PLAN.md`.

//! ## Internal split
//!
//! - `shading.rs`: shading values.
//! - `rendering.rs`: rendering/reflection values.
//! - `lighting.rs`: lighting/refraction data.

mod lighting;
mod rendering;
mod shading;

mod refraction;
