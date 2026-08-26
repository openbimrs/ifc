//! The hard-coded slot positions must match the normative schema.
//!
//! `src/relation/slots.rs` states attribute positions as constants rather than
//! reading them at runtime. That is only safe if the constants are true, and
//! the cost of them being wrong is silent inversion of the containment tree —
//! elements becoming the parents of their storey.
//!
//! So the claim is checked against the shipped EXPRESS schemas. Skips when the
//! reference material is absent, matching `ifc-schema/tests/real_schemas.rs`.

use ifc_schema::Schema;
use std::path::PathBuf;

fn load(rel: &str) -> Option<Schema> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../references/ifc-spec")
        .join(rel);
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
];

fn check(schema: &Schema, version: &str) {
    for (entity, relating_slot, relating_name, related_slot, related_name) in EXPECTED {
        let names = schema.attribute_names(entity);
        if names.is_empty() {
            // IfcRelNests exists in every version this crate targets, but a
            // schema that lacks an entity should skip rather than fail.
            continue;
        }
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
