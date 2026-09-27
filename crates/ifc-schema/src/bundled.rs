//! Compiled schema artifacts bundled inside this crate.
//!
//! A consumer that needs a schema should not have to source the `.exp` file
//! itself, decode it as Latin-1, or reparse hundreds of kilobytes of EXPRESS
//! on every process start.
//!
//! The artifacts under `data/` are the *parsed* schema (entities, attributes,
//! types), not the EXPRESS source text — they never contain normative
//! buildingSMART/ISO 16739 prose, only the structural facts
//! `openbim_step::express::parse` would extract. The build-time `generation`
//! feature that produces them requires a user-supplied copy of the `.exp`
//! file; that file is never vendored into this crate or its published archive
//! (see `tools/generate.rs`).
//!
//! # Bundled versions
//!
//! IFC2x3 TC1, IFC4 ADD2 TC1, and IFC4X3 ADD2 are separate artifacts. Version
//! dispatch never substitutes one table for another; that would turn schema
//! validation into confident nonsense.

use std::sync::OnceLock;

use crate::artifact::decode_schema;
use crate::registry::Schema;
use crate::version::SchemaVersion;

static IFC4: OnceLock<Schema> = OnceLock::new();
static IFC4X3: OnceLock<Schema> = OnceLock::new();
static IFC2X3: OnceLock<Schema> = OnceLock::new();

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
        let parsed = decode_schema(include_bytes!("../data/ifc2x3-tc1.bin")).expect(
            "the bundled IFC2x3 artifact is produced and verified by this crate's own build",
        );
        Schema::from_parsed(parsed)
    })
}

/// The bundled IFC4 ADD2 TC1 schema (776 entities, 397 types).
///
/// Parsed once on first use and cached for the life of the process. Building
/// this schema costs nothing beyond a `bincode` decode of a committed
/// artifact: the 372 KB `IFC4.exp` EXPRESS source is never read at runtime
/// and is not present in the published crate.
///
/// Custom schema files remain available through [`Schema::from_express`] or
/// [`Schema::from_express_bytes`] directly.
#[must_use]
pub fn ifc4() -> &'static Schema {
    IFC4.get_or_init(|| {
        let parsed = decode_schema(include_bytes!("../data/ifc4-add2-tc1.bin"))
            .expect("the bundled IFC4 artifact is produced and verified by this crate's own build");
        Schema::from_parsed(parsed)
    })
}

/// The bundled IFC4X3 ADD2 schema (876 entities, 436 types).
///
/// Parsed once on first use from its own generated artifact. It is never an
/// alias for IFC4: renamed and civil entities require the declared tables.
#[must_use]
pub fn ifc4x3() -> &'static Schema {
    IFC4X3.get_or_init(|| {
        let parsed = decode_schema(include_bytes!("../data/ifc4x3-add2.bin")).expect(
            "the bundled IFC4X3 artifact is produced and verified by this crate's own build",
        );
        Schema::from_parsed(parsed)
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
        let expected: Vec<(usize, usize)> = source
            .split("Target {")
            .skip(1)
            .filter_map(|block| {
                let number = |key: &str| -> Option<usize> {
                    let start = block.find(key)? + key.len();
                    let rest = &block[start..];
                    let end = rest.find(',')?;
                    rest[..end].trim().parse().ok()
                };
                Some((number("entities:")?, number("types:")?))
            })
            .collect();
        assert_eq!(expected.len(), 3, "three schemas are generated");
        assert_eq!(
            expected[0],
            (ifc2x3().entity_count(), ifc2x3().type_count()),
            "ifc2x3 generator guard vs the committed artifact"
        );
        assert_eq!(
            expected[1],
            (ifc4().entity_count(), ifc4().type_count()),
            "ifc4 generator guard vs the committed artifact"
        );
        assert_eq!(
            expected[2],
            (ifc4x3().entity_count(), ifc4x3().type_count()),
            "ifc4x3 generator guard vs the committed artifact"
        );
    }
}
