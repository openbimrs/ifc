//! Compiled schema artifacts bundled inside this crate.
//!
//! [`ifc4`] answers the request in issue #4: a consumer that needs the IFC4
//! ADD2 TC1 schema should not have to source `IFC4.exp` itself, decode it as
//! Latin-1, or reparse 372 KB of EXPRESS on every process start.
//!
//! The artifact under `data/ifc4-add2-tc1.bin` is the *parsed* schema
//! (entities, attributes, types), not the EXPRESS source text — it never
//! contains normative buildingSMART/ISO 16739 prose, only the structural
//! facts `openbim_step::express::parse` would extract from it. The build-time
//! `generation` feature that produces this artifact requires a user-supplied
//! copy of `IFC4.exp`; the file itself is never vendored into this crate or
//! its published archive (see `tools/generate.rs`).

use std::sync::OnceLock;

use crate::artifact::decode_schema;
use crate::registry::Schema;

static IFC4: OnceLock<Schema> = OnceLock::new();

/// The bundled IFC4 ADD2 TC1 schema (776 entities, 397 types).
///
/// Parsed once on first use and cached for the life of the process. Building
/// this schema costs nothing beyond a `bincode` decode of a committed
/// artifact: the 372 KB `IFC4.exp` EXPRESS source is never read at runtime
/// and is not present in the published crate.
///
/// For schemas this crate does not bundle (IFC2x3, IFC4x3, or a custom
/// schema file), use [`Schema::from_express`] or
/// [`Schema::from_express_bytes`] directly.
#[must_use]
pub fn ifc4() -> &'static Schema {
    IFC4.get_or_init(|| {
        let parsed = decode_schema(include_bytes!("../data/ifc4-add2-tc1.bin"))
            .expect("the bundled IFC4 artifact is produced and verified by this crate's own build");
        Schema::from_parsed(parsed)
    })
}

#[cfg(test)]
mod tests {
    use super::ifc4;

    #[test]
    fn bundled_ifc4_matches_the_normative_entity_and_type_counts() {
        let schema = ifc4();
        assert_eq!(schema.entity_count(), 776, "IFC4 ADD2 TC1 entity count");
        assert_eq!(schema.type_count(), 397, "IFC4 ADD2 TC1 type count");
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

    #[test]
    fn repeated_calls_return_the_same_cached_schema() {
        let first = ifc4() as *const _;
        let second = ifc4() as *const _;
        assert_eq!(first, second, "ifc4() must not reparse on every call");
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
}
