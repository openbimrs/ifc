//! EXPRESS rule evaluation: WHERE rules, type rules, global rules and the
//! FUNCTIONs they call, read from the release's own schema text.
//!
//! ```no_run
//! # fn demo(model: &ifc_model::Model, exp: &str) -> Result<(), Box<dyn std::error::Error>> {
//! use ifc_validate::{express::ExpressRules, validate_with_options, ValidateOptions};
//!
//! let schema = ifc_schema::ifc4();
//! // `exp` is the text of IFC4.exp, which the caller supplies.
//! let rules = ExpressRules::parse(exp, schema)?;
//! let report = validate_with_options(model, schema, &ValidateOptions::new().express(&rules));
//! # Ok(()) }
//! ```
//!
//! # Why the rule text is supplied, not bundled
//!
//! The IFC schemas are CC BY-ND 4.0. The bundled tables keep their
//! structure and rule labels, never rule text, and generated code
//! transcribing the rules would be a derivative of it. So the evaluator is
//! an interpreter: [`ExpressRules::parse`] reads the `.exp` file the caller
//! has (`scripts/fetch-ifc-schemas.sh` fetches and checksums all five
//! releases), and nothing derived from it is written anywhere.
//!
//! # Semantics
//!
//! ISO 10303-11, with three-valued logic: a rule is violated when it
//! evaluates to FALSE, and an UNKNOWN or `?` result is reported as
//! indeterminate, never as a pass or a violation. `TYPEOF` answers the
//! SELECT types a value belongs to as well as its own types. One schema
//! defect is worked around: rule strings naming a type in mixed case
//! (`'IFC4X3_ADD2.IfcMappedItem'`) are read upper-cased, as `resolve.rs`
//! explains.
//!
//! # Internal split
//!
//! - `lexer.rs`, `parse.rs`, `parse_expr.rs`, `parse_stmt.rs`: reading the
//!   text into `ast.rs`.
//! - `resolve.rs`: binding names to slots, functions, entities and
//!   enumeration items.
//! - `tables.rs`: attribute layout, cross-checked against the bundled
//!   tables, and `TYPEOF` names.
//! - `value.rs`, `machine.rs`, `eval.rs`, `ops.rs`, `builtins.rs`,
//!   `exec.rs`, `access.rs`: the interpreter.
//! - `run.rs`: applying every rule to a model and reporting.
//! - `rules.rs`, `options.rs`, `coverage.rs`, `error.rs`: the public
//!   surface.

mod access;
mod ast;
mod builtins;
mod coverage;
mod error;
mod eval;
mod exec;
mod lexer;
mod machine;
mod ops;
mod options;
mod parse;
mod parse_expr;
mod parse_stmt;
mod resolve;
mod rules;
mod run;
mod tables;
mod value;

pub use coverage::{Coverage, RuleCount};
pub use error::{ExpressError, SourcePos};
pub use options::{BudgetExceeded, EvalLimits};
pub use rules::ExpressRules;

pub(crate) use run::evaluate;
