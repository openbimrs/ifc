//! Codec state and the shared model-codec adapter.

use crate::{reader, writer, XmlProfile};
use ifc_model::{Codec, Model, ModelError};

/// The ifcXML codec.
///
/// Construct with [`XmlCodec::default`] for the lossless compatibility dialect,
/// [`XmlCodec::strict`] for an exact namespace/release profile, or
/// [`XmlCodec::with_schema_and_profile`] for strict output with schema-correct
/// attribute names.
#[derive(Default)]
pub struct XmlCodec {
    profile: Option<XmlProfile>,
    #[cfg(feature = "schema")]
    schema: Option<std::sync::Arc<ifc_schema::Schema>>,
}

impl XmlCodec {
    /// A codec that enforces one exact ifcXML release namespace and schema token.
    #[must_use]
    pub const fn strict(profile: XmlProfile) -> Self {
        Self {
            profile: Some(profile),
            #[cfg(feature = "schema")]
            schema: None,
        }
    }

    /// The strict release profile, or `None` for compatibility mode.
    #[must_use]
    pub const fn profile(&self) -> Option<XmlProfile> {
        self.profile
    }

    /// A codec that emits schema-correct attribute names.
    #[cfg(feature = "schema")]
    #[must_use]
    pub fn with_schema(schema: std::sync::Arc<ifc_schema::Schema>) -> Self {
        Self {
            profile: None,
            schema: Some(schema),
        }
    }

    /// A strict release-profile codec with schema-backed attribute names.
    #[cfg(feature = "schema")]
    #[must_use]
    pub fn with_schema_and_profile(
        schema: std::sync::Arc<ifc_schema::Schema>,
        profile: XmlProfile,
    ) -> Self {
        Self {
            profile: Some(profile),
            schema: Some(schema),
        }
    }

    /// The schema in use, if any.
    #[cfg(feature = "schema")]
    #[must_use]
    pub fn schema(&self) -> Option<&ifc_schema::Schema> {
        self.schema.as_deref()
    }
}

impl Codec for XmlCodec {
    fn name(&self) -> &'static str {
        "ifcXML"
    }

    fn extensions(&self) -> &'static [&'static str] {
        &["ifcxml", "xml"]
    }

    fn detect(&self, bytes: &[u8]) -> bool {
        reader::looks_like_xml(bytes)
    }

    fn read_bytes(&self, bytes: &[u8]) -> Result<Model, ModelError> {
        reader::read(self, bytes).map_err(|error| ModelError::Syntax {
            offset: 0,
            detail: error.to_string(),
        })
    }

    fn write(&self, model: &Model, out: &mut dyn std::io::Write) -> Result<(), ModelError> {
        let bytes =
            writer::write(self, model).map_err(|error| ModelError::Write(error.to_string()))?;
        out.write_all(&bytes)
            .map_err(|error| ModelError::Io(error.to_string()))
    }
}
