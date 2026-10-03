//! The STEP file header: read it whole, replace it whole.
//!
//! The facade's `Header` is plain data (`FILE_DESCRIPTION`, `FILE_NAME`,
//! `FILE_SCHEMA`) and the model hands it out mutably, so the bindings read
//! and write every field. The JavaScript and Python bindings map the struct
//! to an object or a frozen dataclass; the C binding carries it as a value
//! tape in the order the fields appear in a STEP header, which
//! [`to_tagged`] and [`from_tagged`] define once.

/// The facade's header record, re-exported so hosts need not name the
/// facade.
pub use ifc::Header;

use crate::value::Tagged;
use crate::{BindingError, IfcModel};

/// Number of top-level fields on the tagged form.
pub const FIELDS: usize = 10;

impl IfcModel {
    /// The file header: description, name, time stamp, author,
    /// organization, preprocessor, originating system, authorization and
    /// the schema tokens, as written.
    pub fn header(&self) -> Header {
        self.inner.header().clone()
    }

    /// Replace the file header. Every field is written back as given; the
    /// schema tokens decide which tables subtype queries and validation use.
    pub fn set_header(&mut self, header: Header) {
        *self.inner.header_mut() = header;
    }
}

/// The header as one tagged `list` of ten values, in STEP header order:
/// description (list of text), implementation level, name, time stamp,
/// author (list), organization (list), preprocessor version, originating
/// system, authorization, schema (list).
pub fn to_tagged(header: &Header) -> Tagged {
    let text = |s: &String| Tagged::Text(s.clone());
    let texts = |v: &Vec<String>| Tagged::List(v.iter().map(text).collect());
    Tagged::List(vec![
        texts(&header.description),
        text(&header.implementation_level),
        text(&header.name),
        text(&header.time_stamp),
        texts(&header.author),
        texts(&header.organization),
        text(&header.preprocessor_version),
        text(&header.originating_system),
        text(&header.authorization),
        texts(&header.schema),
    ])
}

/// The inverse of [`to_tagged`]; any other shape is `invalid-value`.
pub fn from_tagged(value: Tagged) -> Result<Header, BindingError> {
    let Tagged::List(fields) = value else {
        return Err(invalid("a header must be a list of 10 fields"));
    };
    let [description, implementation_level, name, time_stamp, author, organization, preprocessor_version, originating_system, authorization, schema]: [Tagged; FIELDS] =
        fields.try_into().map_err(|fields: Vec<Tagged>| {
            invalid(&format!("a header has 10 fields, not {}", fields.len()))
        })?;
    Ok(Header {
        description: texts("description", description)?,
        implementation_level: text("implementation level", implementation_level)?,
        name: text("name", name)?,
        time_stamp: text("time stamp", time_stamp)?,
        author: texts("author", author)?,
        organization: texts("organization", organization)?,
        preprocessor_version: text("preprocessor version", preprocessor_version)?,
        originating_system: text("originating system", originating_system)?,
        authorization: text("authorization", authorization)?,
        schema: texts("schema", schema)?,
    })
}

fn invalid(detail: &str) -> BindingError {
    BindingError::InvalidValue(detail.to_owned())
}

fn text(field: &str, value: Tagged) -> Result<String, BindingError> {
    match value {
        Tagged::Text(s) => Ok(s),
        other => Err(invalid(&format!(
            "header {field} must be text, not {}",
            other.kind().as_str()
        ))),
    }
}

fn texts(field: &str, value: Tagged) -> Result<Vec<String>, BindingError> {
    match value {
        Tagged::List(items) => items.into_iter().map(|item| text(field, item)).collect(),
        other => Err(invalid(&format!(
            "header {field} must be a list of text, not {}",
            other.kind().as_str()
        ))),
    }
}
