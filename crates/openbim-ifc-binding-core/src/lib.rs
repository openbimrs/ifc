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
//! Every read recognizes an ifcZIP archive by its ZIP signature and reads
//! the STEP or ifcXML model it holds within a decompression budget
//! ([`ifczip`], feature `zip`); [`IfcModel::write_ifczip`] writes one.
//!
//! Attributes are read and written by position or by name
//! ([`IfcModel::attribute_by_name`]); a name resolves against the release
//! the header declares, inherited attributes included, and is refused with
//! `unknown-attribute` (or `derived-attribute` for a write to a `*` slot)
//! rather than guessed. A by-name write also takes a plain host value
//! ([`Plain`], #342), coerced against the attribute's declared type
//! ([`coerce`]): a string to a label or an enumeration item, an integer to
//! an `INTEGER` or a `REAL`, refused with `type-mismatch` when it does not
//! fit and `ambiguous-value` when several SELECT members would take it.
//!
//! Property sets resolve per object ([`IfcModel::property_sets`]) or for
//! many objects in one pass ([`IfcModel::property_sets_many`], #358), which
//! validates the file's property relationships once rather than per object.
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

//! Entities are found by `GlobalId` ([`IfcModel::by_guid`], #463) through
//! an index built on the first lookup and rebuilt only after an edit; a
//! `GlobalId` several entities carry is refused with `invalid-model`, never
//! answered with one. [`new_guid`] draws a random `GlobalId` and
//! [`name_based_guid`] derives a reproducible one. An object's type, a
//! type's occurrences, an element's openings and an opening's fillings
//! and host ([`IfcModel::type_object`] and its siblings) read the
//! relationships the declared release states.
//!
//! Selectors (#464, [`selector`]) filter the model by class, attribute,
//! property, type, material, classification, location and group
//! ([`IfcModel::select`]); values are read and written by path
//! ([`IfcModel::get_value`], [`IfcModel::set_value`]). A query that does
//! not parse is refused with `invalid-selector` and the byte span of the
//! offending text ([`BindingError::span`]).
//!
//! Tables of values by path (#474, [`table`]): the entities a selector
//! selects exported as CSV, TSV or XLSX ([`IfcModel::export_table`]), and
//! a table applied back by `GlobalId` or id as one checked transaction
//! ([`IfcModel::import_table`]), with every refusal by row and column and
//! a dry run. XLSX needs feature `sheet-xlsx` (default; the browser
//! package leaves it out and refuses with `feature-disabled`).
//!
//! `INVERSE` attributes read by name too ([`inverse`], #462):
//! `attribute_by_name(wall, "IsDefinedBy")` follows the declared release's
//! `FOR` clause through a reverse index built on first use and rebuilt
//! only after an edit; [`IfcModel::inverse_references`] and
//! [`IfcModel::total_inverses`] list and count every referrer.

mod attribute;
pub mod authoring;
pub mod catalog;
pub mod classification;
pub mod coerce;
pub mod cost;
mod error;
pub mod geometry;
pub mod georef;
pub mod header;
pub mod ifczip;
pub mod inverse;
pub mod material;
mod model;
mod options;
pub mod properties;
pub mod property_edit;
pub mod record;
pub mod relations;
pub mod selector;
pub mod spatial;
pub mod systems;
pub mod table;
pub mod unreachable;
pub mod validation;
pub mod value;
mod xml;

pub use attribute::AttributeInfo;
pub use authoring::{
    AppendOptions, AuthorOp, AuthoringResult, CopiedEntity, CopyProductOptions, CopyReport,
    KeptEntity, PurgeOptions, RemovalReport, RemoveProductOptions, RemovedEntity,
};
pub use coerce::Plain;
pub use error::BindingError;
pub use geometry::curves::{ProductCurves, ProductPolyline};
pub use geometry::graph::{
    GeometryEncoding, ProductGeometry, GEOMETRY_FORMAT, GEOMETRY_FORMAT_VERSION,
};
pub use geometry::{
    GeometryRefusal, ProductMesh, ProductPlacement, SelectedRepresentation, SurfaceColour,
};
pub use inverse::{InverseAttributeInfo, InverseReference};
pub use model::IfcModel;
pub use options::{OnMalformed, ParseOptions};
pub use record::{Field, Record, ToRecord};
pub use relations::{name_based_guid, new_guid, DuplicateGlobalId};
pub use selector::{canonical_selector, Selected, SELECTOR_GRAMMAR_VERSION};
pub use table::{TableChange, TableImport, TableRefusal};
pub use unreachable::UnreachableProduct;
pub use validation::{ValidationFinding, ValidationReport, ValidationSummary};
