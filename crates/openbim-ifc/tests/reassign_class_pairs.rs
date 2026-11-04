//! Moving a type object with its occurrences (#636): rule 1 and 7 of
//! `AuthorOp::ReassignClass` and `ReassignClassOptions::move_pairs`.
//!
//! The walls #20 and #40 of `test/fixtures/synthetic-relations/cascade_*.ifc`
//! are typed by the `IfcWallType` #66 through #67; the door #100 by the
//! `IfcDoorType` #111.

#![cfg(all(
    feature = "step",
    feature = "author",
    feature = "ifc2x3",
    feature = "ifc4"
))]

mod reassign_support;

use ifc::{
    apply_authoring, AuthorOp, AuthoringFailure, EntityId, History, ReassignClassOptions,
    ReassignReport, RelationshipConflict, SchemaVersion, Value,
};
use reassign_support::*;

const WALL: EntityId = EntityId(20);
const OTHER_WALL: EntityId = EntityId(40);
const WALL_TYPE: EntityId = EntityId(66);
const DOOR: EntityId = EntityId(100);
const DOOR_TYPE: EntityId = EntityId(111);

fn op(
    entity: EntityId,
    class: &str,
    attributes: Vec<(&str, Value)>,
    options: ReassignClassOptions,
) -> AuthorOp {
    AuthorOp::ReassignClass {
        entity,
        type_name: class.to_owned(),
        attributes: attributes
            .into_iter()
            .map(|(name, value)| (name.to_owned(), value))
            .collect(),
        options,
    }
}

fn pairs() -> ReassignClassOptions {
    ReassignClassOptions::new().move_pairs(true)
}

fn not_defined(entity: EntityId) -> AuthorOp {
    AuthorOp::Edit {
        entity,
        attributes: vec![("PredefinedType".into(), enumeration("NOTDEFINED"))],
    }
}

fn ifc4_model() -> ifc::Model {
    read("cascade_ifc4.ifc")
}

fn report(outcome: &ifc::AuthoringOutcome, op: usize) -> &ReassignReport {
    outcome.reassignments[op].as_ref().expect("a report")
}

fn entities(reports: &[ReassignReport]) -> Vec<(EntityId, &str, &str)> {
    reports
        .iter()
        .map(|pair| (pair.entity, pair.from.as_str(), pair.to.as_str()))
        .collect()
}

#[test]
fn an_occurrence_moves_its_type_and_the_other_occurrences() {
    let schema = ifc4();
    let ops = [
        not_defined(OTHER_WALL),
        not_defined(WALL_TYPE),
        op(
            WALL,
            "IfcSlab",
            vec![("PredefinedType", enumeration("FLOOR"))],
            pairs(),
        ),
    ];
    let mut model = ifc4_model();
    let original = bytes(&model);
    let mut history = History::new(8);
    let outcome = history
        .track(&mut model, |m| apply_authoring(m, schema, &ops, 5))
        .expect("moves the family");
    let primary = report(&outcome, 2);
    assert_eq!((primary.entity, primary.to.as_str()), (WALL, "IfcSlab"));
    assert_eq!(
        entities(&primary.paired),
        [
            (OTHER_WALL, "IfcWall", "IfcSlab"),
            (WALL_TYPE, "IfcWallType", "IfcSlabType")
        ]
    );
    assert!(primary.paired.iter().all(|pair| pair.paired.is_empty()));
    assert_eq!(class(&model, WALL), "IFCSLAB");
    assert_eq!(class(&model, OTHER_WALL), "IFCSLAB");
    assert_eq!(class(&model, WALL_TYPE), "IFCSLABTYPE");
    let after = bytes(&model);

    history.undo(&mut model).unwrap();
    assert_eq!(bytes(&model), original, "undo");
    history.redo(&mut model).unwrap();
    assert_eq!(bytes(&model), after, "redo");
    history.undo(&mut model).unwrap();
    apply_authoring(&mut model, schema, &ops, 5).unwrap();
    assert_eq!(bytes(&model), after, "replay");
}

#[test]
fn a_pair_that_cannot_move_names_itself() {
    let mut model = ifc4_model();
    let failure = refusal(
        &mut model,
        ifc4(),
        op(
            WALL,
            "IfcSlab",
            vec![("PredefinedType", enumeration("FLOOR"))],
            pairs(),
        ),
    );
    assert!(
        matches!(
            &failure,
            AuthoringFailure::PairNotReassigned { entity, to, .. }
                if *entity == WALL_TYPE && to == "IfcSlabType"
        ),
        "{failure:?}"
    );
}

#[test]
fn a_type_object_is_refused_by_its_occurrences_without_options() {
    let mut model = ifc4_model();
    let slab_type = |options| {
        op(
            WALL_TYPE,
            "IfcSlabType",
            vec![("PredefinedType", enumeration("NOTDEFINED"))],
            options,
        )
    };
    let failure = refusal(&mut model, ifc4(), slab_type(ReassignClassOptions::new()));
    assert!(
        matches!(
            &failure,
            AuthoringFailure::TypeNotAccepted { type_object, to, object, rule, required }
                if *type_object == WALL_TYPE
                    && to == "IfcSlabType"
                    && *object == WALL
                    && rule == "IfcWall.CorrectTypeAssigned"
                    && required == "IfcWallType"
        ),
        "{failure:?}"
    );

    apply(
        &mut model,
        ifc4(),
        &[slab_type(
            ReassignClassOptions::new().remove_relationships(true),
        )],
    )
    .map(|outcome| {
        let removed = &report(&outcome, 0).removed_relationships;
        assert_eq!(removed.len(), 2);
        assert!(removed.iter().all(|entry| {
            entry.relationship == EntityId(67)
                && matches!(
                    &entry.reason,
                    RelationshipConflict::TypeRule { type_object, .. } if *type_object == WALL_TYPE
                )
        }));
        assert!(!removed[0].removed && removed[1].removed);
    })
    .expect("detaches");
    assert_eq!(class(&model, WALL_TYPE), "IFCSLABTYPE");
    assert_eq!(class(&model, WALL), "IFCWALL");
    assert!(model.get(EntityId(67)).is_none());
}

#[test]
fn a_type_object_moves_its_occurrences_with_move_pairs() {
    let mut model = ifc4_model();
    let outcome = apply(
        &mut model,
        ifc4(),
        &[
            not_defined(WALL),
            not_defined(OTHER_WALL),
            op(
                WALL_TYPE,
                "IfcSlabType",
                vec![("PredefinedType", enumeration("NOTDEFINED"))],
                pairs(),
            ),
        ],
    )
    .expect("moves the occurrences");
    assert_eq!(
        entities(&report(&outcome, 2).paired),
        [
            (WALL, "IfcWall", "IfcSlab"),
            (OTHER_WALL, "IfcWall", "IfcSlab")
        ]
    );
    assert_eq!(class(&model, WALL), "IFCSLAB");
    assert_eq!(class(&model, WALL_TYPE), "IFCSLABTYPE");
}

#[test]
fn nothing_moves_where_the_type_rule_is_satisfied() {
    let mut model = ifc4_model();
    let outcome = apply(
        &mut model,
        ifc4(),
        &[op(WALL, "IfcWallStandardCase", vec![], pairs())],
    )
    .expect("reassigns");
    assert!(report(&outcome, 0).paired.is_empty());
    assert_eq!(class(&model, WALL_TYPE), "IFCWALLTYPE");
}

#[test]
fn a_door_type_becomes_a_wall_type_with_fewer_attributes() {
    let mut model = ifc4_model();
    assert_eq!(model.get(DOOR_TYPE).unwrap().attributes.len(), 13);
    let options = pairs().drop_unmapped(true);
    let outcome = apply(
        &mut model,
        ifc4(),
        &[
            not_defined(DOOR),
            not_defined(DOOR_TYPE),
            op(DOOR, "IfcWall", vec![], options),
        ],
    )
    .expect("moves the door and its type");
    assert_eq!(
        entities(&report(&outcome, 2).paired),
        [(DOOR_TYPE, "IfcDoorType", "IfcWallType")]
    );
    assert_eq!(class(&model, DOOR_TYPE), "IFCWALLTYPE");
    assert_eq!(model.get(DOOR_TYPE).unwrap().attributes.len(), 10);
}

#[test]
fn a_typed_object_still_needs_move_pairs_to_leave_its_type_rule() {
    let mut model = ifc4_model();
    let failure = refusal(
        &mut model,
        ifc4(),
        op(
            WALL,
            "IfcSlab",
            vec![("PredefinedType", enumeration("FLOOR"))],
            ReassignClassOptions::new(),
        ),
    );
    assert!(
        matches!(&failure, AuthoringFailure::TypeRuleChanged { type_object, .. } if *type_object == WALL_TYPE),
        "{failure:?}"
    );
}

#[test]
fn occurrences_and_type_objects_do_not_cross() {
    let mut model = ifc4_model();
    for (entity, class, expected) in [
        (WALL_TYPE, "IfcWall", "IfcTypeObject"),
        (WALL, "IfcWallType", "IfcObject that is not an IfcProject"),
    ] {
        let failure = refusal(
            &mut model,
            ifc4(),
            op(entity, class, vec![], ReassignClassOptions::new()),
        );
        assert!(
            matches!(
                &failure,
                AuthoringFailure::NotA { type_name, expected: found }
                    if type_name == class && found == expected
            ),
            "{failure:?}"
        );
    }
}

#[test]
fn ifc2x3_declares_no_type_rule_so_nothing_pairs() {
    let schema = ifc::schema::for_version(SchemaVersion::Ifc2x3).unwrap();
    let mut model = read("cascade_ifc2x3.ifc");
    let outcome = apply(&mut model, schema, &[op(WALL, "IfcSlab", vec![], pairs())])
        .expect("nothing is refused");
    assert!(report(&outcome, 0).paired.is_empty());
    assert_eq!(class(&model, WALL), "IFCSLAB");
    assert_eq!(class(&model, WALL_TYPE), "IFCWALLTYPE");
}
