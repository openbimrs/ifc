//! Host-independent core of the IFC language bindings (ADR 0013, #34).
//!
//! The WebAssembly, C and Python bindings each wrap [`IfcModel`] and move
//! [`value::Tagged`] values into and out of their host. The operations, the
//! lossless value encoding and the error codes live here once, so the three
//! hosts cannot drift apart: a fix or a new operation lands in one place and
//! every binding exposes it.
//!
//! This crate adds no IFC behaviour of its own. It is a thin, host-shaped
//! view of `openbim-ifc`; if a binding needs more, the facade grows first.
//!
//! # Features
//!
//! Beyond the release features (`ifc2x3` ... `ifc4x3`, #112), three
//! default features select facade capabilities that add to a browser
//! build's size: `ifcxml` (the ifcXML codec), `validate` (schema
//! validation) and `unreachable` (the viewer reachability lint). An
//! operation whose feature is off still exists, so every host keeps one
//! surface, and refuses with `feature-disabled`.

pub mod classification;
pub mod cost;
mod error;
pub mod georef;
pub mod header;
pub mod material;
mod model;
mod options;
pub mod properties;
pub mod record;
pub mod spatial;
pub mod systems;
pub mod unreachable;
pub mod validation;
pub mod value;
mod xml;

pub use error::BindingError;
pub use model::IfcModel;
pub use options::{OnMalformed, ParseOptions};
pub use record::{Field, Record, ToRecord};
pub use unreachable::UnreachableProduct;
pub use validation::{ValidationFinding, ValidationReport, ValidationSummary};
