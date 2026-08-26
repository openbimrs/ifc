//! `ifc-author` — schema-checked construction of IFC entities.
//!
//! # Why this is not part of `ifc-model`
//!
//! [`ifc_model::Model::push`] takes a type name and a positional
//! `Vec<Value>`. That is the right primitive for a codec, which is reproducing
//! a record it just parsed, and the wrong one for an application, which is
//! inventing a record and has no way to know it got the arity right.
//!
//! The fix is not typed setters on `Model`: the model is the L0 record core and
//! must not depend on the schema tables. Authoring is therefore an L2 concern
//! that borrows both. See
//! [ADR 0007](https://openbimrs.github.io/ifc/adr/0007-authoring-is-a-schema-layer-not-a-model-layer).
//!
//! # What is checked
//!
//! | Failure | Example |
//! | --- | --- |
//! | Unknown entity | `IfcAnnotaton` (typo) |
//! | Unknown attribute | `IfcAnnotation.Nmae` |
//! | Attribute set twice | two `Name` calls |
//! | Required attribute unset | `IfcAnnotation` with no `GlobalId` |
//! | Declared-type mismatch | a string where `IfcLengthMeasure` is declared |
//! | Scalar/aggregate confusion | a scalar where `LIST OF` is declared |
//! | Malformed GlobalId | not 22 characters in IFC's base-64 alphabet |
//!
//! Slot positions come from [`ifc_schema::Schema::attributes`], which returns
//! inherited attributes first — the ordering STEP records depend on.
//!
//! # What is not checked
//!
//! WHERE rules, inverse attributes, uniqueness, and cross-entity consistency.
//! Those need a whole model rather than one entity, and `ifc-validate` owns
//! them. This crate refuses obviously-wrong construction; it does not certify
//! a valid file.
//!
//! # Example
//!
//! ```
//! use ifc_author::EntityBuilder;
//! use ifc_model::Model;
//! use ifc_schema::Schema;
//!
//! let schema = Schema::from_express(
//!     "SCHEMA IFC4;\n\
//!      TYPE IfcGloballyUniqueId = STRING; END_TYPE;\n\
//!      TYPE IfcLabel = STRING; END_TYPE;\n\
//!      ENTITY IfcAnnotation;\n\
//!        GlobalId : IfcGloballyUniqueId;\n\
//!        Name : OPTIONAL IfcLabel;\n\
//!      END_ENTITY;\n\
//!      END_SCHEMA;",
//! );
//!
//! let mut model = Model::new();
//! let id = EntityBuilder::new(&schema, "IfcAnnotation")
//!     .text("GlobalId", "3vB2YO$MX4xv5uCqZZG05x")
//!     .text("Name", "Brandwand")
//!     .insert(&mut model)?;
//!
//! assert_eq!(model.get(id).unwrap().text(1), Some("Brandwand"));
//! # Ok::<(), ifc_author::AuthorError>(())
//! ```

mod builder;
mod check;
mod error;

pub use builder::EntityBuilder;
pub use error::{AuthorError, AuthorResult};
