//! Parse the real buildingSMART EXPRESS schemas.
//!
//! These are the normative artifacts under `references/ifc-spec/`, which is a
//! symlink to bulk storage and is not committed. The tests skip when it is
//! absent so a fresh clone still passes, and assert loudly when it is present.
//!
//! Expected counts come from `grep -c '^ENTITY' <file>` on the shipped
//! schemas; they are facts about the files, not aspirations.

use ifc_schema::{Schema, SchemaVersion};
use std::path::PathBuf;

fn spec_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../references/ifc-spec")
}

fn load(rel: &str) -> Option<Schema> {
    let path = spec_dir().join(rel);
    let bytes = std::fs::read(&path).ok()?;
    // The schemas are latin-1 with CRLF line endings, not UTF-8.
    let text: String = bytes.iter().map(|&b| b as char).collect();
    Some(Schema::from_express(&text))
}

#[test]
fn parses_ifc2x3_tc1() {
    let Some(schema) = load("ifc2x3-tc1/IFC2X3_TC1.exp") else {
        eprintln!("skipped: references/ifc-spec not present");
        return;
    };
    assert_eq!(schema.version(), Some(SchemaVersion::Ifc2x3));
    assert_eq!(schema.entity_count(), 653, "IFC2x3 TC1 entity count");
}

#[test]
fn parses_ifc4_add2_tc1() {
    let Some(schema) = load("ifc4-add2-tc1/IFC4.exp") else {
        eprintln!("skipped: references/ifc-spec not present");
        return;
    };
    assert_eq!(schema.version(), Some(SchemaVersion::Ifc4));
    assert_eq!(schema.entity_count(), 776, "IFC4 ADD2 TC1 entity count");
    assert_eq!(schema.type_count(), 397, "IFC4 ADD2 TC1 type count");
}

#[test]
fn parses_ifc4x3_add2() {
    let Some(schema) = load("ifc4x3-add2/IFC4X3_ADD2.exp") else {
        eprintln!("skipped: references/ifc-spec not present");
        return;
    };
    assert_eq!(schema.version(), Some(SchemaVersion::Ifc4x3));
    assert_eq!(schema.entity_count(), 876, "IFC4x3 ADD2 entity count");
}

/// Real inheritance chains, read from the real schema.
#[test]
fn resolves_deep_inheritance_in_ifc4() {
    let Some(schema) = load("ifc4-add2-tc1/IFC4.exp") else {
        eprintln!("skipped: references/ifc-spec not present");
        return;
    };

    // IfcWall -> IfcBuildingElement -> IfcElement -> IfcProduct -> IfcObject
    //         -> IfcObjectDefinition -> IfcRoot
    assert!(schema.is_a("IFCWALL", "IfcRoot"), "wall is a root");
    assert!(schema.is_a("IFCWALL", "IfcProduct"), "wall is a product");
    assert!(!schema.is_a("IFCWALL", "IfcCostItem"), "wall is not a cost");

    // The first four slots of every rooted entity, inherited from IfcRoot.
    let names = schema.attribute_names("IFCWALL");
    assert_eq!(
        &names[..4],
        ["GlobalId", "OwnerHistory", "Name", "Description"],
        "IfcRoot's slots must come first"
    );
}

/// The 4x3 rename that justifies schema-as-data.
#[test]
fn ifc4x3_renamed_the_building_element_supertype() {
    let (Some(ifc4), Some(ifc4x3)) = (
        load("ifc4-add2-tc1/IFC4.exp"),
        load("ifc4x3-add2/IFC4X3_ADD2.exp"),
    ) else {
        eprintln!("skipped: references/ifc-spec not present");
        return;
    };

    assert!(ifc4.entity("IfcBuildingElement").is_some());
    assert!(ifc4.entity("IfcBuiltElement").is_none());

    assert!(ifc4x3.entity("IfcBuildingElement").is_none());
    assert!(ifc4x3.entity("IfcBuiltElement").is_some());

    // Same query, both schemas, different answer -- and no code change.
    assert!(ifc4.is_a("IfcWall", "IfcBuildingElement"));
    assert!(ifc4x3.is_a("IfcWall", "IfcBuiltElement"));
}

/// Cost entities exist in the schema without the model knowing about them.
#[test]
fn cost_entities_are_ordinary_schema_rows() {
    let Some(schema) = load("ifc4-add2-tc1/IFC4.exp") else {
        eprintln!("skipped: references/ifc-spec not present");
        return;
    };

    assert!(schema.is_a("IFCCOSTITEM", "IfcControl"));
    let names = schema.attribute_names("IFCCOSTVALUE");
    assert!(
        names.contains(&"AppliedValue"),
        "IfcCostValue should declare AppliedValue, got {names:?}"
    );
}
