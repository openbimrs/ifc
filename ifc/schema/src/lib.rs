//! `ifc-schema` — the IFC schema **as data**, not as 2,500 generated structs.
//!
//! # The decision
//!
//! IfcOpenShell generates a C++ class per IFC entity per schema version. That
//! is a very large amount of code to compile, and it makes supporting a new
//! schema a code-generation event. We instead represent the schema as a table:
//! entity name → type id → supertype chain. Subtype tests become an integer
//! walk, and adding IFC4X3 is adding data.
//!
//! This mirrors the "one record type, not 2,500 classes" decision that the
//! sibling `solibri-rs` workspace validated on real models.
//!
//! # Status
//!
//! Scaffold. The generator that lowers an EXPRESS `.exp` file into this table
//! is Stage 1 in `docs/ROADMAP.md`.

/// Which IFC schema version a table describes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SchemaVersion {
    Ifc2x3,
    Ifc4,
    Ifc4x3,
}

impl SchemaVersion {
    /// Parse the token found in a STEP file's `FILE_SCHEMA` header entry.
    pub fn from_header_token(token: &str) -> Option<Self> {
        match token.trim().to_ascii_uppercase().as_str() {
            "IFC2X3" => Some(Self::Ifc2x3),
            "IFC4" => Some(Self::Ifc4),
            "IFC4X3" | "IFC4X3_ADD2" => Some(Self::Ifc4x3),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recognizes_the_schema_tokens_our_fixtures_carry() {
        assert_eq!(
            SchemaVersion::from_header_token("IFC4"),
            Some(SchemaVersion::Ifc4)
        );
        assert_eq!(
            SchemaVersion::from_header_token("ifc2x3"),
            Some(SchemaVersion::Ifc2x3)
        );
        assert_eq!(SchemaVersion::from_header_token("STEP"), None);
    }
}
