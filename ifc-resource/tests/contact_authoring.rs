//! Authoring roles, addresses, and organisation relationships.
//!
//! Both address types require at least one field that actually
//! reaches someone. An address with every slot unset parses and is
//! useless, so the writers refuse it.

use ifc_model::{Entity, EntityId, Model, Value};
use ifc_resource::{
    ActorRoleDraft, PostalAddressDraft, ResourceEditor, TelecomAddressDraft, TelecomLists,
};

fn model() -> Model {
    let mut model = Model::default();
    model.header_mut().schema = vec!["IFC4".to_owned()];
    model
}

fn organization(model: &mut Model) -> EntityId {
    model.push(Entity::new("IFCORGANIZATION", vec![Value::Null; 5]))
}

/// A role stages with its enum token.
#[test]
fn an_actor_role_stages() {
    let mut model = model();
    let mut editor = ResourceEditor::for_model(&mut model).expect("editor");
    let id = editor
        .create_actor_role(ActorRoleDraft {
            role: "ARCHITECT",
            user_defined_role: None,
            description: Some("Lead designer"),
        })
        .expect("role");

    let staged = model.get(id).expect("staged");
    assert_eq!(staged.type_name.as_ref(), "IfcActorRole");
    assert_eq!(staged.attributes.len(), 3);
    assert_eq!(staged.attributes[0], Value::Enum("ARCHITECT".into()));
}

/// WR1: a USERDEFINED role must name itself.
#[test]
fn a_userdefined_role_without_a_name_is_refused() {
    let mut model = model();
    let mut editor = ResourceEditor::for_model(&mut model).expect("editor");
    assert!(
        editor
            .create_actor_role(ActorRoleDraft {
                role: "USERDEFINED",
                user_defined_role: None,
                description: None,
            })
            .is_err(),
        "a nameless USERDEFINED role was accepted",
    );
    editor
        .create_actor_role(ActorRoleDraft {
            role: "USERDEFINED",
            user_defined_role: Some("BIM coordinator"),
            description: None,
        })
        .expect("a named USERDEFINED role is legal");
}

/// WR1: a postal address must locate something.
#[test]
fn an_empty_postal_address_is_refused() {
    let mut model = model();
    let mut editor = ResourceEditor::for_model(&mut model).expect("editor");
    assert!(
        editor
            .create_postal_address(PostalAddressDraft::default(), &[])
            .is_err(),
        "an address locating nothing was accepted",
    );

    let id = editor
        .create_postal_address(PostalAddressDraft::default(), &["1 Long Street"])
        .expect("address lines alone satisfy WR1");
    let staged = model.get(id).expect("staged");
    assert_eq!(staged.type_name.as_ref(), "IfcPostalAddress");
    assert_eq!(staged.attributes.len(), 10);
    assert_eq!(
        staged.attributes[4],
        Value::List(vec![Value::Text("1 Long Street".into())]),
        "AddressLines is slot 4",
    );
}

/// MinimumDataProvided: a telecom address must reach someone.
#[test]
fn an_unreachable_telecom_address_is_refused() {
    let mut model = model();
    let mut editor = ResourceEditor::for_model(&mut model).expect("editor");
    assert!(
        editor
            .create_telecom_address(TelecomAddressDraft::default(), TelecomLists::default(),)
            .is_err(),
        "an address with no contact route was accepted",
    );

    // A pager alone is enough: the rule is a disjunction, so any one
    // route satisfies it.
    editor
        .create_telecom_address(
            TelecomAddressDraft {
                pager_number: Some("555"),
                ..TelecomAddressDraft::default()
            },
            TelecomLists::default(),
        )
        .expect("a pager number alone satisfies MinimumDataProvided");

    let id = editor
        .create_telecom_address(
            TelecomAddressDraft::default(),
            TelecomLists {
                email: &["a@example.com"],
                ..TelecomLists::default()
            },
        )
        .expect("an email alone satisfies MinimumDataProvided");
    let staged = model.get(id).expect("staged");
    assert_eq!(staged.attributes.len(), 9);
    assert_eq!(
        staged.attributes[6],
        Value::List(vec![Value::Text("a@example.com".into())]),
        "ElectronicMailAddresses is slot 6",
    );
    // An absent list is Null, not an empty list: LIST [1:?] cannot be
    // written empty.
    assert_eq!(staged.attributes[3], Value::Null, "TelephoneNumbers absent");
}

/// An organisation relationship stages, and refuses a self-relation.
#[test]
fn an_organization_relationship_stages() {
    let mut model = model();
    let parent = organization(&mut model);
    let child = organization(&mut model);
    let mut editor = ResourceEditor::for_model(&mut model).expect("editor");

    let id = editor
        .create_organization_relationship(Some("Group"), None, parent, &[child])
        .expect("relationship");

    assert!(
        editor
            .create_organization_relationship(None, None, parent, &[])
            .is_err(),
        "an empty SET [1:?] was accepted",
    );
    assert!(
        editor
            .create_organization_relationship(None, None, parent, &[parent])
            .is_err(),
        "an organisation was related to itself",
    );

    let staged = model.get(id).expect("staged");
    assert_eq!(staged.type_name.as_ref(), "IfcOrganizationRelationship");
    assert_eq!(staged.attributes.len(), 4);
    assert_eq!(staged.attributes[2], Value::Ref(parent));
    assert_eq!(staged.attributes[3], Value::List(vec![Value::Ref(child)]));
}
