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
//! The four severities are not a mood scale. [`Severity::Error`] means the
//! file breaks a schema requirement; [`Severity::EvaluationError`] means an
//! implemented rule applied to an instance but could not be decided for it;
//! [`Severity::Warning`] means it is legal but will behave badly;
//! [`Severity::Unsupported`] is a statement about *this validator*, not about
//! the file. Errors and evaluation errors affect [`Report::is_conformant`]:
//! a rule nobody could decide is not a rule that passed.
//!
//! # What this crate deliberately does not do
//!
//! It does not bundle the schemas' rule text: the IFC schemas are CC BY-ND
//! 4.0, and the bundled tables keep only rule labels. [`validate`] runs a
//! hand-written subset of the WHERE rules ([`where_rule::RULES`]) and
//! reports the rest it knows of as unsupported. Given the release's own
//! `.exp` text, the [`express`] evaluator (feature `express`, on by
//! default) evaluates every WHERE rule, type rule and global rule of the
//! release through [`validate_with_options`], so a clean report never
//! silently means "unchecked".
//!
//! It checks the aggregate bounds, nesting and element uniqueness of
//! explicit attributes, and every `UNIQUE` clause of the declared release
//! ([`structure`]). A bound written as an expression, and the bounds of an
//! aggregate reached through a defined type, are not evaluated.
//!
//! The hand-written rules do not derive `INVERSE` relationships, so one
//! whose IFC2X3 form depends on an inverse is reported unsupported; the
//! [`express`] evaluator resolves INVERSE attributes through the reverse
//! index.
//!
//! # Module map
//!
//! | Module | Role |
//! |---|---|
//! | [`header`] | Declared schema and implementation level |
//! | [`structure`] | References, required slots, cardinality, bounds, UNIQUE clauses |
//! | [`type_check`] | Values against their declared EXPRESS types |
//! | [`where_rule`] | Native rules, and honest reporting of the rest |
//! | [`express`] | Every EXPRESS rule, evaluated from the release's text |
//! | [`property_set`] | `Pset_`/`Qto_` names, and the members of catalog sets |
//! | [`options`] | What a run is given: budget, rules, catalog |
//! | [`report`] | Findings, paths, severities, summaries |
//! | [`error`] | Why validation could not run |

pub mod error;
#[cfg(feature = "express")]
pub mod express;
pub mod header;
pub mod options;
pub mod property_set;
pub mod report;
pub mod structure;
pub mod type_check;
pub mod where_rule;

pub use error::ValidateError;
pub use options::{validate_with_options, ValidateOptions};
pub use property_set::{
    CatalogMember, CatalogMemberForm, CatalogQuantityKind, CatalogSet, CatalogSetKind,
    PropertySetCatalog,
};
pub use report::{
    BudgetExceeded, CatalogMemberDetail, CatalogMismatch, Finding, FindingDetail, NotEvaluated,
    Path, Report, ReservedSetName, RuleDetail, RuleOutcome, RuleScope, Severity, Summary,
};
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
    validate_with_options(model, schema, &ValidateOptions::new().budget(budget))
}

/// Validate against the schema the file itself declares.
///
/// # Errors
///
/// Returns [`ValidateError`] when the file declares no schema, or declares
/// one this build has no tables for. A release known by name but published
/// without an EXPRESS schema (`IFC4X3_TC1`, `IFC4X3_ADD1`) is
/// [`ValidateError::UnpublishedSchema`], never validated as IFC4X3 ADD2 by
/// assumption; a caller choosing to do so passes those tables to
/// [`validate`]. Validating an IFC2X3 file against IFC4
/// tables would produce confident nonsense, so it is refused rather than
/// approximated. Which releases have tables is the build's choice: the
/// release features (`ifc2x3`, `ifc4`, `ifc4x1`, `ifc4x2`, `ifc4x3`, all on
/// by default) each bundle one.
pub fn validate_declared(model: &Model) -> Result<Report, ValidateError> {
    use ifc_schema::SchemaVersion;

    let token = model
        .header()
        .schema_token()
        .ok_or(ValidateError::NoSchemaDeclared)?;
    let version = SchemaVersion::from_header_token(token).ok_or_else(|| {
        if ifc_schema::UnpublishedRelease::from_header_token(token).is_some() {
            ValidateError::UnpublishedSchema(token.to_string())
        } else {
            ValidateError::UnknownSchema(token.to_string())
        }
    })?;
    // `for_version` refuses a recognised schema this build does not bundle
    // with `NotBundled`. Both cases are refusals, but they are different
    // facts: one is "no idea what that token is", the other is "known schema,
    // no tables".
    let schema = ifc_schema::for_version(version)
        .map_err(|_| ValidateError::UnbundledSchema(token.to_string()))?;
    Ok(validate(model, schema))
}
