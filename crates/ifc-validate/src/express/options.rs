//! Limits on EXPRESS evaluation.

pub use crate::report::BudgetExceeded;

/// How much work rule evaluation may do on one model.
///
/// A WHERE rule can call FUNCTIONs that loop and recurse, over data a user
/// did not write. These limits turn a pathological file into typed
/// [`BudgetExceeded`] results instead of a hang or a stack overflow:
///
/// - [`Self::steps_per_rule`] bounds one rule on one instance (each
///   expression node and statement is a step); exceeding it reports that
///   rule on that instance as not evaluated and moves on;
/// - [`Self::max_depth`] bounds nested FUNCTION calls and derived
///   attributes, which is what keeps the native stack safe;
/// - [`Self::total_steps`] bounds the whole run; exceeding it stops rule
///   evaluation and marks the report truncated.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct EvalLimits {
    /// Steps one rule may take on one instance.
    pub steps_per_rule: u64,
    /// Nesting of FUNCTION calls and derived attributes.
    pub max_depth: usize,
    /// Steps the whole run may take.
    pub total_steps: u64,
}

impl EvalLimits {
    /// Generous for real files: the IFC rules take tens to a few thousand
    /// steps per instance, and nest a handful of calls deep.
    pub const DEFAULT: Self = Self {
        steps_per_rule: 1_000_000,
        max_depth: 48,
        total_steps: 2_000_000_000,
    };

    /// These limits with a different per-rule step limit.
    #[must_use]
    pub const fn with_steps_per_rule(mut self, steps: u64) -> Self {
        self.steps_per_rule = steps;
        self
    }

    /// These limits with a different nesting limit.
    #[must_use]
    pub const fn with_max_depth(mut self, depth: usize) -> Self {
        self.max_depth = depth;
        self
    }

    /// These limits with a different limit for the whole run.
    #[must_use]
    pub const fn with_total_steps(mut self, steps: u64) -> Self {
        self.total_steps = steps;
        self
    }
}

impl Default for EvalLimits {
    fn default() -> Self {
        Self::DEFAULT
    }
}
