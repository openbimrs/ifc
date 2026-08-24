use crate::{Codec, Model, ModelError};

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

    let available = codecs();
    for codec in &available {
        if codec.detect(&bytes) {
            return codec.read_bytes(&bytes);
        }
    }
    for codec in &available {
        if codec.extensions().contains(&extension.as_str()) {
            return codec.read_bytes(&bytes);
        }
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
