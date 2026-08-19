//! `ifc-model` — the IFC entity graph, free of both domain semantics and
//! serialization.
//!
//! # The two separations this crate exists to enforce
//!
//! **1. Model vs. domain meaning.** [`Model`] stores entities. It does not know
//! what a cost item, a task, a wall, or a material *is*. Domain crates
//! (`ifc-cost`, `ifc-schedule`, `ifc-properties`, ...) borrow a `&Model` and
//! interpret it. Consequences:
//!
//! - a thin application compiles only the domains it uses;
//! - **data we do not understand still round-trips perfectly**, because it is
//!   stored structurally rather than as a domain struct. A file full of cost
//!   entities parses and re-exports intact in a build with no cost crate at
//!   all. This is verified by `tests/roundtrip.rs`.
//!
//! **2. Model vs. serialization.** [`Model`] is not "the STEP model". STEP,
//! ifcXML and a prospective IFC-JSON are encodings of the same graph, and each
//! is a separate crate implementing [`codec::Codec`]. Nothing here depends on
//! any of them — which is why format conversion is just "read with one, write
//! with another".
//!
//! ```text
//!   ifc-step ──┐                        ┌── ifc-cost
//!   ifc-xml  ──┼── Codec ──> Model <────┼── ifc-schedule      (views)
//!   ifc-json ──┘   (this crate)         └── ifc-properties
//! ```
//!
//! # Modules
//!
//! | Module | Role |
//! | --- | --- |
//! | [`value`] | The serialization-independent value model |
//! | [`entity`] | Type name plus positional attributes |
//! | [`model`] | Storage, ordering, type index, reference integrity |
//! | [`header`] | File metadata and the declared schema token |
//! | [`codec`] | The read/write trait every serialization implements |
//! | [`guid`] | IFC's base-64 GlobalId encoding |
//! | `index` | Derived indices: inverse references |
//! | `relation` | Structural relationship traversal (no domain meaning) |
//! | `spatial` | The spatial containment tree |
//! | `traverse` | Graph walks over references |
//! | [`error`] | Failure modes |

pub mod codec;
pub mod entity;
pub mod error;
pub mod guid;
pub mod header;
mod index;
pub mod model;
mod mutation;
mod provenance;
mod relation;
mod spatial;
mod traverse;
pub mod value;

pub use codec::Codec;
pub use entity::Entity;
pub use error::{ModelError, ModelResult};
pub use header::Header;
pub use model::Model;
pub use value::{EntityId, Value};
