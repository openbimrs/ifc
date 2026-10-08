//! Reading a model, the release it declares, and finding an entity in it.
//!
//! The codec is chosen by content first and extension second, as the
//! facade's `read_path` does, because the ifcXML layout has to be chosen
//! too: the native layout's root element carries a `schema` attribute, and
//! an XSD-configuration document declares its release's namespace. A
//! document that shows neither is refused rather than read in a guessed
//! layout; `--input-layout` names it explicitly.

use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;

use ifc::schema::for_version;
use ifc::{Codec, EntityId, Model, ModelError, Schema, SchemaVersion, StepCodec, XmlCodec};
use ifc::{XmlLayout, XmlProfile};

use crate::cli::InputLayout;
use crate::error::{CliError, CliResult};

/// The serialization a model was read from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Source {
    /// ISO 10303-21.
    Step,
    /// ISO 10303-28, in the given layout.
    IfcXml(XmlLayout),
}

/// A model read from a file, with what the reports need about its source.
#[derive(Debug)]
pub(crate) struct Loaded {
    /// The model.
    pub(crate) model: Model,
    /// The path as the user gave it.
    pub(crate) path: String,
    /// What it was read from.
    pub(crate) source: Source,
    /// For a STEP file, the line each `#id=` record starts on.
    pub(crate) lines: Option<LineIndex>,
}

/// Read `path` as STEP or ifcXML.
pub(crate) fn read(path: &Path, layout: InputLayout) -> CliResult<Loaded> {
    let shown = path.display().to_string();
    let bytes = std::fs::read(path).map_err(|error| CliError::Io {
        path: shown.clone(),
        detail: error.to_string(),
    })?;
    let extension = path
        .extension()
        .and_then(|extension| extension.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    let xml = XmlCodec::default();
    let is_xml = xml.detect(&bytes);
    let is_step = !is_xml
        && (StepCodec.detect(&bytes) || StepCodec.extensions().contains(&extension.as_str()));
    let parse = |error: ModelError| match error {
        ModelError::Io(detail) => CliError::Io {
            path: shown.clone(),
            detail,
        },
        other => CliError::Parse {
            path: shown.clone(),
            detail: other.to_string(),
        },
    };
    if is_step {
        let lines = LineIndex::of(&bytes);
        let model = StepCodec.read_owned(bytes).map_err(parse)?;
        return Ok(Loaded {
            model,
            path: shown,
            source: Source::Step,
            lines: Some(lines),
        });
    }
    if !is_xml && !xml.extensions().contains(&extension.as_str()) {
        return Err(CliError::Parse {
            path: shown,
            detail: "neither a STEP (ISO-10303-21) nor an ifcXML document".to_owned(),
        });
    }
    let codec = xml_reader(&bytes, layout).map_err(|error| match error {
        CliError::Unsupported(detail) => CliError::Parse {
            path: shown.clone(),
            detail,
        },
        other => other,
    })?;
    let source = Source::IfcXml(codec.layout());
    let model = codec.read_owned(bytes).map_err(parse)?;
    Ok(Loaded {
        model,
        path: shown,
        source,
        lines: None,
    })
}

/// The ifcXML codec that reads this document in the layout asked for.
fn xml_reader(bytes: &[u8], layout: InputLayout) -> CliResult<XmlCodec> {
    let root = root_tag(bytes);
    let profile = xsd_profile_of(&root);
    let schema_attribute = attribute(&root, "schema");
    let layout = match layout {
        InputLayout::Native => XmlLayout::Native,
        InputLayout::Xsd => XmlLayout::Xsd,
        InputLayout::Auto if schema_attribute.is_some() => XmlLayout::Native,
        InputLayout::Auto if profile.is_some() => XmlLayout::Xsd,
        InputLayout::Auto => {
            return Err(CliError::Unsupported(
                "cannot tell the ifcXML layout: the root element has no `schema` attribute \
                 (native layout) and no IFC4 or IFC4X3_ADD2 namespace (XSD layout); \
                 name it with --input-layout"
                    .to_owned(),
            ))
        }
    };
    if layout == XmlLayout::Xsd {
        let profile = profile.ok_or_else(|| {
            CliError::Unsupported(
                "an XSD-layout document must declare the IFC4 ADD2 TC1 or IFC4X3 ADD2 namespace"
                    .to_owned(),
            )
        })?;
        return Ok(XmlCodec::xsd(release_tables(profile.version())?, profile));
    }
    // Named attributes need the release's tables to find their slots; a
    // document written without a schema reads without one.
    let bundled = schema_attribute
        .as_deref()
        .and_then(SchemaVersion::from_header_token)
        .and_then(|version| release_tables(version).ok());
    Ok(bundled.map_or_else(XmlCodec::default, XmlCodec::with_schema))
}

/// The release a profile writes, as the shared tables an XML codec takes.
fn release_tables(version: SchemaVersion) -> CliResult<Arc<Schema>> {
    for_version(version)
        .map(|schema| Arc::new(schema.clone()))
        .map_err(|refused| {
            CliError::UnsupportedSchema(format!("{}: {refused}", version.release_id()))
        })
}

/// The XSD profile whose namespace the root tag declares.
fn xsd_profile_of(root: &str) -> Option<XmlProfile> {
    [XmlProfile::Ifc4Add2Tc1, XmlProfile::Ifc4x3Add2]
        .into_iter()
        .find(|profile| {
            profile
                .namespaces()
                .iter()
                .any(|namespace| root.contains(&format!("\"{namespace}\"")))
        })
}

/// The XSD profile of the release `version`, if the codec has one.
pub(crate) fn xsd_profile_for(version: SchemaVersion) -> Option<XmlProfile> {
    [XmlProfile::Ifc4Add2Tc1, XmlProfile::Ifc4x3Add2]
        .into_iter()
        .find(|profile| profile.version() == version)
}

/// The text of the document's root start tag, from `<` to `>`.
fn root_tag(bytes: &[u8]) -> String {
    let head = String::from_utf8_lossy(&bytes[..bytes.len().min(64 * 1024)]);
    let mut rest = head.as_ref();
    while let Some(start) = rest.find('<') {
        let after = &rest[start + 1..];
        // Declarations, processing instructions and comments come first.
        if after.starts_with('?') || after.starts_with('!') {
            let end = if after.starts_with("!--") {
                after.find("-->").map(|end| end + 3)
            } else {
                after.find('>').map(|end| end + 1)
            };
            match end {
                Some(end) => rest = &after[end..],
                None => return String::new(),
            }
            continue;
        }
        return after
            .find('>')
            .map_or(after, |end| &after[..end])
            .to_owned();
    }
    String::new()
}

/// The value of attribute `name` in a start tag.
fn attribute(tag: &str, name: &str) -> Option<String> {
    let mut rest = tag;
    while let Some(at) = rest.find(name) {
        let before = rest[..at].chars().next_back();
        let after = rest[at + name.len()..].trim_start();
        if before.is_some_and(char::is_whitespace) {
            if let Some(value) = after.strip_prefix('=') {
                let value = value.trim_start();
                let quote = value.chars().next()?;
                if quote == '"' || quote == '\'' {
                    let body = &value[1..];
                    return body.find(quote).map(|end| body[..end].to_owned());
                }
            }
        }
        rest = &rest[at + name.len()..];
    }
    None
}

/// The bundled tables of the release the header declares.
///
/// Refused when the header declares none, one this crate does not know, or
/// one this build bundles no tables for: answering from another release's
/// tables would be confident nonsense.
pub(crate) fn declared_schema(model: &Model) -> CliResult<&'static Schema> {
    let token = model.header().schema_token().ok_or_else(|| {
        CliError::UnsupportedSchema("the header declares no FILE_SCHEMA".to_owned())
    })?;
    let version = SchemaVersion::from_header_token(token).ok_or_else(|| {
        CliError::UnsupportedSchema(format!("unrecognised schema token {token:?}"))
    })?;
    for_version(version)
        .map_err(|refused| CliError::UnsupportedSchema(format!("{token}: {refused}")))
}

/// The entity `reference` names: `#12`, `12`, or a 22-character `GlobalId`.
pub(crate) fn find_entity(model: &Model, schema: &Schema, reference: &str) -> CliResult<EntityId> {
    let trimmed = reference.trim();
    let digits = trimmed.strip_prefix('#').unwrap_or(trimmed);
    if !digits.is_empty() && digits.bytes().all(|byte| byte.is_ascii_digit()) {
        let id = digits
            .parse::<u64>()
            .map(EntityId)
            .map_err(|_| CliError::MissingEntity(format!("#{digits} is out of range")))?;
        return if model.contains(id) {
            Ok(id)
        } else {
            Err(CliError::MissingEntity(format!(
                "#{} is not in the model",
                id.0
            )))
        };
    }
    if trimmed.len() != 22 {
        return Err(CliError::Usage(format!(
            "{trimmed:?} is neither an entity id (#12) nor a 22-character GlobalId"
        )));
    }
    let mut found = None;
    for id in model.ids() {
        let Some(identity) = ifc::root_identity(model, schema, id) else {
            continue;
        };
        if identity.global_id == Some(trimmed) {
            if let Some(EntityId(first)) = found {
                return Err(CliError::InvalidModel(format!(
                    "GlobalId {trimmed} is on both #{first} and #{}",
                    id.0
                )));
            }
            found = Some(id);
        }
    }
    found.ok_or_else(|| CliError::MissingEntity(format!("no entity has GlobalId {trimmed}")))
}

/// The line each STEP record starts on, for reports that point into the file.
#[derive(Debug, Default)]
pub(crate) struct LineIndex {
    lines: HashMap<u64, usize>,
}

impl LineIndex {
    /// Index the records of a STEP file: a line starting with `#<id>=`.
    pub(crate) fn of(bytes: &[u8]) -> Self {
        let mut lines = HashMap::new();
        for (index, line) in bytes.split(|byte| *byte == b'\n').enumerate() {
            let line = line.trim_ascii_start();
            let Some(rest) = line.strip_prefix(b"#") else {
                continue;
            };
            let digits = rest.iter().take_while(|byte| byte.is_ascii_digit()).count();
            if digits == 0 || !rest[digits..].trim_ascii_start().starts_with(b"=") {
                continue;
            }
            let id = std::str::from_utf8(&rest[..digits])
                .ok()
                .and_then(|text| text.parse::<u64>().ok());
            if let Some(id) = id {
                lines.entry(id).or_insert(index + 1);
            }
        }
        Self { lines }
    }

    /// The one-based line entity `id` starts on.
    pub(crate) fn line(&self, id: u64) -> Option<usize> {
        self.lines.get(&id).copied()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn records_are_indexed_by_their_line() {
        let index = LineIndex::of(b"DATA;\n#1=IFCWALL();\n  #20 = IFCSLAB();\n#3IFCX;\n");
        assert_eq!(index.line(1), Some(2));
        assert_eq!(index.line(20), Some(3));
        assert_eq!(index.line(3), None, "no `=`: not a record start");
    }

    #[test]
    fn the_root_tag_skips_the_prolog() {
        let tag =
            root_tag(b"<?xml version=\"1.0\"?>\n<!-- c > d -->\n<ifcXML schema=\"IFC4\" a='b'>");
        assert_eq!(tag, "ifcXML schema=\"IFC4\" a='b'");
        assert_eq!(attribute(&tag, "schema").as_deref(), Some("IFC4"));
        assert_eq!(attribute(&tag, "a").as_deref(), Some("b"));
        assert_eq!(attribute("x xschema=\"1\"", "schema"), None);
    }
}
