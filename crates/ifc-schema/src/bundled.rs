//! Compiled schema artifacts bundled inside this crate.
//!
//! A consumer that needs a schema should not have to source the `.exp` file
//! itself, decode it as Latin-1, or reparse hundreds of kilobytes of EXPRESS
//! on every process start.
//!
//! The artifacts under `data/` are the *parsed* schema (entities, attributes,
//! types), not the EXPRESS source text — they never contain normative
//! buildingSMART/ISO 16739 prose, only the structural facts
//! the EXPRESS extractor would read, decoded into this crate's own types. The build-time `generation`
//! feature that produces them requires a user-supplied copy of the `.exp`
//! file; that file is never vendored into this crate or its published archive
//! (see `tools/generate.rs`).
//!
//! # Bundled versions
//!
//! IFC2x3 TC1, IFC4 ADD2 TC1, IFC4X1 FINAL, IFC4X2 FINAL and IFC4X3 ADD2 are
//! separate artifacts. Version
//! dispatch never substitutes one table for another; that would turn schema
//! validation into confident nonsense.

use std::sync::OnceLock;

use crate::artifact::decode_schema;
use crate::registry::Schema;
use crate::version::SchemaVersion;

static IFC4: OnceLock<Schema> = OnceLock::new();
static IFC4X3: OnceLock<Schema> = OnceLock::new();
static IFC2X3: OnceLock<Schema> = OnceLock::new();
static IFC4X1: OnceLock<Schema> = OnceLock::new();
static IFC4X2: OnceLock<Schema> = OnceLock::new();

/// The bundled IFC2x3 TC1 schema (653 entities, 327 types).
///
/// Still the most common schema in the wild. Its layouts differ from IFC4 in
/// ways that silently corrupt a reader that assumes the newer tables:
/// `IfcWallStandardCase` has 8 attributes here and 9 in IFC4, because IFC4
/// inserts `PredefinedType`.
///
/// Parsed once on first use and cached for the life of the process.
#[must_use]
pub fn ifc2x3() -> &'static Schema {
    IFC2X3.get_or_init(|| {
        decode_schema(include_bytes!("../data/ifc2x3-tc1.bin")).expect(
            "the bundled IFC2x3 artifact is produced and verified by this crate's own build",
        )
    })
}

/// The bundled IFC4 ADD2 TC1 schema (776 entities, 397 types).
///
/// Parsed once on first use and cached for the life of the process. Building
/// this schema costs nothing beyond a `bincode` decode of a committed
/// artifact: the 372 KB `IFC4.exp` EXPRESS source is never read at runtime
/// and is not present in the published crate.
///
/// Custom schema files remain available through `Schema::from_express` (the
/// `express` feature) or [`Schema::new`].
#[must_use]
pub fn ifc4() -> &'static Schema {
    IFC4.get_or_init(|| {
        decode_schema(include_bytes!("../data/ifc4-add2-tc1.bin"))
            .expect("the bundled IFC4 artifact is produced and verified by this crate's own build")
    })
}

/// The bundled IFC4X1 FINAL schema (801 entities, 400 types).
///
/// Its own artifact: IFC4X1 adds the alignment entities to IFC4 and is not
/// an alias for either IFC4 or IFC4X3.
#[must_use]
pub fn ifc4x1() -> &'static Schema {
    IFC4X1.get_or_init(|| {
        decode_schema(include_bytes!("../data/ifc4x1-final.bin")).expect(
            "the bundled IFC4X1 artifact is produced and verified by this crate's own build",
        )
    })
}

/// The bundled IFC4X2 FINAL schema (816 entities, 407 types).
///
/// Its own artifact: IFC4X2 adds bridges to IFC4X1 and is not an alias for
/// IFC4 or IFC4X3.
#[must_use]
pub fn ifc4x2() -> &'static Schema {
    IFC4X2.get_or_init(|| {
        decode_schema(include_bytes!("../data/ifc4x2-final.bin")).expect(
            "the bundled IFC4X2 artifact is produced and verified by this crate's own build",
        )
    })
}

/// The bundled IFC4X3 ADD2 schema (876 entities, 436 types).
///
/// Parsed once on first use from its own generated artifact. It is never an
/// alias for IFC4: renamed and civil entities require the declared tables.
#[must_use]
pub fn ifc4x3() -> &'static Schema {
    IFC4X3.get_or_init(|| {
        decode_schema(include_bytes!("../data/ifc4x3-add2.bin")).expect(
            "the bundled IFC4X3 artifact is produced and verified by this crate's own build",
        )
    })
}

/// The bundled schema for `version`, or `None` when none is bundled.
///
/// This is the lookup a reader should use after parsing `FILE_SCHEMA`, so an
/// unbundled schema becomes an explicit "cannot check this" rather than a
/// silent fallback to the wrong tables.
#[must_use]
pub fn for_version(version: SchemaVersion) -> Option<&'static Schema> {
    match version {
        SchemaVersion::Ifc2x3 => Some(ifc2x3()),
        SchemaVersion::Ifc4 => Some(ifc4()),
        SchemaVersion::Ifc4x1 => Some(ifc4x1()),
        SchemaVersion::Ifc4x2 => Some(ifc4x2()),
        SchemaVersion::Ifc4x3 => Some(ifc4x3()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bundled_ifc4_matches_the_normative_entity_and_type_counts() {
        let schema = ifc4();
        assert_eq!(schema.entity_count(), 776, "IFC4 ADD2 TC1 entity count");
        assert_eq!(schema.type_count(), 397, "IFC4 ADD2 TC1 type count");
    }

    #[test]
    fn bundled_ifc4x3_matches_the_normative_entity_and_type_counts() {
        let schema = ifc4x3();
        assert_eq!(schema.entity_count(), 876, "IFC4X3 ADD2 entity count");
        assert_eq!(schema.type_count(), 436, "IFC4X3 ADD2 type count");
        assert!(schema.entity("IfcBuiltElement").is_some());
        assert!(schema.entity("IfcBuildingElement").is_none());
        assert!(std::ptr::eq(schema, ifc4x3()), "constructor must cache");
    }

    #[test]
    fn bundled_ifc2x3_matches_the_normative_entity_and_type_counts() {
        let schema = ifc2x3();
        assert_eq!(schema.entity_count(), 653, "IFC2x3 TC1 entity count");
        assert_eq!(schema.type_count(), 327, "IFC2x3 TC1 type count");
    }

    #[test]
    fn bundled_ifc4_resolves_the_deep_inheritance_chain() {
        let schema = ifc4();
        assert!(schema.is_a("IFCWALL", "IfcRoot"), "wall is a root");
        assert!(schema.is_a("IFCWALL", "IfcProduct"), "wall is a product");
        assert_eq!(
            &schema.attribute_names("IFCWALL")[..4],
            ["GlobalId", "OwnerHistory", "Name", "Description"],
            "IfcRoot's slots must come first"
        );
    }

    /// Every recognised version resolves to its independent bundled table.
    #[test]
    fn version_lookup_returns_the_matching_table() {
        assert_eq!(
            for_version(SchemaVersion::Ifc4).map(|s| s.entity_count()),
            Some(776)
        );
        assert_eq!(
            for_version(SchemaVersion::Ifc2x3).map(|s| s.entity_count()),
            Some(653)
        );
        assert_eq!(
            for_version(SchemaVersion::Ifc4x3).map(|s| s.entity_count()),
            Some(876),
            "IFC4X3 must select its own bundled tables"
        );
    }

    /// The IFC2x3 and IFC4 bundled schemas must be distinct tables.
    ///
    /// Wiring both constructors to the same artifact would pass every count
    /// test above if the counts happened to be read from the same file, so
    /// pin a layout that genuinely differs between the versions.
    #[test]
    fn the_ifc2x3_and_ifc4_bundles_are_not_the_same_table() {
        // IFC4 inserts PredefinedType; IFC2x3 stops at Tag.
        assert_eq!(
            ifc2x3().attribute_names("IFCWALLSTANDARDCASE"),
            [
                "GlobalId",
                "OwnerHistory",
                "Name",
                "Description",
                "ObjectType",
                "ObjectPlacement",
                "Representation",
                "Tag"
            ],
        );
        assert_eq!(
            ifc4().attribute_names("IFCWALLSTANDARDCASE"),
            [
                "GlobalId",
                "OwnerHistory",
                "Name",
                "Description",
                "ObjectType",
                "ObjectPlacement",
                "Representation",
                "Tag",
                "PredefinedType"
            ],
        );
    }

    /// IFC2x3 entities that IFC4 removed must resolve only in IFC2x3.
    #[test]
    fn version_specific_entities_resolve_in_their_own_schema() {
        assert!(
            !ifc2x3().attributes("IFC2DCOMPOSITECURVE").is_empty(),
            "Ifc2DCompositeCurve exists in IFC2x3"
        );
        assert!(
            ifc4().type_def("IfcHeatFluxDensityMeasure").is_some(),
            "IFC4 keeps the derived measure types"
        );
    }

    #[test]
    fn repeated_calls_return_the_same_cached_schema() {
        let first = ifc4() as *const _;
        let second = ifc4() as *const _;
        assert_eq!(first, second, "ifc4() must not reparse on every call");
        let first = ifc2x3() as *const _;
        let second = ifc2x3() as *const _;
        assert_eq!(first, second, "ifc2x3() must not reparse on every call");
    }

    /// An inline `UNIQUE` in an aggregate must not truncate the slot list.
    ///
    /// `IfcTypeProduct.RepresentationMaps` is declared
    /// `OPTIONAL LIST [1:?] OF UNIQUE IfcRepresentationMap`. A parser that
    /// treats that `UNIQUE` as the start of a UNIQUE block drops it and `Tag`,
    /// which shifts every following slot of all 124 entities inheriting from
    /// `IfcTypeProduct` -- silently, since the values still look plausible.
    ///
    /// Expected layouts are IfcOpenShell's, which is an independent
    /// implementation of the same normative schema.
    #[test]
    fn type_product_subtypes_keep_their_full_slot_layout() {
        let schema = ifc4();
        assert_eq!(
            schema.attribute_names("IFCTYPEPRODUCT"),
            [
                "GlobalId",
                "OwnerHistory",
                "Name",
                "Description",
                "ApplicableOccurrence",
                "HasPropertySets",
                "RepresentationMaps",
                "Tag"
            ],
        );
        assert_eq!(
            schema.attribute_names("IFCWALLTYPE"),
            [
                "GlobalId",
                "OwnerHistory",
                "Name",
                "Description",
                "ApplicableOccurrence",
                "HasPropertySets",
                "RepresentationMaps",
                "Tag",
                "ElementType",
                "PredefinedType"
            ],
            "ElementType sits at 8, not 6"
        );
    }

    /// The same inline-`UNIQUE` hazard exists in IFC2x3.
    #[test]
    fn ifc2x3_type_product_keeps_its_full_slot_layout() {
        assert_eq!(
            ifc2x3().attribute_names("IFCTYPEPRODUCT"),
            [
                "GlobalId",
                "OwnerHistory",
                "Name",
                "Description",
                "ApplicableOccurrence",
                "HasPropertySets",
                "RepresentationMaps",
                "Tag"
            ],
        );
    }

    /// The other IFC4 declarations carrying an inline `UNIQUE`.
    #[test]
    fn every_inline_unique_aggregate_survives_parsing() {
        let schema = ifc4();
        assert!(
            schema
                .attribute_names("IFCGRID")
                .contains(&"PredefinedType"),
            "IfcGrid declares UAxes/VAxes/WAxes with inline UNIQUE"
        );
        assert_eq!(schema.attribute_names("IFCPOLYLOOP"), ["Polygon"]);
        assert_eq!(
            schema.attribute_names("IFCPROPERTYTABLEVALUE")[7],
            "CurveInterpolation",
            "slot 7 after two inherited IfcProperty slots"
        );
    }

    /// The generator's expected counts must match the committed artifacts.
    ///
    /// The counts in `tools/generate.rs` are the only guard against pointing
    /// the generator at the wrong `.exp` -- a mistake that yields a
    /// plausible-looking artifact describing the wrong schema. Nothing else
    /// checks them, because the generator needs a normative source file that
    /// CI does not have. Pinning them against the artifacts that shipped
    /// keeps the guard honest without needing the source.
    #[test]
    fn the_generator_guards_match_the_bundled_artifacts() {
        // Parsed out of the generator's TARGETS table so the two cannot drift.
        let source = include_str!("../tools/generate.rs");
        let expected: Vec<(String, usize, usize)> = source
            .split("Target {")
            .skip(1)
            .filter_map(|block| {
                let field = |key: &str| -> Option<&str> {
                    let start = block.find(key)? + key.len();
                    let rest = &block[start..];
                    let end = rest.find(',')?;
                    Some(rest[..end].trim())
                };
                Some((
                    field("selector:")?.trim_matches('"').to_owned(),
                    field("entities:")?.parse().ok()?,
                    field("types:")?.parse().ok()?,
                ))
            })
            .collect();
        let bundled = [
            ("ifc2x3", SchemaVersion::Ifc2x3, ifc2x3()),
            ("ifc4", SchemaVersion::Ifc4, ifc4()),
            ("ifc4x1", SchemaVersion::Ifc4x1, ifc4x1()),
            ("ifc4x2", SchemaVersion::Ifc4x2, ifc4x2()),
            ("ifc4x3", SchemaVersion::Ifc4x3, ifc4x3()),
        ];
        assert_eq!(expected.len(), bundled.len(), "one generator target each");
        for ((selector, entities, types), (name, version, schema)) in expected.iter().zip(bundled) {
            assert_eq!(selector, name, "generator target order");
            assert_eq!(
                (*entities, *types),
                (schema.entity_count(), schema.type_count()),
                "{name} generator guard vs the committed artifact"
            );
            assert_eq!(
                (*entities, *types),
                (
                    version.expected_entity_count(),
                    version.expected_type_count()
                ),
                "{name} generator guard vs SchemaVersion"
            );
            assert_eq!(schema.version(), Some(version), "{name} declared name");
        }
    }

    /// IFC4X1 and IFC4X2 are their own tables, not a paste of a neighbour:
    /// each count differs from IFC4 and IFC4X3, and each carries the
    /// entities its release introduced and none its successor added.
    #[test]
    fn the_intermediate_releases_are_distinct_tables() {
        for schema in [ifc4x1(), ifc4x2()] {
            for neighbour in [ifc4(), ifc4x3()] {
                assert_ne!(schema.entity_count(), neighbour.entity_count());
            }
        }
        assert_ne!(ifc4x1().entity_count(), ifc4x2().entity_count());

        assert!(ifc4().entity("IfcAlignment").is_none());
        assert!(ifc4x1().entity("IfcAlignment").is_some());
        assert!(ifc4x1().entity("IfcAlignmentCurve").is_some());
        assert!(ifc4x1().entity("IfcBridge").is_none());
        assert!(ifc4x2().entity("IfcBridge").is_some());
        assert!(ifc4x2().entity("IfcBuiltElement").is_none());
        assert!(ifc4x3().entity("IfcAlignmentCurve").is_none());

        assert_eq!(
            for_version(SchemaVersion::Ifc4x1).map(Schema::name),
            Some("IFC4X1")
        );
        assert_eq!(
            for_version(SchemaVersion::Ifc4x2).map(Schema::name),
            Some("IFC4X2")
        );
        assert!(std::ptr::eq(ifc4x1(), ifc4x1()), "constructor must cache");
    }
}
