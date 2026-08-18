//! The public façade: open a file, get a parsed model.
//!
//! Everything else in this crate is an implementation stage. Consumers should
//! need only this module, so the pipeline can be re-shaped internally without
//! breaking callers.
//!
//! Not yet implemented — Stage 1 in `docs/ROADMAP.md`.

use crate::error::ParseError;
use std::path::Path;

/// Read a STEP physical file from disk.
///
/// Will mmap rather than read-to-string: models routinely exceed 1 GB and the
/// scan is read-only, so paging beats copying.
pub fn open(_path: &Path) -> Result<(), ParseError> {
    unimplemented!("Stage 1: reader façade")
}
