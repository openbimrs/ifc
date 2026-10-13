//! ifcZIP behind the [`Codec`] interface.

use std::fmt;
use std::sync::Arc;

use ifc_model::{Codec, Model, ModelError};

use super::{extract, pack, read_entry_as_model, Budget, Failure, IfcZipError, InnerFormat};

/// The ifcZIP codec (`.ifczip`).
///
/// Reads an archive holding STEP or ifcXML, whichever it holds, within its
/// [`Budget`]; writes the model with the codec it wraps and packs that
/// document as one Deflate entry. [`IfcZipCodec::default`] writes STEP as
/// `model.ifc`, as IfcOpenShell does; [`IfcZipCodec::wrapping`] writes
/// with any codec, such as an ifcXML codec in either layout.
///
/// Through the [`Codec`] interface an archive refusal is a
/// [`ModelError::WrongFormat`] naming ifcZIP and the refusal, and the inner
/// codec's refusal is that codec's own error; [`super::read`] returns the
/// typed [`IfcZipError`] instead.
#[derive(Clone)]
pub struct IfcZipCodec {
    budget: Budget,
    writer: Arc<dyn Codec + Send + Sync>,
    entry_name: String,
}

impl IfcZipCodec {
    /// A codec that writes with `codec` as the entry `entry_name`.
    ///
    /// # Errors
    ///
    /// [`IfcZipError::InvalidEntryName`] for a name a reader would not take
    /// as the model: not at the archive root, or not ending in `.ifc` or
    /// `.ifcXML`.
    pub fn wrapping(
        codec: impl Codec + Send + Sync + 'static,
        entry_name: impl Into<String>,
    ) -> Result<Self, IfcZipError> {
        let entry_name = entry_name.into();
        // `pack` refuses the same names; checking here fails at
        // construction rather than at the first write.
        if !super::archive::is_model_name(&entry_name) {
            return Err(IfcZipError::InvalidEntryName(entry_name));
        }
        Ok(Self {
            budget: Budget::DEFAULT,
            writer: Arc::new(codec),
            entry_name,
        })
    }

    /// This codec with a different decompression budget.
    #[must_use]
    pub fn with_budget(mut self, budget: Budget) -> Self {
        self.budget = budget;
        self
    }

    /// The decompression budget reads keep to.
    #[must_use]
    pub const fn budget(&self) -> &Budget {
        &self.budget
    }

    /// The name of the entry a write packs the document as.
    #[must_use]
    pub fn entry_name(&self) -> &str {
        &self.entry_name
    }

    /// The name of the codec a write uses for the document.
    #[must_use]
    pub fn inner_codec(&self) -> &'static str {
        self.writer.name()
    }

    /// The format of the document a write packs, `None` for a codec that
    /// writes neither STEP nor ifcXML (which `pack` then refuses).
    #[must_use]
    pub fn inner_format(&self) -> Option<InnerFormat> {
        match self.writer.name() {
            "STEP" => Some(InnerFormat::Step),
            "ifcXML" => Some(InnerFormat::IfcXml),
            _ => None,
        }
    }
}

/// STEP as `model.ifc`.
#[cfg(feature = "step")]
impl Default for IfcZipCodec {
    fn default() -> Self {
        Self {
            budget: Budget::DEFAULT,
            writer: Arc::new(ifc_step::StepCodec),
            entry_name: InnerFormat::Step.default_entry_name().to_owned(),
        }
    }
}

impl fmt::Debug for IfcZipCodec {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("IfcZipCodec")
            .field("budget", &self.budget)
            .field("writer", &self.writer.name())
            .field("entry_name", &self.entry_name)
            .finish()
    }
}

impl Codec for IfcZipCodec {
    fn name(&self) -> &'static str {
        "ifcZIP"
    }

    fn extensions(&self) -> &'static [&'static str] {
        &["ifczip"]
    }

    fn detect(&self, bytes: &[u8]) -> bool {
        super::is_zip(bytes)
    }

    fn read_bytes(&self, bytes: &[u8]) -> Result<Model, ModelError> {
        let entry = extract(bytes, &self.budget)?;
        read_entry_as_model(entry).map_err(|failure| match failure {
            Failure::Zip(error) => error.into(),
            Failure::Codec(error) => error,
        })
    }

    fn write(&self, model: &Model, out: &mut dyn std::io::Write) -> Result<(), ModelError> {
        let document = self.writer.write_bytes(model)?;
        let archive = pack(&self.entry_name, &document)?;
        out.write_all(&archive)
            .map_err(|error| ModelError::Io(error.to_string()))
    }
}
