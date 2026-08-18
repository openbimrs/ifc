//! Why a lowering failed.
//!
//! Structured per representation family so a failure names the entity and the
//! reason, rather than collapsing to a generic "bad geometry".

use thiserror::Error;

/// Why an IFC representation item could not be lowered to geometry.
///
/// A model that partly fails is normal: real files contain unsupported or
/// malformed items, and one bad wall must not abort the import. Callers are
/// expected to collect these per entity and continue.
#[derive(Debug, Error, PartialEq)]
pub enum ShapeError {
    /// The representation type is valid IFC but not yet lowered by this crate.
    #[error("representation `{0}` is not yet supported")]
    Unsupported(&'static str),

    /// A placement chain referenced itself.
    #[error("cyclic placement chain at entity #{0}")]
    CyclicPlacement(u64),

    /// A mapped item nested beyond the allowed depth.
    #[error("mapped item nesting exceeded depth {0}")]
    MappedItemTooDeep(u32),

    /// The geometry kernel rejected the operation; carries its message.
    #[error("kernel error: {0}")]
    Kernel(String),
}
