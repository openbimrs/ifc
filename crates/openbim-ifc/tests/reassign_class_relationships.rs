//! Taking the entity out of the relationships the new class cannot hold
//! (#635): `ReassignClassOptions::remove_relationships`.
//!
//! IFC4 scenes are built in memory; the door #100 of
//! `test/fixtures/synthetic-relations/cascade_ifc4.ifc` shows the type rule.

#![cfg(all(feature = "step", feature = "author", feature = "ifc4"))]

mod reassign_support;

use ifc::{
    apply_authoring, authoring_handle, AuthorOp, AuthoringFailure, Entity, EntityId, History,
    ReassignClassOptions, RelateOptions, RelationshipConflict, RemovedRelationship, Value,
};
use reassign_support::*;

fn fields(removed: &[RemovedRelationship]) -> Vec<(EntityId, &str, bool, &RelationshipConflict)> {
    removed
        .iter()
        .map(|entry| {
            (
                entry.relationship,
                entry.type_name.as_str(),
                entry.removed,
                &entry.reason,
            )
        })
        .collect()
}

/// The containment relationship of `element`.
fn containing(model: &ifc::Model, element: EntityId) -> EntityId {
    model
        .ids_of_type("IFCRELCONTAINEDINSPATIALSTRUCTURE")
        .iter()
        .copied()
        .find(|id| {
            model.get(*id).unwrap().attributes.iter().any(
                |value| matches!(value, Value::List(items) if items.contains(&Value::Ref(element))),
            )
        })
        .expect("contained")
}

fn reassign_removing(entity: EntityId, type_name: &str, drop: bool) -> AuthorOp {
    AuthorOp::ReassignClass {
        entity,
        type_name: type_name.to_owned(),
        attributes: vec![],
        options: ReassignClassOptions::new()
            .drop_unmapped(drop)
            .remove_relationships(true),
    }
}

#[test]
fn a_contained_element_leaves_the_containment() {
    let (mut model, scene) = scene();
    let containment = containing(&model, scene.contained_wall);
    let outcome = apply(
        &mut model,
        ifc4(),
        &[reassign_removing(scene.contained_wall, "IfcSpace", true)],
    )
    .expect("reassigns");
    assert_eq!(class(&model, scene.contained_wall), "IFCSPACE");
    let report = outcome.reassignments[0].as_ref().unwrap();
    assert_eq!(
        fields(&report.removed_relationships),
        vec![(
            containment,
            "IfcRelContainedInSpatialStructure",
            true,
            &RelationshipConflict::Lost {
                inverse: "ContainedInStructure".into()
            },
        )]
    );
}

#[test]
fn a_containment_with_other_elements_stays() {
    let (mut model, scene) = scene();
    let storey = model.ids_of_type("IFCBUILDINGSTOREY")[0];
    apply(
        &mut model,
        ifc4(),
        &[AuthorOp::Contain {
            structure: storey,
            elements: vec![scene.wall, scene.contained_wall],
            owner_history: None,
            options: RelateOptions::new().replace(true),
        }],
    )
    .expect("contains");
    let containment = containing(&model, scene.contained_wall);
    assert_eq!(containing(&model, scene.wall), containment, "shared");
    let outcome = apply(
        &mut model,
        ifc4(),
        &[reassign_removing(scene.contained_wall, "IfcSpace", true)],
    )
    .expect("reassigns");
    let report = outcome.reassignments[0].as_ref().unwrap();
    assert_eq!(report.removed_relationships.len(), 1);
    assert_eq!(report.removed_relationships[0].relationship, containment);
    assert!(!report.removed_relationships[0].removed);
    let attributes = &model.get(containment).expect("stays").attributes;
    assert!(!attributes.contains(&Value::List(vec![Value::Ref(scene.contained_wall)])));
    assert!(!format!("{attributes:?}").contains(&format!("Ref({})", scene.contained_wall.0)));
}

#[test]
fn a_removal_may_not_empty_a_mandatory_inverse() {
    let (mut model, scene) = scene();
    let opening = model.ids_of_type("IFCOPENINGELEMENT")[0];
    let relationship = model.ids_of_type("IFCRELVOIDSELEMENT")[0];
    let failure = refusal(
        &mut model,
        ifc4(),
        reassign_removing(scene.voided_wall, "IfcSpace", true),
    );
    assert_eq!(
        failure,
        AuthoringFailure::RelationshipRequired {
            relationship,
            entity: opening,
            inverse: "VoidsElements".into(),
            lower: 1,
        }
    );
}

#[test]
fn the_option_is_off_by_default() {
    let (mut model, scene) = scene();
    let failure = refusal(
        &mut model,
        ifc4(),
        reassign(scene.contained_wall, "IfcSpace", vec![]),
    );
    assert!(matches!(failure, AuthoringFailure::RelationshipLost { .. }));
}

#[test]
fn a_referrer_that_is_no_relationship_is_still_refused() {
    let (mut model, _) = scene();
    let ops = [
        create(
            "IfcStructuralLoadGroup",
            vec![
                ("PredefinedType", enumeration("LOAD_CASE")),
                ("ActionType", enumeration("PERMANENT_G")),
                ("ActionSource", enumeration("DEAD_LOAD_G")),
            ],
        ),
        create(
            "IfcStructuralResultGroup",
            vec![
                ("TheoryType", enumeration("FIRST_ORDER_THEORY")),
                ("ResultForLoadGroup", Value::Ref(authoring_handle(0))),
                ("IsLinear", Value::Bool(false)),
            ],
        ),
        reassign_removing(authoring_handle(0), "IfcGroup", true),
    ];
    let error = apply(&mut model, ifc4(), &ops).expect_err("refused");
    assert!(
        matches!(
            &*error.failure,
            AuthoringFailure::WrongReferenceType { attribute, .. }
                if attribute == "ResultForLoadGroup"
        ),
        "{error:?}"
    );
}

#[test]
fn an_inverse_upper_bound_is_still_refused() {
    let (mut model, _) = scene();
    let storey = model.ids_of_type("IFCBUILDINGSTOREY")[0];
    let containment = |structure: EntityId| {
        create(
            "IfcRelContainedInSpatialStructure",
            vec![
                (
                    "RelatedElements",
                    Value::List(vec![Value::Ref(authoring_handle(0))]),
                ),
                ("RelatingStructure", Value::Ref(structure)),
            ],
        )
    };
    let ops = [
        product(
            "IfcProxy",
            None,
            vec![("ProxyType", enumeration("PRODUCT"))],
        ),
        create("IfcBuildingStorey", vec![("Name", text("Second"))]),
        containment(storey),
        containment(authoring_handle(1)),
        reassign_removing(authoring_handle(0), "IfcAnnotation", true),
    ];
    let error = apply(&mut model, ifc4(), &ops).expect_err("refused");
    assert!(
        matches!(
            &*error.failure,
            AuthoringFailure::InverseCardinality { inverse, found: 2, .. }
                if inverse == "ContainedInStructure"
        ),
        "{error:?}"
    );
}

/// Door #100 becomes a wall: its type assignment #120 goes with `TypeRule`,
/// the fill #109 stays.
#[test]
fn a_typed_object_leaves_its_type_assignment() {
    let schema = ifc4();
    let mut model = read("cascade_ifc4.ifc");
    let original = bytes(&model);
    let op = AuthorOp::ReassignClass {
        entity: EntityId(100),
        type_name: "IfcWall".into(),
        attributes: vec![("PredefinedType".into(), enumeration("NOTDEFINED"))],
        options: ReassignClassOptions::new()
            .drop_unmapped(true)
            .remove_relationships(true),
    };
    let mut history = History::new(4);
    let outcome = history
        .track(&mut model, |m| {
            apply_authoring(m, schema, std::slice::from_ref(&op), 5)
        })
        .expect("reassigns");
    let report = outcome.reassignments[0].as_ref().unwrap();
    assert_eq!(
        fields(&report.removed_relationships),
        vec![(
            EntityId(120),
            "IfcRelDefinesByType",
            true,
            &RelationshipConflict::TypeRule {
                type_object: EntityId(111),
                to_rule: "IfcWall.CorrectTypeAssigned".into(),
            },
        )]
    );
    assert!(model.get(EntityId(120)).is_none());
    assert!(model.get(EntityId(109)).is_some(), "the fill stays");
    assert_eq!(class(&model, EntityId(100)), "IFCWALL");
    let after = bytes(&model);

    history.undo(&mut model).unwrap();
    assert_eq!(bytes(&model), original, "undo");
    history.redo(&mut model).unwrap();
    assert_eq!(bytes(&model), after, "redo");
    history.undo(&mut model).unwrap();
    apply_authoring(&mut model, schema, &[op], 5).unwrap();
    assert_eq!(bytes(&model), after, "replay");
}

/// Door #100 in a second type assignment: both go, ascending.
#[test]
fn every_conflicting_type_assignment_is_removed() {
    let mut model = read("cascade_ifc4.ifc");
    let mut attributes = model.get(EntityId(120)).unwrap().attributes.clone();
    attributes[0] = text("3YQ3wW59sf1Vy_ef$WmQgE");
    let second = model.push(Entity::new("IFCRELDEFINESBYTYPE", attributes));
    let report = apply(
        &mut model,
        ifc4(),
        &[AuthorOp::ReassignClass {
            entity: EntityId(100),
            type_name: "IfcWall".into(),
            attributes: vec![("PredefinedType".into(), enumeration("NOTDEFINED"))],
            options: ReassignClassOptions::new()
                .drop_unmapped(true)
                .remove_relationships(true),
        }],
    )
    .expect("reassigns")
    .reassignments[0]
        .clone()
        .unwrap();
    let rule = RelationshipConflict::TypeRule {
        type_object: EntityId(111),
        to_rule: "IfcWall.CorrectTypeAssigned".into(),
    };
    assert_eq!(
        fields(&report.removed_relationships),
        vec![
            (EntityId(120), "IfcRelDefinesByType", true, &rule),
            (second, "IfcRelDefinesByType", true, &rule),
        ]
    );
    assert!(model.get(EntityId(120)).is_none() && model.get(second).is_none());
}

/// Without the option the first refusal stands, even when a later referrer
/// is malformed.
#[test]
fn the_first_refusal_wins_over_a_later_malformed_referrer() {
    let (mut model, _) = scene();
    apply(
        &mut model,
        ifc4(),
        &[
            create(
                "IfcStructuralLoadGroup",
                vec![
                    ("PredefinedType", enumeration("LOAD_CASE")),
                    ("ActionType", enumeration("PERMANENT_G")),
                    ("ActionSource", enumeration("DEAD_LOAD_G")),
                ],
            ),
            create(
                "IfcStructuralResultGroup",
                vec![
                    ("TheoryType", enumeration("FIRST_ORDER_THEORY")),
                    ("ResultForLoadGroup", Value::Ref(authoring_handle(0))),
                    ("IsLinear", Value::Bool(false)),
                ],
            ),
        ],
    )
    .expect("builds");
    let group = model.ids_of_type("IFCSTRUCTURALLOADGROUP")[0];
    model.push(Entity::new("IFCRELASSIGNSTOGROUP", vec![Value::Ref(group)]));
    let op = AuthorOp::ReassignClass {
        entity: group,
        type_name: "IfcGroup".into(),
        attributes: vec![],
        options: ReassignClassOptions::new().drop_unmapped(true),
    };
    let failure = refusal(&mut model, ifc4(), op);
    assert!(
        matches!(failure, AuthoringFailure::WrongReferenceType { .. }),
        "{failure:?}"
    );
}

#[test]
fn a_required_relationship_names_the_entity() {
    let failure = AuthoringFailure::RelationshipRequired {
        relationship: EntityId(7),
        entity: EntityId(9),
        inverse: "VoidsElements".into(),
        lower: 1,
    };
    assert_eq!(
        failure.to_string(),
        "removing #7 would leave #9 without its VoidsElements, which requires at least 1: \
         remove or edit #9 first"
    );
}
