//! Codec state and the shared model-codec adapter.

use crate::{reader, writer, XmlProfile};
use ifc_model::{Codec, Model, ModelError};

/// Which XML layout a codec reads and writes.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
#[non_exhaustive]
pub enum XmlLayout {
    /// This crate's own lossless layout: one top-level element per entity,
    /// `i<n>` ids and references, explicit `kind` markers where an attribute
    /// string could not carry a value's kind. Reads and writes.
    #[default]
    Native,
    /// The buildingSMART ifcXML configuration of ISO 10303-28 that the
    /// release XSD declares: entities nested and defined in place, `ref` /
    /// `href` references, inverse attributes, `-wrapper` typed values and
    /// space-separated list attributes. Reads and writes, always
    /// schema-strict; written documents validate against the release XSD.
    Xsd,
}

/// How a native-layout codec with a schema treats content the schema does
/// not declare.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
#[non_exhaustive]
pub enum SchemaReading {
    /// Every value is typed from its attribute's declaration, never from its
    /// text, and an entity, attribute or value the schema does not declare
    /// is a typed error naming it. The default with a schema.
    #[default]
    Strict,
    /// The pre-0.4 behaviour: names resolve through the schema when they
    /// can, values are inferred from their text, and unknown names are kept
    /// after the declared slots. Round-trips models the schema does not
    /// describe, at the price of reading `Name="1"` as an integer.
    Lenient,
}

/// The ifcXML codec.
///
/// Construct with [`XmlCodec::default`] for the lossless compatibility dialect,
/// [`XmlCodec::strict`] for an exact namespace/release profile,
/// [`XmlCodec::with_schema_and_profile`] for strict output with schema-correct
/// attribute names, or [`XmlCodec::xsd`] to read and write the buildingSMART
/// XSD configuration.
#[derive(Debug, Clone, Default)]
pub struct XmlCodec {
    profile: Option<XmlProfile>,
    layout: XmlLayout,
    reading: SchemaReading,
    #[cfg(feature = "schema")]
    schema: Option<std::sync::Arc<ifc_schema::Schema>>,
}

impl XmlCodec {
    /// A codec that enforces one exact ifcXML release namespace and schema token.
    ///
    /// It writes and reads the native layout under that namespace. The
    /// output is well-formed and its root is the XSD's `ifcXML` element, but
    /// it is not valid against the release XSD: see [`XmlProfile`].
    #[must_use]
    pub const fn strict(profile: XmlProfile) -> Self {
        Self {
            profile: Some(profile),
            layout: XmlLayout::Native,
            reading: SchemaReading::Strict,
            #[cfg(feature = "schema")]
            schema: None,
        }
    }

    /// The strict release profile, or `None` for compatibility mode.
    #[must_use]
    pub const fn profile(&self) -> Option<XmlProfile> {
        self.profile
    }

    /// The XML layout this codec reads and writes.
    #[must_use]
    pub const fn layout(&self) -> XmlLayout {
        self.layout
    }

    /// How content the schema does not declare is treated. Applies to the
    /// native layout with a schema; the XSD layout is always strict.
    #[must_use]
    pub const fn reading(&self) -> SchemaReading {
        match self.layout {
            XmlLayout::Xsd => SchemaReading::Strict,
            XmlLayout::Native => self.reading,
        }
    }

    /// The same codec reading native documents with `reading`.
    ///
    /// [`SchemaReading::Lenient`] restores the pre-0.4 schema-aware read.
    /// Has no effect on an [`XmlLayout::Xsd`] codec, which has no lenient
    /// mode.
    #[must_use]
    pub const fn with_reading(mut self, reading: SchemaReading) -> Self {
        self.reading = reading;
        self
    }

    /// A codec that emits schema-correct attribute names and reads strictly.
    ///
    /// Reading types every value from its declaration and refuses names the
    /// schema does not declare ([`SchemaReading::Strict`]); chain
    /// [`Self::with_reading`] for the lenient read.
    #[cfg(feature = "schema")]
    #[must_use]
    pub fn with_schema(schema: std::sync::Arc<ifc_schema::Schema>) -> Self {
        Self {
            profile: None,
            layout: XmlLayout::Native,
            reading: SchemaReading::Strict,
            schema: Some(schema),
        }
    }

    /// A strict release-profile codec with schema-backed attribute names.
    ///
    /// Reads strictly, as [`Self::with_schema`] does. Like [`Self::strict`],
    /// it writes the native layout, not the XSD configuration.
    #[cfg(feature = "schema")]
    #[must_use]
    pub fn with_schema_and_profile(
        schema: std::sync::Arc<ifc_schema::Schema>,
        profile: XmlProfile,
    ) -> Self {
        Self {
            profile: Some(profile),
            layout: XmlLayout::Native,
            reading: SchemaReading::Strict,
            schema: Some(schema),
        }
    }

    /// A reader and writer of the buildingSMART XSD configuration of
    /// `profile`'s release.
    ///
    /// `schema` must be that release's schema; a mismatch is refused when
    /// reading or writing. A document reads into the same [`Model`] as its
    /// STEP form. A model writes as a document that validates against the
    /// release XSD and reads back to the same model, its entities numbered
    /// in model order; what the configuration cannot carry exactly is
    /// refused with [`crate::XmlError::Unrepresentable`] or another typed
    /// error, never written differently. The model's header must declare
    /// the profile's schema token.
    #[cfg(feature = "schema")]
    #[must_use]
    pub fn xsd(schema: std::sync::Arc<ifc_schema::Schema>, profile: XmlProfile) -> Self {
        Self {
            profile: Some(profile),
            layout: XmlLayout::Xsd,
            reading: SchemaReading::Strict,
            schema: Some(schema),
        }
    }

    /// The schema in use, if any.
    #[cfg(feature = "schema")]
    #[must_use]
    pub fn schema(&self) -> Option<&ifc_schema::Schema> {
        self.schema.as_deref()
    }

    /// The schema a native read types values from, when reading strictly.
    #[cfg(feature = "schema")]
    pub(crate) fn strict_schema(&self) -> Option<&ifc_schema::Schema> {
        match self.reading {
            SchemaReading::Strict => self.schema(),
            SchemaReading::Lenient => None,
        }
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
