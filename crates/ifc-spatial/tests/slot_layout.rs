//! The hard-coded slot positions must match the normative schema.
//!
//! `src/relation/slots.rs` states attribute positions as constants rather than
//! reading them at runtime. That is only safe if the constants are true, and
//! the cost of them being wrong is silent inversion of the containment tree —
//! elements becoming the parents of their storey.
//!
//! So the claim is checked against the shipped EXPRESS schemas. Skips when the
//! reference material is absent, matching `crates/ifc-schema/tests/real_schemas.rs`.

use ifc_schema::Schema;
use std::path::PathBuf;

/// Locate `references/ifc-spec`, which sits at a different depth
/// depending on how this repository is checked out.
///
/// As a submodule of the openbim superproject the crate is at
/// `packages/ifc/crates/<crate>`, so references is four levels up. Checked out
/// standalone -- which is what CI does -- the repository root IS
/// `packages/ifc`, so it is two levels up. Trying only one of those makes
/// the schema tests silently skip in the other layout.
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

fn load(rel: &str) -> Option<Schema> {
    let Some(root) = spec_root() else {
        // No schemas anywhere. CI sets IFC_SPEC_REQUIRED so this is a
        // failure there; locally it stays a skip so a fresh clone passes.
        assert!(
            std::env::var_os("IFC_SPEC_REQUIRED").is_none(),
            "IFC_SPEC_REQUIRED is set but references/ifc-spec was not found; \
             run scripts/fetch-ifc-schemas.sh"
        );
        return None;
    };
    let path = root.join(rel);
    let bytes = std::fs::read(path).ok()?;
    let text: String = bytes.iter().map(|&b| b as char).collect();
    Some(Schema::from_express(&text))
}

/// The exact layout `slots.rs` encodes, including the inversion between the
/// two relationship types.
const EXPECTED: &[(&str, usize, &str, usize, &str)] = &[
    ("IfcRelAggregates", 4, "RelatingObject", 5, "RelatedObjects"),
    (
        "IfcRelContainedInSpatialStructure",
        5,
        "RelatingStructure",
        4,
        "RelatedElements",
    ),
    ("IfcRelNests", 4, "RelatingObject", 5, "RelatedObjects"),
    (
        "IfcRelSpaceBoundary",
        4,
        "RelatingSpace",
        5,
        "RelatedBuildingElement",
    ),
    (
        "IfcRelSpaceBoundary1stLevel",
        4,
        "RelatingSpace",
        5,
        "RelatedBuildingElement",
    ),
    (
        "IfcRelSpaceBoundary2ndLevel",
        4,
        "RelatingSpace",
        5,
        "RelatedBuildingElement",
    ),
    (
        "IfcRelCoversBldgElements",
        4,
        "RelatingBuildingElement",
        5,
        "RelatedCoverings",
    ),
    (
        "IfcRelCoversSpaces",
        4,
        "RelatingSpace",
        5,
        "RelatedCoverings",
    ),
    // The connects family shifts its ends to 5/6: ConnectionGeometry is 4.
    (
        "IfcRelConnectsElements",
        5,
        "RelatingElement",
        6,
        "RelatedElement",
    ),
    (
        "IfcRelConnectsPathElements",
        5,
        "RelatingElement",
        6,
        "RelatedElement",
    ),
    (
        "IfcRelConnectsWithRealizingElements",
        5,
        "RelatingElement",
        6,
        "RelatedElement",
    ),
    // Interference is NOT a connects subtype and keeps 4/5.
    (
        "IfcRelInterferesElements",
        4,
        "RelatingElement",
        5,
        "RelatedElement",
    ),
    // The assigns family brackets RelatedObjectsType (slot 5) between its
    // two ends: related FIRST at 4, relating at 6.
    (
        "IfcRelAssignsToActor",
        6,
        "RelatingActor",
        4,
        "RelatedObjects",
    ),
    (
        "IfcRelAssignsToProcess",
        6,
        "RelatingProcess",
        4,
        "RelatedObjects",
    ),
    (
        "IfcRelAssignsToProduct",
        6,
        "RelatingProduct",
        4,
        "RelatedObjects",
    ),
    (
        "IfcRelAssignsToGroupByFactor",
        6,
        "RelatingGroup",
        4,
        "RelatedObjects",
    ),
    // These five disagree with each other: three relating-first, two not.
    (
        "IfcRelDeclares",
        4,
        "RelatingContext",
        5,
        "RelatedDefinitions",
    ),
    (
        "IfcRelDefinesByObject",
        5,
        "RelatingObject",
        4,
        "RelatedObjects",
    ),
    (
        "IfcRelFlowControlElements",
        5,
        "RelatingFlowElement",
        4,
        "RelatedControlElements",
    ),
    (
        "IfcRelServicesBuildings",
        4,
        "RelatingSystem",
        5,
        "RelatedBuildings",
    ),
    (
        "IfcRelConnectsWithEccentricity",
        4,
        "RelatingStructuralMember",
        5,
        "RelatedStructuralConnection",
    ),
];

/// Attributes IFC4 renamed, with the name the older schema uses.
///
/// IFC2X3 spells IfcRelCoversSpaces slot 4 `RelatedSpace`; IFC4 renamed it
/// to `RelatingSpace`. The slot POSITION is 4 in both, so the reader is
/// correct in either -- only the name differs, and asserting the IFC4 name
/// against IFC2X3 fails on a schema the crate genuinely supports.
const RENAMED_BEFORE_IFC4: &[(&str, &str, &str)] =
    &[("IfcRelCoversSpaces", "RelatingSpace", "RelatedSpace")];

/// The name `entity.attribute` carries in this schema version.
fn expected_name<'a>(version: &str, entity: &str, ifc4_name: &'a str) -> &'a str {
    if version != "IFC2X3" {
        return ifc4_name;
    }
    for (e, new, old) in RENAMED_BEFORE_IFC4 {
        if e == &entity && new == &ifc4_name {
            return old;
        }
    }
    ifc4_name
}

fn check(schema: &Schema, version: &str) {
    for (entity, relating_slot, relating_name, related_slot, related_name) in EXPECTED {
        let names = schema.attribute_names(entity);
        if names.is_empty() {
            // IfcRelNests exists in every version this crate targets, but a
            // schema that lacks an entity should skip rather than fail.
            continue;
        }
        let relating_name = &expected_name(version, entity, relating_name);
        let related_name = &expected_name(version, entity, related_name);
        assert_eq!(
            names.get(*relating_slot),
            Some(relating_name),
            "{version}: {entity} slot {relating_slot} must be {relating_name}, got {names:?}"
        );
        assert_eq!(
            names.get(*related_slot),
            Some(related_name),
            "{version}: {entity} slot {related_slot} must be {related_name}, got {names:?}"
        );
    }
}

#[test]
fn slot_constants_match_ifc4() {
    let Some(schema) = load("ifc4-add2-tc1/IFC4.exp") else {
        eprintln!("skipped: references/ifc-spec not present");
        return;
    };
    check(&schema, "IFC4");
}

#[test]
fn slot_constants_match_ifc2x3() {
    let Some(schema) = load("ifc2x3-tc1/IFC2X3_TC1.exp") else {
        eprintln!("skipped: references/ifc-spec not present");
        return;
    };
    check(&schema, "IFC2X3");
}

#[test]
fn slot_constants_match_ifc4x3() {
    let Some(schema) = load("ifc4x3-add2/IFC4X3_ADD2.exp") else {
        eprintln!("skipped: references/ifc-spec not present");
        return;
    };
    check(&schema, "IFC4X3");
}

/// The inversion is the entire reason this test file exists; assert it
/// explicitly so a future edit cannot "tidy" the constants into agreement.
#[test]
fn the_two_relationships_really_do_disagree() {
    let Some(schema) = load("ifc4-add2-tc1/IFC4.exp") else {
        eprintln!("skipped: references/ifc-spec not present");
        return;
    };
    let aggregates = schema.attribute_names("IfcRelAggregates");
    let contained = schema.attribute_names("IfcRelContainedInSpatialStructure");

    assert!(aggregates[4].starts_with("Relating"), "{aggregates:?}");
    assert!(
        contained[4].starts_with("Related"),
        "if this ever matches IfcRelAggregates, slots.rs must be revisited: {contained:?}"
    );
}

/// Slots the boundary reader uses beyond the two ends.
///
/// `boundary.rs` reads the enumerations at 7/8 and the subtype links at
/// 9/10. Those are as load-bearing as the ends: reading 8 instead of 7 would
/// classify every boundary's exposure as its physicality and silently
/// mislabel the lot.
const BOUNDARY_DEEP: &[(&str, usize, &str)] = &[
    ("IfcRelSpaceBoundary", 6, "ConnectionGeometry"),
    ("IfcRelSpaceBoundary", 7, "PhysicalOrVirtualBoundary"),
    ("IfcRelSpaceBoundary", 8, "InternalOrExternalBoundary"),
    ("IfcRelSpaceBoundary1stLevel", 9, "ParentBoundary"),
    ("IfcRelSpaceBoundary2ndLevel", 9, "ParentBoundary"),
    ("IfcRelSpaceBoundary2ndLevel", 10, "CorrespondingBoundary"),
];

fn check_deep(schema: &Schema, version: &str) {
    for (entity, slot, name) in BOUNDARY_DEEP {
        let names = schema.attribute_names(entity);
        if names.is_empty() {
            // IFC2x3 has no 1st/2nd-level subtypes; skip rather than fail.
            continue;
        }
        assert_eq!(
            names.get(*slot),
            Some(name),
            "{version}: {entity} slot {slot} must be {name}, got {names:?}"
        );
    }
}

#[test]
fn boundary_deep_slots_match_ifc4() {
    let Some(schema) = load("ifc4-add2-tc1/IFC4.exp") else {
        eprintln!("skipped: references/ifc-spec not present");
        return;
    };
    check_deep(&schema, "IFC4");
}

#[test]
fn boundary_deep_slots_match_ifc4x3() {
    let Some(schema) = load("ifc4x3-add2/IFC4X3_ADD2.exp") else {
        eprintln!("skipped: references/ifc-spec not present");
        return;
    };
    check_deep(&schema, "IFC4X3");
}

#[test]
fn boundary_deep_slots_match_ifc2x3() {
    let Some(schema) = load("ifc2x3-tc1/IFC2X3_TC1.exp") else {
        eprintln!("skipped: references/ifc-spec not present");
        return;
    };
    check_deep(&schema, "IFC2X3");
}

/// `SpaceBoundary::connection_geometry` reads slot 6 on every concrete
/// boundary type and treats `$` as "no shape".
///
/// Checked against the three BUNDLED schema tables rather than the `.exp`
/// references, so this never skips: the slot, its name, its declared type
/// and its optionality must hold in IFC2x3 TC1, IFC4 ADD2 TC1 and IFC4X3
/// ADD2. IFC2x3 has no 1st/2nd-level subtypes; asserting that explicitly
/// keeps the skip from hiding a table that lost them.
#[test]
fn connection_geometry_slot_matches_every_bundled_schema() {
    for (version, schema, has_levels) in [
        ("IFC2X3", ifc_schema::ifc2x3(), false),
        ("IFC4", ifc_schema::ifc4(), true),
        ("IFC4X3", ifc_schema::ifc4x3(), true),
    ] {
        for entity in [
            "IfcRelSpaceBoundary",
            "IfcRelSpaceBoundary1stLevel",
            "IfcRelSpaceBoundary2ndLevel",
        ] {
            let attributes = schema.attributes(entity);
            if attributes.is_empty() {
                assert!(
                    !has_levels && entity != "IfcRelSpaceBoundary",
                    "{version}: {entity} missing from the bundled table"
                );
                continue;
            }
            assert!(
                has_levels || entity == "IfcRelSpaceBoundary",
                "{version}: unexpected {entity}"
            );
            let slot = attributes
                .get(6)
                .unwrap_or_else(|| panic!("{version}: {entity} has no slot 6"));
            assert_eq!(slot.name, "ConnectionGeometry", "{version}: {entity}");
            assert_eq!(
                slot.type_name, "IfcConnectionGeometry",
                "{version}: {entity}"
            );
            assert!(
                slot.optional,
                "{version}: {entity}.ConnectionGeometry is OPTIONAL"
            );
            assert!(
                !slot.aggregate,
                "{version}: {entity}.ConnectionGeometry is scalar"
            );
        }
    }
}

/// `boundary.rs` accepts exactly these as a boundary's connection geometry.
///
/// Must equal the union of the concrete `IfcConnectionGeometry` subtypes
/// over the three bundled schemas: a subtype missing here would be reported
/// as a wrong-kind anomaly for a valid file, and one added here that no
/// schema declares would accept garbage as a shape. `tests/boundary_geometry.rs`
/// proves the reader accepts each of them.
const CONNECTION_GEOMETRY_TYPES: [&str; 6] = [
    "IFCCONNECTIONCURVEGEOMETRY",
    "IFCCONNECTIONPOINTECCENTRICITY",
    "IFCCONNECTIONPOINTGEOMETRY",
    "IFCCONNECTIONPORTGEOMETRY",
    "IFCCONNECTIONSURFACEGEOMETRY",
    "IFCCONNECTIONVOLUMEGEOMETRY",
];

#[test]
fn connection_geometry_subtypes_match_the_bundled_schemas() {
    let mut union = std::collections::BTreeSet::new();
    for (version, schema) in [
        ("IFC2X3", ifc_schema::ifc2x3()),
        ("IFC4", ifc_schema::ifc4()),
        ("IFC4X3", ifc_schema::ifc4x3()),
    ] {
        let subtypes = schema.subtypes("IfcConnectionGeometry");
        assert!(
            !subtypes.is_empty(),
            "{version}: no IfcConnectionGeometry subtypes"
        );
        union.extend(subtypes.into_iter().map(str::to_ascii_uppercase));
    }
    let expected: std::collections::BTreeSet<String> = CONNECTION_GEOMETRY_TYPES
        .iter()
        .map(|name| (*name).to_owned())
        .collect();
    assert_eq!(union, expected);
}
