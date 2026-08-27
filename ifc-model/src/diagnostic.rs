//! Codec-neutral, non-fatal problems found while reading a model.
//!
//! A codec that recovers from damaged input must be able to say what it
//! dropped: silent recovery turns a corrupt file into a plausible-looking
//! model with no trace of the loss. These diagnostics are attached to the
//! [`Model`](crate::Model) rather than returned separately so the information
//! survives being passed around an application.
//!
//! The type is codec-neutral on purpose. STEP reports byte offsets, ifcXML
//! would report element positions, so the location is an optional byte range
//! rather than a STEP span, and this crate names no codec type.

use std::fmt;
use std::ops::Range;

/// How serious a non-fatal finding is.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Severity {
    /// Input was read, but not fully represented — something was dropped.
    #[default]
    Warning,
}

/// A non-fatal problem found while reading a model.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagnostic {
    severity: Severity,
    byte_range: Option<Range<usize>>,
    detail: String,
}

impl Diagnostic {
    /// Records a warning covering a byte range of the source.
    pub fn warning(byte_range: Range<usize>, detail: impl Into<String>) -> Self {
        Self {
            severity: Severity::Warning,
            byte_range: Some(byte_range),
            detail: detail.into(),
        }
    }

    /// Records a warning with no source location.
    pub fn unlocated(detail: impl Into<String>) -> Self {
        Self {
            severity: Severity::Warning,
            byte_range: None,
            detail: detail.into(),
        }
    }

    /// Severity of this finding.
    pub const fn severity(&self) -> Severity {
        self.severity
    }

    /// Byte range of the source this finding covers, when the codec has one.
    pub const fn byte_range(&self) -> Option<&Range<usize>> {
        self.byte_range.as_ref()
    }

    /// Human-readable description without a location prefix.
    pub fn detail(&self) -> &str {
        &self.detail
    }
}

impl fmt::Display for Diagnostic {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.byte_range {
            Some(range) => write!(
                formatter,
                "warning at bytes {}..{}: {}",
                range.start, range.end, self.detail
            ),
            None => write!(formatter, "warning: {}", self.detail),
        }
    }
}
