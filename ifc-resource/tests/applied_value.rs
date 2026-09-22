//! Sweep the newly authored `IfcAppliedValue`.
//!
//! Every slot is OPTIONAL, so the schema alone permits an empty
//! record. `ArithmeticOperator` and `Components` are meaningful
//! only together: an operator with nothing to combine states an
//! operation over no operands.

use ifc_model::{Model, Value};
use ifc_resource::{AppliedValueDraft, ResourceEditor};

fn model() -> Model {
    let mut model = Model::default();
    // ResourceEditor resolves its schema from the header.
    model.header_mut().schema = vec!["IFC4".to_owned()];
    model
}

/// An applied value stages its ten slots.
#[test]
fn an_applied_value_stages() {
    let mut model = model();
    let mut editor = ResourceEditor::for_model(&mut model).expect("editor");
    let id = editor
        .create_applied_value(
            AppliedValueDraft {
                name: Some("Unit rate"),
                category: Some("Labour"),
                ..AppliedValueDraft::default()
            },
            &[],
        )
        .expect("applied value");

    let staged = model.get(id).expect("staged");
    // `ifc-resource` stores type names as written in `build_entity`,
    // mixed case. `ids_of_type` uppercases both sides of the index, so
    // lookups are unaffected by the spelling.
    assert_eq!(staged.type_name.as_ref(), "IfcAppliedValue");
    assert_eq!(staged.attributes.len(), 10);
    assert_eq!(
        staged.attributes[9],
        Value::Null,
        "an absent Components stays null, not an empty list",
    );
}

/// The operator and its components are stated together.
#[test]
fn an_operator_without_components_is_refused() {
    let mut model = model();
    let mut editor = ResourceEditor::for_model(&mut model).expect("editor");
    assert!(
        editor
            .create_applied_value(
                AppliedValueDraft {
                    arithmetic_operator: Some("ADD"),
                    ..AppliedValueDraft::default()
                },
                &[]
            )
            .is_err(),
        "accepted an operator over no operands",
    );
}

/// Components must themselves be applied values.
#[test]
fn a_nested_component_must_be_an_applied_value() {
    let mut model = model();
    let mut editor = ResourceEditor::for_model(&mut model).expect("editor");
    let inner = editor
        .create_applied_value(
            AppliedValueDraft {
                name: Some("Base"),
                ..AppliedValueDraft::default()
            },
            &[],
        )
        .expect("inner");

    let outer = editor
        .create_applied_value(
            AppliedValueDraft {
                arithmetic_operator: Some("ADD"),
                ..AppliedValueDraft::default()
            },
            &[inner],
        )
        .expect("outer");
    let staged = model.get(outer).expect("staged");
    assert_eq!(
        staged.attributes[9],
        Value::List(vec![Value::Ref(inner)]),
        "Components is a LIST [1:?] OF IfcAppliedValue",
    );
}
