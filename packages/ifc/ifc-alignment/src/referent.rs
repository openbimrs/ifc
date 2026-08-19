//! `IfcReferent` stationing and chainage.
//!
//! Stationing is not arc length: it restarts at equations and can run backwards.
//! Treating them as interchangeable is the classic linear-referencing bug.
//!
//! Implementation is tracked in `../PLAN.md`.

//! ## Internal split
//!
//! - `station.rs`: station referents.

mod station;
