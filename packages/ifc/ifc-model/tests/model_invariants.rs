//! Structural invariants of the model itself.
//!
//! These are properties the entity graph must hold regardless of which codec
//! produced it, so they live here rather than in a codec's test suite.

use ifc_model::{Entity, EntityId, Model, Value};

/// A file may legally reuse an id only by replacing the entity. The order list
/// must not grow a second entry, or a re-export would emit the entity twice
/// and the file would gain a duplicate record.
///
/// Found by mutation testing: removing the `is_none()` guard in `Model::insert`
/// left every existing test green while corrupting export order.
#[test]
fn reinserting_an_id_replaces_rather_than_duplicating() {
    let mut model = Model::new();
    model.insert(EntityId(1), Entity::new("IFCWALL", vec![Value::Null]));
    model.insert(EntityId(1), Entity::new("IFCSLAB", vec![Value::Null]));

    assert_eq!(model.len(), 1, "id reuse must replace, not add");
    assert_eq!(
        model.ids().count(),
        1,
        "export order must not list the id twice"
    );
    assert!(
        model.get(EntityId(1)).unwrap().is_type("IFCSLAB"),
        "the later entity should win"
    );
}

/// `push` must never collide with an id already in the file.
#[test]
fn push_allocates_above_the_highest_existing_id() {
    let mut model = Model::new();
    model.insert(EntityId(500), Entity::new("IFCWALL", vec![]));
    model.insert(EntityId(12), Entity::new("IFCSLAB", vec![]));

    let id = model.push(Entity::new("IFCBEAM", vec![]));
    assert_eq!(id, EntityId(501), "must not reuse or undercut existing ids");
    assert_eq!(model.len(), 3);
}

/// Insertion order is the export order, and it is stable.
#[test]
fn iteration_follows_insertion_order_not_hash_order() {
    let mut model = Model::new();
    for id in [900u64, 3, 47, 12] {
        model.insert(EntityId(id), Entity::new("IFCWALL", vec![]));
    }
    let ids: Vec<u64> = model.ids().map(|i| i.0).collect();
    assert_eq!(ids, vec![900, 3, 47, 12]);
}

/// The type index must stay consistent with the entity store.
#[test]
fn type_index_matches_stored_entities() {
    let mut model = Model::new();
    model.insert(EntityId(1), Entity::new("IFCWALL", vec![]));
    model.insert(EntityId(2), Entity::new("IFCWALL", vec![]));
    model.insert(EntityId(3), Entity::new("IFCSLAB", vec![]));

    assert_eq!(model.ids_of_type("IFCWALL").len(), 2);
    assert_eq!(model.ids_of_type("ifcwall").len(), 2, "case-insensitive");
    assert_eq!(
        model.ids_of_type("IFCBEAM").len(),
        0,
        "absent type is empty"
    );

    let histogram = model.type_histogram();
    assert_eq!(histogram[0], ("IFCWALL", 2), "sorted by count descending");
}

/// Dangling references are found at any nesting depth, since real files bury
/// references inside aggregates.
#[test]
fn dangling_references_are_found_inside_nested_aggregates() {
    let mut model = Model::new();
    model.insert(
        EntityId(1),
        Entity::new(
            "IFCRELAGGREGATES",
            vec![Value::List(vec![
                Value::Ref(EntityId(2)),
                Value::List(vec![Value::Ref(EntityId(999))]),
            ])],
        ),
    );
    model.insert(EntityId(2), Entity::new("IFCWALL", vec![]));

    let dangling = model.dangling_references();
    assert_eq!(dangling, vec![(EntityId(1), EntityId(999))]);
}

/// The model must not interpret entities. This is the architectural rule from
/// ADR 0006, checked against the source rather than trusted.
#[test]
fn the_model_source_contains_no_domain_knowledge() {
    let src_dir = concat!(env!("CARGO_MANIFEST_DIR"), "/src");
    let mut offenders = Vec::new();

    for entry in std::fs::read_dir(src_dir).expect("src dir") {
        let path = entry.unwrap().path();
        if path.extension().is_none_or(|e| e != "rs") {
            continue;
        }
        let text = std::fs::read_to_string(&path).unwrap();
        for (n, line) in text.lines().enumerate() {
            // Ignore comments and doc comments: prose may legitimately mention
            // IfcWall when explaining why the model does not know about it.
            let code = line.trim_start();
            if code.starts_with("//") {
                continue;
            }
            // A string literal naming a concrete entity type means this crate
            // is interpreting, which is the domain crates' job.
            for marker in ["\"IFCWALL", "\"IFCCOSTITEM", "\"IFCPROJECT", "\"IFCSLAB"] {
                if code.to_ascii_uppercase().contains(marker) {
                    offenders.push(format!("{}:{}: {}", path.display(), n + 1, code.trim()));
                }
            }
        }
    }

    assert!(
        offenders.is_empty(),
        "ifc-model must not name concrete entity types -- that is domain \
         knowledge and belongs in a view crate. See docs/adr/0006.\n{}",
        offenders.join("\n")
    );
}
