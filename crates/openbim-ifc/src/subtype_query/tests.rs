//! Unit tests for subtype-inclusive type queries.
//!
//! Uses the bundled IFC4 and IFC4X3 schemas, not a synthetic one: the point
//! is that the real inheritance tree is honoured, including the parts that
//! differ between versions.

use ifc_model::{Entity, EntityId, Model};
use ifc_schema::{ifc4, ifc4x3};

use super::ids_of_type_including_subtypes;

/// Interleaves element and non-element types so file order is observable.
fn model() -> Model {
    let mut model = Model::new();
    for type_name in [
        "IFCPROJECT",
        "IFCWALL",
        "IFCPROPERTYSET",
        "IFCDOOR",
        "IFCWALLSTANDARDCASE",
        "IFCBEAM",
        "IFCWALL",
    ] {
        model.push(Entity::new(type_name, Vec::new()));
    }
    model
}

#[test]
fn a_supertype_query_returns_every_subtype_in_file_order() {
    let model = model();
    let elements = ids_of_type_including_subtypes(&model, ifc4(), "IfcElement");
    assert_eq!(
        elements,
        [
            EntityId(2),
            EntityId(4),
            EntityId(5),
            EntityId(6),
            EntityId(7)
        ],
        "walls, the door, the standard-case wall and the beam, in file order"
    );
}

#[test]
fn the_exact_type_is_included_alongside_its_subtypes() {
    let model = model();
    assert_eq!(
        ids_of_type_including_subtypes(&model, ifc4(), "IfcWall"),
        [EntityId(2), EntityId(5), EntityId(7)],
        "IfcWall instances plus the IfcWallStandardCase between them"
    );
}

/// The plain exact-type index is the baseline this module exists to fix.
#[test]
fn the_exact_type_index_alone_misses_them() {
    assert!(model().ids_of_type("IfcElement").is_empty());
}

#[test]
fn a_misspelled_type_finds_nothing() {
    assert!(ids_of_type_including_subtypes(&model(), ifc4(), "IfcWal").is_empty());
}

/// `IfcBuiltElement` exists only in IFC4X3, so the same query means different
/// things under different schemas.
#[test]
fn the_schema_decides_the_tree() {
    let model = model();
    assert!(ids_of_type_including_subtypes(&model, ifc4(), "IfcBuiltElement").is_empty());
    assert_eq!(
        ids_of_type_including_subtypes(&model, ifc4x3(), "IfcBuiltElement"),
        [
            EntityId(2),
            EntityId(4),
            EntityId(5),
            EntityId(6),
            EntityId(7)
        ],
        "walls, the door and the beam are all built elements in IFC4X3"
    );
}
