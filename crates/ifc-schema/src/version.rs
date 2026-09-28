//! Which IFC schema version a file declares.
//!
//! The token comes from the `FILE_SCHEMA` entry in a STEP header. Real files
//! carry variants (`IFC4X3_ADD2` as well as `IFC4X3`), so matching is explicit
//! rather than a prefix test: `IFC4X1` and `IFC4X2` are their own releases,
//! never a prefix match onto IFC4. Each release's accepted tokens are its
//! `SCHEMA` name as declared in its normative EXPRESS file (`SCHEMA IFC4X1;`,
//! `SCHEMA IFC4X2;`), which is what ISO 10303-21 writes into `FILE_SCHEMA`.

/// Which IFC schema version a table describes.
///
/// `#[non_exhaustive]`: a release added later is a new variant, and a
/// consumer's `match` must say what it does with a release it has not
/// proven -- refuse it, never alias it to a neighbour.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum SchemaVersion {
    /// IFC2x3 TC1 — 653 entities. Still the most common in the wild.
    Ifc2x3,
    /// IFC4 ADD2 TC1 — 776 entities. The ISO-standard release.
    Ifc4,
    /// IFC4x1 FINAL — 801 entities. Adds alignment (`IfcAlignment` and its
    /// curves) to IFC4.
    Ifc4x1,
    /// IFC4x2 FINAL — 816 entities. Adds bridges (`IfcBridge`, `IfcFacility`)
    /// to IFC4x1.
    Ifc4x2,
    /// IFC4x3 ADD2 — 876 entities. Adds alignment and civil infrastructure.
    Ifc4x3,
}

impl SchemaVersion {
    /// Every version this crate knows, oldest first.
    ///
    /// Which of them have bundled tables in a given build is a separate
    /// question; see `for_version`.
    pub const ALL: [Self; 5] = [
        Self::Ifc2x3,
        Self::Ifc4,
        Self::Ifc4x1,
        Self::Ifc4x2,
        Self::Ifc4x3,
    ];

    /// Exact published release identifier for the bundled structural table.
    #[must_use]
    pub const fn release_id(self) -> &'static str {
        match self {
            Self::Ifc2x3 => "IFC2X3_TC1",
            Self::Ifc4 => "IFC4_ADD2_TC1",
            Self::Ifc4x1 => "IFC4X1_FINAL",
            Self::Ifc4x2 => "IFC4X2_FINAL",
            Self::Ifc4x3 => "IFC4X3_ADD2",
        }
    }

    /// Entity count asserted when the bundled artifact is generated.
    #[must_use]
    pub const fn expected_entity_count(self) -> usize {
        match self {
            Self::Ifc2x3 => 653,
            Self::Ifc4 => 776,
            Self::Ifc4x1 => 801,
            Self::Ifc4x2 => 816,
            Self::Ifc4x3 => 876,
        }
    }

    /// Defined-type count asserted when the bundled artifact is generated.
    #[must_use]
    pub const fn expected_type_count(self) -> usize {
        match self {
            Self::Ifc2x3 => 327,
            Self::Ifc4 => 397,
            Self::Ifc4x1 => 400,
            Self::Ifc4x2 => 407,
            Self::Ifc4x3 => 436,
        }
    }

    /// Header tokens accepted for this release, excluding case-only variants.
    #[must_use]
    pub const fn header_tokens(self) -> &'static [&'static str] {
        match self {
            Self::Ifc2x3 => &["IFC2X3"],
            Self::Ifc4 => &["IFC4"],
            Self::Ifc4x1 => &["IFC4X1"],
            Self::Ifc4x2 => &["IFC4X2"],
            Self::Ifc4x3 => &["IFC4X3", "IFC4X3_ADD2"],
        }
    }

    /// Parse the token found in a STEP file's `FILE_SCHEMA` header entry.
    pub fn from_header_token(token: &str) -> Option<Self> {
        match token.trim().to_ascii_uppercase().as_str() {
            "IFC2X3" => Some(Self::Ifc2x3),
            "IFC4" => Some(Self::Ifc4),
            "IFC4X1" => Some(Self::Ifc4x1),
            "IFC4X2" => Some(Self::Ifc4x2),
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

    /// Every declared header token maps back to its own version, and the
    /// release identifiers are distinct: IFC4X1 and IFC4X2 are neither IFC4
    /// nor IFC4X3.
    #[test]
    fn header_tokens_and_release_ids_round_trip_for_every_version() {
        for version in SchemaVersion::ALL {
            for token in version.header_tokens() {
                assert_eq!(SchemaVersion::from_header_token(token), Some(version));
                assert_eq!(
                    SchemaVersion::from_header_token(&token.to_ascii_lowercase()),
                    Some(version)
                );
            }
        }
        assert_eq!(
            SchemaVersion::from_header_token("IFC4X1"),
            Some(SchemaVersion::Ifc4x1)
        );
        assert_eq!(
            SchemaVersion::from_header_token(" ifc4x2 "),
            Some(SchemaVersion::Ifc4x2)
        );
        assert_eq!(SchemaVersion::Ifc4x1.release_id(), "IFC4X1_FINAL");
        assert_eq!(SchemaVersion::Ifc4x2.release_id(), "IFC4X2_FINAL");
        let ids: std::collections::BTreeSet<_> =
            SchemaVersion::ALL.iter().map(|v| v.release_id()).collect();
        assert_eq!(ids.len(), SchemaVersion::ALL.len());
    }
}
