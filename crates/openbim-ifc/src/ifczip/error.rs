//! Why an ifcZIP archive was refused.

use std::fmt;

use ifc_model::ModelError;

use super::Limit;

/// Why an ifcZIP archive could not be read or written.
///
/// Each refusal is its own variant: an archive that is not a ZIP, one with
/// no model or several, an entry in a format this library does not read,
/// one it cannot decrypt or decompress, and one larger than the
/// decompression [`Budget`](super::Budget) are different problems with
/// different remedies.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum IfcZipError {
    /// The bytes are not a readable ZIP archive (a damaged central
    /// directory, a checksum mismatch), or not one at all.
    Malformed(String),
    /// No model entry: no `.ifc` or `.ifcXML` file at the archive root.
    NoModel {
        /// Every file entry the archive does hold.
        entries: Vec<String>,
    },
    /// Several model entries at the archive root; an ifcZIP holds one.
    SeveralModels {
        /// The model entries' names.
        names: Vec<String>,
    },
    /// The entry is neither a STEP physical file nor an ifcXML document.
    UnknownInnerFormat {
        /// The entry's name.
        name: String,
    },
    /// The entry is encrypted; ifcZIP excludes encryption.
    Encrypted {
        /// The entry's name.
        name: String,
    },
    /// The entry is compressed with a method other than Stored or Deflate.
    UnsupportedCompression {
        /// The entry's name.
        name: String,
        /// The method, by name and number, e.g. `Deflate64 (9)`.
        method: String,
    },
    /// The entry inflates past the decompression budget.
    BudgetExceeded {
        /// The entry's name.
        name: String,
        /// The limit it exceeded.
        limit: Limit,
    },
    /// The entry is in a format whose codec this build leaves out.
    CodecNotCompiled {
        /// The entry's name.
        name: String,
        /// The codec, `STEP` or `ifcXML`.
        codec: &'static str,
        /// The facade feature that compiles it in.
        feature: &'static str,
    },
    /// The entry's codec refused the document it holds.
    Read {
        /// The entry's name.
        name: String,
        /// The codec's error, as text; the [`Codec`](ifc_model::Codec)
        /// interface returns it as the codec's own `ModelError`.
        detail: String,
    },
    /// An entry name an ifcZIP reader would not take as its model: it must
    /// sit at the archive root and end in `.ifc` or `.ifcXML`.
    InvalidEntryName(String),
    /// The model or the archive could not be written.
    Write(String),
}

impl fmt::Display for IfcZipError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Malformed(detail) => write!(f, "not a readable ZIP archive: {detail}"),
            Self::NoModel { entries } => write!(
                f,
                "no .ifc or .ifcXML entry at the archive root (entries: {})",
                listed(entries)
            ),
            Self::SeveralModels { names } => write!(
                f,
                "several model entries ({}); an ifcZIP holds one",
                listed(names)
            ),
            Self::UnknownInnerFormat { name } => write!(
                f,
                "entry {name:?} is neither a STEP physical file nor an ifcXML document"
            ),
            Self::Encrypted { name } => {
                write!(f, "entry {name:?} is encrypted; ifcZIP excludes encryption")
            }
            Self::UnsupportedCompression { name, method } => write!(
                f,
                "entry {name:?} is compressed with {method}; ifcZIP uses Stored or Deflate"
            ),
            Self::BudgetExceeded { name, limit } => {
                write!(f, "entry {name:?} inflates past {limit}")
            }
            Self::CodecNotCompiled {
                name,
                codec,
                feature,
            } => write!(
                f,
                "entry {name:?} is {codec}, and this build leaves out the `{feature}` feature"
            ),
            Self::Read { name, detail } => write!(f, "entry {name:?}: {detail}"),
            Self::InvalidEntryName(name) => write!(
                f,
                "entry name {name:?} is not a .ifc or .ifcXML file at the archive root"
            ),
            Self::Write(detail) => write!(f, "cannot write the ifcZIP archive: {detail}"),
        }
    }
}

impl std::error::Error for IfcZipError {}

/// A model-level error: every archive refusal as [`ModelError::WrongFormat`]
/// naming ifcZIP and the refusal. (The [`Codec`](ifc_model::Codec)
/// interface returns an inner codec's refusal as that codec's own error.)
impl From<IfcZipError> for ModelError {
    fn from(error: IfcZipError) -> Self {
        match error {
            IfcZipError::Write(detail) => ModelError::Write(detail),
            other => ModelError::WrongFormat {
                expected: "ifcZIP",
                detail: other.to_string(),
            },
        }
    }
}

fn listed(names: &[String]) -> String {
    if names.is_empty() {
        "none".to_owned()
    } else {
        names.join(", ")
    }
}
