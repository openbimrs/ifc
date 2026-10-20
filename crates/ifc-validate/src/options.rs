//! [`ValidateOptions`]: what a validation run checks beyond the bundled
//! tables, and how much it may report.

use ifc_model::Model;
use ifc_schema::Schema;

#[cfg(feature = "express")]
use crate::express::ExpressRules;
use crate::property_set::{self, PropertySetCatalog};
use crate::report::Report;
#[cfg(feature = "express")]
use crate::report::{Finding, Path};
use crate::where_rule::Budget;

/// Everything a validation run may be given besides the model and its
/// tables.
///
/// ```no_run
/// # fn demo(model: &ifc_model::Model) {
/// use ifc_validate::{validate_with_options, Budget, ValidateOptions};
/// let options = ValidateOptions::new().budget(Budget { max_findings: 100 });
/// let report = validate_with_options(model, ifc_schema::ifc4(), &options);
/// # }
/// ```
#[derive(Clone, Copy, Default)]
#[non_exhaustive]
pub struct ValidateOptions<'a> {
    /// How many findings to record.
    pub budget: Budget,
    /// The release's EXPRESS rules, read from its schema text. With them,
    /// every WHERE rule, type rule and global rule is evaluated, and the
    /// hand-written rules they cover are not run.
    #[cfg(feature = "express")]
    pub express: Option<&'a ExpressRules>,
    /// The PSD/QTO catalog that `Pset_` and `Qto_` set names are checked
    /// against.
    pub catalog: Option<&'a dyn PropertySetCatalog>,
    /// Check every INVERSE clause's cardinality
    /// (`structure.inverse.cardinality`). Off unless asked for, or unless
    /// EXPRESS rules are supplied: it is part of full EXPRESS conformance,
    /// and IFC2X3's `PlacesObject : SET [1:1]` alone turns many files that
    /// share a placement between products non-conformant.
    pub inverse_cardinality: bool,
}

impl std::fmt::Debug for ValidateOptions<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut debug = f.debug_struct("ValidateOptions");
        debug.field("budget", &self.budget);
        #[cfg(feature = "express")]
        debug.field("express", &self.express.map(ExpressRules::schema_name));
        debug.field("catalog", &self.catalog.is_some()).finish()
    }
}

impl<'a> ValidateOptions<'a> {
    /// The default budget, no EXPRESS rules, no catalog: what
    /// [`validate`](crate::validate) runs.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// These options with another finding budget.
    #[must_use]
    pub fn budget(mut self, budget: Budget) -> Self {
        self.budget = budget;
        self
    }

    /// These options evaluating `rules`, and checking INVERSE cardinality
    /// with them.
    #[cfg(feature = "express")]
    #[must_use]
    pub fn express(mut self, rules: &'a ExpressRules) -> Self {
        self.express = Some(rules);
        self.inverse_cardinality = true;
        self
    }

    /// These options with INVERSE cardinality checked, or not.
    #[must_use]
    pub fn inverse_cardinality(mut self, check: bool) -> Self {
        self.inverse_cardinality = check;
        self
    }

    /// These options checking set names against `catalog`.
    #[must_use]
    pub fn catalog(mut self, catalog: &'a dyn PropertySetCatalog) -> Self {
        self.catalog = Some(catalog);
        self
    }
}

/// Validate a model with everything `options` supplies.
///
/// Runs what [`validate_with`](crate::validate_with) runs, then what the
/// options ask for: INVERSE cardinality, every EXPRESS rule of the release
/// (replacing the hand-written rules with the same id), and the reserved
/// set-name check against a catalog.
#[must_use]
pub fn validate_with_options(
    model: &Model,
    schema: &Schema,
    options: &ValidateOptions<'_>,
) -> Report {
    let budget = options.budget;
    let mut report = Report::with_max_findings(budget.max_findings);
    crate::header::check(model, &mut report);
    crate::structure::check(model, schema, budget, &mut report);
    crate::type_check::check(model, schema, budget, &mut report);
    if options.inverse_cardinality {
        crate::structure::inverse_cardinality(model, schema, &mut report);
    }
    #[cfg(feature = "express")]
    let express = options.express.filter(|rules| {
        let paired = rules.schema_name().eq_ignore_ascii_case(schema.name());
        if !paired {
            report.push(Finding::unsupported(
                "where.express.release",
                Path::File,
                format!(
                    "the EXPRESS rules are {}'s, the tables {}'s; the rules were not evaluated",
                    rules.schema_name(),
                    schema.name()
                ),
            ));
        }
        paired
    });
    #[cfg(not(feature = "express"))]
    let express: Option<&()> = None;
    #[cfg(feature = "express")]
    let covers = |id: &str| express.is_some_and(|rules| rules.covers(id));
    #[cfg(not(feature = "express"))]
    let covers = |_: &str| false;
    crate::where_rule::evaluate_except(
        model,
        schema,
        budget,
        &mut report,
        &covers,
        express.is_some(),
    );
    #[cfg(feature = "express")]
    if let Some(rules) = express {
        crate::express::evaluate(model, rules, budget, &mut report);
    }
    property_set::check(model, schema, options.catalog, &mut report);
    report
}
