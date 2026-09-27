//! The last five relationship families, whose layouts disagree.

use ifc_model::{Entity, EntityId, Model, Value};
use ifc_spatial::{relation, RelationshipKind};

fn put(model: &mut Model, id: u64, type_name: &str, attributes: Vec<Value>) -> EntityId {
    let entity_id = EntityId(id);
    model.insert(entity_id, Entity::new(type_name, attributes));
    entity_id
}

/// Relating at 4, related list at 5.
fn relating_first(relating: EntityId, related: &[EntityId]) -> Vec<Value> {
    let mut a = vec![Value::Null; 4];
    a.push(Value::Ref(relating));
    a.push(Value::List(
        related.iter().copied().map(Value::Ref).collect(),
    ));
    a
}

/// Related list at 4, relating at 5 -- the inverse.
fn related_first(related: &[EntityId], relating: EntityId) -> Vec<Value> {
    let mut a = vec![Value::Null; 4];
    a.push(Value::List(
        related.iter().copied().map(Value::Ref).collect(),
    ));
    a.push(Value::Ref(relating));
    a
}

/// Two neighbours in the schema use opposite layouts. Getting either wrong
/// inverts the direction of the relationship without failing.
#[test]
fn declares_and_defines_by_object_use_opposite_layouts() {
    let mut model = Model::new();
    let project = put(&mut model, 1, "IFCPROJECT", vec![]);
    let wall_type = put(&mut model, 2, "IFCWALLTYPE", vec![]);
    let source = put(&mut model, 3, "IFCWALL", vec![]);
    let copy = put(&mut model, 4, "IFCWALL", vec![]);

    put(
        &mut model,
        10,
        "IFCRELDECLARES",
        relating_first(project, &[wall_type]),
    );
    put(
        &mut model,
        11,
        "IFCRELDEFINESBYOBJECT",
        related_first(&[copy], source),
    );

    let all = relation::all(&model);
    let d = all
        .iter()
        .find(|r| r.kind == RelationshipKind::Declares)
        .expect("declares read");
    assert_eq!(d.relating, Some(project), "the context declares");
    assert_eq!(d.related, vec![wall_type]);

    let o = all
        .iter()
        .find(|r| r.kind == RelationshipKind::DefinesByObject)
        .expect("defines-by-object read");
    assert_eq!(o.relating, Some(source), "the source defines the copy");
    assert_eq!(o.related, vec![copy]);
}

/// A thermostat controls a boiler, not the reverse.
#[test]
fn flow_control_direction_is_not_inverted() {
    let mut model = Model::new();
    let boiler = put(&mut model, 1, "IFCBOILER", vec![]);
    let thermostat = put(&mut model, 2, "IFCCONTROLLER", vec![]);
    put(
        &mut model,
        10,
        "IFCRELFLOWCONTROLELEMENTS",
        related_first(&[thermostat], boiler),
    );

    let all = relation::all(&model);
    let f = all
        .iter()
        .find(|r| r.kind == RelationshipKind::FlowControlElements)
        .expect("flow control read");
    assert_eq!(f.relating, Some(boiler), "the flow element is governed");
    assert_eq!(f.related, vec![thermostat], "the controls do the governing");
}

/// A system serves buildings; the system is the relating end.
#[test]
fn a_system_serves_its_buildings() {
    let mut model = Model::new();
    let system = put(&mut model, 1, "IFCDISTRIBUTIONSYSTEM", vec![]);
    let building = put(&mut model, 2, "IFCBUILDING", vec![]);
    let annex = put(&mut model, 3, "IFCBUILDING", vec![]);
    put(
        &mut model,
        10,
        "IFCRELSERVICESBUILDINGS",
        relating_first(system, &[building, annex]),
    );

    let all = relation::all(&model);
    let s = all
        .iter()
        .find(|r| r.kind == RelationshipKind::ServicesBuildings)
        .expect("services read");
    assert_eq!(s.relating, Some(system));
    assert_eq!(s.related, vec![building, annex]);
}

/// A concrete subtype ifc-structural cannot see by exact name.
#[test]
fn eccentric_connection_is_reachable() {
    let mut model = Model::new();
    let member = put(&mut model, 1, "IFCSTRUCTURALCURVEMEMBER", vec![]);
    let connection = put(&mut model, 2, "IFCSTRUCTURALPOINTCONNECTION", vec![]);
    put(
        &mut model,
        10,
        "IFCRELCONNECTSWITHECCENTRICITY",
        relating_first(member, &[connection]),
    );

    let found: Vec<_> = relation::all(&model)
        .into_iter()
        .filter(|r| r.kind == RelationshipKind::ConnectsWithEccentricity)
        .collect();
    assert_eq!(found.len(), 1, "the eccentric subtype is found");
    assert_eq!(found[0].relating, Some(member));
}

/// All five coexist without one shadowing another.
#[test]
fn all_five_families_are_distinguished() {
    let mut model = Model::new();
    let a = put(&mut model, 1, "IFCPROJECT", vec![]);
    let b = put(&mut model, 2, "IFCWALL", vec![]);

    put(&mut model, 10, "IFCRELDECLARES", relating_first(a, &[b]));
    put(
        &mut model,
        11,
        "IFCRELDEFINESBYOBJECT",
        related_first(&[b], a),
    );
    put(
        &mut model,
        12,
        "IFCRELFLOWCONTROLELEMENTS",
        related_first(&[b], a),
    );
    put(
        &mut model,
        13,
        "IFCRELSERVICESBUILDINGS",
        relating_first(a, &[b]),
    );
    put(
        &mut model,
        14,
        "IFCRELCONNECTSWITHECCENTRICITY",
        relating_first(a, &[b]),
    );

    let all = relation::all(&model);
    for kind in [
        RelationshipKind::Declares,
        RelationshipKind::DefinesByObject,
        RelationshipKind::FlowControlElements,
        RelationshipKind::ServicesBuildings,
        RelationshipKind::ConnectsWithEccentricity,
    ] {
        assert_eq!(
            all.iter().filter(|r| r.kind == kind).count(),
            1,
            "exactly one {kind:?}"
        );
    }
    // Every one resolved both ends.
    for r in &all {
        assert!(r.relating.is_some(), "relating resolved for {r:?}");
        assert_eq!(r.related, vec![b], "related resolved for {r:?}");
    }
}
