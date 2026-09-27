//! File-level metadata, independent of serialization.
//!
//! STEP writes this as `FILE_DESCRIPTION`/`FILE_NAME`/`FILE_SCHEMA` inside a
//! `HEADER;` section; ifcXML writes it as attributes on the root element. The
//! fields are the same, so they live here and each codec maps to its own
//! syntax.

/// Metadata describing the file and the schema it claims to follow.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Header {
    /// `FILE_DESCRIPTION` description strings, e.g. a view definition.
    pub description: Vec<String>,
    /// `FILE_DESCRIPTION` implementation level, conventionally `2;1`.
    pub implementation_level: String,
    /// Originating file name.
    pub name: String,
    /// ISO-8601 timestamp as written.
    pub time_stamp: String,
    /// Author entries.
    pub author: Vec<String>,
    /// Organization entries.
    pub organization: Vec<String>,
    /// Preprocessor that produced the file.
    pub preprocessor_version: String,
    /// Originating application.
    pub originating_system: String,
    /// Authorization field.
    pub authorization: String,
    /// `FILE_SCHEMA` tokens, e.g. `IFC4X3_ADD2`.
    ///
    /// Kept as written rather than parsed into an enum: an unrecognized schema
    /// token must survive a round-trip, and refusing to store it would corrupt
    /// files from schema versions this build predates.
    pub schema: Vec<String>,
}

impl Header {
    /// The declared schema token, if any.
    pub fn schema_token(&self) -> Option<&str> {
        self.schema.first().map(|s| s.as_str())
    }
}
