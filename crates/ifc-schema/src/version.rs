//! Which IFC schema version a file declares.
//!
//! The token comes from the `FILE_SCHEMA` entry in a STEP header. Real files
//! carry variants (`IFC4X3_ADD2` as well as `IFC4X3`), so matching is explicit
//! rather than a prefix test: `IFC4X1` and `IFC4X2` are their own releases,
//! never a prefix match onto IFC4. Each release's accepted tokens are its
//! `SCHEMA` name as declared in its normative EXPRESS file (`SCHEMA IFC4X1;`,
//! `SCHEMA IFC4X2;`), which is what ISO 10303-21 writes into `FILE_SCHEMA`.
//!
//! # Known releases without a published schema
//!
//! `IFC4X3_TC1` (IFC 4.3.0.1) and `IFC4X3_ADD1` (IFC 4.3.1.0) are real
//! identifiers: buildingSMART's IFC4.x-development generator wrote them as
//! the `SCHEMA` name of the ISO build before `IFC4X3_ADD2` (4.3.2.0).
//! buildingSMART publishes no EXPRESS file for either, and their content
//! differs from ADD2 (TC1 predates `IfcGeographicCRS`, `IfcWellKnownText`
//! and others), so neither can have tables of its own nor be read as
//! ADD2 by assumption. [`SchemaVersion::from_header_token`] does not
//! recognise them; [`UnpublishedRelease::from_header_token`] names them, so
//! a caller can refuse them precisely rather than as an unknown token
//! (#467). A caller that chooses to read such a file as IFC4X3 ADD2 does
//! so explicitly, by passing [`UnpublishedRelease::superseded_by`]'s tables
//! itself, and owns that choice.

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

    /// The `ifc-schema` cargo feature that bundles this release's table.
    #[must_use]
    pub const fn feature_name(self) -> &'static str {
        match self {
            Self::Ifc2x3 => "ifc2x3",
            Self::Ifc4 => "ifc4",
            Self::Ifc4x1 => "ifc4x1",
            Self::Ifc4x2 => "ifc4x2",
            Self::Ifc4x3 => "ifc4x3",
        }
    }

    /// Whether this build bundles this release's table, i.e. whether
    /// `for_version(self)` succeeds.
    #[must_use]
    pub const fn is_bundled(self) -> bool {
        match self {
            Self::Ifc2x3 => cfg!(feature = "ifc2x3"),
            Self::Ifc4 => cfg!(feature = "ifc4"),
            Self::Ifc4x1 => cfg!(feature = "ifc4x1"),
            Self::Ifc4x2 => cfg!(feature = "ifc4x2"),
            Self::Ifc4x3 => cfg!(feature = "ifc4x3"),
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

/// An IFC release whose identifier is known but whose normative EXPRESS
/// schema buildingSMART does not publish (#467).
///
/// No tables exist for it, and it is never aliased to a neighbouring
/// release: [`SchemaVersion::from_header_token`] returns `None` for its
/// identifier. This type lets a caller say *which* release a file
/// declares when refusing it.
///
/// Source of each identifier and version number: the `SCHEMA_NAME` and
/// documentation header of buildingSMART/IFC4.x-development's generator,
/// `IFC4X3_TC1` "IFC 4.3.0.1" (commits `3629c4ec3e` and `623808efce`,
/// 2022-08) and `IFC4X3_ADD1` "IFC 4.3.1.0" (commit `29f283e98f`,
/// 2022-09), both superseded by `IFC4X3_ADD2` "IFC 4.3.2.0" (commit
/// `524daac53c`, 2023-09), the only IFC 4.3 release buildingSMART lists
/// and publishes as EXPRESS.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum UnpublishedRelease {
    /// `IFC4X3_TC1`, IFC 4.3.0.1.
    Ifc4x3Tc1,
    /// `IFC4X3_ADD1`, IFC 4.3.1.0.
    Ifc4x3Add1,
}

impl UnpublishedRelease {
    /// Every unpublished release this crate names, oldest first.
    pub const ALL: [Self; 2] = [Self::Ifc4x3Tc1, Self::Ifc4x3Add1];

    /// The `SCHEMA` identifier the release's files declare in `FILE_SCHEMA`.
    #[must_use]
    pub const fn identifier(self) -> &'static str {
        match self {
            Self::Ifc4x3Tc1 => "IFC4X3_TC1",
            Self::Ifc4x3Add1 => "IFC4X3_ADD1",
        }
    }

    /// The four-part IFC version number of the release.
    #[must_use]
    pub const fn version_number(self) -> &'static str {
        match self {
            Self::Ifc4x3Tc1 => "4.3.0.1",
            Self::Ifc4x3Add1 => "4.3.1.0",
        }
    }

    /// The published release that superseded it. Not a claim that the two
    /// schemas are identical: they are not proven to be.
    #[must_use]
    pub const fn superseded_by(self) -> SchemaVersion {
        match self {
            Self::Ifc4x3Tc1 | Self::Ifc4x3Add1 => SchemaVersion::Ifc4x3,
        }
    }

    /// The unpublished release a `FILE_SCHEMA` token names, if any. Case
    /// and surrounding whitespace are ignored, as for
    /// [`SchemaVersion::from_header_token`].
    pub fn from_header_token(token: &str) -> Option<Self> {
        let token = token.trim();
        Self::ALL
            .into_iter()
            .find(|release| release.identifier().eq_ignore_ascii_case(token))
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

    /// IFC4X3_TC1 and IFC4X3_ADD1 are named, never mapped onto the
    /// bundled IFC4X3 ADD2 tables (#467).
    #[test]
    fn unpublished_ifc4x3_releases_are_named_not_aliased() {
        for (token, release, number) in [
            ("IFC4X3_TC1", UnpublishedRelease::Ifc4x3Tc1, "4.3.0.1"),
            (" ifc4x3_add1 ", UnpublishedRelease::Ifc4x3Add1, "4.3.1.0"),
        ] {
            assert_eq!(SchemaVersion::from_header_token(token), None, "{token}");
            assert_eq!(UnpublishedRelease::from_header_token(token), Some(release));
            assert_eq!(release.version_number(), number);
            assert_eq!(release.superseded_by(), SchemaVersion::Ifc4x3);
        }
        for version in SchemaVersion::ALL {
            for token in version.header_tokens() {
                assert_eq!(UnpublishedRelease::from_header_token(token), None);
            }
        }
        for token in ["IFC4X3", "IFC4X3_ADD2", "IFC4X3_RC4", "IFC4X3_TC"] {
            assert_eq!(UnpublishedRelease::from_header_token(token), None);
        }
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
