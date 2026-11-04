//! Changing an object's class in place (#483): `AuthorOp::ReassignClass`.
//!
//! IFC4 scenes are built in memory by `apply_authoring`; the refusals that
//! need the release tables of all three also run over
//! `test/fixtures/synthetic-relations/cascade_*.ifc` (Wall A #20, Door 1 #100).

#![cfg(all(
    feature = "step",
    feature = "author",
    feature = "ifc2x3",
    feature = "ifc4",
    feature = "ifc4x3"
))]

mod reassign_support;

use ifc::{
    apply_authoring, authoring_handle, stage_authoring, AuthorOp, AuthoringFailure, EntityId,
    History, Model, ReassignClassOptions, SchemaVersion, Value,
};
use reassign_support::*;

#[test]
fn a_column_becomes_a_standard_case_and_back_in_ifc4() {
    let (mut model, scene) = scene();
    let before = model.get(scene.typed_column).unwrap().attributes.clone();
    let revision = model.revision();

    let outcome = apply(
        &mut model,
        ifc4(),
        &[reassign(
            scene.typed_column,
            "IfcColumnStandardCase",
            vec![],
        )],
    )
    .expect("reassigns");
    assert_eq!(outcome.ids, vec![Some(scene.typed_column)]);
    let report = outcome.reassignments[0].as_ref().expect("a report");
    assert_eq!(report.entity, scene.typed_column);
    assert_eq!(
        (report.from.as_str(), report.to.as_str()),
        ("IfcColumn", "IfcColumnStandardCase")
    );
    assert!(report.dropped.is_empty());
    assert!(outcome.revision > revision);
    assert_eq!(class(&model, scene.typed_column), "IFCCOLUMNSTANDARDCASE");
    assert_eq!(model.get(scene.typed_column).unwrap().attributes, before);
    assert_eq!(
        model.ids_of_type("IFCCOLUMNSTANDARDCASE"),
        [scene.typed_column]
    );
    assert_eq!(model.ids_of_type("IFCCOLUMN"), [scene.column]);
    let typed_by = model
        .ids_of_type("IFCRELDEFINESBYTYPE")
        .iter()
        .filter_map(|id| model.get(*id))
        .any(|rel| rel.attributes.contains(&Value::Ref(scene.column_type)));
    assert!(typed_by, "the type relationship is intact");

    apply(
        &mut model,
        ifc4(),
        &[reassign(scene.typed_column, "IfcColumn", vec![])],
    )
    .expect("back");
    assert_eq!(class(&model, scene.typed_column), "IFCCOLUMN");
    assert_eq!(model.get(scene.typed_column).unwrap().attributes, before);
    assert!(model.ids_of_type("IFCCOLUMNSTANDARDCASE").is_empty());
}

#[test]
fn a_created_entity_is_reassigned_in_its_batch() {
    let (model, _) = scene();
    let staged = stage_authoring(
        &model,
        ifc4(),
        &[
            product("IfcColumn", None, vec![]),
            reassign(authoring_handle(0), "IfcColumnStandardCase", vec![]),
        ],
        3,
    )
    .expect("stages");
    assert_eq!(staged.ids[0], staged.ids[1]);
    let edits = staged.transaction.edits();
    let creates: Vec<_> = edits
        .iter()
        .filter_map(|edit| match edit {
            ifc_model::Edit::Create { entity, .. } => Some(entity.type_name.to_string()),
            _ => None,
        })
        .collect();
    assert_eq!(creates, ["IFCCOLUMNSTANDARDCASE"]);
    assert!(edits
        .iter()
        .all(|edit| !matches!(edit, ifc_model::Edit::Retype { .. })));
}

#[test]
fn a_reassignment_composes_with_edits() {
    let (model, scene) = scene();
    let rename = |name: &str| AuthorOp::Edit {
        entity: scene.typed_column,
        attributes: vec![("Name".into(), text(name))],
    };
    let swap = || reassign(scene.typed_column, "IfcColumnStandardCase", vec![]);
    for ops in [[swap(), rename("A")], [rename("A"), swap()]] {
        let mut copy = model.clone();
        apply(&mut copy, ifc4(), &ops).expect("applies");
        let column = copy.get(scene.typed_column).unwrap();
        assert_eq!(&*column.type_name, "IFCCOLUMNSTANDARDCASE");
        assert_eq!(column.attributes[2], text("A"));
    }
}

#[test]
fn the_predefined_type_must_fit_the_new_class() {
    let (mut model, scene) = scene();
    // A column with PredefinedType COLUMN is not a beam (and is untyped, so
    // no type rule is in play).
    apply(
        &mut model,
        ifc4(),
        &[AuthorOp::Edit {
            entity: scene.column,
            attributes: vec![("PredefinedType".into(), enumeration("COLUMN"))],
        }],
    )
    .unwrap();
    let failure = refusal(
        &mut model,
        ifc4(),
        reassign(scene.column, "IfcBeam", vec![]),
    );
    assert!(
        matches!(
            failure,
            AuthoringFailure::Author(ifc::author::AuthorError::NotEnumerationMember { .. })
        ),
        "{failure:?}"
    );
    apply(
        &mut model,
        ifc4(),
        &[reassign(
            scene.column,
            "IfcBeam",
            vec![("PredefinedType", enumeration("BEAM"))],
        )],
    )
    .expect("accepted with the override");
    assert_eq!(class(&model, scene.column), "IFCBEAM");
}

#[test]
fn unmapped_attributes_are_refused_unless_dropped() {
    let (mut model, scene) = scene();
    let failure = refusal(
        &mut model,
        ifc4(),
        reassign(scene.door, "IfcWindow", vec![]),
    );
    assert_eq!(
        failure,
        AuthoringFailure::AttributesDropped {
            entity: scene.door,
            from: "IfcDoor".into(),
            to: "IfcWindow".into(),
            attributes: vec!["OperationType".into()],
        }
    );
    let outcome = apply(
        &mut model,
        ifc4(),
        &[AuthorOp::ReassignClass {
            entity: scene.door,
            type_name: "IfcWindow".into(),
            attributes: vec![("PredefinedType".into(), enumeration("WINDOW"))],
            options: ReassignClassOptions::new().drop_unmapped(true),
        }],
    )
    .expect("accepted");
    let report = outcome.reassignments[0].as_ref().unwrap();
    assert_eq!(report.dropped, ["OperationType"]);
    assert_eq!(class(&model, scene.door), "IFCWINDOW");
}

#[test]
fn a_referrer_must_accept_the_new_class() {
    let (mut model, scene) = scene();
    let failure = refusal(
        &mut model,
        ifc4(),
        reassign(scene.voided_wall, "IfcSpace", vec![]),
    );
    assert!(
        matches!(
            &failure,
            AuthoringFailure::WrongReferenceType { entity, attribute, .. }
                if entity == "IFCRELVOIDSELEMENT" && attribute == "RelatingBuildingElement"
        ),
        "{failure:?}"
    );
}

#[test]
fn a_relationship_the_new_class_cannot_hold_is_refused() {
    let (mut model, scene) = scene();
    let failure = refusal(
        &mut model,
        ifc4(),
        reassign(scene.contained_wall, "IfcSpace", vec![]),
    );
    assert!(
        matches!(
            &failure,
            AuthoringFailure::RelationshipLost { entity, inverse, to, .. }
                if *entity == scene.contained_wall && inverse == "ContainedInStructure" && to == "IfcSpace"
        ),
        "{failure:?}"
    );
}

#[test]
fn an_inverse_the_new_class_requires_must_be_filled() {
    let (mut model, scene) = scene();
    let failure = refusal(
        &mut model,
        ifc4(),
        reassign(scene.wall, "IfcOpeningElement", vec![]),
    );
    assert_eq!(
        failure,
        AuthoringFailure::InverseCardinality {
            entity: scene.wall,
            to: "IfcOpeningElement".into(),
            inverse: "VoidsElements".into(),
            found: 0,
            lower: 1,
            upper: Some(1),
        }
    );
}

#[test]
fn an_inverse_upper_bound_is_enforced() {
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
        AuthorOp::ReassignClass {
            entity: authoring_handle(0),
            type_name: "IfcAnnotation".into(),
            attributes: vec![],
            options: ReassignClassOptions::new().drop_unmapped(true),
        },
    ];
    let error = apply(&mut model, ifc4(), &ops).expect_err("refused");
    assert!(
        matches!(
            &*error.failure,
            AuthoringFailure::InverseCardinality { inverse, found: 2, lower: 0, upper: Some(1), .. }
                if inverse == "ContainedInStructure"
        ),
        "{error:?}"
    );
}

#[test]
fn refusals_leave_the_model_unchanged() {
    let (mut model, scene) = scene();
    let failure = refusal(
        &mut model,
        ifc4(),
        reassign(scene.wall, "IfcBuildingElement", vec![]),
    );
    assert_eq!(
        failure,
        AuthoringFailure::AbstractEntity {
            type_name: "IfcBuildingElement".into()
        }
    );
    let failure = refusal(
        &mut model,
        ifc4(),
        reassign(scene.wall, "IfcWallType", vec![]),
    );
    assert!(
        matches!(failure, AuthoringFailure::NotA { .. }),
        "{failure:?}"
    );
    let failure = refusal(
        &mut model,
        ifc4(),
        reassign(EntityId(9999), "IfcWall", vec![]),
    );
    assert_eq!(failure, AuthoringFailure::MissingEntity(EntityId(9999)));

    let mut model = read("cascade_ifc4x3.ifc");
    let schema = ifc::schema::for_version(SchemaVersion::Ifc4x3).unwrap();
    let failure = refusal(
        &mut model,
        schema,
        reassign(EntityId(20), "IfcColumnStandardCase", vec![]),
    );
    assert!(
        matches!(failure, AuthoringFailure::UnknownEntity { .. }),
        "{failure:?}"
    );
}

const FIXTURES: [(&str, SchemaVersion); 3] = [
    ("cascade_ifc2x3.ifc", SchemaVersion::Ifc2x3),
    ("cascade_ifc4.ifc", SchemaVersion::Ifc4),
    ("cascade_ifc4x3.ifc", SchemaVersion::Ifc4x3),
];

/// Wall A #20, typed by #66, contained, voided and classified, becomes a
/// standard case; undo, redo and replay give the exact bytes.
#[test]
fn undo_redo_and_replay_are_byte_identical() {
    for (name, version) in FIXTURES {
        let schema = ifc::schema::for_version(version).unwrap();
        let mut model = read(name);
        let original = bytes(&model);
        let ops = [reassign(EntityId(20), "IfcWallStandardCase", vec![])];
        let mut history = History::new(8);
        history
            .track(&mut model, |m| apply_authoring(m, schema, &ops, 5))
            .unwrap_or_else(|error| panic!("{name}: {error}"));
        let after = bytes(&model);
        let expected = String::from_utf8(original.clone())
            .unwrap()
            .replace("#20=IFCWALL(", "#20=IFCWALLSTANDARDCASE(")
            .into_bytes();
        assert_eq!(after, expected, "{name}: the output");

        history.undo(&mut model).unwrap();
        assert_eq!(bytes(&model), original, "{name}: undo");
        history.redo(&mut model).unwrap();
        assert_eq!(bytes(&model), after, "{name}: redo");
        history.undo(&mut model).unwrap();
        apply_authoring(&mut model, schema, &ops, 5).unwrap();
        assert_eq!(bytes(&model), after, "{name}: replay");
    }
}

#[test]
fn ifc2x3_has_no_type_rule() {
    let to_slab = |predefined: bool| {
        reassign(
            EntityId(20),
            "IfcSlab",
            if predefined {
                vec![("PredefinedType", enumeration("FLOOR"))]
            } else {
                vec![]
            },
        )
    };
    let mut model = read("cascade_ifc2x3.ifc");
    let schema = ifc::schema::for_version(SchemaVersion::Ifc2x3).unwrap();
    assert_eq!(model.get(EntityId(20)).unwrap().attributes.len(), 8);
    let original = bytes(&model);
    let mut history = History::new(4);
    history
        .track(&mut model, |m| apply(m, schema, &[to_slab(false)]))
        .expect("IFC2X3 has no type rule");
    assert_eq!(class(&model, EntityId(20)), "IFCSLAB");
    assert_eq!(model.get(EntityId(20)).unwrap().attributes.len(), 9);
    history.undo(&mut model).unwrap();
    assert_eq!(bytes(&model), original, "undo restores the shorter record");

    for (name, version) in [
        ("cascade_ifc4.ifc", SchemaVersion::Ifc4),
        ("cascade_ifc4x3.ifc", SchemaVersion::Ifc4x3),
    ] {
        let schema = ifc::schema::for_version(version).unwrap();
        let mut model = read(name);
        let failure = refusal(&mut model, schema, to_slab(true));
        assert!(
            matches!(
                &failure,
                AuthoringFailure::TypeRuleChanged { object, type_object, from_rule, to_rule }
                    if *object == EntityId(20)
                        && *type_object == EntityId(66)
                        && from_rule.as_deref() == Some("IfcWall.CorrectTypeAssigned")
                        && to_rule == "IfcSlab.CorrectTypeAssigned"
            ),
            "{name}: {failure:?}"
        );
    }
}

/// `IfcEvent.CorrectTypeAssigned` checks the event trigger type, not the type
/// object (#670): a typed process moves to `IfcEvent` and keeps its type.
#[test]
fn a_typed_task_becomes_an_event() {
    let mut model = Model::new();
    let ops = [
        AuthorOp::TypeObject {
            type_name: "IfcTaskType".into(),
            attributes: vec![("PredefinedType".into(), enumeration("NOTDEFINED"))],
            owner_history: None,
        },
        create(
            "IfcTask",
            vec![("Name", text("T")), ("IsMilestone", Value::Bool(false))],
        ),
        create(
            "IfcRelDefinesByType",
            vec![
                (
                    "RelatedObjects",
                    Value::List(vec![Value::Ref(authoring_handle(1))]),
                ),
                ("RelatingType", Value::Ref(authoring_handle(0))),
            ],
        ),
    ];
    let built = apply(&mut model, ifc4(), &ops).expect("builds");
    let task = built.ids[1].expect("task");
    let event = AuthorOp::ReassignClass {
        entity: task,
        type_name: "IfcEvent".into(),
        attributes: vec![("PredefinedType".into(), enumeration("NOTDEFINED"))],
        options: ReassignClassOptions::new().drop_unmapped(true),
    };
    apply(&mut model, ifc4(), &[event]).expect("a typed task is not refused");
    assert_eq!(class(&model, task), "IFCEVENT");
}
