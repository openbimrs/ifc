//! ifcZIP: one STEP or ifcXML model in a ZIP archive (#450).
//!
//! buildingSMART lists ifcZIP (`.ifcZIP`, `application/zip`) among the IFC
//! formats: "IFC data may be embedded within a ZIP file. The embedded data
//! may be encoded as either SPF or XML". Implementation agreement
//! CV-2x3-154 fixes the container: PKZIP 2.04g compression, which excludes
//! encryption and Deflate64, and "only a single *.ifc file (or only a
//! single ifcXML file) in the main directory".
//!
//! Reading takes that agreement literally and refuses everything else by
//! name ([`IfcZipError`]): no model entry, several, an entry that is
//! neither STEP nor ifcXML, an encrypted entry, a compression method other
//! than Stored or Deflate. Decompression is bounded by a [`Budget`], an
//! absolute size and a ratio to the compressed size, checked while
//! inflating, so a ZIP bomb is refused before it is held in memory.
//!
//! The inner document is read by the existing codecs: STEP by
//! `ifc-step`, ifcXML by `ifc-xml` in the layout its root element declares
//! ([`declared_xml_layout`](crate::xml_layout::declared_xml_layout)). Writing packs a
//! codec's output as one Deflate entry.
//!
//! [`IfcZipCodec`] puts the format behind the [`Codec`] interface, so
//! [`read_path`](crate::read_path) and [`codecs`](crate::codecs) take
//! ifcZIP like any other encoding.
//!
//! The ZIP signature check ([`is_zip`]) is always compiled, so a build
//! without the `zip` feature can still tell an archive from a damaged IFC
//! file; everything else needs the feature.

#[cfg(feature = "zip")]
use crate::{Codec, Model, ModelError};

#[cfg(feature = "zip")]
mod archive;
#[cfg(feature = "zip")]
mod budget;
#[cfg(feature = "zip")]
mod codec;
#[cfg(feature = "zip")]
mod error;

#[cfg(feature = "zip")]
pub use self::{
    archive::{extract, pack, Entry},
    budget::{Budget, Limit},
    codec::IfcZipCodec,
    error::IfcZipError,
};

/// A ZIP local file header's signature, the first four bytes of a ZIP
/// archive with at least one entry.
pub const SIGNATURE: [u8; 4] = *b"PK\x03\x04";

/// The signature of an empty ZIP archive: its end of central directory
/// record is all it holds.
pub const EMPTY_SIGNATURE: [u8; 4] = *b"PK\x05\x06";

/// Do `bytes` start like a ZIP archive, empty or not?
#[must_use]
pub fn is_zip(bytes: &[u8]) -> bool {
    bytes.starts_with(&SIGNATURE) || bytes.starts_with(&EMPTY_SIGNATURE)
}

/// The two serializations an ifcZIP may hold.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum InnerFormat {
    /// An ISO 10303-21 STEP physical file (`.ifc`).
    Step,
    /// An ISO 10303-28 ifcXML document (`.ifcXML`), either layout.
    IfcXml,
}

impl InnerFormat {
    /// The format's name: `STEP` or `ifcXML`.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Step => "STEP",
            Self::IfcXml => "ifcXML",
        }
    }

    /// The conventional entry name for this format: `model.ifc` or
    /// `model.ifcXML`.
    #[must_use]
    pub const fn default_entry_name(self) -> &'static str {
        match self {
            Self::Step => "model.ifc",
            Self::IfcXml => "model.ifcXML",
        }
    }

    /// The format of a document, from its first bytes: `ISO-10303-21;`
    /// opens a STEP file, an XML declaration or an `ifcXML` root an ifcXML
    /// document (after an optional byte order mark and white space).
    #[must_use]
    pub fn of_content(bytes: &[u8]) -> Option<Self> {
        let head = &bytes[..bytes.len().min(512)];
        let head = head.strip_prefix(b"\xEF\xBB\xBF").unwrap_or(head);
        let head = head.trim_ascii_start();
        if head.starts_with(b"ISO-10303-21") {
            return Some(Self::Step);
        }
        let xml_root = head.starts_with(b"<?xml")
            || head.starts_with(b"<ifcXML")
            || (head.starts_with(b"<")
                && head[1..]
                    .split(|byte| byte.is_ascii_whitespace() || *byte == b'>')
                    .next()
                    .is_some_and(|name| name.ends_with(b":ifcXML")));
        xml_root.then_some(Self::IfcXml)
    }
}

/// Read the model in an ifcZIP archive.
///
/// [`extract`]s the model entry within `budget`, then reads it with
/// [`read_entry`].
///
/// # Errors
///
/// As [`extract`] and [`read_entry`].
#[cfg(feature = "zip")]
pub fn read(bytes: &[u8], budget: &Budget) -> Result<Model, IfcZipError> {
    read_entry(extract(bytes, budget)?)
}

/// Read an extracted model entry with the codec its format needs.
///
/// STEP reads strictly through `ifc-step`. ifcXML reads in the layout the
/// document's root declares, as [`read_path`](crate::read_path) and the
/// command line do: the XSD configuration of its release when the root
/// declares a release namespace (which needs the release's bundled tables),
/// otherwise the native layout, with the declared release's tables when
/// this build bundles them.
///
/// # Errors
///
/// [`IfcZipError::CodecNotCompiled`] when the build leaves out the codec,
/// or the schema tables an XSD-layout read needs; [`IfcZipError::Read`]
/// when the codec refuses the document.
#[cfg(feature = "zip")]
pub fn read_entry(entry: Entry) -> Result<Model, IfcZipError> {
    let name = entry.name.clone();
    read_entry_as_model(entry).map_err(|failure| match failure {
        Failure::Zip(error) => error,
        Failure::Codec(error) => IfcZipError::Read {
            name,
            detail: error.to_string(),
        },
    })
}

/// A read that failed in the archive, or in the inner codec.
#[cfg(feature = "zip")]
pub(crate) enum Failure {
    Zip(IfcZipError),
    // Never built in a `zip` build with neither codec.
    #[cfg_attr(not(any(feature = "step", feature = "ifcxml")), allow(dead_code))]
    Codec(ModelError),
}

#[cfg(feature = "zip")]
pub(crate) fn read_entry_as_model(entry: Entry) -> Result<Model, Failure> {
    let not_compiled = |codec: &'static str, feature: &'static str| {
        Failure::Zip(IfcZipError::CodecNotCompiled {
            name: entry.name.clone(),
            codec,
            feature,
        })
    };
    match entry.format {
        InnerFormat::Step => {
            #[cfg(feature = "step")]
            return ifc_step::StepCodec
                .read_owned(entry.bytes)
                .map_err(Failure::Codec);
            #[cfg(not(feature = "step"))]
            return Err(not_compiled("STEP", "step"));
        }
        InnerFormat::IfcXml => {
            #[cfg(feature = "ifcxml")]
            {
                let _ = not_compiled;
                let codec = xml_reader(&entry.bytes, &entry.name)?;
                codec.read_owned(entry.bytes).map_err(Failure::Codec)
            }
            #[cfg(not(feature = "ifcxml"))]
            Err(not_compiled("ifcXML", "ifcxml"))
        }
    }
}

/// The ifcXML codec for the layout `bytes` declares.
///
/// Refused when the document declares the XSD layout of a release this
/// build has no tables for: the schema API left out (`CodecNotCompiled`),
/// or the release not bundled (a codec error naming it).
#[cfg(all(feature = "zip", feature = "ifcxml"))]
fn xml_reader(bytes: &[u8], name: &str) -> Result<ifc_xml::XmlCodec, Failure> {
    use crate::xml_layout::{declared_xml_layout, DeclaredXmlLayout};
    use ifc_xml::XmlCodec;

    match declared_xml_layout(bytes) {
        #[cfg(feature = "schema-api")]
        DeclaredXmlLayout::Xsd(profile) => ifc_schema::for_version(profile.version())
            .map(|schema| XmlCodec::xsd(std::sync::Arc::new(schema.clone()), profile))
            .map_err(|refused| {
                Failure::Codec(ModelError::WrongFormat {
                    expected: "ifcXML",
                    detail: format!(
                        "the XSD layout of {} needs its schema tables: {refused}",
                        profile.schema_token()
                    ),
                })
            }),
        #[cfg(not(feature = "schema-api"))]
        DeclaredXmlLayout::Xsd(_) => Err(Failure::Zip(IfcZipError::CodecNotCompiled {
            name: name.to_owned(),
            codec: "ifcXML in the XSD layout",
            feature: "schema-api",
        })),
        // Named attributes need the release's tables to find their slots; a
        // document written without a schema reads without one. Positional
        // names (`a0`, written without tables) read leniently, which
        // resolves schema and positional names both, as the codec without
        // tables would; schema-named documents read strictly.
        #[cfg(feature = "schema-api")]
        DeclaredXmlLayout::Native { schema, positional } => {
            let _ = name;
            let tables = ifc_schema::SchemaVersion::from_header_token(&schema)
                .and_then(|version| ifc_schema::for_version(version).ok());
            Ok(match tables {
                Some(tables) => {
                    let codec = XmlCodec::with_schema(std::sync::Arc::new(tables.clone()));
                    if positional {
                        codec.with_reading(ifc_xml::SchemaReading::Lenient)
                    } else {
                        codec
                    }
                }
                None => XmlCodec::default(),
            })
        }
        _ => {
            let _ = name;
            Ok(XmlCodec::default())
        }
    }
}

/// Write `model` with `codec` and pack the document as the ifcZIP entry
/// `entry_name` ([`pack`]).
///
/// # Errors
///
/// [`IfcZipError::Write`] when the codec refuses the model, otherwise as
/// [`pack`].
#[cfg(feature = "zip")]
pub fn write(model: &Model, codec: &dyn Codec, entry_name: &str) -> Result<Vec<u8>, IfcZipError> {
    let document = codec
        .write_bytes(model)
        .map_err(|error| IfcZipError::Write(format!("{}: {error}", codec.name())))?;
    pack(entry_name, &document)
}
