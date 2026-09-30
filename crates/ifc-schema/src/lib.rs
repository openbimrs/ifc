//! `ifc-schema` — the IFC schema **as data**, not as 2,500 generated structs.
//!
//! # The decision
//!
//! IfcOpenShell generates a class per IFC entity per schema version. That is a
//! very large amount of code, and it must be regenerated for every schema
//! release. This crate instead reads the normative EXPRESS files into tables
//! and answers questions against them.
//!
//! The evidence that this is the right call: IFC4x3 renames
//! `IfcBuildingElement` to `IfcBuiltElement` and drops `IfcProxy` and the whole
//! `*StandardCase` family. Generated types would fork the entire API surface;
//! a table just holds different rows.
//!
//! # What this crate is for
//!
//! | Module | Role |
//! | --- | --- |
//! | [`version`] | Which schema a file declares |
//! | `express` | Parsing `.exp` source (opt-in `express` feature) |
//! | [`entity`] | Entity descriptors: name, supertype, slots |
//! | [`attribute`] | Attribute descriptors and declared types |
//! | [`types`] | Defined types, enumerations, selects |
//! | [`registry`] | The assembled, queryable schema |
//!
//! # Relationship to the model
//!
//! `ifc-model` does **not** depend on this crate.
//! The model stores whatever a file contains, valid or not. The schema is what
//! you consult to *interpret* what was stored, and it is optional: a file whose
//! schema is unknown still parses, and its entities still round-trip.
//!
//! # Features
//!
//! | Feature | Default | Provides |
//! | --- | --- | --- |
//! | `ifc2x3`, `ifc4`, `ifc4x1`, `ifc4x2`, `ifc4x3` | yes | That release's bundled table and accessor (`ifc2x3()`, ...) |
//! | `artifact` | via any release | The compiled-artifact decoder |
//! | `express` | no | `Schema::from_express` through `openbim-step` |
//! | `generation` | no | The artifact generator binary |
//!
//! A default build bundles every release. A build that turns defaults off
//! and names one release still recognises every [`SchemaVersion`], and
//! [`for_version`] refuses the others with [`NotBundled`] -- a typed error,
//! never a panic, and distinct from an unknown header token.
//!
//! # Owned declaration types
//!
//! [`EntityDef`], [`Attribute`], [`TypeDef`], [`TypeKind`] and [`WhereRule`]
//! belong to this crate and are `#[non_exhaustive]`. The bundled tables
//! decode straight into them without any parser crate; the `openbim-step`
//! EXPRESS extractor is linked only by the opt-in `express` feature (for
//! `Schema::from_express`) and by `generation`.
//!
//! ```
//! # #[cfg(feature = "ifc4")] {
//! let schema = ifc_schema::ifc4();
//! assert!(schema.is_a("IFCWALL", "IfcRoot"));
//! assert_eq!(
//!     &schema.attribute_names("IfcWall")[..2],
//!     ["GlobalId", "OwnerHistory"]
//! );
//! # }
//! ```

#[cfg(feature = "artifact")]
mod artifact;
pub mod attribute;
mod bundled;
pub mod completeness;
pub mod entity;
pub mod export;
#[cfg(feature = "express")]
mod express;
pub mod registry;
pub mod types;
pub mod version;

#[cfg(feature = "artifact")]
pub use artifact::decode_schema as artifact_decode_schema;
#[cfg(feature = "generation")]
pub use artifact::encode_schema as artifact_encode_schema;
#[cfg(feature = "artifact")]
pub use artifact::BundledSchemaError;
pub use attribute::{AggregateKind, Aggregation, Attribute, Bound};
#[cfg(feature = "ifc2x3")]
pub use bundled::ifc2x3;
#[cfg(feature = "ifc4")]
pub use bundled::ifc4;
#[cfg(feature = "ifc4x1")]
pub use bundled::ifc4x1;
#[cfg(feature = "ifc4x2")]
pub use bundled::ifc4x2;
#[cfg(feature = "ifc4x3")]
pub use bundled::ifc4x3;
pub use bundled::{for_version, NotBundled};
pub use entity::{EntityDef, InverseAttribute, UniqueRule, WhereRule};
pub use export::{
    write_direct_structural_catalog, write_structural_catalog, StructuralCatalogSummary,
};
pub use registry::Schema;
pub use types::{TypeDef, TypeKind};
pub use version::SchemaVersion;
