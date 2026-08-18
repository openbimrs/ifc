//! Placement chains: local placement to world transform.
//!
//! `IfcLocalPlacement` nests: an element sits in a storey, which sits in a
//! building, which sits on a site. Resolving world position means walking that
//! chain and composing transforms.
//!
//! # Pitfall
//!
//! The chain can be deep and is walked for every element, so results must be
//! cached per placement rather than recomputed per shape. Cycles are malformed
//! but do occur in real files and must be detected, not stack-overflowed.
//!
//! Not yet implemented -- see `docs/ROADMAP.md`.
