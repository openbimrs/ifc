//! `ifc-validate` -- Schema and model validation: is this file actually legal IFC?
//!
//!
//! Split from parsing on purpose. A reader that rejects everything imperfect is
//! useless on real data -- roughly half of production files violate something --
//! so parsing is permissive and validation is an explicit, separate pass.
//!
//! The `test/fixtures/ifcopenshell-validate/` corpus is named `pass-*` and
//! `fail-*` precisely to drive this crate.
//!
//! # Module map
//!
//! | Module | Role |
//! |---|---|
//! | [`header`] | Header well-formedness and schema declaration checks |
//! | [`cardinality`] | Required attributes present, aggregate sizes in range |
//! | [`type_check`] | Attribute values match their declared EXPRESS types |
//! | [`where_rule`] | EXPRESS `WHERE` rules and the 2 global rules in IFC4 |
//! | [`uniqueness`] | GlobalId uniqueness and duplicate detection |
//! | [`mod@reference`] | Dangling references and orphaned entities |
//! | [`report`] | Structured findings: severity, entity, rule, message |
//! | [`error`] | Why validation could not run |
//!
//! # Status
//!
//! Scaffold -- modules are reserved with intent, not implemented. See
//! `docs/ROADMAP.md` for the stage that fills them.

pub mod cardinality;
pub mod error;
pub mod header;
pub mod reference;
pub mod report;
pub mod type_check;
pub mod uniqueness;
pub mod where_rule;
