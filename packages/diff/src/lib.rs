//! `diff` — semantic difference between two IFC revisions.
//!
//! Not a text diff. Two exports of the same model from the same authoring tool
//! differ in entity numbering, ordering, and formatting while being semantically
//! identical, so a line-based diff reports everything and means nothing. This
//! crate matches elements by GUID and compares *meaning*: added, removed,
//! moved, and property-changed.
//!
//! # Status
//!
//! Reserved. See `../AGENTS.md` for the boundary and `../PLAN.md` for the
//! work queue.
