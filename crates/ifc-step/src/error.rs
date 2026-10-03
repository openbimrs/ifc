//! Why a STEP parse failed at the IFC adapter boundary.

use ifc_model::{EntityId, ModelError};
use thiserror::Error;

/// Failures specific to reading or writing IFC STEP text.
#[derive(Debug, Error)]
#[non_exhaustive]
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

    /// A REAL token has no decimal point (`1E-05`).
    ///
    /// ISO 10303-21 requires the point, so a strict read refuses the token.
    /// A lenient read ([`crate::StepCodec::lenient`]) reads it as the REAL
    /// it spells and reports a diagnostic.
    #[error("syntax error at byte {offset}: REAL `{token}` has no decimal point (ISO 10303-21 requires one)")]
    RealWithoutDecimalPoint {
        /// Byte offset of the token in the source.
        offset: usize,
        /// The token as written.
        token: String,
    },

    /// The model contains a REAL value Part 21 cannot represent.
    #[error("entity #{entity} slot {slot} contains a non-finite REAL")]
    NonFiniteReal {
        /// Entity containing the value.
        entity: EntityId,
        /// Top-level positional attribute containing the value.
        slot: usize,
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
            StepError::RealWithoutDecimalPoint { offset, token } => ModelError::Syntax {
                offset,
                detail: format!(
                    "REAL `{token}` has no decimal point (ISO 10303-21 requires one; \
                     a lenient read accepts it)"
                ),
            },
            StepError::NonFiniteReal { entity, slot } => ModelError::Write(format!(
                "entity #{entity} slot {slot} contains a non-finite REAL"
            )),
            StepError::Io(message) => ModelError::Io(message),
        }
    }
}
