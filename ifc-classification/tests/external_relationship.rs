use ifc_classification::{
    create_external_reference_relationship, ClassificationError, ClassificationView,
    ExternalReferenceRelationshipDraft,
};
use ifc_model::{Entity, Model, Transaction, Value};

fn text(value: &str) -> Value {
    Value::Text(value.into())
}

#[test]
fn schema_pins_external_reference_relationship_slots() {
    assert_eq!(
        ifc_schema::ifc4().attribute_names("IFCEXTERNALREFERENCERELATIONSHIP"),
        [
            "Name",
            "Description",
            "RelatingReference",
            "RelatedResourceObjects"
        ]
    );
}

#[test]
fn authored_relationship_round_trips_and_queries_by_resource() {
    let mut model = Model::new();
    let reference = model.push(Entity::new(
        "IFCCLASSIFICATIONREFERENCE",
        vec![
            text("uri"),
            text("id"),
            Value::Null,
            Value::Null,
            Value::Null,
            Value::Null,
        ],
    ));
    let approval = model.push(Entity::new(
        "IFCAPPROVAL",
        vec![
            text("A"),
            Value::Null,
            Value::Null,
            Value::Null,
            Value::Null,
            Value::Null,
            Value::Null,
            Value::Null,
            Value::Null,
        ],
    ));
    let mut tx = Transaction::new(&model);
    let relation = create_external_reference_relationship(
        &mut tx,
        &model,
        ExternalReferenceRelationshipDraft {
            name: Some("evidence"),
            description: None,
            relating_reference: reference,
            related_resources: &[approval],
        },
    )
    .unwrap();
    tx.commit(&mut model).unwrap();

    let view = ClassificationView::new(&model);
    let projected = view.external_reference_relationship(relation).unwrap();
    assert_eq!(projected.name().unwrap(), Some("evidence"));
    assert_eq!(projected.relating_reference().unwrap(), reference);
    assert_eq!(projected.related_resources().unwrap(), [approval]);
    assert_eq!(
        view.external_references_for(approval)
            .unwrap()
            .into_iter()
            .map(|item| item.id())
            .collect::<Vec<_>>(),
        [relation]
    );
}

#[test]
fn invalid_selects_and_malformed_sets_are_refused() {
    let mut model = Model::new();
    let wall = model.push(Entity::new("IFCWALL", vec![]));
    let person = model.push(Entity::new("IFCPERSON", vec![]));
    let reference = model.push(Entity::new(
        "IFCDOCUMENTREFERENCE",
        vec![
            text("uri"),
            text("id"),
            Value::Null,
            Value::Null,
            Value::Null,
        ],
    ));
    let mut tx = Transaction::new(&model);
    let before = tx.len();
    assert!(matches!(
        create_external_reference_relationship(
            &mut tx,
            &model,
            ExternalReferenceRelationshipDraft {
                name: None,
                description: None,
                relating_reference: reference,
                related_resources: &[wall],
            }
        ),
        Err(ClassificationError::AuthoringReferenceType { target, .. }) if target == wall
    ));
    assert_eq!(tx.len(), before);
    assert!(matches!(
        create_external_reference_relationship(
            &mut tx,
            &model,
            ExternalReferenceRelationshipDraft {
                name: None,
                description: None,
                relating_reference: reference,
                related_resources: &[person, person],
            }
        ),
        Err(ClassificationError::AuthoringInvalid {
            attribute: "RelatedResourceObjects",
            ..
        })
    ));
    assert_eq!(tx.len(), before);

    let malformed = model.push(Entity::new(
        "IFCEXTERNALREFERENCERELATIONSHIP",
        vec![
            Value::Null,
            Value::Null,
            Value::Ref(reference),
            Value::List(vec![Value::Ref(wall)]),
        ],
    ));
    assert!(matches!(
        ClassificationView::new(&model).external_reference_relationship(malformed),
        Err(ClassificationError::ReferenceType { target, .. }) if target == wall
    ));
}
