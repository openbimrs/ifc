//! Which IFC schema version a file declares.
//!
//! The token comes from the `FILE_SCHEMA` entry in a STEP header. Real files
//! carry variants (`IFC4X3_ADD2` as well as `IFC4X3`), so matching is explicit
//! rather than a prefix test.

/// Which IFC schema version a table describes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SchemaVersion {
    /// IFC2x3 TC1 — 653 entities. Still the most common in the wild.
    Ifc2x3,
    /// IFC4 ADD2 TC1 — 776 entities. The ISO-standard release.
    Ifc4,
    /// IFC4x3 ADD2 — 876 entities. Adds alignment and civil infrastructure.
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
