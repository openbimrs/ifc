//! `ifc-validate` -- Schema and model validation: is this file actually legal IFC?
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
//! | `header` | Header well-formedness and schema declaration checks |
//! | `structure` (private scaffold) | Required/cardinality/reference/UNIQUE checks |
//! | `type_check` | Attribute values match their declared EXPRESS types |
//! | `where_rule` | EXPRESS `WHERE` rules and the 2 global rules in IFC4 |
//! | `report` | Structured findings: severity, entity, rule, message |
//! | `error` | Why validation could not run |
//!
//! # Status
//!
//! Scaffold -- contract-free modules remain private; behavior and deliberate
//! public contracts remain tracked in `../PLAN.md`.

mod error;
mod header;
mod report;
mod structure;
mod type_check;
mod where_rule;
