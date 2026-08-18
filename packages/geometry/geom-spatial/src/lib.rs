//! `geom-spatial` — acceleration structures.
//!
//! # Why this is its own crate
//!
//! Clash detection, ray casting, nearest-element search and spatial containment
//! all need the same thing: a way to avoid testing every pair. Building that
//! once, generic over the payload, keeps it out of both the boolean kernel and
//! the clash engine.
//!
//! # Scope
//!
//! - BVH (SAH-built) for static scenes, uniform grid for dense uniform data,
//!   octree where hierarchy matters
//! - Queries: ray, frustum, radius, k-nearest, overlapping-pairs
//! - Generic over payload — it indexes bounded things, not IFC elements
//!
//! # Why this is the right place for parallelism and SIMD
//!
//! Broad-phase overlap is wide, regular, branch-light work over `Aabb`s: the
//! shape of problem where SIMD and rayon actually pay. Topological work in the
//! boolean kernel is not. See `docs/adr/0002`.
