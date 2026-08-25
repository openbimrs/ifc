//! Why a STEP parse failed at the IFC adapter boundary.

use ifc_model::ModelError;
use thiserror::Error;

/// Failures specific to reading or writing IFC STEP text.
#[derive(Debug, Error)]
pub enum StepError {
    /// The bytes do not begin with the ISO-10303-21 magic.
    #[error("not a STEP physical file: {0}")]
    NotStep(String),

    /// Malformed syntax at a known byte offset.
    #[error("syntax error at byte {offset}: {detail}")]
    Syntax {
        /// Byte offset into the source.
        offset: usize,
        /// What went wrong.
        detail: String,
    },

    /// A record in the DATA section had no `#id=` prefix.
    #[error("entity record without an id at byte {offset}")]
    MissingEntityId {
        /// Byte offset into the source.
        offset: usize,
    },

    /// Underlying I/O failure.
    #[error("io error: {0}")]
    Io(String),
}

impl From<openbim_step::StepError> for StepError {
    fn from(error: openbim_step::StepError) -> Self {
        if error.is_not_step() {
            Self::NotStep(error.detail().to_owned())
        } else {
            Self::Syntax {
                offset: error.span().start,
                detail: error.detail().to_owned(),
            }
        }
    }
}

impl From<StepError> for ModelError {
    fn from(error: StepError) -> Self {
        match error {
            StepError::NotStep(detail) => ModelError::WrongFormat {
                expected: "STEP",
                detail,
            },
            StepError::Syntax { offset, detail } => ModelError::Syntax { offset, detail },
            StepError::MissingEntityId { offset } => ModelError::Syntax {
                offset,
                detail: "entity record without an id".into(),
            },
            StepError::Io(message) => ModelError::Io(message),
        }
    }
}
