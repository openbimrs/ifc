//! The STEP codec: format detection, parse policy, and model I/O.
//!
//! Two types rather than one configurable type. [`StepCodec`] is the strict
//! reader and stays zero-sized, so it can be named as a value wherever a codec
//! is needed. [`StepReader`] carries an explicit [`ParseOptions`] for consumers
//! that opt into malformed-record recovery. Both implement [`Codec`], so either
//! can be stored in a `Box<dyn Codec>` alongside the other formats.
//!
//! # Lazy loading
//!
//! A strict read validates every record and then decodes each entity on
//! first access (see `lazy.rs`): opening a file costs a syntax pass instead of
//! building every value, and memory holds the source plus what was touched.
//! The model keeps its source, so [`Codec::read_bytes`] copies the input once;
//! [`Codec::read_owned`] and [`Codec::read_path`] hand over or read a buffer
//! without that copy. [`StepReader::eager`] restores decode-everything-now.

use crate::lazy::{self, Bytes};
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
    /// Decode every entity during the read instead of on first access.
    eager: bool,
}

impl StepReader {
    /// A reader applying `options`.
    #[must_use]
    pub const fn new(options: ParseOptions) -> Self {
        Self {
            options,
            eager: false,
        }
    }

    /// Decodes every entity during the read, as before lazy loading.
    ///
    /// For a consumer that will touch nearly every entity anyway and wants
    /// the source released after the read. Recovery and reference-check
    /// options always read eagerly.
    #[must_use]
    pub const fn eager(mut self) -> Self {
        self.eager = true;
        self
    }

    /// Reads a memory-mapped file.
    ///
    /// Avoids copying the file: pages are loaded from the page cache as
    /// they are touched and are not part of the process's own heap. A lazily
    /// loaded model keeps the mapping and decodes from it for as long as it
    /// lives.
    ///
    /// # Safety
    ///
    /// The file must not be modified or truncated while the returned model
    /// -- or any clone of it -- is alive. Truncation can end the process
    /// with `SIGBUS` when an entity is decoded; a rewrite makes decoding
    /// panic or, if the new bytes still parse, read other content. Prefer
    /// [`Codec::read_path`], which owns its copy, unless the file is known
    /// to stay put.
    ///
    /// # Errors
    ///
    /// As [`Codec::read_path`].
    pub unsafe fn read_path_mapped(&self, path: &Path) -> Result<Model, ModelError> {
        let file = std::fs::File::open(path).map_err(|e| ModelError::Io(e.to_string()))?;
        // SAFETY: the caller guarantees the file stays unchanged while the
        // mapping lives, which is the model's lifetime when it keeps it.
        let map =
            unsafe { memmap2::Mmap::map(&file) }.map_err(|e| ModelError::Io(e.to_string()))?;
        if !is_step_file(&map) {
            return Err(wrong_format());
        }
        if self.is_lazy() {
            return lazy::read(Bytes::Mapped(map)).map_err(Into::into);
        }
        parser::parse(&map, self.options).map_err(Into::into)
    }

    fn is_lazy(&self) -> bool {
        !self.eager && lazy::is_lazy(self.options)
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

    fn read_owned(&self, bytes: Vec<u8>) -> Result<Model, ModelError> {
        StepReader::new(ParseOptions::strict()).read_owned(bytes)
    }

    fn write(&self, model: &Model, out: &mut dyn Write) -> Result<(), ModelError> {
        writer::write(model, out).map_err(|e| ModelError::Write(e.to_string()))
    }

    fn read_path(&self, path: &Path) -> Result<Model, ModelError> {
        StepReader::new(ParseOptions::strict()).read_path(path)
    }
}

fn wrong_format() -> ModelError {
    ModelError::WrongFormat {
        expected: "STEP",
        detail: "missing ISO-10303-21 magic".into(),
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
            return Err(wrong_format());
        }
        if self.is_lazy() {
            // The model keeps its source, so the borrowed input is copied.
            return lazy::read(Bytes::Owned(bytes.to_vec())).map_err(Into::into);
        }
        parser::parse(bytes, self.options).map_err(Into::into)
    }

    fn read_owned(&self, bytes: Vec<u8>) -> Result<Model, ModelError> {
        if !is_step_file(&bytes) {
            return Err(wrong_format());
        }
        if self.is_lazy() {
            return lazy::read(Bytes::Owned(bytes)).map_err(Into::into);
        }
        parser::parse(&bytes, self.options).map_err(Into::into)
    }

    fn write(&self, model: &Model, out: &mut dyn Write) -> Result<(), ModelError> {
        writer::write(model, out).map_err(|e| ModelError::Write(e.to_string()))
    }

    /// Reads the file into a buffer the model owns.
    ///
    /// Not memory-mapped: a lazily loaded model decodes from its source for
    /// as long as it lives, and a mapping of a file that another process
    /// changes in that time is undefined behaviour. The mapped read is
    /// [`StepReader::read_path_mapped`], an `unsafe` opt-in.
    fn read_path(&self, path: &Path) -> Result<Model, ModelError> {
        let bytes = std::fs::read(path).map_err(|e| ModelError::Io(e.to_string()))?;
        self.read_owned(bytes)
    }
}
