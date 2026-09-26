use crate::{Codec, Model, ModelError};

/// Parse a `.ifc` (STEP physical file) buffer without importing [`Codec`] or
/// naming [`ifc_step::StepCodec`] directly.
///
/// Delegates to `StepCodec::read_bytes`; exists because "just parse these
/// bytes" is the overwhelmingly common entry point and forcing every
/// consumer to `use ifc::Codec` first for a single call is friction the
/// facade can absorb.
#[cfg(feature = "step")]
pub fn from_step_bytes(bytes: &[u8]) -> Result<Model, ModelError> {
    ifc_step::StepCodec.read_bytes(bytes)
}

/// Every codec compiled into this build.
///
/// Lets an application accept whatever the user hands it without hard-coding a
/// format, and shrinks to nothing when only one codec is enabled.
// Each push is `cfg`-gated, so clippy's `vec![]` suggestion is not applicable:
// the contents depend on which features are enabled at compile time.
#[allow(clippy::vec_init_then_push)]
pub fn codecs() -> Vec<Box<dyn Codec>> {
    #[allow(unused_mut)]
    let mut out: Vec<Box<dyn Codec>> = Vec::new();
    #[cfg(feature = "step")]
    out.push(Box::new(ifc_step::StepCodec));
    #[cfg(feature = "ifcxml")]
    out.push(Box::new(ifc_xml::XmlCodec::default()));
    out
}

/// Read a file, choosing the codec by content sniffing then extension.
///
/// The file is read into a buffer the model owns, not memory-mapped: a
/// lazily loaded model decodes from it for its whole lifetime. The mapped
/// read is `ifc_step::StepReader::read_path_mapped`, an `unsafe` opt-in.
///
/// Returns [`ModelError::WrongFormat`] when no compiled-in codec recognizes the
/// input, which is a more useful failure than a syntax error from the wrong
/// parser.
pub fn read_path(path: &std::path::Path) -> Result<Model, ModelError> {
    let bytes = std::fs::read(path).map_err(|e| ModelError::Io(e.to_string()))?;
    let extension = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();

    // The chosen codec takes the buffer (`read_owned`): a lazily loading
    // codec keeps it as the model's source instead of copying it again.
    let available = codecs();
    let chosen = available
        .iter()
        .position(|codec| codec.detect(&bytes))
        .or_else(|| {
            available
                .iter()
                .position(|codec| codec.extensions().contains(&extension.as_str()))
        });
    if let Some(index) = chosen {
        return available[index].read_owned(bytes);
    }
    Err(ModelError::WrongFormat {
        expected: "IFC",
        detail: format!(
            "no compiled-in codec recognized this input (available: {})",
            available
                .iter()
                .map(|c| c.name())
                .collect::<Vec<_>>()
                .join(", ")
        ),
    })
}
