//! Events and their stated times.
//!
//! ## Internal split
//!
//! - `definition.rs`: `IfcEvent` and `IfcEventTime`.

mod definition;

#[allow(deprecated)]
pub use definition::events;
pub use definition::{
    read_events, slot as event_slot, time_slot as event_time_slot, Event, EventTime,
};
