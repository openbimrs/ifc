//! Events and their stated times.
//!
//! ## Internal split
//!
//! - `definition.rs`: `IfcEvent` and `IfcEventTime`.

mod definition;

pub use definition::{events, slot as event_slot, time_slot as event_time_slot, Event, EventTime};
