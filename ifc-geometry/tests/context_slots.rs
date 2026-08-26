//! The context slot constants must match the normative schemas.
//!
//! `input/context.rs` states attribute positions as constants. The dangerous
//! one is the sub-context: it inherits all six attributes of its supertype, so
//! its own start at 6. Off by one and `TargetScale` is read as the target view,
//! which silently reclassifies every drawing context in the file.
//!
//! Follows the pattern of `ifc-spatial/tests/slot_layout.rs` (ADR 0008).

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

/// (entity, slot, expected attribute name)
const EXPECTED: &[(&str, usize, &str)] = &[
    ("IfcGeometricRepresentationContext", 0, "ContextIdentifier"),
    ("IfcGeometricRepresentationContext", 1, "ContextType"),
    (
        "IfcGeometricRepresentationContext",
        2,
        "CoordinateSpaceDimension",
    ),
    ("IfcGeometricRepresentationContext", 3, "Precision"),
    (
        "IfcGeometricRepresentationContext",
        4,
        "WorldCoordinateSystem",
    ),
    ("IfcGeometricRepresentationContext", 5, "TrueNorth"),
    // The subtype's own attributes begin after all six inherited ones.
    ("IfcGeometricRepresentationSubContext", 6, "ParentContext"),
    ("IfcGeometricRepresentationSubContext", 7, "TargetScale"),
    ("IfcGeometricRepresentationSubContext", 8, "TargetView"),
    (
        "IfcGeometricRepresentationSubContext",
        9,
        "UserDefinedTargetView",
    ),
    ("IfcShapeRepresentation", 0, "ContextOfItems"),
    ("IfcShapeRepresentation", 1, "RepresentationIdentifier"),
    ("IfcShapeRepresentation", 2, "RepresentationType"),
    ("IfcShapeRepresentation", 3, "Items"),
];

fn check(schema: &Schema, version: &str) {
    for (entity, slot, expected) in EXPECTED {
        let names = schema.attribute_names(entity);
        if names.is_empty() {
            continue;
        }
        assert_eq!(
            names.get(*slot),
            Some(expected),
            "{version}: {entity} slot {slot} must be {expected}, got {names:?}"
        );
    }
}

#[test]
fn context_slots_match_ifc4() {
    let Some(schema) = load("ifc4-add2-tc1/IFC4.exp") else {
        eprintln!("skipped: references/ifc-spec not present");
        return;
    };
    check(&schema, "IFC4");
}

#[test]
fn context_slots_match_ifc2x3() {
    let Some(schema) = load("ifc2x3-tc1/IFC2X3_TC1.exp") else {
        eprintln!("skipped: references/ifc-spec not present");
        return;
    };
    check(&schema, "IFC2X3");
}

#[test]
fn context_slots_match_ifc4x3() {
    let Some(schema) = load("ifc4x3-add2/IFC4X3_ADD2.exp") else {
        eprintln!("skipped: references/ifc-spec not present");
        return;
    };
    check(&schema, "IFC4X3");
}

/// The sub-context must keep inheriting exactly six attributes. If a schema
/// ever changes that, every constant above shifts and this fails first.
#[test]
fn the_sub_context_still_inherits_six_attributes() {
    let Some(schema) = load("ifc4-add2-tc1/IFC4.exp") else {
        eprintln!("skipped: references/ifc-spec not present");
        return;
    };
    let parent = schema.attribute_names("IfcGeometricRepresentationContext");
    let child = schema.attribute_names("IfcGeometricRepresentationSubContext");

    assert_eq!(parent.len(), 6, "{parent:?}");
    assert_eq!(child.len(), 10, "{child:?}");
    assert_eq!(
        child[..6],
        parent[..],
        "the subtype must inherit the supertype's slots in order"
    );
}
