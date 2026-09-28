//! Authoring API round-trips: draft in, projected view out.

use ifc_material::{
    associate_material, create_layer, create_layer_set, create_material, LayerDraft, LayerSetDraft,
    LogicalValue, MaterialAssignmentDraft, MaterialDraft, MaterialView,
};
use ifc_model::{Entity, EntityId, Model, Transaction, Value};

fn gid() -> String {
    ifc_model::guid::Guid::from_uuid([7; 16]).to_string()
}

#[test]
fn authored_layered_material_roundtrips_and_resolves() {
    let mut model = Model::new();
    let wall = model.push(Entity::new("IFCWALL", vec![]));
    let mut tx = Transaction::new(&model);
    let material = create_material(
        &mut tx,
        &model,
        MaterialDraft::new("Concrete").category("Structural"),
    )
    .unwrap();
    let layer = create_layer(
        &mut tx,
        &model,
        LayerDraft::new(0.2)
            .material(material)
            .is_ventilated(LogicalValue::False)
            .name("Core")
            .priority(80),
    )
    .unwrap();
    let set = create_layer_set(&mut tx, &model, LayerSetDraft::new(&[layer]).name("Wall")).unwrap();
    let relationship = associate_material(
        &mut tx,
        &model,
        MaterialAssignmentDraft::new(&gid(), &[wall], set),
    )
    .unwrap();
    tx.commit(&mut model).unwrap();

    let view = MaterialView::new(&model);
    assert_eq!(view.materials().next().unwrap().name().unwrap(), "Concrete");
    assert_eq!(
        view.total_thickness(view.layer_sets().next().unwrap())
            .unwrap(),
        0.2
    );
    let assignment = view.assignments_for(wall).unwrap();
    assert_eq!(assignment.len(), 1);
    assert_eq!(assignment[0].id(), relationship);
    assert_eq!(assignment[0].relating_material_id().unwrap(), set);
}

#[test]
fn invalid_authoring_refuses_to_stage_a_partial_material_graph() {
    let model = Model::new();
    let mut tx = Transaction::new(&model);
    let result = create_layer(&mut tx, &model, LayerDraft::new(f64::NAN));
    assert!(result.is_err());
    assert!(
        tx.is_empty(),
        "invalid data must not reserve a partial graph"
    );

    let result = create_layer_set(&mut tx, &model, LayerSetDraft::new(&[]));
    assert!(result.is_err());
    assert!(tx.is_empty());
}

#[test]
fn failed_commit_rolls_back_the_entire_authored_graph() {
    let mut model = Model::new();
    let wall = model.push(Entity::new("IFCWALL", vec![]));
    let mut tx = Transaction::new(&model);
    let material = create_material(&mut tx, &model, MaterialDraft::new("Concrete")).unwrap();
    let relationship = associate_material(
        &mut tx,
        &model,
        MaterialAssignmentDraft::new(&gid(), &[wall], material),
    )
    .unwrap();
    tx.set_attribute(relationship, 5, Value::Ref(EntityId(99_999)));
    assert!(tx.commit(&mut model).is_err());
    assert_eq!(MaterialView::new(&model).materials().count(), 0);
    assert_eq!(MaterialView::new(&model).assignments().count(), 0);
}

#[test]
fn malformed_guid_and_non_material_reference_are_refused_before_staging() {
    let mut model = Model::new();
    let wall = model.push(Entity::new("IFCWALL", vec![]));
    let mut tx = Transaction::new(&model);
    let material = create_material(&mut tx, &model, MaterialDraft::new("Concrete")).unwrap();
    let invalid_guid = associate_material(
        &mut tx,
        &model,
        MaterialAssignmentDraft::new("not-an-ifc-guid", &[wall], material),
    );
    assert!(invalid_guid.is_err());
    assert_eq!(tx.len(), 1, "the invalid relationship itself is not staged");

    let wrong_material = create_layer(&mut tx, &model, LayerDraft::new(0.1).material(wall));
    assert!(wrong_material.is_err());
    assert_eq!(tx.len(), 1);
    assert!(create_layer(&mut tx, &model, LayerDraft::new(0.1).material(material),).is_ok());
}
