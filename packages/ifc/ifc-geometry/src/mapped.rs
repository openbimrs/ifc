//! `IfcMappedItem` instancing.
//!
//! A mapped item reuses one representation at many placements -- the mechanism
//! behind repeated windows, doors and furniture. Lowering it naively duplicates
//! geometry per instance and is a major memory cost on large models, so the
//! source representation is lowered once and cached.
//!
//! # Pitfall
//!
//! Mapped items nest, and malformed files contain cycles (see the
//! `nested_mapped_item_cycle` fixture). Depth must be bounded.
//!
//! Not yet implemented -- see `docs/ROADMAP.md`.
