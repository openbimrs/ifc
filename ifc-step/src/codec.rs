//! The STEP codec: format detection, parse policy, and model I/O.
//!
//! Two types rather than one configurable type. [`StepCodec`] is the strict
//! reader and stays zero-sized, so it can be named as a value wherever a codec
//! is needed. [`StepReader`] carries an explicit [`ParseOptions`] for consumers
//! that opt into malformed-record recovery. Both implement [`Codec`], so either
//! can be stored in a `Box<dyn Codec>` alongside the other formats.

use crate::{parser, writer};
use ifc_model::{Codec, Model, ModelError};
use openbim_step::{is_step_file, OnMalformed, ParseOptions};
use std::io::Write;
use std::path::Path;

/// The STEP physical file codec.
///
/// Strict: a record this codec cannot read is an error, because an authoring
/// tool that silently drops entities corrupts the file it edits.
///
/// A consumer that would rather load a damaged export uses
/// [`StepCodec::lenient`] and reads [`Model::diagnostics`] to see what was
/// dropped.
///
/// ```
/// use ifc_model::Codec;
/// use ifc_step::StepCodec;
///
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// # let bytes = b"ISO-10303-21;\nHEADER;\nFILE_DESCRIPTION((''),'2;1');\n\
/// # FILE_NAME('n','t',(''),(''),'p','o','a');\nFILE_SCHEMA(('IFC4'));\nENDSEC;\n\
/// # DATA;\n#1= IFCPERSON($,$,'a',$,$,$,$,$);\n#2\nENDSEC;\nEND-ISO-10303-21;\n";
/// // A damaged export: strict reading refuses it.
/// assert!(StepCodec.read_bytes(bytes).is_err());
///
/// // A viewer opts into recovery and reports what was lost.
/// let model = StepCodec::lenient().read_bytes(bytes)?;
/// assert_eq!(model.len(), 1);
/// assert_eq!(model.diagnostics().len(), 1);
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Clone, Copy, Default)]
pub struct StepCodec;

impl StepCodec {
    /// A reader that skips unreadable data records and reports each one as a
    /// [`Model`] diagnostic.
    ///
    /// Header structure and the physical-file marker remain fatal: a file
    /// whose identity cannot be established is not partially readable.
    #[must_use]
    pub const fn lenient() -> StepReader {
        StepReader::new(ParseOptions::lenient())
    }

    /// A reader with an explicit malformed-record policy.
    #[must_use]
    pub const fn with_options(options: ParseOptions) -> StepReader {
        StepReader::new(options)
    }
}

/// A STEP codec carrying an explicit parse policy.
#[derive(Debug, Clone, Copy, Default)]
pub struct StepReader {
    options: ParseOptions,
}

impl StepReader {
    /// A reader applying `options`.
    #[must_use]
    pub const fn new(options: ParseOptions) -> Self {
        Self { options }
    }

    /// Sets the malformed-record policy.
    #[must_use]
    pub const fn on_malformed_record(mut self, policy: OnMalformed) -> Self {
        self.options = self.options.on_malformed_record(policy);
        self
    }

    /// The policy this reader applies.
    #[must_use]
    pub const fn options(&self) -> ParseOptions {
        self.options
    }
}

impl Codec for StepCodec {
    fn name(&self) -> &'static str {
        StepReader::new(ParseOptions::strict()).name()
    }

    fn extensions(&self) -> &'static [&'static str] {
        StepReader::new(ParseOptions::strict()).extensions()
    }

    fn detect(&self, bytes: &[u8]) -> bool {
        is_step_file(bytes)
    }

    fn read_bytes(&self, bytes: &[u8]) -> Result<Model, ModelError> {
        StepReader::new(ParseOptions::strict()).read_bytes(bytes)
    }

    fn write(&self, model: &Model, out: &mut dyn Write) -> Result<(), ModelError> {
        writer::write(model, out).map_err(|e| ModelError::Write(e.to_string()))
    }

    fn read_path(&self, path: &Path) -> Result<Model, ModelError> {
        StepReader::new(ParseOptions::strict()).read_path(path)
    }
}

impl Codec for StepReader {
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
        parser::parse(bytes, self.options).map_err(Into::into)
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
