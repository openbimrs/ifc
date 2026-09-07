//! Coverings: finishes applied to elements and bounding spaces.

use ifc_model::{Entity, EntityId, Model, Value};
use ifc_spatial::{relation, RelationshipKind};

fn put(model: &mut Model, id: u64, type_name: &str, attributes: Vec<Value>) -> EntityId {
    let entity_id = EntityId(id);
    model.insert(entity_id, Entity::new(type_name, attributes));
    entity_id
}

/// Four IfcRoot slots, then relating at 4 and the related list at 5.
fn rel(relating: EntityId, related: &[EntityId]) -> Vec<Value> {
    let mut a = vec![Value::Null; 4];
    a.push(Value::Ref(relating));
    a.push(Value::List(
        related.iter().copied().map(Value::Ref).collect(),
    ));
    a
}

/// The same covering can clad an element AND bound a space. The two
/// relationships answer different questions, so they must stay apart.
#[test]
fn element_and_space_coverings_are_distinguished() {
    let mut model = Model::new();
    let slab = put(&mut model, 1, "IFCSLAB", vec![]);
    let space = put(&mut model, 2, "IFCSPACE", vec![]);
    let ceiling = put(&mut model, 3, "IFCCOVERING", vec![]);

    put(
        &mut model,
        10,
        "IFCRELCOVERSBLDGELEMENTS",
        rel(slab, &[ceiling]),
    );
    put(&mut model, 11, "IFCRELCOVERSSPACES", rel(space, &[ceiling]));

    let all = relation::all(&model);
    let by_element: Vec<_> = all
        .iter()
        .filter(|r| r.kind == RelationshipKind::CoversElements)
        .collect();
    let by_space: Vec<_> = all
        .iter()
        .filter(|r| r.kind == RelationshipKind::CoversSpaces)
        .collect();

    assert_eq!(by_element.len(), 1, "element covering read");
    assert_eq!(by_space.len(), 1, "space covering read");
    assert_eq!(by_element[0].relating, Some(slab));
    assert_eq!(by_space[0].relating, Some(space));
    // Same covering on both ends -- the distinction is the relating end.
    assert_eq!(by_element[0].related, vec![ceiling]);
    assert_eq!(by_space[0].related, vec![ceiling]);
}

/// RelatedCoverings is SET [1:?]: several finishes on one element.
#[test]
fn multiple_coverings_keep_file_order() {
    let mut model = Model::new();
    let wall = put(&mut model, 1, "IFCWALL", vec![]);
    let plaster = put(&mut model, 2, "IFCCOVERING", vec![]);
    let paint = put(&mut model, 3, "IFCCOVERING", vec![]);
    let tile = put(&mut model, 4, "IFCCOVERING", vec![]);

    put(
        &mut model,
        10,
        "IFCRELCOVERSBLDGELEMENTS",
        rel(wall, &[plaster, paint, tile]),
    );

    let all = relation::all(&model);
    let covering = all
        .iter()
        .find(|r| r.kind == RelationshipKind::CoversElements)
        .expect("covering found");
    assert_eq!(
        covering.related,
        vec![plaster, paint, tile],
        "file order preserved: layer order is meaningful for a build-up"
    );
}

/// The inverse query must reach coverings too, not just containment.
#[test]
fn naming_finds_the_covering_that_names_an_element() {
    let mut model = Model::new();
    let wall = put(&mut model, 1, "IFCWALL", vec![]);
    let plaster = put(&mut model, 2, "IFCCOVERING", vec![]);
    put(
        &mut model,
        10,
        "IFCRELCOVERSBLDGELEMENTS",
        rel(wall, &[plaster]),
    );

    let naming = relation::naming(&model, plaster);
    assert_eq!(naming.len(), 1);
    assert_eq!(naming[0].kind, RelationshipKind::CoversElements);
    assert_eq!(naming[0].relating, Some(wall));
}
