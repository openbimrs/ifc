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

use crate::EntityId;

mod damage;

pub use damage::{RecordDamage, UnclassifiedDiagnostic};

/// How serious a non-fatal finding is.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Severity {
    /// Input was read, but not fully represented — something was dropped.
    #[default]
    Warning,
}

/// What a [`Diagnostic`] is about, for a caller that acts on it instead of
/// printing it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum DiagnosticKind {
    /// A record was dropped because it was malformed or unrepresentable.
    SkippedRecord,
    /// Two records declared the same id.
    DuplicateId,
    /// A record references an id no record declares.
    DanglingReference,
    /// A real number was written without a decimal point.
    RealWithoutPoint,
    /// Anything else, including findings built by [`Diagnostic::warning`].
    Other,
}

/// A line and column in the source text, both 1-based.
///
/// The column counts Unicode characters of the line before the position.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct SourcePosition {
    line: usize,
    column: usize,
}

impl SourcePosition {
    /// A position at a 1-based line and column.
    pub const fn new(line: usize, column: usize) -> Self {
        Self { line, column }
    }

    /// 1-based line number.
    pub const fn line(&self) -> usize {
        self.line
    }

    /// 1-based column, counted in characters.
    pub const fn column(&self) -> usize {
        self.column
    }
}

impl fmt::Display for SourcePosition {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "line {}, column {}", self.line, self.column)
    }
}

/// A non-fatal problem found while reading a model.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagnostic {
    severity: Severity,
    kind: DiagnosticKind,
    byte_range: Option<Range<usize>>,
    position: Option<SourcePosition>,
    instance: Option<EntityId>,
    referrer: Option<EntityId>,
    excerpt: Option<String>,
    detail: String,
}

impl Diagnostic {
    /// Records a warning of `kind` with no source location; add one with the
    /// `with_*` builders.
    pub fn new(kind: DiagnosticKind, detail: impl Into<String>) -> Self {
        Self {
            severity: Severity::Warning,
            kind,
            byte_range: None,
            position: None,
            instance: None,
            referrer: None,
            excerpt: None,
            detail: detail.into(),
        }
    }

    /// Records a [`DiagnosticKind::Other`] warning covering a byte range of
    /// the source.
    pub fn warning(byte_range: Range<usize>, detail: impl Into<String>) -> Self {
        Self::new(DiagnosticKind::Other, detail).with_byte_range(byte_range)
    }

    /// Records a [`DiagnosticKind::Other`] warning with no source location.
    pub fn unlocated(detail: impl Into<String>) -> Self {
        Self::new(DiagnosticKind::Other, detail)
    }

    /// Sets the byte range of the source this finding covers.
    #[must_use]
    pub fn with_byte_range(mut self, range: Range<usize>) -> Self {
        self.byte_range = Some(range);
        self
    }

    /// Sets the line and column where the finding starts.
    #[must_use]
    pub fn with_position(mut self, position: SourcePosition) -> Self {
        self.position = Some(position);
        self
    }

    /// Sets the instance at fault: a skipped record's id, the duplicated id
    /// or the missing id of a dangling reference.
    #[must_use]
    pub fn with_instance(mut self, id: EntityId) -> Self {
        self.instance = Some(id);
        self
    }

    /// Sets the record holding a dangling reference.
    #[must_use]
    pub fn with_referrer(mut self, id: EntityId) -> Self {
        self.referrer = Some(id);
        self
    }

    /// Sets a short quote of the source this finding covers.
    #[must_use]
    pub fn with_excerpt(mut self, excerpt: impl Into<String>) -> Self {
        self.excerpt = Some(excerpt.into());
        self
    }

    /// Severity of this finding.
    pub const fn severity(&self) -> Severity {
        self.severity
    }

    /// What this finding is about.
    pub const fn kind(&self) -> DiagnosticKind {
        self.kind
    }

    /// Byte range of the source this finding covers, when the codec has one.
    pub const fn byte_range(&self) -> Option<&Range<usize>> {
        self.byte_range.as_ref()
    }

    /// The instance at fault: a skipped record's id when it was read, the
    /// duplicated id, or the missing id of a dangling reference. `None` for
    /// [`DiagnosticKind::RealWithoutPoint`] and [`DiagnosticKind::Other`].
    pub const fn instance(&self) -> Option<EntityId> {
        self.instance
    }

    /// The record holding the reference, for a
    /// [`DiagnosticKind::DanglingReference`] only.
    pub const fn referrer(&self) -> Option<EntityId> {
        self.referrer
    }

    /// Line and column where the finding starts, when the codec has one.
    pub const fn position(&self) -> Option<SourcePosition> {
        self.position
    }

    /// A short quote of the source this finding covers.
    pub fn excerpt(&self) -> Option<&str> {
        self.excerpt.as_deref()
    }

    /// Human-readable description without a location prefix.
    pub fn detail(&self) -> &str {
        &self.detail
    }
}

impl fmt::Display for Diagnostic {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match (&self.position, &self.byte_range) {
            (Some(position), Some(range)) => write!(
                formatter,
                "warning at {position} (bytes {}..{}): {}",
                range.start, range.end, self.detail
            ),
            (Some(position), None) => {
                write!(formatter, "warning at {position}: {}", self.detail)
            }
            (None, Some(range)) => write!(
                formatter,
                "warning at bytes {}..{}: {}",
                range.start, range.end, self.detail
            ),
            (None, None) => write!(formatter, "warning: {}", self.detail),
        }
    }
}
