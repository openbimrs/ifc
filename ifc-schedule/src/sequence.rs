//! Task sequencing: predecessor/successor links, lag, and cycles.
//!
//! ## Internal split
//!
//! - `relation.rs`: `IfcRelSequence`, `IfcLagTime`, and the bounded graph walk.
//! - `lag.rs`, `graph.rs`: planned owners, kept for when lag arithmetic and
//!   graph algorithms outgrow the relation reader.

mod graph;
mod lag;
mod relation;

pub use relation::{
    downstream_of, find_cycle, predecessors_of, sequences, successors_of, Lag, Sequence,
    SequenceCycle, SequenceType, MAX_SEQUENCE_DEPTH,
};
