//! Why a command could not answer, and the exit code each reason gets.
//!
//! The kinds mirror the typed refusals of the library: unsupported,
//! invalid, missing-reference and budget-exceeded stay distinct, so a
//! script reading stderr can tell "this build cannot" from "this file is
//! broken". Every kind exits 2; findings are not errors and exit 1 through
//! [`Outcome`].

use std::fmt;

/// How a command that ran to completion ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Outcome {
    /// Done, nothing to report against the input.
    Clean,
    /// `validate` or `lint` reported findings that fail the run.
    Findings,
    /// A multi-file command could not check some file; each such file was
    /// already reported in the output and on stderr.
    Incomplete,
}

impl Outcome {
    /// The process exit code.
    pub(crate) fn code(self) -> u8 {
        match self {
            Self::Clean => 0,
            Self::Findings => 1,
            Self::Incomplete => 2,
        }
    }
}

/// A command refused or failed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum CliError {
    /// The arguments are consistent with the grammar but not with each
    /// other or with the input (an output format no extension implies).
    Usage(String),
    /// Reading or writing a file failed.
    Io {
        /// The file, or `-` for a standard stream.
        path: String,
        /// The operating system's reason.
        detail: String,
    },
    /// The input is not a STEP or ifcXML file, or does not parse as one.
    Parse {
        /// The input file.
        path: String,
        /// The codec's reason.
        detail: String,
    },
    /// The header declares no schema, or one this build bundles no tables
    /// for; the command needs them and does not guess another release.
    UnsupportedSchema(String),
    /// Something this tool or the library does not support for this input.
    Unsupported(String),
    /// An entity id or `GlobalId` the model does not hold.
    MissingEntity(String),
    /// An entity of a type the command cannot answer for.
    WrongEntityType(String),
    /// A reference in the model names an entity the model does not hold.
    MissingReference(String),
    /// The model is malformed or ambiguous where the command needs an
    /// exact answer.
    InvalidModel(String),
    /// A traversal hit its budget before it could answer exactly.
    BudgetExceeded(String),
    /// Writing the output format refused the model.
    Write(String),
}

impl CliError {
    /// The process exit code: 2 for every refusal and failure.
    pub(crate) fn exit_code(&self) -> u8 {
        2
    }

    /// A short, stable name for the kind, printed before the detail.
    pub(crate) fn kind(&self) -> &'static str {
        match self {
            Self::Usage(_) => "usage",
            Self::Io { .. } => "io",
            Self::Parse { .. } => "parse",
            Self::UnsupportedSchema(_) => "unsupported-schema",
            Self::Unsupported(_) => "unsupported",
            Self::MissingEntity(_) => "missing-entity",
            Self::WrongEntityType(_) => "wrong-entity-type",
            Self::MissingReference(_) => "missing-reference",
            Self::InvalidModel(_) => "invalid-model",
            Self::BudgetExceeded(_) => "budget-exceeded",
            Self::Write(_) => "write",
        }
    }

    /// A write to standard output failed.
    pub(crate) fn stdout(error: &std::io::Error) -> Self {
        Self::Io {
            path: "-".to_owned(),
            detail: error.to_string(),
        }
    }
}

impl fmt::Display for CliError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: ", self.kind())?;
        match self {
            Self::Io { path, detail } | Self::Parse { path, detail } => {
                write!(f, "{path}: {detail}")
            }
            Self::Usage(detail)
            | Self::UnsupportedSchema(detail)
            | Self::Unsupported(detail)
            | Self::MissingEntity(detail)
            | Self::WrongEntityType(detail)
            | Self::MissingReference(detail)
            | Self::InvalidModel(detail)
            | Self::BudgetExceeded(detail)
            | Self::Write(detail) => f.write_str(detail),
        }
    }
}

/// Shorthand for command results.
pub(crate) type CliResult<T> = Result<T, CliError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn findings_and_refusals_exit_differently() {
        assert_eq!(Outcome::Clean.code(), 0);
        assert_eq!(Outcome::Findings.code(), 1);
        assert_eq!(Outcome::Incomplete.code(), 2);
        let refusal = CliError::UnsupportedSchema("IFC9".into());
        assert_eq!(refusal.exit_code(), 2);
        assert_eq!(refusal.to_string(), "unsupported-schema: IFC9");
    }
}
