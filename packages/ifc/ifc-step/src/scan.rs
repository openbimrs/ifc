//! Parallel body scan producing raw records.
//!
//! Consumes [`crate::partition::Partition`]s, emits one [`RawRecord`] per
//! `#id=TYPE(...)` statement. Runs under rayon; each partition is independent,
//! which is the whole point of aligning them.
//!
//! Not yet implemented — Stage 1 in `docs/ROADMAP.md`.

/// One entity instance as it appears in the file, before reference resolution.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RawRecord<'a> {
    /// The `#id` as written in the file. Sparse and not necessarily ordered.
    pub id: u64,
    /// Entity type name, uppercase as written (e.g. `IFCWALL`).
    pub type_name: &'a [u8],
    /// The still-unparsed attribute list between the outer parentheses.
    pub body: &'a [u8],
}
