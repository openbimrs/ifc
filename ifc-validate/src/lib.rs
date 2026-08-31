//! `ifc-validate` -- Schema and model validation: is this file actually legal IFC?
//!
//! Split from parsing on purpose. A reader that rejects everything imperfect is
//! useless on real data -- roughly half of production files violate something --
//! so parsing is permissive and validation is an explicit, separate pass.
//!
//! # What a report means
//!
//! ```no_run
//! # use ifc_model::Model;
//! # fn demo(model: &Model) {
//! let report = ifc_validate::validate(model, ifc_schema::ifc4());
//! if report.is_conformant() {
//!     // No *errors*. There may still be warnings, and there are certainly
//!     // rules this validator does not evaluate -- `report.summary()`
//!     // says how many.
//! }
//! # }
//! ```
//!
//! The three severities are not a mood scale. [`Severity::Error`] means the
//! file breaks a schema requirement; [`Severity::Warning`] means it is legal
//! but will behave badly; [`Severity::Unsupported`] is a statement about
//! *this validator*, not about the file. Only errors affect
//! [`Report::is_conformant`].
//!
//! # What this crate deliberately does not do
//!
//! It does not evaluate arbitrary EXPRESS `WHERE` expressions -- there is no
//! expression evaluator here. Rules that need one are registered as
//! unsupported and reported, so a clean report never silently means
//! "unchecked". See [`where_rule::RULES`].
//!
//! It does not check aggregate bounds (`LIST [3:?]`), because the schema
//! parser retains whether an attribute is an aggregate but not its bounds.
//! Claiming otherwise would be worse than the gap.
//!
//! It also does not derive `INVERSE` relationship semantics. A selected rule
//! whose IFC2X3 form depends on an inverse is therefore reported unsupported,
//! even when a later schema revision exposes equivalent direct attributes.
//!
//! # Module map
//!
//! | Module | Role |
//! |---|---|
//! | [`header`] | Declared schema and implementation level |
//! | [`structure`] | References, required slots, cardinality, unique ids |
//! | [`type_check`] | Values against their declared EXPRESS types |
//! | [`where_rule`] | Native rules, and honest reporting of the rest |
//! | [`report`] | Findings, paths, severities, summaries |
//! | [`error`] | Why validation could not run |

pub mod error;
pub mod header;
pub mod report;
pub mod structure;
pub mod type_check;
pub mod where_rule;

pub use error::ValidateError;
pub use report::{Finding, Path, Report, Severity, Summary};
pub use where_rule::Budget;

use ifc_model::Model;
use ifc_schema::Schema;

/// Validate a model against a schema with the default budget.
///
/// Runs every check this crate implements: header, structure, types, and the
/// natively implemented rules. Unsupported rules are recorded rather than
/// skipped.
#[must_use]
pub fn validate(model: &Model, schema: &Schema) -> Report {
    validate_with(model, schema, Budget::DEFAULT)
}

/// Validate under an explicit budget.
///
/// A budget bounds how many findings are recorded. Hitting it marks the
/// report truncated: `12 errors` from a truncated report means "at least 12".
#[must_use]
pub fn validate_with(model: &Model, schema: &Schema, budget: Budget) -> Report {
    let mut report = Report::with_max_findings(budget.max_findings);
    header::check(model, &mut report);
    structure::check(model, schema, budget, &mut report);
    type_check::check(model, schema, budget, &mut report);
    where_rule::evaluate(model, schema, budget, &mut report);
    report
}

/// Validate against the schema the file itself declares.
///
/// # Errors
///
/// Returns [`ValidateError`] when the file declares no schema, or declares
/// one this build has no tables for. Validating an IFC2X3 file against IFC4
/// tables would produce confident nonsense, so it is refused rather than
/// approximated.
#[cfg(feature = "ifc4")]
pub fn validate_declared(model: &Model) -> Result<Report, ValidateError> {
    use ifc_schema::SchemaVersion;

    let token = model
        .header()
        .schema_token()
        .ok_or(ValidateError::NoSchemaDeclared)?;
    let version = SchemaVersion::from_header_token(token)
        .ok_or_else(|| ValidateError::UnknownSchema(token.to_string()))?;
    // `for_version` returns None for a recognised schema this build does not
    // bundle. Both cases are refusals, but they are different facts: one is
    // "no idea what that token is", the other is "known schema, no tables".
    let schema = ifc_schema::for_version(version)
        .ok_or_else(|| ValidateError::UnbundledSchema(token.to_string()))?;
    Ok(validate(model, schema))
}
