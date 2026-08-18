//! Type buckets and reverse-reference indices.
//!
//! `all_of_type("IfcWall")` and 'who references me' must both be O(1)-ish; a
//! linear scan over millions of entities per query is the naive trap.
//!
//! Not yet implemented -- see `docs/ROADMAP.md`.
