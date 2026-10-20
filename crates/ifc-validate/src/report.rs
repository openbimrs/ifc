//! Structured findings: severity, entity, rule, message.
//!
//! A validation result must be machine-readable so a downstream consumer can
//! act on it and a CI gate can fail on it.
//!
//! ## Internal split
//!
//! - `finding.rs`: severity and the individual finding.
//! - `detail.rs`: typed detail behind a finding's message.
//! - `path.rs`: entity and attribute paths, and their total order.
//! - `summary.rs`: counts and the assembled report.

mod detail;
mod finding;
mod path;
mod summary;

pub use detail::{
    BudgetExceeded, FindingDetail, NotEvaluated, ReservedSetName, RuleDetail, RuleOutcome,
    RuleScope,
};
pub use finding::{Finding, Severity};
pub use path::Path;
pub use summary::{Report, Summary};
