//! Unit tests for the object-relation queries' refusals, over models built
//! in memory. The answers across IFC2X3, IFC4 and IFC4X3 files are pinned
//! by `tests/object_relations.rs`.

use ifc_model::{Entity, EntityId, Model, Value};
use ifc_schema::Schema;

use super::*;

fn schema() -> &'static Schema {
    crate::schema::for_version(crate::SchemaVersion::Ifc4)
        .or_else(|_| crate::schema::for_version(crate::SchemaVersion::Ifc4x3))
        .or_else(|_| crate::schema::for_version(crate::SchemaVersion::Ifc2x3))
        .expect("the facade's tests bundle a release")
}

fn rel(type_name: &str, from: Value, to: Value) -> Entity {
    Entity::new(
        type_name,
        vec![
            Value::Text("2O2Fr$t4X7Zf8NOew3FLOH".into()),
            Value::Null,
            Value::Null,
            Value::Null,
            from,
            to,
        ],
    )
}

fn refs(ids: &[u64]) -> Value {
    Value::List(ids.iter().map(|id| Value::Ref(EntityId(*id))).collect())
}

/// #1 wall, #2 and #3 wall types, #4 opening, #5 door, #6 point.
fn base() -> Model {
    let mut model = Model::new();
    for type_name in [
        "IFCWALL",
        "IFCWALLTYPE",
        "IFCWALLTYPE",
        "IFCOPENINGELEMENT",
        "IFCDOOR",
        "IFCCARTESIANPOINT",
    ] {
        model.push(Entity::new(type_name, Vec::new()));
    }
    model
}

#[test]
fn a_query_refuses_a_missing_id_and_a_wrong_type() {
    let model = base();
    let schema = schema();
    assert_eq!(
        type_of(&model, schema, EntityId(99)),
        Err(ObjectRelationError::MissingEntity(EntityId(99)))
    );
    let wrong = type_of(&model, schema, EntityId(6)).unwrap_err();
    assert!(
        matches!(
            &wrong,
            ObjectRelationError::WrongEntityType { entity: EntityId(6), found, expected: "IfcObject" }
                if found == "IFCCARTESIANPOINT"
        ),
        "{wrong:?}"
    );
    assert!(matches!(
        occurrences_of(&model, schema, EntityId(1)),
        Err(ObjectRelationError::WrongEntityType {
            expected: "IfcTypeObject",
            ..
        })
    ));
    assert!(matches!(
        fillings_of(&model, schema, EntityId(1)),
        Err(ObjectRelationError::WrongEntityType {
            expected: "IfcOpeningElement",
            ..
        })
    ));
    assert!(matches!(
        voided_element_of(&model, schema, EntityId(5)),
        Err(ObjectRelationError::WrongEntityType {
            expected: "IfcFeatureElementSubtraction",
            ..
        })
    ));
    assert!(matches!(
        openings_of(&model, schema, EntityId(2)),
        Err(ObjectRelationError::WrongEntityType {
            expected: "IfcElement",
            ..
        })
    ));
}

#[test]
fn nothing_stated_is_none_or_empty() {
    let model = base();
    let schema = schema();
    assert_eq!(type_of(&model, schema, EntityId(1)), Ok(None));
    assert_eq!(occurrences_of(&model, schema, EntityId(2)), Ok(vec![]));
    assert_eq!(openings_of(&model, schema, EntityId(1)), Ok(vec![]));
    assert_eq!(fillings_of(&model, schema, EntityId(4)), Ok(vec![]));
    assert_eq!(voided_element_of(&model, schema, EntityId(4)), Ok(None));
}

/// The schema allows one type per object: two different ones are refused
/// with both, never one picked.
#[test]
fn two_types_for_one_object_are_ambiguous() {
    let mut model = base();
    let schema = schema();
    model.push(rel(
        "IFCRELDEFINESBYTYPE",
        refs(&[1]),
        Value::Ref(EntityId(2)),
    )); // #7
    model.push(rel(
        "IFCRELDEFINESBYTYPE",
        refs(&[1]),
        Value::Ref(EntityId(3)),
    )); // #8
    let error = type_of(&model, schema, EntityId(1)).unwrap_err();
    assert_eq!(
        error,
        ObjectRelationError::Ambiguous {
            entity: EntityId(1),
            candidates: vec![EntityId(2), EntityId(3)],
            relationships: vec![EntityId(7), EntityId(8)],
        }
    );
    assert_eq!(
        error.to_string(),
        "#1 has 2 answers where the schema allows one: #2, #3 (stated by #7, #8)"
    );
}

/// The same type stated twice is one answer, and an object listed twice is
/// one occurrence.
#[test]
fn a_repeated_statement_is_one_answer() {
    let mut model = base();
    let schema = schema();
    model.push(rel(
        "IFCRELDEFINESBYTYPE",
        refs(&[1, 1]),
        Value::Ref(EntityId(2)),
    ));
    model.push(rel(
        "IFCRELDEFINESBYTYPE",
        refs(&[1]),
        Value::Ref(EntityId(2)),
    ));
    assert_eq!(type_of(&model, schema, EntityId(1)), Ok(Some(EntityId(2))));
    assert_eq!(
        occurrences_of(&model, schema, EntityId(2)),
        Ok(vec![EntityId(1)])
    );
}

#[test]
fn two_hosts_for_one_opening_are_ambiguous() {
    let mut model = base();
    let schema = schema();
    model.push(Entity::new("IFCWALL", Vec::new())); // #7
    model.push(rel(
        "IFCRELVOIDSELEMENT",
        Value::Ref(EntityId(1)),
        Value::Ref(EntityId(4)),
    ));
    model.push(rel(
        "IFCRELVOIDSELEMENT",
        Value::Ref(EntityId(7)),
        Value::Ref(EntityId(4)),
    ));
    assert!(matches!(
        voided_element_of(&model, schema, EntityId(4)),
        Err(ObjectRelationError::Ambiguous { entity: EntityId(4), ref candidates, .. })
            if candidates == &[EntityId(1), EntityId(7)]
    ));
    // Each host still lists the opening.
    assert_eq!(
        openings_of(&model, schema, EntityId(1)),
        Ok(vec![EntityId(4)])
    );
    assert_eq!(
        openings_of(&model, schema, EntityId(7)),
        Ok(vec![EntityId(4)])
    );
}

#[test]
fn a_malformed_end_is_refused_by_name() {
    let mut model = base();
    let schema = schema();
    // RelatedObjects must be a list; RelatingType a reference.
    model.push(rel(
        "IFCRELDEFINESBYTYPE",
        Value::Ref(EntityId(1)),
        Value::Ref(EntityId(2)),
    ));
    assert_eq!(
        type_of(&model, schema, EntityId(1)),
        Err(ObjectRelationError::Malformed {
            relationship: EntityId(7),
            attribute: "RelatedObjects",
        })
    );
    let mut model = base();
    model.push(rel(
        "IFCRELFILLSELEMENT",
        Value::Ref(EntityId(4)),
        Value::Null,
    ));
    assert_eq!(
        fillings_of(&model, schema, EntityId(4)),
        Err(ObjectRelationError::Malformed {
            relationship: EntityId(7),
            attribute: "RelatedBuildingElement",
        })
    );
}

#[test]
fn an_answer_the_model_lacks_is_a_missing_reference() {
    let mut model = base();
    let schema = schema();
    model.push(rel(
        "IFCRELFILLSELEMENT",
        Value::Ref(EntityId(4)),
        Value::Ref(EntityId(42)),
    ));
    assert_eq!(
        fillings_of(&model, schema, EntityId(4)),
        Err(ObjectRelationError::MissingReference {
            relationship: EntityId(7),
            target: EntityId(42),
        })
    );
}
