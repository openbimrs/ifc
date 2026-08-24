//! `ifc-step` — the STEP physical file (ISO 10303-21) codec.
//!
//! # This crate is a codec, not the model
//!
//! It implements [`ifc_model::Codec`] and owns no data model of its own. The
//! entity graph lives in `ifc-model`; this crate only translates between that
//! graph and `.ifc` text. An ifcXML or IFC-JSON codec is a sibling crate
//! implementing the same trait, which is what makes format conversion a matter
//! of "read with one, write with another".
//!
//! # What it deliberately does not know
//!
//! The parser understands STEP *syntax* only — it never asks what an entity
//! means. An entity type introduced in a future schema parses correctly here
//! with no change, which is what allows unknown data to survive a round-trip.
//!
//! # Modules
//!
//! | Module | Role |
//! | --- | --- |
//! | [`lexer`] | Byte-level tokenizer: comments, escapes, STEP number forms |
//! | [`parser`] | Tokens to `Model`, including the `HEADER;` section |
//! | [`writer`] | `Model` back to STEP text |
//! | [`escape`] | The `\S\`, `\X\`, `\X2\`, `\X4\` string codec |
//! | [`header`] | File magic detection |
//! | [`partition`] | Record-aligned splitting for the parallel scan |
//! | [`error`] | Failure modes |

pub mod error;
pub mod escape;
pub mod header;
pub mod lexer;
pub mod parser;
pub mod partition;
pub mod writer;

pub use error::StepError;
pub use header::is_step_file;

use ifc_model::{Codec, Model, ModelError};
use std::io::Write;
use std::path::Path;

/// The STEP physical file codec.
///
/// Zero-sized: configuration would go here, but there is none yet.
#[derive(Debug, Clone, Copy, Default)]
pub struct StepCodec;

impl Codec for StepCodec {
    fn name(&self) -> &'static str {
        "STEP"
    }

    fn extensions(&self) -> &'static [&'static str] {
        &["ifc", "step", "stp"]
    }

    fn detect(&self, bytes: &[u8]) -> bool {
        is_step_file(bytes)
    }

    fn read_bytes(&self, bytes: &[u8]) -> Result<Model, ModelError> {
        if !is_step_file(bytes) {
            return Err(ModelError::WrongFormat {
                expected: "STEP",
                detail: "missing ISO-10303-21 magic".into(),
            });
        }
        parser::parse(bytes).map_err(Into::into)
    }

    fn write(&self, model: &Model, out: &mut dyn Write) -> Result<(), ModelError> {
        writer::write(model, out).map_err(|e| ModelError::Write(e.to_string()))
    }

    /// Memory-maps the file rather than reading it into a heap buffer.
    ///
    /// Large models are hundreds of megabytes; mapping avoids a full copy and
    /// lets the OS page in only what the parse touches.
    fn read_path(&self, path: &Path) -> Result<Model, ModelError> {
        let file = std::fs::File::open(path).map_err(|e| ModelError::Io(e.to_string()))?;
        // SAFETY: the file is opened read-only and not mutated for the
        // lifetime of the mapping; truncation by another process would be
        // required to invalidate it, which we accept as out of scope.
        let mmap =
            unsafe { memmap2::Mmap::map(&file) }.map_err(|e| ModelError::Io(e.to_string()))?;
        self.read_bytes(&mmap)
    }
}
