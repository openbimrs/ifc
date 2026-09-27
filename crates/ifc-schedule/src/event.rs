//! Events and their stated times.
//!
//! ## Internal split
//!
//! - `definition.rs`: `IfcEvent` and `IfcEventTime`.
//! - `time.rs`: planned owner for event time variants.

mod definition;
mod time;

pub use definition::{events, slot as event_slot, time_slot as event_time_slot, Event, EventTime};
