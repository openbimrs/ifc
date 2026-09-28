//! Why a cost lookup failed.

use thiserror::Error;

/// Failures specific to interpreting cost data.
#[derive(Debug, Error)]
#[non_exhaustive]
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

    /// The model's header declares one schema the cost readers are not
    /// verified against (anything but IFC2X3, IFC4 and IFC4X3), so no
    /// attribute position can be trusted. Never read as another release.
    #[error("the header declares {schema}, which the cost readers are not verified against")]
    UnsupportedSchema {
        /// The `FILE_SCHEMA` token as written, or the release identifier.
        schema: String,
    },

    /// The model's header declares several schemas; a reader binds to
    /// exactly one release.
    #[error("the header declares {schemas} schemas; a reader binds to exactly one")]
    MultipleSchemas {
        /// Number of `FILE_SCHEMA` declarations.
        schemas: usize,
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
