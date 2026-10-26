//! The `GlobalId`s a batch derives are reproducible from the seed and the
//! operations, whatever entity ids the model allocates (#548), and a seed
//! names one batch.

#![cfg(all(feature = "step", feature = "author", feature = "ifc4"))]

use std::collections::BTreeSet;

use ifc::{
    apply_authoring, AuthorOp, AuthoringFailure, Codec, Entity, EntityId, Model,
    RemoveProductOptions, StepCodec, Value,
};

fn model() -> Model {
    let mut model = Model::new();
    model.header_mut().schema = vec!["IFC4".to_owned()];
    model
}

fn apply(model: &mut Model, ops: &[AuthorOp], seed: u128) -> Vec<Option<EntityId>> {
    apply_authoring(model, ifc::schema::ifc4(), ops, seed)
        .expect("batch applies")
        .ids
}

fn named(name: &str) -> Vec<(String, Value)> {
    vec![("Name".to_owned(), Value::Text(name.into()))]
}

fn project() -> AuthorOp {
    AuthorOp::Project {
        attributes: named("P"),
        owner_history: None,
    }
}

fn spatial(type_name: &str, parent: EntityId, name: &str) -> AuthorOp {
    AuthorOp::Spatial {
        type_name: type_name.to_owned(),
        parent,
        attributes: named(name),
        placement: None,
        owner_history: None,
    }
}

fn roundtrip(model: &Model) -> Model {
    let bytes = StepCodec.write_bytes(model).expect("writes");
    StepCodec.read_bytes(&bytes).expect("reads")
}

fn global_id(model: &Model, id: EntityId) -> String {
    match model.get(id).and_then(|entity| entity.attributes.first()) {
        Some(Value::Text(text)) => text.to_string(),
        other => panic!("{id}: no GlobalId, {other:?}"),
    }
}

/// `(type, GlobalId)` of every `IfcRoot`.
fn identities(model: &Model) -> BTreeSet<(String, String)> {
    let schema = ifc::schema::ifc4();
    model
        .iter()
        .filter(|(_, entity)| schema.is_a(&entity.type_name, "IfcRoot"))
        .map(|(id, entity)| (entity.type_name.to_string(), global_id(model, id)))
        .collect()
}

/// Each `IFCRELAGGREGATES` as (its GlobalId, the relating object's, the related objects').
fn aggregates(model: &Model) -> BTreeSet<(String, String, Vec<String>)> {
    let schema = ifc::schema::ifc4();
    let slot = |name: &str| {
        schema
            .attributes("IfcRelAggregates")
            .iter()
            .position(|attribute| attribute.name == name)
            .expect("attribute")
    };
    let (relating, related) = (slot("RelatingObject"), slot("RelatedObjects"));
    model
        .of_type("IFCRELAGGREGATES")
        .map(|(id, entity)| {
            let Value::Ref(whole) = entity.attributes[relating] else {
                panic!("RelatingObject")
            };
            let Value::List(parts) = &entity.attributes[related] else {
                panic!("RelatedObjects")
            };
            let parts = parts
                .iter()
                .map(|part| match part {
                    Value::Ref(part) => global_id(model, *part),
                    other => panic!("{other:?}"),
                })
                .collect();
            (global_id(model, id), global_id(model, whole), parts)
        })
        .collect()
}

fn first(model: &mut Model) -> EntityId {
    let project = apply(model, &[project()], 0x548)[0].expect("project");
    let site = apply(model, &[spatial("IfcSite", project, "S1")], 1)[0].expect("site");
    apply(
        model,
        &[AuthorOp::RemoveProduct {
            product: site,
            options: RemoveProductOptions::new(),
        }],
        2,
    );
    project
}

#[test]
fn a_replayed_journal_gives_the_same_global_ids_after_a_write_and_read() {
    let mut a = model();
    let project = first(&mut a);
    let a_next = a.next_id();
    apply(&mut a, &[spatial("IfcSite", project, "S3")], 3);

    let mut b = model();
    first(&mut b);
    let mut b = roundtrip(&b);
    assert_ne!(
        a_next,
        b.next_id(),
        "precondition: the write/read cycle shifts the next id"
    );
    let project = b.ids_of_type("IFCPROJECT")[0];
    apply(&mut b, &[spatial("IfcSite", project, "S3")], 3);

    assert_eq!(identities(&a), identities(&b));
    assert_eq!(aggregates(&a), aggregates(&b));
    assert_eq!(identities(&a).len(), 3);
}

fn type_journal(model: &mut Model) {
    let project = apply(model, &[project()], 1)[0].expect("project");
    let storey =
        apply(model, &[spatial("IfcBuildingStorey", project, "L0")], 2)[0].expect("storey");
    let wall_type = apply(
        model,
        &[AuthorOp::TypeObject {
            type_name: "IfcWallType".to_owned(),
            attributes: {
                let mut attributes = named("T");
                attributes.push((
                    "PredefinedType".to_owned(),
                    Value::Enum("NOTDEFINED".into()),
                ));
                attributes
            },
            owner_history: None,
        }],
        3,
    )[0]
    .expect("type");
    for seed in [4, 5] {
        apply(
            model,
            &[AuthorOp::Product {
                type_name: "IfcWall".to_owned(),
                container: Some(storey),
                attributes: Vec::new(),
                placement: None,
                type_object: Some(wall_type),
                owner_history: None,
            }],
            seed,
        );
    }
}

#[test]
fn global_ids_do_not_depend_on_entity_ids() {
    let mut a = model();
    let mut b = model();
    b.push(Entity::new(
        "IFCCARTESIANPOINT",
        vec![Value::List(vec![
            Value::Real(0.0),
            Value::Real(0.0),
            Value::Real(0.0),
        ])],
    ));
    type_journal(&mut a);
    type_journal(&mut b);
    assert_eq!(identities(&a), identities(&b));
    assert_eq!(a.ids_of_type("IFCRELDEFINESBYTYPE").len(), 1);
    assert_eq!(b.ids_of_type("IFCRELDEFINESBYTYPE").len(), 1);
}

#[test]
fn a_seed_reused_for_a_second_batch_is_refused() {
    let mut model = model();
    let project = apply(&mut model, &[project()], 1)[0].expect("project");
    let storey_a =
        apply(&mut model, &[spatial("IfcBuildingStorey", project, "A")], 2)[0].expect("storey");
    let (revision, len) = (model.revision(), model.len());
    let error = apply_authoring(
        &mut model,
        ifc::schema::ifc4(),
        &[spatial("IfcBuildingStorey", project, "B")],
        2,
    )
    .expect_err("seed 2 names the first storey's batch");
    match *error.failure {
        AuthoringFailure::SeedReused { holder, .. } => assert_eq!(holder, storey_a),
        other => panic!("{other:?}"),
    }
    assert_eq!((model.revision(), model.len()), (revision, len));
}

#[test]
fn a_seed_whose_entities_were_removed_may_be_used_again() {
    let mut model = model();
    let project = apply(&mut model, &[project()], 1)[0].expect("project");
    let storey =
        apply(&mut model, &[spatial("IfcBuildingStorey", project, "A")], 2)[0].expect("storey");
    let before = global_id(&model, storey);
    apply(
        &mut model,
        &[AuthorOp::RemoveProduct {
            product: storey,
            options: RemoveProductOptions::new(),
        }],
        9,
    );
    let again =
        apply(&mut model, &[spatial("IfcBuildingStorey", project, "A")], 2)[0].expect("storey");
    assert_eq!(global_id(&model, again), before);
}
