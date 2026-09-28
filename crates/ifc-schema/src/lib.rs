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
//! | `inheritance` | Supertype-chain walking |
//!
//! # Relationship to the model
//!
//! `ifc-model` does **not** depend on this crate.
//! The model stores whatever a file contains, valid or not. The schema is what
//! you consult to *interpret* what was stored, and it is optional: a file whose
//! schema is unknown still parses, and its entities still round-trip.
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

#[cfg(feature = "ifc4")]
mod artifact;
pub mod attribute;
#[cfg(feature = "ifc4")]
mod bundled;
pub mod completeness;
pub mod entity;
#[cfg(feature = "ifc4")]
pub mod export;
#[cfg(feature = "express")]
mod express;
mod inheritance;
pub mod registry;
pub mod types;
pub mod version;

#[cfg(feature = "ifc4")]
pub use artifact::decode_schema as artifact_decode_schema;
#[cfg(feature = "generation")]
pub use artifact::encode_schema as artifact_encode_schema;
#[cfg(feature = "ifc4")]
pub use artifact::BundledSchemaError;
pub use attribute::Attribute;
#[cfg(feature = "ifc4")]
pub use bundled::{for_version, ifc2x3, ifc4, ifc4x1, ifc4x2, ifc4x3};
pub use entity::{EntityDef, WhereRule};
#[cfg(feature = "ifc4")]
pub use export::{
    write_direct_structural_catalog, write_structural_catalog, StructuralCatalogSummary,
};
pub use registry::Schema;
pub use types::{TypeDef, TypeKind};
pub use version::SchemaVersion;
