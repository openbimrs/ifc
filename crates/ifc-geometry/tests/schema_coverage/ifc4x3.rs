//! The IFC4X3 ADD2 half of the inventory gate.
//!
//! `IFC4X3_ADDITIONS` lives in `schema_coverage.rs`, whose `"Ifc..."`
//! literals also give the generated capability tables their casing. This
//! module checks that list against the schema and holds the registries to it.

use std::collections::BTreeSet;
use std::path::PathBuf;

use crate::{CONCRETE_ENTITIES, IFC4X3_ADDITIONS};

/// The roots that make an IFC4X3 addition a geometry entity of this crate.
const IFC4X3_GEOMETRY_ROOTS: &[&str] = &[
    "IfcRepresentationItem",
    "IfcProfileDef",
    "IfcObjectPlacement",
];

/// Every other concrete, non-`IfcRoot` entity IFC4X3 ADD2 adds, and why it is
/// not geometry this crate lowers.
const IFC4X3_OUT_OF_SCOPE: &[(&str, &str)] = &[
    (
        "IfcAlignmentCantSegment",
        "alignment design parameters, not geometry; read by ifc-alignment",
    ),
    (
        "IfcAlignmentHorizontalSegment",
        "alignment design parameters, not geometry; read by ifc-alignment",
    ),
    (
        "IfcAlignmentVerticalSegment",
        "alignment design parameters, not geometry; read by ifc-alignment",
    ),
    (
        "IfcGeographicCRS",
        "coordinate reference system; read by ifc-georef",
    ),
    (
        "IfcIndexedPolygonalTextureMap",
        "presentation appearance (texture coordinates for a polygonal face \
         set), not shape",
    ),
    (
        "IfcMapConversionScaled",
        "coordinate operation; read by ifc-georef",
    ),
    (
        "IfcQuantityNumber",
        "element quantity; read by ifc-properties",
    ),
    (
        "IfcRigidOperation",
        "coordinate operation; read by ifc-georef",
    ),
    (
        "IfcTextureCoordinateIndices",
        "presentation appearance (texture coordinate indices), not shape",
    ),
    (
        "IfcTextureCoordinateIndicesWithVoids",
        "presentation appearance (texture coordinate indices), not shape",
    ),
    (
        "IfcWellKnownText",
        "coordinate reference system definition text; read by ifc-georef",
    ),
];

/// The concrete, non-`IfcRoot` entities `later` adds over `earlier`.
fn added_entities(earlier: &ifc_schema::Schema, later: &ifc_schema::Schema) -> BTreeSet<String> {
    later
        .entity_names()
        .filter(|name| {
            earlier.entity(name).is_none()
                && !later.entity(name).expect("declared").abstract_
                && !later.is_a(name, "IfcRoot")
        })
        .map(str::to_ascii_uppercase)
        .collect()
}

/// The IFC4X3 additions and out-of-scope lists, partitioned by root.
fn assert_ifc4x3_partition(release: &str, ifc4: &ifc_schema::Schema, ifc4x3: &ifc_schema::Schema) {
    let upper = |names: &mut dyn Iterator<Item = &str>| -> BTreeSet<String> {
        names.map(str::to_ascii_uppercase).collect()
    };
    let additions = upper(&mut IFC4X3_ADDITIONS.iter().copied());
    let out_of_scope = upper(&mut IFC4X3_OUT_OF_SCOPE.iter().map(|(name, _)| *name));
    assert_eq!(
        additions.len(),
        IFC4X3_ADDITIONS.len(),
        "duplicate addition"
    );
    assert!(additions.is_disjoint(&out_of_scope), "classified twice");
    for (name, reason) in IFC4X3_OUT_OF_SCOPE {
        assert!(!reason.trim().is_empty(), "{name} has no reason");
    }

    let declared: BTreeSet<String> = additions.union(&out_of_scope).cloned().collect();
    let added = added_entities(ifc4, ifc4x3);
    assert_eq!(
        declared,
        added,
        "{release}: IFC4X3 additions drifted; missing={:?}, extra={:?}",
        added.difference(&declared).collect::<Vec<_>>(),
        declared.difference(&added).collect::<Vec<_>>()
    );
    for name in &added {
        let geometry = IFC4X3_GEOMETRY_ROOTS
            .iter()
            .any(|root| ifc4x3.is_a(name, root));
        assert_eq!(
            geometry,
            additions.contains(name),
            "{release}: {name} is filed on the wrong side of the geometry roots"
        );
    }

    // The IFC4 inventory carries over unchanged: nothing removed, nothing
    // made abstract.
    for entity in CONCRETE_ENTITIES {
        let def = ifc4x3
            .entity(entity)
            .unwrap_or_else(|| panic!("{release}: IFC4X3 removed {entity}"));
        assert!(!def.abstract_, "{release}: IFC4X3 made {entity} abstract");
    }
}

/// The IFC4X3 inventory is what the bundled IFC4X3 table adds over IFC4.
///
/// The bundled tables are generated from the normative `.exp` files, so this
/// runs in a fresh clone. The next test re-derives it from the `.exp` itself.
#[test]
fn the_ifc4x3_inventory_matches_the_bundled_schema() {
    assert_eq!(
        IFC4X3_ADDITIONS.len(),
        19,
        "IFC4X3 ADD2 adds 19 concrete geometry, profile and placement \
         entities. Changing this number means the inventory was edited \
         rather than the code fixed."
    );
    assert_ifc4x3_partition("bundled", ifc_schema::ifc4(), ifc_schema::ifc4x3());
}

/// Locate `references/ifc-spec` in either checkout layout.
fn spec_root() -> Option<PathBuf> {
    let crate_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    [
        "../../../../references/ifc-spec",
        "../../references/ifc-spec",
    ]
    .into_iter()
    .map(|rel| crate_dir.join(rel))
    .find(|path| path.is_dir())
}

/// The IFC4X3 inventory is what `IFC4X3_ADD2.exp` adds over `IFC4.exp`.
#[test]
fn the_ifc4x3_inventory_matches_the_normative_express() {
    let Some(root) = spec_root() else {
        assert!(
            std::env::var_os("IFC_SPEC_REQUIRED").is_none(),
            "IFC_SPEC_REQUIRED is set but references/ifc-spec was not found"
        );
        eprintln!("skipped: references/ifc-spec not present");
        return;
    };
    let read = |rel: &str| {
        let bytes = std::fs::read(root.join(rel)).unwrap_or_else(|e| panic!("read {rel}: {e}"));
        ifc_schema::Schema::from_express_bytes(&bytes)
    };
    let ifc4 = read("ifc4-add2-tc1/IFC4.exp");
    let ifc4x3 = read("ifc4x3-add2/IFC4X3_ADD2.exp");
    assert_ifc4x3_partition("IFC4X3_ADD2.exp", &ifc4, &ifc4x3);
}

/// Every IFC4X3 representation item is lowered, declared planned, or nested.
///
/// The IFC4 counterpart in `tests/lower_dispatch_corpus.rs` classifies the
/// IFC4 items against `data/ifc4-representation-item-dispositions.tsv`. This
/// gate enumerates IFC4X3 from its own table, so an IFC4X3-only family that
/// is neither in `dispatch::IMPLEMENTED` nor in `dispatch::PLANNED` fails
/// here instead of reaching a caller as the generic "not lowered yet".
#[cfg(feature = "lowering")]
#[test]
fn every_concrete_ifc4x3_representation_item_is_lowered_or_declared() {
    use ifc_geometry::lower::dispatch::{IMPLEMENTED, PARTIAL, PLANNED};

    const DISPOSITIONS: &str = include_str!("../../data/ifc4-representation-item-dispositions.tsv");

    let schema = ifc_schema::ifc4x3();
    let expected: BTreeSet<String> = schema
        .entity_names()
        .filter(|name| {
            schema.is_a(name, "IfcRepresentationItem")
                && !schema.entity(name).expect("declared").abstract_
        })
        .map(str::to_ascii_uppercase)
        .collect();
    assert_eq!(
        expected.len(),
        128,
        "IFC4X3 ADD2 concrete representation items"
    );

    let implemented: BTreeSet<String> = IMPLEMENTED.iter().map(|n| (*n).to_owned()).collect();
    let planned: BTreeSet<String> = PLANNED.iter().map(|(n, _)| (*n).to_owned()).collect();
    // Nested input and non-shape items, whose IFC4 rows hold for IFC4X3
    // because IFC4X3 keeps every IFC4 item concrete.
    let nested: BTreeSet<String> = DISPOSITIONS
        .lines()
        .skip(1)
        .filter_map(|line| {
            let mut fields = line.split('\t');
            let name = fields.next()?;
            (fields.next()? != "planned-exact").then(|| name.to_owned())
        })
        .collect();

    for (left, right, what) in [
        (&implemented, &planned, "IMPLEMENTED and PLANNED"),
        (
            &implemented,
            &nested,
            "IMPLEMENTED and the disposition ledger",
        ),
        (&planned, &nested, "PLANNED and the disposition ledger"),
    ] {
        let both: Vec<_> = left.intersection(right).collect();
        assert!(both.is_empty(), "{what} overlap: {both:?}");
    }

    let classified: BTreeSet<String> = implemented
        .iter()
        .chain(&planned)
        .chain(&nested)
        .cloned()
        .collect();
    let missing: Vec<_> = expected.difference(&classified).collect();
    let extra: Vec<_> = classified.difference(&expected).collect();
    assert!(
        missing.is_empty() && extra.is_empty(),
        "IFC4X3 representation-item drift: missing={missing:?}; extra={extra:?}"
    );

    // A partial family is an implemented one, and each IFC4X3 addition that
    // is a representation item lands in IMPLEMENTED or PLANNED by name.
    for variant in PARTIAL {
        assert!(implemented.contains(variant.family), "{}", variant.family);
    }
    for addition in IFC4X3_ADDITIONS {
        let upper = addition.to_ascii_uppercase();
        if schema.is_a(addition, "IfcRepresentationItem") {
            assert!(
                implemented.contains(&upper) || planned.contains(&upper),
                "{addition} is an IFC4X3 representation item with no registry entry"
            );
        }
    }
}

/// Each routed subtype is a real subtype that adds only the named attributes.
///
/// `dispatch::SPECIALISATIONS` sends a subtype to its supertype's lowering.
/// That is exact only for the attributes the row has reviewed, so a schema
/// that adds one more must fail here rather than be ignored at runtime.
#[cfg(feature = "lowering")]
#[test]
fn every_specialisation_matches_the_ifc4x3_schema() {
    use ifc_geometry::lower::dispatch::{IMPLEMENTED, SPECIALISATIONS};

    assert!(
        !SPECIALISATIONS.is_empty(),
        "the table is read, not skipped"
    );
    let schema = ifc_schema::ifc4x3();
    for row in SPECIALISATIONS {
        assert_ne!(row.subtype, row.supertype);
        assert!(
            schema.is_a(row.subtype, row.supertype),
            "{} is not an IFC4X3 subtype of {}",
            row.subtype,
            row.supertype
        );
        for name in [row.subtype, row.supertype] {
            assert!(IMPLEMENTED.contains(&name), "{name} must be IMPLEMENTED");
        }
        let parent = schema.attribute_names(row.supertype);
        let child = schema.attribute_names(row.subtype);
        assert_eq!(
            child[..parent.len()],
            parent[..],
            "{}: inherited slots must read as on {}",
            row.subtype,
            row.supertype
        );
        assert_eq!(
            &child[parent.len()..],
            row.added_attributes,
            "{}: the added attributes changed; review the rationale",
            row.subtype
        );
        assert!(!row.rationale.trim().is_empty(), "{}", row.subtype);
    }
}
