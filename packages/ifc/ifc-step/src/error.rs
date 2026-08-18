//! Why a parse failed.

use thiserror::Error;

/// Every way reading a STEP physical file can fail.
///
/// Variants carry enough context to point a user at the offending byte or
/// record — "malformed file" alone is not actionable on a 500 MB model.
#[derive(Debug, Error, PartialEq)]
pub enum ParseError {
    #[error("not a STEP physical file: missing ISO-10303-21 magic")]
    NotStepFile,
    #[error("malformed header: {0}")]
    MalformedHeader(String),
}
