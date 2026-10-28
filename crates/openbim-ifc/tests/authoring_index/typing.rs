//! Type assignments: one relationship joined one object at a time.

use crate::lab::*;

/// The objects a type's relationship lists, in order.
fn typed_objects(model: &Model, type_object: EntityId) -> Vec<EntityId> {
    let relationships: Vec<EntityId> = model
        .reverse_index()
        .referrers(type_object)
        .iter()
        .map(|referrer| referrer.from)
        .filter(|id| {
            model
                .get(*id)
                .is_some_and(|entity| entity.is_type("IFCRELDEFINESBYTYPE"))
        })
        .collect();
    assert_eq!(relationships.len(), 1, "one relationship for the type");
    let mut objects = Vec::new();
    model.get(relationships[0]).expect("present").attributes[4]
        .for_each_ref(&mut |object| objects.push(object));
    objects
}

/// `count` loose walls and a type, in one batch.
fn walls_and_type(lab: &mut Lab, count: usize) -> (EntityId, Vec<EntityId>) {
    let mut ops = vec![wall_type("T")];
    ops.extend((0..count).map(|_| AuthorOp::Product {
        type_name: "IfcWall".into(),
        container: None,
        attributes: vec![],
        placement: None,
        type_object: None,
        owner_history: None,
    }));
    let ids = lab.apply("walls and a type", &ops);
    (
        ids[0].expect("the type"),
        ids[1..].iter().map(|id| id.expect("a wall")).collect(),
    )
}

fn assign(type_object: EntityId, objects: &[EntityId]) -> AuthorOp {
    AuthorOp::AssignType {
        type_object,
        objects: objects.to_vec(),
        owner_history: None,
        options: AssignTypeOptions::new(),
    }
}

/// Typing walls one operation at a time lists the same objects in the same
/// order as one operation typing all of them.
#[test]
fn typing_walls_one_by_one_equals_typing_them_together() {
    let mut lab = Lab::new();
    let (ty, walls) = walls_and_type(&mut lab, 2_000);
    let one_by_one: Vec<AuthorOp> = walls.iter().map(|wall| assign(ty, &[*wall])).collect();
    let mut separate = lab.model.clone();
    apply_authoring(&mut separate, lab.schema, &one_by_one, 11).expect("one by one");
    let mut together = lab.model.clone();
    apply_authoring(&mut together, lab.schema, &[assign(ty, &walls)], 11).expect("together");
    assert_eq!(typed_objects(&separate, ty), walls);
    assert_eq!(typed_objects(&together, ty), walls);
    assert_eq!(separate.len(), together.len());
    consistent(&separate, "one by one");
    consistent(&together, "together");
}

/// An object named twice joins once, whether in one operation or two.
#[test]
fn a_repeated_object_joins_once() {
    let mut lab = Lab::new();
    let (ty, walls) = walls_and_type(&mut lab, 3);
    let ops = [
        assign(ty, &[walls[0]]),
        assign(ty, &[walls[1], walls[1], walls[2]]),
    ];
    assert!(stage_both(&lab.model, lab.schema, &ops, "repeated"));
    lab.apply("repeated", &ops);
    assert_eq!(typed_objects(&lab.model, ty), walls);
}

/// A relationship joined in a batch and then edited by name is checked
/// whole again: the edit may not break it.
#[test]
fn a_joined_relationship_edited_afterwards_is_checked_whole() {
    let mut lab = Lab::new();
    let (ty, walls) = walls_and_type(&mut lab, 3);
    let batch = |last: AuthorOp| {
        vec![
            assign(ty, &[walls[0]]),
            assign(ty, &[walls[1]]),
            // The relationship of the first operation.
            last,
        ]
    };
    let relationship = authoring_handle(0);
    let edit = |value| AuthorOp::Edit {
        entity: relationship,
        attributes: vec![("RelatedObjects".into(), value)],
    };
    // A list of a wall and a type object is refused by the whole-record check.
    let bad = edit(Value::List(vec![Value::Ref(walls[0]), Value::Ref(ty)]));
    let before = fingerprint(&lab.model);
    let error = apply_authoring(&mut lab.model, lab.schema, &batch(bad), 12).expect_err("refused");
    assert!(matches!(
        *error.failure,
        AuthoringFailure::WrongReferenceType { .. }
    ));
    assert_eq!(fingerprint(&lab.model), before);
    // Dropping an object the join added leaves the other joined.
    let good = edit(Value::List(vec![Value::Ref(walls[1])]));
    let ids = lab.apply("edit after joins", &batch(good));
    let relationship = ids[0].expect("the relationship");
    let mut listed = Vec::new();
    lab.model.get(relationship).expect("present").attributes[4]
        .for_each_ref(&mut |object| listed.push(object));
    assert_eq!(listed, [walls[1]]);
}

/// A relationship that takes at most two objects: the third, joined later in
/// the batch, is refused by the append as the whole-record check would.
#[test]
fn a_join_past_the_declared_upper_bound_is_refused() {
    let schema = Schema::from_express(
        "SCHEMA IFC4;\n\
         TYPE IfcGloballyUniqueId = STRING; END_TYPE;\n\
         TYPE IfcLabel = STRING; END_TYPE;\n\
         ENTITY IfcRoot ABSTRACT SUPERTYPE OF (ONEOF(IfcObjectDefinition, IfcRelationship));\n\
           GlobalId : IfcGloballyUniqueId;\n\
           Name : OPTIONAL IfcLabel;\n\
         END_ENTITY;\n\
         ENTITY IfcObjectDefinition ABSTRACT SUPERTYPE OF (ONEOF(IfcObject, IfcTypeObject))\n\
           SUBTYPE OF (IfcRoot);\n\
         END_ENTITY;\n\
         ENTITY IfcObject SUBTYPE OF (IfcObjectDefinition);\n\
         END_ENTITY;\n\
         ENTITY IfcTypeObject SUBTYPE OF (IfcObjectDefinition);\n\
         END_ENTITY;\n\
         ENTITY IfcRelationship ABSTRACT SUBTYPE OF (IfcRoot);\n\
         END_ENTITY;\n\
         ENTITY IfcRelDefinesByType SUBTYPE OF (IfcRelationship);\n\
           RelatedObjects : SET [1:2] OF IfcObject;\n\
           RelatingType : IfcTypeObject;\n\
         END_ENTITY;\n\
         END_SCHEMA;",
    );
    let mut model = Model::new();
    let h = authoring_handle;
    let object = || create("IfcObject", vec![]);
    let batch = |assigned: usize| {
        let mut ops = vec![
            AuthorOp::TypeObject {
                type_name: "IfcTypeObject".into(),
                attributes: vec![],
                owner_history: None,
            },
            object(),
            object(),
            object(),
        ];
        ops.extend((1..=assigned).map(|n| assign(h(0), &[h(n)])));
        ops
    };
    apply_authoring(&mut model, &schema, &batch(2), 1).expect("two objects fit");
    let before = fingerprint(&model);
    let mut fresh = Model::new();
    let error = apply_authoring(&mut fresh, &schema, &batch(3), 1).expect_err("three do not");
    assert_eq!(error.op, Some(6));
    assert!(
        matches!(
            &*error.failure,
            AuthoringFailure::Cardinality { detail, .. } if detail == "3 items; at most 2"
        ),
        "{error:?}"
    );
    assert!(fresh.is_empty());
    assert_eq!(fingerprint(&model), before);
}

/// Objects appended by a later join are checked one by one, with the
/// refusals the whole-record check gives.
#[test]
fn a_later_join_checks_each_object() {
    let mut lab = Lab::new();
    let (ty, walls) = walls_and_type(&mut lab, 3);
    let other_type = lab.apply("another type", &[wall_type("U")])[0].expect("the type");
    for (what, object) in [
        ("a type object", other_type),
        ("a missing one", EntityId(999_999)),
    ] {
        let ops = [
            assign(ty, &[walls[0]]),
            assign(ty, &[walls[1]]),
            assign(ty, &[object]),
        ];
        let kept = stage_authoring(&lab.model, lab.schema, &ops, 3);
        let mut fresh = lab.model.clone();
        fresh.drop_derived_indexes();
        let rebuilt = stage_authoring(&fresh, lab.schema, &ops, 3);
        let (kept, rebuilt) = (kept.expect_err(what), rebuilt.expect_err(what));
        assert_eq!(kept, rebuilt, "{what}");
        assert_eq!(kept.op, Some(2), "{what}");
        assert!(
            matches!(
                *kept.failure,
                AuthoringFailure::WrongReferenceType { .. }
                    | AuthoringFailure::MissingReference { .. }
            ),
            "{what}: {kept:?}"
        );
    }
}
