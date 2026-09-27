//! Element connection and interference.
//!
//! These two families look alike and are laid out differently, which is the
//! whole reason this file exists.

use ifc_model::{Entity, EntityId, Model, Value};
use ifc_spatial::{relation, RelationshipKind};

fn put(model: &mut Model, id: u64, type_name: &str, attributes: Vec<Value>) -> EntityId {
    let entity_id = EntityId(id);
    model.insert(entity_id, Entity::new(type_name, attributes));
    entity_id
}

/// `IfcRelConnectsElements`: geometry at 4, ends at 5 and 6.
fn connects(geometry: Option<EntityId>, relating: EntityId, related: EntityId) -> Vec<Value> {
    let mut a = vec![Value::Null; 4];
    a.push(geometry.map_or(Value::Null, Value::Ref));
    a.push(Value::Ref(relating));
    a.push(Value::Ref(related));
    a
}

/// `IfcRelInterferesElements`: ends at 4 and 5, geometry after.
fn interferes(relating: EntityId, related: EntityId) -> Vec<Value> {
    let mut a = vec![Value::Null; 4];
    a.push(Value::Ref(relating));
    a.push(Value::Ref(related));
    a
}

/// The trap: `ConnectionGeometry` sits at slot 4, so the ends are at 5/6.
/// Reading 4/5 would name the geometry as the relating element.
#[test]
fn connection_ends_are_not_the_connection_geometry() {
    let mut model = Model::new();
    let wall = put(&mut model, 1, "IFCWALL", vec![]);
    let slab = put(&mut model, 2, "IFCSLAB", vec![]);
    let geometry = put(&mut model, 3, "IFCCONNECTIONSURFACEGEOMETRY", vec![]);

    put(
        &mut model,
        10,
        "IFCRELCONNECTSELEMENTS",
        connects(Some(geometry), wall, slab),
    );

    let all = relation::all(&model);
    let c = all
        .iter()
        .find(|r| r.kind == RelationshipKind::ConnectsElements)
        .expect("connection read");
    assert_eq!(
        c.relating,
        Some(wall),
        "relating is the wall, not the shape"
    );
    assert_eq!(c.related, vec![slab]);
    assert_ne!(
        c.relating,
        Some(geometry),
        "reading slot 4 would put the connection geometry here"
    );
}

/// Interference uses 4/5, the opposite convention to its look-alike.
#[test]
fn interference_ends_use_the_other_layout() {
    let mut model = Model::new();
    let duct = put(&mut model, 1, "IFCDUCTSEGMENT", vec![]);
    let beam = put(&mut model, 2, "IFCBEAM", vec![]);
    put(
        &mut model,
        10,
        "IFCRELINTERFERESELEMENTS",
        interferes(duct, beam),
    );

    let all = relation::all(&model);
    let i = all
        .iter()
        .find(|r| r.kind == RelationshipKind::InterferesElements)
        .expect("interference read");
    assert_eq!(i.relating, Some(duct));
    assert_eq!(i.related, vec![beam]);
}

/// All three concrete connects types must be found, not just the supertype.
#[test]
fn connects_subtypes_are_found() {
    let mut model = Model::new();
    let a = put(&mut model, 1, "IFCWALL", vec![]);
    let b = put(&mut model, 2, "IFCWALL", vec![]);

    put(
        &mut model,
        10,
        "IFCRELCONNECTSELEMENTS",
        connects(None, a, b),
    );
    put(
        &mut model,
        11,
        "IFCRELCONNECTSPATHELEMENTS",
        connects(None, a, b),
    );
    put(
        &mut model,
        12,
        "IFCRELCONNECTSWITHREALIZINGELEMENTS",
        connects(None, a, b),
    );

    let found: Vec<_> = relation::all(&model)
        .into_iter()
        .filter(|r| r.kind == RelationshipKind::ConnectsElements)
        .collect();
    assert_eq!(
        found.len(),
        3,
        "all three concrete connects types, got {found:?}"
    );
}

/// Connection and interference must not be confused: a clash is not an
/// adjacency, and a scheduler acting on the wrong one builds the wrong thing.
#[test]
fn connection_and_interference_stay_distinct() {
    let mut model = Model::new();
    let a = put(&mut model, 1, "IFCWALL", vec![]);
    let b = put(&mut model, 2, "IFCBEAM", vec![]);
    put(
        &mut model,
        10,
        "IFCRELCONNECTSELEMENTS",
        connects(None, a, b),
    );
    put(&mut model, 11, "IFCRELINTERFERESELEMENTS", interferes(a, b));

    let all = relation::all(&model);
    let connections = all
        .iter()
        .filter(|r| r.kind == RelationshipKind::ConnectsElements)
        .count();
    let clashes = all
        .iter()
        .filter(|r| r.kind == RelationshipKind::InterferesElements)
        .count();
    assert_eq!((connections, clashes), (1, 1));
}
