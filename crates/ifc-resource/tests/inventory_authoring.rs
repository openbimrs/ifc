//! Authoring `IfcInventory`.
//!
//! `ResponsiblePersons` is `SET [1:?]`: an inventory nobody is
//! responsible for cannot be stated as an empty set.

use ifc_model::{Entity, EntityId, Model, Value};
use ifc_resource::{InventoryDraft, ResourceEditor};

const GUID: &str = "1jQ2A$rnvCJhUvFV5RxFtz";

fn model_with_person() -> (Model, EntityId) {
    let mut model = Model::default();
    model.header_mut().schema = vec!["IFC4".to_owned()];
    let person = model.push(Entity::new("IFCPERSON", vec![Value::Null; 8]));
    (model, person)
}

fn draft() -> InventoryDraft<'static> {
    InventoryDraft {
        global_id: GUID,
        name: Some("Fixed assets"),
        predefined_type: Some("ASSETINVENTORY"),
        ..InventoryDraft::default()
    }
}

/// An inventory stages its eleven slots.
#[test]
fn an_inventory_stages() {
    let (mut model, person) = model_with_person();
    let mut editor = ResourceEditor::for_model(&mut model).expect("editor");
    let id = editor
        .create_inventory(draft(), &[person])
        .expect("inventory");

    let staged = model.get(id).expect("staged");
    assert_eq!(staged.type_name.as_ref(), "IfcInventory");
    assert_eq!(staged.attributes.len(), 11);
    assert_eq!(staged.attributes[5], Value::Enum("ASSETINVENTORY".into()));
    assert_eq!(
        staged.attributes[7],
        Value::List(vec![Value::Ref(person)]),
        "ResponsiblePersons is slot 7",
    );
}

/// An empty responsible-person set is refused.
#[test]
fn an_inventory_without_a_responsible_person_is_refused() {
    let (mut model, _) = model_with_person();
    let mut editor = ResourceEditor::for_model(&mut model).expect("editor");
    assert!(
        editor.create_inventory(draft(), &[]).is_err(),
        "an empty SET [1:?] was accepted",
    );
}

/// A USERDEFINED inventory must name itself.
#[test]
fn a_userdefined_inventory_without_an_object_type_is_refused() {
    let (mut model, person) = model_with_person();
    let mut editor = ResourceEditor::for_model(&mut model).expect("editor");

    let mut bare = draft();
    bare.predefined_type = Some("USERDEFINED");
    assert!(
        editor.create_inventory(bare, &[person]).is_err(),
        "a nameless USERDEFINED inventory was accepted",
    );

    let mut named = draft();
    named.predefined_type = Some("USERDEFINED");
    named.object_type = Some("Tooling register");
    editor
        .create_inventory(named, &[person])
        .expect("a named USERDEFINED inventory is legal");
}
