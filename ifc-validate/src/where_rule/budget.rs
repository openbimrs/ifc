//! Bounds on how much work one validation run may do.
//!
//! # Why a validator needs a budget
//!
//! Validation runs on files a user did not write, in CI, on a schedule. A
//! pathological or hostile file must not turn that into an unbounded run: a
//! 2 GB model with a million dangling references would otherwise produce a
//! million findings and exhaust memory before reporting anything.
//!
//! The budget is a *reporting* limit, not a correctness compromise. When it
//! is hit the report is marked truncated, so "12 errors" never silently means
//! "at least 12 errors".

/// Limits applied to one validation run.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Budget {
    /// Stop recording after this many findings.
    pub max_findings: usize,
    /// Give up on a supertype or SELECT walk after this many steps.
    pub max_depth: usize,
}

impl Budget {
    /// A budget large enough for real files and small enough to bound memory.
    ///
    /// 10,000 findings is far past the point where a report is actionable --
    /// a file with that many defects needs a different conversation -- and
    /// costs a few hundred KB to hold.
    pub const DEFAULT: Self = Self {
        max_findings: 10_000,
        max_depth: 64,
    };

    /// An explicitly unbounded budget, for tests and for callers that have
    /// already decided the input is trustworthy.
    pub const UNLIMITED: Self = Self {
        max_findings: usize::MAX,
        max_depth: usize::MAX,
    };
}

impl Default for Budget {
    fn default() -> Self {
        Self::DEFAULT
    }
}
