//! Authoring actors, occupants, and assets.
//!
//! `IfcActor` and `IfcOccupant` share the same prefix and differ
//! only in the occupant's trailing `PredefinedType`, so one path
//! serves both: supplying a predefined type selects the occupant.

use ifc_model::{Entity, Model, Value};
use ifc_resource::{ActorDraft, AssetDraft, ResourceEditor};

const GUID: &str = "1jQ2A$rnvCJhUvFV5RxFtz";
const GUID2: &str = "3rT4B$mkwDKiVwGW6SyGua";

fn schema_model() -> Model {
    let mut model = Model::default();
    // ResourceEditor resolves its schema from the header; without a
    // token every call fails as MissingSchema before any rule runs.
    model.header_mut().schema = vec!["IFC4".to_owned()];
    model
}

fn model_with_person() -> (Model, ifc_model::EntityId) {
    let mut model = schema_model();
    let person = model.push(Entity::new("IFCPERSON", vec![Value::Null; 8]));
    (model, person)
}

/// A predefined type selects the occupant; its absence keeps the actor.
#[test]
fn the_predefined_type_selects_the_occupant() {
    let (mut model, person) = model_with_person();
    let mut editor = ResourceEditor::for_model(&mut model).expect("editor");
    let actor = editor
        .create_actor(ActorDraft {
            global_id: GUID,
            the_actor: person,
            name: Some("Client"),
            description: None,
            object_type: None,
            predefined_type: None,
        })
        .expect("actor");
    let occupant = editor
        .create_actor(ActorDraft {
            global_id: GUID2,
            the_actor: person,
            name: Some("Tenant"),
            description: None,
            object_type: None,
            predefined_type: Some("LESSEE"),
        })
        .expect("occupant");

    let a = model.get(actor).expect("actor");
    assert_eq!(a.type_name.as_ref(), "IfcActor");
    assert_eq!(a.attributes.len(), 6, "IfcActor arity");
    assert_eq!(a.attributes[5], Value::Ref(person), "TheActor");

    let o = model.get(occupant).expect("occupant");
    assert_eq!(o.type_name.as_ref(), "IfcOccupant");
    assert_eq!(o.attributes.len(), 7, "IfcOccupant arity");
    assert_eq!(
        o.attributes[6],
        Value::Enum("LESSEE".into()),
        "PredefinedType"
    );
}

/// WR31: USERDEFINED names a role the enum has no token for.
#[test]
fn userdefined_without_an_object_type_is_refused() {
    let (mut model, person) = model_with_person();
    let mut editor = ResourceEditor::for_model(&mut model).expect("editor");
    editor
        .create_actor(ActorDraft {
            global_id: GUID,
            the_actor: person,
            name: None,
            description: None,
            object_type: None,
            predefined_type: Some("USERDEFINED"),
        })
        .expect_err("WR31");
    editor
        .create_actor(ActorDraft {
            global_id: GUID,
            the_actor: person,
            name: None,
            description: None,
            object_type: Some("Facility manager"),
            predefined_type: Some("USERDEFINED"),
        })
        .expect("named role is accepted");
}

/// A party outside `IfcActorSelect` is refused.
///
/// The select admits a person, an organization, or the pairing of
/// the two; anything else names no party.
#[test]
fn an_actor_outside_the_select_is_refused() {
    let mut model = schema_model();
    let wall = model.push(Entity::new("IFCWALL", vec![Value::Null; 8]));
    let mut editor = ResourceEditor::for_model(&mut model).expect("editor");
    editor
        .create_actor(ActorDraft {
            global_id: GUID,
            the_actor: wall,
            name: None,
            description: None,
            object_type: None,
            predefined_type: None,
        })
        .expect_err("a wall is not an IfcActorSelect");
}

/// An asset's value slots must resolve to cost values.
///
/// Every one is optional, so the reference check is the only thing
/// standing between a draft and an asset claiming a value the
/// model does not hold.
#[test]
fn asset_value_slots_must_be_cost_values() {
    let mut model = schema_model();
    let cost = model.push(Entity::new("IFCCOSTVALUE", vec![Value::Null; 6]));
    let wall = model.push(Entity::new("IFCWALL", vec![Value::Null; 8]));
    let mut editor = ResourceEditor::for_model(&mut model).expect("editor");
    editor
        .create_asset(AssetDraft {
            global_id: GUID,
            original_value: Some(cost),
            ..AssetDraft::default()
        })
        .expect("a cost value is accepted");
    editor
        .create_asset(AssetDraft {
            global_id: GUID2,
            original_value: Some(wall),
            ..AssetDraft::default()
        })
        .expect_err("a wall is not an IfcCostValue");
}

/// The asset's fourteen slots land where the schema declares them.
#[test]
fn the_asset_tail_lands_in_the_declared_slots() {
    let mut model = schema_model();
    let cost = model.push(Entity::new("IFCCOSTVALUE", vec![Value::Null; 6]));
    let person = model.push(Entity::new("IFCPERSON", vec![Value::Null; 8]));
    let mut editor = ResourceEditor::for_model(&mut model).expect("editor");
    let asset = editor
        .create_asset(AssetDraft {
            global_id: GUID,
            name: Some("Chiller"),
            identification: Some("AST-001"),
            current_value: Some(cost),
            owner: Some(person),
            responsible_person: Some(person),
            incorporation_date: Some("2026-09-22"),
            ..AssetDraft::default()
        })
        .expect("asset");
    let a = model.get(asset).expect("asset");
    assert_eq!(a.attributes.len(), 14, "IfcAsset arity");
    assert_eq!(
        a.attributes[5],
        Value::Text("AST-001".into()),
        "Identification"
    );
    assert_eq!(a.attributes[7], Value::Ref(cost), "CurrentValue");
    assert_eq!(a.attributes[9], Value::Ref(person), "Owner");
    assert_eq!(a.attributes[11], Value::Ref(person), "ResponsiblePerson");
    assert_eq!(
        a.attributes[12],
        Value::Text("2026-09-22".into()),
        "IncorporationDate"
    );
}
