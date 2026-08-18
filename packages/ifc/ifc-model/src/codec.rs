//! The serialization seam.
//!
//! # Why a trait, and why it lives here
//!
//! IFC is a *data model* with several concrete encodings: STEP physical file
//! (`.ifc`), ifcXML (`.ifcxml`), and prospectively IFC-JSON. They differ only
//! in syntax — the entity graph is identical.
//!
//! Defining [`Codec`] in `ifc-model` means:
//!
//! - the data model never depends on any particular encoding;
//! - a new encoding is a new crate implementing this trait, with no change
//!   here and no change to any consumer;
//! - conversion between encodings is free — parse with one, write with
//!   another, because both speak [`Model`].
//!
//! The inverse layering (a model that depends on the STEP reader) would make
//! ifcXML support a second parallel stack and make cross-format conversion
//! lossy.

use crate::error::ModelError;
use crate::model::Model;
use std::io::{Read, Write};
use std::path::Path;

/// Read and write one concrete IFC serialization.
///
/// Implementors are stateless; they carry configuration only.
pub trait Codec {
    /// Human-readable name for diagnostics, e.g. `STEP`.
    fn name(&self) -> &'static str;

    /// Conventional file extensions, lower-case, without the dot.
    fn extensions(&self) -> &'static [&'static str];

    /// Does this look like a file this codec can read?
    ///
    /// Content sniffing, so a file with the wrong extension still opens.
    /// Defaults to `false` — a codec that cannot cheaply recognize its own
    /// format should say so rather than claim every input, which would make
    /// codec selection order-dependent.
    fn detect(&self, _bytes: &[u8]) -> bool {
        false
    }

    /// Parse a model from bytes.
    ///
    /// Bytes rather than `&str` because IFC files are not guaranteed UTF-8:
    /// STEP escapes non-ASCII text, and a stray raw byte must not abort the
    /// parse of an otherwise valid file.
    fn read_bytes(&self, bytes: &[u8]) -> Result<Model, ModelError>;

    /// Serialize a model.
    fn write(&self, model: &Model, out: &mut dyn Write) -> Result<(), ModelError>;

    /// Parse from any reader. Override when the format can stream.
    fn read_from(&self, reader: &mut dyn Read) -> Result<Model, ModelError> {
        let mut buf = Vec::new();
        reader
            .read_to_end(&mut buf)
            .map_err(|e| ModelError::Io(e.to_string()))?;
        self.read_bytes(&buf)
    }

    /// Parse a file from disk.
    ///
    /// Override this when the codec can memory-map instead of reading into a
    /// heap buffer; `ifc-step` does exactly that for large models.
    fn read_path(&self, path: &Path) -> Result<Model, ModelError> {
        let bytes = std::fs::read(path).map_err(|e| ModelError::Io(e.to_string()))?;
        self.read_bytes(&bytes)
    }

    /// Serialize to a file on disk.
    fn write_path(&self, model: &Model, path: &Path) -> Result<(), ModelError> {
        let mut file = std::fs::File::create(path).map_err(|e| ModelError::Io(e.to_string()))?;
        self.write(model, &mut file)
    }

    /// Serialize to a byte vector.
    fn write_bytes(&self, model: &Model) -> Result<Vec<u8>, ModelError> {
        let mut out = Vec::new();
        self.write(model, &mut out)?;
        Ok(out)
    }
}
