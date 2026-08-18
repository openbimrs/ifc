//! `#id` to dense-index resolution.
//!
//! File ids are sparse (`#1`, `#7`, `#4021`), which makes them a poor direct
//! index. This stage builds the sparse-to-dense map once so every downstream
//! consumer can use a compact `EntityId` and array-index into parallel storage
//! instead of hashing on every traversal.
//!
//! Not yet implemented — Stage 1 in `docs/ROADMAP.md`.

/// A dense, zero-based entity handle. Stable for the lifetime of a model.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct EntityId(pub u32);
