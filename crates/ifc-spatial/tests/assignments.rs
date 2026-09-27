//! Assignment relationships, whose ends bracket an enumeration.

use ifc_model::{Entity, EntityId, Model, Value};
use ifc_spatial::{relation, RelationshipKind};

fn put(model: &mut Model, id: u64, type_name: &str, attributes: Vec<Value>) -> EntityId {
    let entity_id = EntityId(id);
    model.insert(entity_id, Entity::new(type_name, attributes));
    entity_id
}

/// `IfcRelAssigns*`: related list at 4, RelatedObjectsType at 5, relating
/// at 6. The enumeration sits BETWEEN the two ends.
fn assigns(related: &[EntityId], relating: EntityId) -> Vec<Value> {
    let mut a = vec![Value::Null; 4];
    a.push(Value::List(
        related.iter().copied().map(Value::Ref).collect(),
    ));
    a.push(Value::Enum("PRODUCT".into()));
    a.push(Value::Ref(relating));
    a
}

/// The trap: slot 5 holds an enumeration, not a reference. A reader
/// assuming the usual "relating at 5" finds no reference there and the
/// assignment vanishes silently.
#[test]
fn relating_end_is_at_six_not_five() {
    let mut model = Model::new();
    let task = put(&mut model, 1, "IFCTASK", vec![]);
    let wall = put(&mut model, 2, "IFCWALL", vec![]);
    put(
        &mut model,
        10,
        "IFCRELASSIGNSTOPROCESS",
        assigns(&[wall], task),
    );

    let all = relation::all(&model);
    let a = all
        .iter()
        .find(|r| r.kind == RelationshipKind::AssignsToProcess)
        .expect("assignment read");
    assert_eq!(
        a.relating,
        Some(task),
        "relating must come from slot 6; slot 5 is an enum"
    );
    assert_eq!(a.related, vec![wall]);
}

/// All four assignment kinds are distinguished, not merged.
#[test]
fn each_assignment_kind_is_distinct() {
    let mut model = Model::new();
    let object = put(&mut model, 1, "IFCWALL", vec![]);
    let actor = put(&mut model, 2, "IFCACTOR", vec![]);
    let process = put(&mut model, 3, "IFCTASK", vec![]);
    let product = put(&mut model, 4, "IFCPRODUCT", vec![]);
    let group = put(&mut model, 5, "IFCGROUP", vec![]);

    put(
        &mut model,
        10,
        "IFCRELASSIGNSTOACTOR",
        assigns(&[object], actor),
    );
    put(
        &mut model,
        11,
        "IFCRELASSIGNSTOPROCESS",
        assigns(&[object], process),
    );
    put(
        &mut model,
        12,
        "IFCRELASSIGNSTOPRODUCT",
        assigns(&[object], product),
    );
    put(
        &mut model,
        13,
        "IFCRELASSIGNSTOGROUPBYFACTOR",
        assigns(&[object], group),
    );

    let all = relation::all(&model);
    let kind_of = |k: RelationshipKind| all.iter().filter(|r| r.kind == k).count();
    assert_eq!(kind_of(RelationshipKind::AssignsToActor), 1);
    assert_eq!(kind_of(RelationshipKind::AssignsToProcess), 1);
    assert_eq!(kind_of(RelationshipKind::AssignsToProduct), 1);
    assert_eq!(kind_of(RelationshipKind::AssignsToGroupByFactor), 1);

    // Every one resolves both ends: none silently emptied by the enum slot.
    for r in &all {
        assert_eq!(r.related, vec![object], "related end resolved for {r:?}");
        assert!(r.relating.is_some(), "relating end resolved for {r:?}");
    }
}

/// A by-factor assignment is a concrete subtype of IfcRelAssignsToGroup and
/// is invisible to an exact-name lookup of the supertype.
#[test]
fn group_by_factor_is_reachable() {
    let mut model = Model::new();
    let object = put(&mut model, 1, "IFCWALL", vec![]);
    let group = put(&mut model, 2, "IFCGROUP", vec![]);
    put(
        &mut model,
        10,
        "IFCRELASSIGNSTOGROUPBYFACTOR",
        assigns(&[object], group),
    );

    let found: Vec<_> = relation::all(&model)
        .into_iter()
        .filter(|r| r.kind == RelationshipKind::AssignsToGroupByFactor)
        .collect();
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].relating, Some(group));
}

/// The inverse query reaches assignments too.
#[test]
fn naming_finds_the_assignment_for_an_object() {
    let mut model = Model::new();
    let wall = put(&mut model, 1, "IFCWALL", vec![]);
    let actor = put(&mut model, 2, "IFCACTOR", vec![]);
    put(
        &mut model,
        10,
        "IFCRELASSIGNSTOACTOR",
        assigns(&[wall], actor),
    );

    let naming = relation::naming(&model, wall);
    assert_eq!(naming.len(), 1);
    assert_eq!(naming[0].kind, RelationshipKind::AssignsToActor);
    assert_eq!(naming[0].relating, Some(actor));
}
