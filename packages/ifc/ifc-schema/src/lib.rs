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
//! | [`express`] | Parser for the official `.exp` files |
//! | [`entity`] | Entity descriptors: name, supertype, slots |
//! | [`attribute`] | Attribute descriptors and declared types |
//! | [`types`] | Defined types, enumerations, selects |
//! | [`registry`] | The assembled, queryable schema |
//! | `inheritance` | Supertype-chain walking |
//!
//! # Relationship to the model
//!
//! [`ifc_model`](https://docs.rs/ifc-model) does **not** depend on this crate.
//! The model stores whatever a file contains, valid or not. The schema is what
//! you consult to *interpret* what was stored, and it is optional: a file whose
//! schema is unknown still parses, and its entities still round-trip.
//!
//! ```
//! use ifc_schema::Schema;
//!
//! let schema = Schema::from_express(
//!     "SCHEMA IFC4;\n\
//!      ENTITY IfcRoot; GlobalId : IfcGloballyUniqueId; END_ENTITY;\n\
//!      ENTITY IfcWall SUBTYPE OF (IfcRoot); Name : IfcLabel; END_ENTITY;\n\
//!      END_SCHEMA;",
//! );
//!
//! assert!(schema.is_a("IFCWALL", "IfcRoot"));
//! assert_eq!(schema.attribute_names("IfcWall"), ["GlobalId", "Name"]);
//! ```

pub mod attribute;
pub mod entity;
pub mod express;
mod inheritance;
pub mod registry;
pub mod types;
pub mod version;

pub use attribute::Attribute;
pub use entity::EntityDef;
pub use registry::Schema;
pub use types::{TypeDef, TypeKind};
pub use version::SchemaVersion;
