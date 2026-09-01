//! Schema-checked updates of existing entities.

use ifc_author::{AuthorError, EntityEditor};
use ifc_model::{Conflict, Entity, EntityId, Model, Transaction, Value};
use ifc_schema::Schema;

const SUBSET: &str = "\
SCHEMA IFC4;
TYPE IfcGloballyUniqueId = STRING; END_TYPE;
TYPE IfcLabel = STRING; END_TYPE;
TYPE IfcLengthMeasure = REAL; END_TYPE;
ENTITY IfcRoot;
  GlobalId : IfcGloballyUniqueId;
  Name : OPTIONAL IfcLabel;
END_ENTITY;
ENTITY IfcAnnotation SUBTYPE OF (IfcRoot);
  ObjectType : OPTIONAL IfcLabel;
END_ENTITY;
ENTITY IfcCartesianPoint;
  Coordinates : LIST [1:3] OF IfcLengthMeasure;
END_ENTITY;
ENTITY IfcHolder;
  Target : IfcAnnotation;
END_ENTITY;
END_SCHEMA;
";

const GUID: &str = "3vB2YO$MX4xv5uCqZZG05x";

fn schema() -> Schema {
    Schema::from_express(SUBSET)
}

fn annotation() -> Entity {
    Entity::new(
        "IFCANNOTATION",
        vec![
            Value::Text(GUID.into()),
            Value::Text("old name".into()),
            Value::Text("old type".into()),
        ],
    )
}

#[test]
fn edit_stages_named_slots_and_preserves_untouched_values() {
    let schema = schema();
    let mut model = Model::new();
    let id = model.push(annotation());
    let mut tx = Transaction::new(&model);

    EntityEditor::new(&schema, &model, id)
        .expect("entity exists")
        .text("Name", "new name")
        .stage(&mut tx)
        .expect("projected entity is valid");

    assert_eq!(model.get(id).unwrap().text(1), Some("old name"));
    assert_eq!(tx.len(), 1, "one named edit becomes one slot write");
    tx.commit(&mut model).expect("transaction is current");
    assert_eq!(model.get(id).unwrap().text(0), Some(GUID));
    assert_eq!(model.get(id).unwrap().text(1), Some("new name"));
    assert_eq!(model.get(id).unwrap().text(2), Some("old type"));
}

#[test]
fn edit_names_are_case_insensitive() {
    let schema = schema();
    let mut model = Model::new();
    let id = model.push(annotation());
    let mut tx = Transaction::new(&model);
    EntityEditor::new(&schema, &model, id)
        .unwrap()
        .text("NAME", "new")
        .stage(&mut tx)
        .unwrap();
    tx.commit(&mut model).unwrap();
    assert_eq!(model.get(id).unwrap().text(1), Some("new"));
}

#[test]
fn duplicate_or_unknown_edits_stage_nothing() {
    let schema = schema();
    let mut model = Model::new();
    let id = model.push(annotation());

    let mut duplicate_tx = Transaction::new(&model);
    let error = EntityEditor::new(&schema, &model, id)
        .unwrap()
        .text("Name", "first")
        .text("name", "second")
        .stage(&mut duplicate_tx)
        .expect_err("case-insensitive duplicate must be refused");
    assert!(matches!(error, AuthorError::DuplicateAttribute { .. }));
    assert!(duplicate_tx.is_empty());

    let mut unknown_tx = Transaction::new(&model);
    let error = EntityEditor::new(&schema, &model, id)
        .unwrap()
        .text("Nmae", "typo")
        .stage(&mut unknown_tx)
        .expect_err("unknown attribute must be refused");
    assert!(matches!(error, AuthorError::UnknownAttribute { .. }));
    assert!(unknown_tx.is_empty());
}

#[test]
fn invalid_projected_value_or_existing_arity_stages_nothing() {
    let schema = schema();
    let mut model = Model::new();
    let id = model.push(annotation());
    let mut tx = Transaction::new(&model);
    let error = EntityEditor::new(&schema, &model, id)
        .unwrap()
        .set("GlobalId", Value::Null)
        .stage(&mut tx)
        .expect_err("required GlobalId cannot become null");
    assert!(matches!(error, AuthorError::MissingRequired { .. }));
    assert!(tx.is_empty());

    let malformed = model.push(Entity::new("IFCANNOTATION", vec![Value::Text(GUID.into())]));
    let mut malformed_tx = Transaction::new(&model);
    let error = EntityEditor::new(&schema, &model, malformed)
        .unwrap()
        .text("Name", "cannot repair an unknown full state")
        .stage(&mut malformed_tx)
        .expect_err("the projected entity must have schema arity");
    assert!(matches!(error, AuthorError::ArityMismatch { .. }));
    assert!(malformed_tx.is_empty());
}

#[test]
fn missing_target_and_stale_transaction_are_explicit_and_atomic() {
    let schema = schema();
    let model = Model::new();
    let error = EntityEditor::new(&schema, &model, EntityId(99))
        .expect_err("missing target is reported before staging");
    assert!(matches!(
        error,
        AuthorError::MissingEntity { id: EntityId(99) }
    ));

    let mut model = Model::new();
    let id = model.push(annotation());
    let mut tx = Transaction::new(&model);
    EntityEditor::new(&schema, &model, id)
        .unwrap()
        .text("Name", "staged")
        .stage(&mut tx)
        .unwrap();
    model.set_attribute(id, 2, Value::Text("concurrent".into()));

    let errors = tx.commit(&mut model).expect_err("stale write is refused");
    assert!(matches!(
        errors.as_slice(),
        [Conflict::StaleRevision { .. }]
    ));
    assert_eq!(model.get(id).unwrap().text(1), Some("old name"));
    assert_eq!(model.get(id).unwrap().text(2), Some("concurrent"));
}

#[test]
fn dangling_reference_is_refused_by_transaction_preflight() {
    let schema = schema();
    let mut model = Model::new();
    let annotation = model.push(annotation());
    let holder = model.push(Entity::new("IFCHOLDER", vec![Value::Ref(annotation)]));
    let mut tx = Transaction::new(&model);

    EntityEditor::new(&schema, &model, holder)
        .unwrap()
        .reference("Target", EntityId(999))
        .stage(&mut tx)
        .expect("the reference has the declared entity shape");
    let errors = tx
        .commit(&mut model)
        .expect_err("transaction preflight rejects the dangling target");

    assert!(matches!(
        errors.as_slice(),
        [Conflict::DanglingReference {
            from,
            target,
            ..
        }] if *from == holder && *target == EntityId(999)
    ));
    assert_eq!(
        model.get(holder).unwrap().attribute(0),
        Some(&Value::Ref(annotation))
    );
}
