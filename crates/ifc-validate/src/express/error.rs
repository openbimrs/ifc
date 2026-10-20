//! Why EXPRESS rule text could not be loaded.

use std::fmt;

/// A line in the EXPRESS source, for messages.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SourcePos {
    /// One-based line number.
    pub line: u32,
}

/// The EXPRESS source could not be turned into evaluable rules.
///
/// Distinct from a finding: this is about the rule text a caller supplied,
/// not about any model.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum ExpressError {
    /// The source is not EXPRESS this reader understands.
    Syntax {
        /// Where the reader stopped.
        line: u32,
        /// What it expected or found.
        message: String,
    },
    /// The source names a schema other than the tables it was paired with,
    /// e.g. `IFC4.exp` supplied for an IFC4X3 model.
    SchemaMismatch {
        /// The `SCHEMA` name in the source.
        source: String,
        /// The name of the bundled tables.
        tables: String,
    },
    /// The source parses, but disagrees with the paired tables about an
    /// entity's attributes, so slots read through it would be wrong.
    TableMismatch {
        /// The entity whose layout differs.
        entity: String,
        /// How it differs.
        detail: String,
    },
    /// An identifier in a rule or function names nothing in scope.
    Unresolved {
        /// The declaration holding the expression (`IfcWall.WR1`, a
        /// function name).
        context: String,
        /// The identifier.
        name: String,
    },
}

impl ExpressError {
    pub(crate) fn syntax(pos: SourcePos, message: impl Into<String>) -> Self {
        Self::Syntax {
            line: pos.line,
            message: message.into(),
        }
    }
}

impl fmt::Display for ExpressError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Syntax { line, message } => write!(f, "EXPRESS line {line}: {message}"),
            Self::SchemaMismatch { source, tables } => write!(
                f,
                "the EXPRESS source declares schema {source}, the tables are {tables}"
            ),
            Self::TableMismatch { entity, detail } => {
                write!(
                    f,
                    "the EXPRESS source and the tables disagree on {entity}: {detail}"
                )
            }
            Self::Unresolved { context, name } => {
                write!(f, "{context}: `{name}` names nothing in scope")
            }
        }
    }
}

impl std::error::Error for ExpressError {}
