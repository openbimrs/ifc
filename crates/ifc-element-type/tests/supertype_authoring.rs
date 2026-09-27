//! Staging the type definitions that carry no `PredefinedType`.

use ifc_element_type::{
    create_supertype, create_type, ElementTypeError, SupertypeDraft, TypeDraft, ALL,
    ALL_SUPERTYPES, BUILT_ELEMENT_TYPE, CIVIL_ELEMENT_TYPE, TYPE_OBJECT, TYPE_PRODUCT,
};
use ifc_model::{Entity, Model, Transaction, Value};
use ifc_schema::{ifc4, ifc4x3};

const GUID: &str = "0EI0MSHbX9gg8Fxwar7lb8";

fn invalid(err: &ElementTypeError) -> bool {
    matches!(err, ElementTypeError::Invalid { .. })
}

/// Each declared arity matches the shipped schema.
///
/// The writer fills positional slots from these constants, so they are
/// checked against the schema rather than trusted. The three distinct
/// arities are the point: one shared value would misplace attributes on
/// four of the seven.
#[test]
fn declared_arities_match_the_schema() {
    for schema in [ifc4(), ifc4x3()] {
        for kind in ALL_SUPERTYPES {
            let declared = schema.attributes(kind.type_name);
            if declared.is_empty() {
                continue;
            }
            assert_eq!(
                declared.len(),
                kind.arity,
                "{} in {}",
                kind.type_name,
                schema.name()
            );
        }
    }
}

/// NameRequired: a type object must carry a name.
///
/// `Name` is OPTIONAL in the slot table and mandatory by WHERE rule. A
/// nameless type parses and then cannot be referred to by anything.
#[test]
fn a_nameless_supertype_is_refused() {
    let model = Model::new();
    let mut tx = Transaction::new(&model);
    for name in ["", "   "] {
        let err = create_supertype(&mut tx, TYPE_OBJECT, GUID, name, SupertypeDraft::default())
            .expect_err("NameRequired");
        assert!(invalid(&err), "{err}");
    }
    assert!(tx.is_empty(), "nothing is staged when the rule fails");
    create_supertype(
        &mut tx,
        TYPE_OBJECT,
        GUID,
        "Wall type",
        SupertypeDraft::default(),
    )
    .expect("a named type object is legal");
}

/// The same rule reaches the 132 catalogue types, which inherit it.
#[test]
fn nameless_catalogue_types_are_refused_too() {
    let model = Model::new();
    let mut tx = Transaction::new(&model);
    let kind = ALL.iter().find(|k| !k.members.is_empty()).expect("a type");
    let token = kind.members[0];
    let err = create_type(&mut tx, *kind, GUID, Some(token), TypeDraft::default())
        .expect_err("NameRequired is inherited by every type definition");
    assert!(invalid(&err), "{err}");
}

/// An attribute the entity does not declare is refused, not dropped.
///
/// `IfcTypeObject` stops at slot 5. Accepting a Tag and discarding it
/// would write a file missing data the caller believes they supplied.
#[test]
fn attributes_beyond_the_arity_are_refused() {
    let mut model = Model::new();
    let map = model.push(Entity::new("IFCREPRESENTATIONMAP", vec![]));
    let mut tx = Transaction::new(&model);

    let err = create_supertype(
        &mut tx,
        TYPE_OBJECT,
        GUID,
        "T",
        SupertypeDraft {
            tag: Some("A-1"),
            ..SupertypeDraft::default()
        },
    )
    .expect_err("IfcTypeObject has no Tag");
    assert!(invalid(&err), "{err}");

    let err = create_supertype(
        &mut tx,
        TYPE_OBJECT,
        GUID,
        "T",
        SupertypeDraft {
            representation_maps: &[map],
            ..SupertypeDraft::default()
        },
    )
    .expect_err("IfcTypeObject has no RepresentationMaps");
    assert!(invalid(&err), "{err}");

    let err = create_supertype(
        &mut tx,
        TYPE_PRODUCT,
        GUID,
        "T",
        SupertypeDraft {
            element_type: Some("Wall"),
            ..SupertypeDraft::default()
        },
    )
    .expect_err("IfcTypeProduct has no ElementType");
    assert!(invalid(&err), "{err}");

    create_supertype(
        &mut tx,
        TYPE_PRODUCT,
        GUID,
        "T",
        SupertypeDraft {
            representation_maps: &[map],
            tag: Some("A-1"),
            ..SupertypeDraft::default()
        },
    )
    .expect("IfcTypeProduct does declare both");
}

/// Each arity tier writes its own attributes into the right slots.
#[test]
fn each_tier_lands_its_attributes_in_the_declared_slots() {
    let mut model = Model::new();
    let map = model.push(Entity::new("IFCREPRESENTATIONMAP", vec![]));
    let mut tx = Transaction::new(&model);

    let id = create_supertype(
        &mut tx,
        BUILT_ELEMENT_TYPE,
        GUID,
        "Generic wall",
        SupertypeDraft {
            description: Some("d"),
            applicable_occurrence: Some("occ"),
            representation_maps: &[map],
            tag: Some("A-1"),
            element_type: Some("WALL"),
            ..SupertypeDraft::default()
        },
    )
    .expect("a fully populated built element type");
    tx.commit(&mut model).expect("commits");

    let entity = model.get(id).expect("in the model");
    assert_eq!(entity.type_name.as_ref(), "IFCBUILTELEMENTTYPE");
    assert_eq!(entity.attributes.len(), 9);
    assert_eq!(entity.attributes[2], Value::Text("Generic wall".into()));
    assert_eq!(entity.attributes[4], Value::Text("occ".into()));
    assert_eq!(entity.attributes[6], Value::List(vec![Value::Ref(map)]));
    assert_eq!(entity.attributes[7], Value::Text("A-1".into()));
    assert_eq!(entity.attributes[8], Value::Text("WALL".into()));
}

/// UniquePropertySetNames: two sets of one name make lookup ambiguous.
#[test]
fn duplicate_property_set_names_are_refused() {
    let mut model = Model::new();
    let a = model.push(Entity::new("IFCPROPERTYSET", vec![]));
    let b = model.push(Entity::new("IFCPROPERTYSET", vec![]));
    let mut tx = Transaction::new(&model);

    let err = create_supertype(
        &mut tx,
        CIVIL_ELEMENT_TYPE,
        GUID,
        "Civil",
        SupertypeDraft {
            property_sets: &[("Pset_Common", a), ("Pset_Common", b)],
            ..SupertypeDraft::default()
        },
    )
    .expect_err("UniquePropertySetNames");
    assert!(invalid(&err), "{err}");

    let id = create_supertype(
        &mut tx,
        CIVIL_ELEMENT_TYPE,
        GUID,
        "Civil",
        SupertypeDraft {
            property_sets: &[("Pset_Common", a), ("Pset_Other", b)],
            ..SupertypeDraft::default()
        },
    )
    .expect("distinct names are legal");
    tx.commit(&mut model).expect("commits");
    assert_eq!(
        model.get(id).expect("staged").attributes[5],
        Value::List(vec![Value::Ref(a), Value::Ref(b)]),
        "HasPropertySets sits at slot 5"
    );
}

/// A malformed GlobalId is refused before anything is staged.
#[test]
fn a_malformed_guid_is_refused() {
    let model = Model::new();
    let mut tx = Transaction::new(&model);
    let err = create_supertype(
        &mut tx,
        TYPE_PRODUCT,
        "nope",
        "T",
        SupertypeDraft::default(),
    )
    .expect_err("four characters is not a GUID");
    assert!(invalid(&err), "{err}");
    assert!(tx.is_empty(), "nothing staged");
}

/// All seven stage on both shipped schemas.
#[test]
fn every_supertype_stages_on_both_schemas() {
    for kind in ALL_SUPERTYPES {
        let mut model = Model::new();
        let mut tx = Transaction::new(&model);
        let id = create_supertype(&mut tx, *kind, GUID, "T", SupertypeDraft::default())
            .unwrap_or_else(|e| panic!("{} should stage: {e}", kind.type_name));
        tx.commit(&mut model).expect("commits");
        let entity = model.get(id).expect("in the model");
        assert_eq!(
            entity.attributes.len(),
            kind.arity,
            "{} arity",
            kind.type_name
        );
        assert_eq!(
            entity.type_name.as_ref(),
            kind.type_name.to_ascii_uppercase(),
            "type name is normalised"
        );
    }
}

/// A blank property-set name is refused.
///
/// Two unnamed sets are not distinguishable by the name the
/// uniqueness rule compares, so a blank name defeats the rule
/// rather than satisfying it.
#[test]
fn a_blank_property_set_name_is_refused() {
    let model = Model::new();
    let mut tx = Transaction::new(&model);
    let pset = tx.create(Entity::new("IFCPROPERTYSET", vec![]));
    let err = create_supertype(
        &mut tx,
        TYPE_OBJECT,
        GUID,
        "Named",
        SupertypeDraft {
            property_sets: &[("   ", pset)],
            ..SupertypeDraft::default()
        },
    )
    .expect_err("a blank set name is refused");
    assert!(format!("{err}").contains("HasPropertySets"), "{err}");
}
