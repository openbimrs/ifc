//! Task sequencing: predecessor/successor links, lag, and cycles.
//!
//! ## Internal split
//!
//! - `relation.rs`: `IfcRelSequence`, `IfcLagTime`, and the bounded graph walk.

pub(crate) mod relation;

pub use relation::{
    downstream_of, find_cycle, lag_slot, predecessors_of, sequences, successors_of, Lag, Sequence,
    SequenceCycle, SequenceType, MAX_SEQUENCE_DEPTH,
};
