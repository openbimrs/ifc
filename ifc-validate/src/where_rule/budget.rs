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
//!
//! # Why there is no depth limit
//!
//! Every graph walk this crate performs -- supertype chains, SELECT
//! membership, defined-type alias chains -- runs over the *schema's* type
//! graph, never over the file's entity graph. The schema is bundled and
//! finite, so those walks are bounded by its size and guarded against cycles
//! by a visited set (`type_check::select`); a file cannot lengthen them. The
//! per-entity checks are single passes over each record's own slots.
//!
//! A caller-tunable depth would therefore bound nothing a file controls, and
//! a small value would only turn "not yet searched" into "not a member" --
//! the false accusation `type_check::select` documents. The one quantity a
//! file does control is how many findings it provokes, and that is what this
//! budget caps.

/// Limits applied to one validation run.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Budget {
    /// Stop recording after this many findings.
    pub max_findings: usize,
}

impl Budget {
    /// A budget large enough for real files and small enough to bound memory.
    ///
    /// 10,000 findings is far past the point where a report is actionable --
    /// a file with that many defects needs a different conversation -- and
    /// costs a few hundred KB to hold.
    pub const DEFAULT: Self = Self {
        max_findings: 10_000,
    };

    /// An explicitly unbounded budget, for tests and for callers that have
    /// already decided the input is trustworthy.
    pub const UNLIMITED: Self = Self {
        max_findings: usize::MAX,
    };
}

impl Default for Budget {
    fn default() -> Self {
        Self::DEFAULT
    }
}
