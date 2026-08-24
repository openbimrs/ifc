//! Why a cost lookup failed.

use thiserror::Error;

/// Failures specific to interpreting cost data.
#[derive(Debug, Error)]
pub enum CostError {
    /// An entity was expected to be a cost entity but is not.
    #[error("entity #{id} is {actual}, not {expected}")]
    WrongType {
        /// The entity id.
        id: u64,
        /// The type it actually has.
        actual: String,
        /// The type that was expected.
        expected: &'static str,
    },

    /// A referenced cost value or quantity does not exist.
    #[error("cost item #{from} references missing entity #{to}")]
    MissingReference {
        /// The referring cost item.
        from: u64,
        /// The missing target.
        to: u64,
    },
}
