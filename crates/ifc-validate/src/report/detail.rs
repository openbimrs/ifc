//! Typed detail behind a finding's message.
//!
//! The message is for people; this is for programs. A caller that wants to
//! group EXPRESS rule results by declaration, tell an indeterminate result
//! from a violation, or list the rules a budget stopped, matches on these
//! instead of parsing text.

/// Structured detail of a finding, when its check provides one.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum FindingDetail {
    /// An EXPRESS WHERE rule, global rule or type rule evaluated from the
    /// schema's own text.
    Rule(RuleDetail),
    /// A property or quantity set whose name uses a prefix buildingSMART
    /// reserves, but which the release's catalog does not define.
    ReservedSetName(ReservedSetName),
    /// A property or quantity of a catalog set that the release's catalog
    /// does not declare, or declares differently (`property.undeclared_member`,
    /// `property.member_type`, #537).
    CatalogMember(CatalogMemberDetail),
    /// The file declares a release known by name whose normative EXPRESS
    /// is not published (`header.schema.unpublished`, #467): the report
    /// was not made against that release's own tables.
    UnpublishedSchema(ifc_schema::UnpublishedRelease),
}

/// One EXPRESS rule result.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct RuleDetail {
    /// The schema the rule text came from, as its `SCHEMA` line names it
    /// (`IFC4`, `IFC4X3_ADD2`).
    pub release: String,
    /// The declaration the rule belongs to.
    pub scope: RuleScope,
    /// The rule's label as the schema writes it (`WR11`,
    /// `CorrectItemsForType`).
    pub label: String,
    /// What evaluation concluded.
    pub outcome: RuleOutcome,
}

/// The declaration holding a rule.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum RuleScope {
    /// A WHERE rule of an entity, binding it and every subtype.
    Entity {
        /// The entity declaring the rule.
        declared_on: String,
    },
    /// A WHERE rule of a defined type, binding every value of it.
    Type {
        /// The type declaring the rule.
        type_name: String,
    },
    /// A global `RULE`.
    Global {
        /// The rule's name.
        rule: String,
    },
}

/// What evaluating a rule on one instance, value or file concluded.
///
/// A satisfied rule produces no finding, so there is no variant for it.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum RuleOutcome {
    /// The rule evaluated to FALSE.
    Violated,
    /// The rule evaluated to UNKNOWN or indeterminate (`?`). ISO 10303-11
    /// does not count that as a violation; it is reported so a reader knows
    /// the data could not decide the rule.
    Indeterminate,
    /// The rule could not be evaluated.
    NotEvaluated(NotEvaluated),
}

/// Why a rule was not evaluated.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum NotEvaluated {
    /// The rule needs something this evaluator does not implement; the
    /// construct or function is named.
    Unsupported {
        /// What is missing, e.g. `FORMAT` or `assignment to a file
        /// instance's attribute`.
        construct: String,
    },
    /// A limit stopped evaluation.
    BudgetExceeded(BudgetExceeded),
    /// The file's data has a shape the rule cannot read, e.g. a string
    /// where the rule compares numbers.
    InvalidData {
        /// What was found.
        detail: String,
    },
}

/// Which limit stopped an evaluation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum BudgetExceeded {
    /// One rule on one instance took more steps than allowed.
    Steps {
        /// The limit.
        limit: u64,
    },
    /// Function calls and derived attributes nested deeper than allowed.
    Depth {
        /// The limit.
        limit: usize,
    },
    /// The whole run took more steps than allowed; the remaining rules
    /// were not evaluated.
    Total {
        /// The limit.
        limit: u64,
    },
}

/// A set name using a reserved prefix the catalog does not define.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct ReservedSetName {
    /// The set's `Name`.
    pub name: String,
    /// The reserved prefix it uses, `Pset_` or `Qto_`.
    pub prefix: String,
    /// The release whose catalog was consulted.
    pub release: String,
}

/// A member of a catalog set that disagrees with the catalog's template.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct CatalogMemberDetail {
    /// The set's `Name`.
    pub set: String,
    /// The member's dotted path inside the set (`Outer.Inner` for a
    /// property nested in a complex property); `None` when the set itself
    /// is at fault.
    pub member: Option<String>,
    /// The release whose catalog was consulted.
    pub release: String,
    /// How it disagrees.
    pub mismatch: CatalogMismatch,
}

/// How a set or member disagrees with its catalog template.
///
/// Names are in the schema's spelling (`IfcLabel`), never the upper-cased
/// token a file writes.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum CatalogMismatch {
    /// The catalog declares no member of that name.
    UndeclaredMember,
    /// The set is an `IfcPropertySet` where the catalog declares a quantity
    /// set, or the reverse.
    SetKind {
        /// The entity the catalog declares.
        declared: String,
        /// The entity the file uses.
        found: String,
    },
    /// The member is written as another entity than the catalog declares.
    Form {
        /// The entity the catalog declares.
        declared: String,
        /// The entity the file uses.
        found: String,
    },
    /// A value has another type than the catalog declares.
    DataType {
        /// The type (or, for a reference, entity) the catalog declares.
        declared: String,
        /// The type the file uses.
        found: String,
    },
}

impl std::fmt::Display for BudgetExceeded {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Steps { limit } => write!(f, "more than {limit} evaluation steps"),
            Self::Depth { limit } => write!(f, "calls nested deeper than {limit}"),
            Self::Total { limit } => write!(f, "the run's {limit} evaluation steps were spent"),
        }
    }
}

impl std::fmt::Display for NotEvaluated {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unsupported { construct } => {
                write!(
                    f,
                    "needs {construct}, which this evaluator does not implement"
                )
            }
            Self::BudgetExceeded(budget) => write!(f, "stopped after {budget}"),
            Self::InvalidData { detail } => write!(f, "cannot read the data: {detail}"),
        }
    }
}
