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
//!
//! Attributes are read and written by position or by name
//! ([`IfcModel::attribute_by_name`]); a name resolves against the release
//! the header declares, inherited attributes included, and is refused with
//! `unknown-attribute` (or `derived-attribute` for a write to a `*` slot)
//! rather than guessed.
//!
//! Entities are created by type and named attributes (#330, feature
//! `author`), alone or with the spatial, product, type and placement
//! builders in one checked batch ([`IfcModel::author`]); a refused batch
//! leaves the model unchanged.
//!
//! Geometry crosses at two levels (#328, ADR 0021): each product's world
//! placement and selected Body representation ([`IfcModel::product_placements`],
//! feature `placements`, default), and triangle meshes from the reference
//! backend ([`IfcModel::product_meshes`], feature `mesh`, opt-in because it
//! links an execution provider). A product that cannot be placed or
//! meshed is a record with a typed refusal, never a failed call.
//!
//! Property edits check `Pset_`/`Qto_` sets against the PSD/QTO catalog
//! ([`catalog`]): embedded with `property-catalog` (default), or loaded at
//! runtime from pinned snapshot files with `property-catalog-runtime`, the
//! npm package's choice, where a write before loading refuses with
//! `catalog-not-loaded`.

mod attribute;
pub mod authoring;
pub mod catalog;
pub mod classification;
pub mod cost;
mod error;
pub mod geometry;
pub mod georef;
pub mod header;
pub mod material;
mod model;
mod options;
pub mod properties;
pub mod property_edit;
pub mod record;
pub mod spatial;
pub mod systems;
pub mod unreachable;
pub mod validation;
pub mod value;
mod xml;

pub use attribute::AttributeInfo;
pub use authoring::{AuthorOp, AuthoringResult};
pub use error::BindingError;
pub use geometry::{GeometryRefusal, ProductMesh, ProductPlacement, SelectedRepresentation};
pub use model::IfcModel;
pub use options::{OnMalformed, ParseOptions};
pub use record::{Field, Record, ToRecord};
pub use unreachable::UnreachableProduct;
pub use validation::{ValidationFinding, ValidationReport, ValidationSummary};
