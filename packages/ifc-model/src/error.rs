//! Why a model operation failed.

use crate::value::EntityId;
use thiserror::Error;

/// Errors from reading, writing, or querying a model.
///
/// Codec-specific detail is carried as a message rather than a nested error
/// type: this crate must not depend on any codec, so it cannot name their
/// error types.
#[derive(Debug, Error)]
pub enum ModelError {
    /// Underlying I/O failure.
    #[error("io error: {0}")]
    Io(String),

    /// The bytes are not this format at all.
    #[error("not a {expected} file: {detail}")]
    WrongFormat {
        /// Format the codec expected.
        expected: &'static str,
        /// What was seen instead.
        detail: String,
    },

    /// Syntax error at a known location.
    #[error("syntax error at byte {offset}: {detail}")]
    Syntax {
        /// Byte offset into the source.
        offset: usize,
        /// What went wrong.
        detail: String,
    },

    /// An entity referenced an id that does not exist.
    #[error("entity {from} references missing entity {to}")]
    DanglingReference {
        /// The referring entity.
        from: EntityId,
        /// The missing target.
        to: EntityId,
    },

    /// The same id was defined twice.
    #[error("duplicate entity id {0}")]
    DuplicateId(EntityId),

    /// Serialization failed.
    #[error("write error: {0}")]
    Write(String),
}

/// Convenience alias.
pub type ModelResult<T> = Result<T, ModelError>;
