//! Structured failure. A geometry operation that cannot succeed says why.

use thiserror::Error;

/// Result alias for kernel operations.
pub type GeomResult<T> = Result<T, GeomError>;

/// Why a geometry operation failed.
///
/// These are **structured diagnostics, not strings**: the IFC layer needs to
/// distinguish "this model is dirty" (report and continue processing the other
/// 40,000 elements) from "this backend cannot do that" (fall back to another
/// backend). A stringly-typed error forces the caller to guess.
#[derive(Debug, Error, PartialEq)]
pub enum GeomError {
    /// The input mesh is not manifold, and this operation requires manifoldness.
    /// Common in real IFC data — expected, not exceptional.
    #[error("input mesh is not manifold: {0}")]
    NotManifold(String),

    /// Index buffer references a vertex that does not exist, or is not a whole
    /// number of triangles.
    #[error("mesh is structurally invalid: {0}")]
    StructurallyInvalid(String),

    /// The operation is defined but this particular backend does not implement
    /// it. The dispatcher uses this to fall back to another backend rather than
    /// failing the whole model.
    #[error("operation not supported by backend `{backend}`: {operation}")]
    Unsupported {
        backend: &'static str,
        operation: &'static str,
    },

    /// The algorithm ran but could not produce a reliable result (degenerate
    /// configuration, coplanar overlap it cannot resolve).
    #[error("numerically degenerate input: {0}")]
    Degenerate(String),
}
