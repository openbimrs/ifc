mod support;

use ifc_model::Transaction;
use ifc_schema::{ifc2x3, ifc4};
use ifc_structural::{
    stage_action, stage_activity_assignment, stage_connection, stage_load, stage_member,
    stage_member_connection, ActionDraft, ActionDraftKind, ActivityAssignmentDraft,
    ConnectionDraft, ConnectionDraftKind, CoordinateSystem, LoadDraft, MemberConnectionDraft,
    MemberDraft, MemberDraftKind, MemberPredefinedType, ProjectedOrTrue, RelationshipRootDraft,
    StructuralError, StructuralRootDraft, StructuralView,
};

use support::{model, named};

fn root(guid: &str) -> StructuralRootDraft {
    StructuralRootDraft {
        global_id: guid.into(),
        ..StructuralRootDraft::default()
    }
}

fn relation_root(guid: &str) -> RelationshipRootDraft {
    RelationshipRootDraft {
        global_id: guid.into(),
        ..RelationshipRootDraft::default()
    }
}

#[test]
fn stages_structural_graph_atomically_and_reads_it_back() {
    let schema = ifc4();
    let mut model = model("IFC4");
    let axis = model.push(named(schema, "IfcDirection", &[]));
    let condition = model.push(named(schema, "IfcBoundaryNodeCondition", &[]));
    let mut tx = Transaction::new(&model);
    let member = stage_member(
        &mut tx,
        &model,
        schema,
        MemberDraft {
            root: root("0YvctVUKbD0xjK5xJ8Jg11"),
            kind: MemberDraftKind::Curve {
                predefined_type: MemberPredefinedType::RigidJoinedMember,
                axis: Some(axis),
            },
        },
    )
    .unwrap();
    let connection = stage_connection(
        &mut tx,
        &model,
        schema,
        ConnectionDraft {
            root: root("0YvctVUKbD0xjK5xJ8Jg12"),
            kind: ConnectionDraftKind::Curve {
                applied_condition: Some(condition),
                axis: Some(axis),
            },
        },
    )
    .unwrap();
    stage_member_connection(
        &mut tx,
        &model,
        schema,
        MemberConnectionDraft {
            root: relation_root("0YvctVUKbD0xjK5xJ8Jg13"),
            member,
            connection,
            applied_condition: Some(condition),
            additional_conditions: None,
            supported_length: Some(2.5),
            condition_coordinate_system: None,
        },
    )
    .unwrap();
    let load = stage_load(
        &mut tx,
        schema,
        LoadDraft::LinearForce {
            name: None,
            force: [Some(1.0), None, None],
            moment: [None; 3],
        },
    )
    .unwrap();
    let action = stage_action(
        &mut tx,
        &model,
        schema,
        ActionDraft {
            root: root("0YvctVUKbD0xjK5xJ8Jg14"),
            applied_load: load,
            coordinate_system: CoordinateSystem::Global,
            destabilizing_load: None,
            caused_by: None,
            kind: ActionDraftKind::Linear {
                projected_or_true: Some(ProjectedOrTrue::TrueLength),
            },
        },
    )
    .unwrap();
    stage_activity_assignment(
        &mut tx,
        &model,
        schema,
        ActivityAssignmentDraft {
            root: relation_root("0YvctVUKbD0xjK5xJ8Jg15"),
            relating_element: member,
            activity: action,
        },
    )
    .unwrap();
    tx.commit(&mut model).unwrap();
    let view = StructuralView::new(&model, schema);
    assert_eq!(view.member(member).unwrap().axis().unwrap(), Some(axis));
    assert_eq!(
        view.connection(connection)
            .unwrap()
            .applied_condition()
            .unwrap(),
        Some(condition)
    );
    assert_eq!(
        view.member_connections(member).unwrap()[0].connection,
        connection
    );
    assert_eq!(view.activities_for(member).unwrap()[0].activity, action);
}

#[test]
fn stages_ifc2x3_legacy_structural_types() {
    let schema = ifc2x3();
    let mut model = model("IFC2X3");
    let owner = model.push(named(schema, "IfcOwnerHistory", &[]));
    let mut common = root("0YvctVUKbD0xjK5xJ8Jg21");
    common.owner_history = Some(owner);
    let mut tx = Transaction::new(&model);
    let member = stage_member(
        &mut tx,
        &model,
        schema,
        MemberDraft {
            root: common,
            kind: MemberDraftKind::Curve {
                predefined_type: MemberPredefinedType::RigidJoinedMember,
                axis: None,
            },
        },
    )
    .unwrap();
    let mut connection_root = root("0YvctVUKbD0xjK5xJ8Jg22");
    connection_root.owner_history = Some(owner);
    let connection = stage_connection(
        &mut tx,
        &model,
        schema,
        ConnectionDraft {
            root: connection_root,
            kind: ConnectionDraftKind::Curve {
                applied_condition: None,
                axis: None,
            },
        },
    )
    .unwrap();
    let load = stage_load(
        &mut tx,
        schema,
        LoadDraft::LinearForce {
            name: None,
            force: [Some(2.0), None, None],
            moment: [None; 3],
        },
    )
    .unwrap();
    let mut action_root = root("0YvctVUKbD0xjK5xJ8Jg23");
    action_root.owner_history = Some(owner);
    let action = stage_action(
        &mut tx,
        &model,
        schema,
        ActionDraft {
            root: action_root,
            applied_load: load,
            coordinate_system: CoordinateSystem::Global,
            destabilizing_load: Some(false),
            caused_by: None,
            kind: ActionDraftKind::Linear {
                projected_or_true: Some(ProjectedOrTrue::ProjectedLength),
            },
        },
    )
    .unwrap();
    tx.commit(&mut model).unwrap();
    let view = StructuralView::new(&model, schema);
    assert_eq!(view.member(member).unwrap().axis().unwrap(), None);
    assert_eq!(view.connection(connection).unwrap().axis().unwrap(), None);
    assert_eq!(view.action(action).unwrap().applied_load().unwrap(), load);
}

#[test]
fn invalid_drafts_do_not_stage_partial_edits() {
    let schema = ifc4();
    let model = model("IFC4");
    let mut tx = Transaction::new(&model);
    let base = tx.len();
    let missing_axis = stage_member(
        &mut tx,
        &model,
        schema,
        MemberDraft {
            root: root("0YvctVUKbD0xjK5xJ8Jg31"),
            kind: MemberDraftKind::Curve {
                predefined_type: MemberPredefinedType::RigidJoinedMember,
                axis: None,
            },
        },
    );
    assert!(matches!(
        missing_axis,
        Err(StructuralError::MissingRequired { .. })
    ));
    assert_eq!(tx.len(), base);
    let shell = stage_member(
        &mut tx,
        &model,
        schema,
        MemberDraft {
            root: root("0YvctVUKbD0xjK5xJ8Jg32"),
            kind: MemberDraftKind::Surface {
                predefined_type: MemberPredefinedType::Shell,
                thickness: None,
            },
        },
    );
    assert!(matches!(
        shell,
        Err(StructuralError::InvalidDraftValue { .. })
    ));
    assert_eq!(tx.len(), base);
}

#[test]
fn projected_removals_and_wrong_relationship_values_are_refused() {
    let schema = ifc4();
    let mut model = model("IFC4");
    let member = model.push(named(schema, "IfcStructuralCurveMember", &[]));
    let connection = model.push(named(schema, "IfcStructuralPointConnection", &[]));
    let mut tx = Transaction::new(&model);
    tx.remove(member);
    let before = tx.len();
    let removed = stage_member_connection(
        &mut tx,
        &model,
        schema,
        MemberConnectionDraft {
            root: relation_root("0YvctVUKbD0xjK5xJ8Jg41"),
            member,
            connection,
            applied_condition: None,
            additional_conditions: None,
            supported_length: None,
            condition_coordinate_system: None,
        },
    );
    assert!(matches!(
        removed,
        Err(StructuralError::DanglingReference { .. })
    ));
    assert_eq!(tx.len(), before);

    let mut tx = Transaction::new(&model);
    let invalid_length = stage_member_connection(
        &mut tx,
        &model,
        schema,
        MemberConnectionDraft {
            root: relation_root("0YvctVUKbD0xjK5xJ8Jg42"),
            member,
            connection,
            applied_condition: None,
            additional_conditions: None,
            supported_length: Some(0.0),
            condition_coordinate_system: None,
        },
    );
    assert!(matches!(
        invalid_length,
        Err(StructuralError::InvalidDraftValue { .. })
    ));
    assert!(tx.is_empty());
}

#[test]
fn an_activity_cannot_be_attached_twice_in_projected_state() {
    let schema = ifc4();
    let mut model = model("IFC4");
    let product = model.push(named(schema, "IfcWall", &[]));
    let activity = model.push(named(schema, "IfcStructuralPointAction", &[]));
    let mut tx = Transaction::new(&model);
    stage_activity_assignment(
        &mut tx,
        &model,
        schema,
        ActivityAssignmentDraft {
            root: relation_root("0YvctVUKbD0xjK5xJ8Jg51"),
            relating_element: product,
            activity,
        },
    )
    .unwrap();
    let before = tx.len();
    let duplicate = stage_activity_assignment(
        &mut tx,
        &model,
        schema,
        ActivityAssignmentDraft {
            root: relation_root("0YvctVUKbD0xjK5xJ8Jg52"),
            relating_element: product,
            activity,
        },
    );
    assert!(matches!(
        duplicate,
        Err(StructuralError::SemanticViolation { .. })
    ));
    assert_eq!(tx.len(), before);
}

#[test]
fn incompatible_action_load_is_refused_before_staging() {
    let schema = ifc4();
    let mut model = model("IFC4");
    let linear = model.push(named(schema, "IfcStructuralLoadLinearForce", &[]));
    let mut tx = Transaction::new(&model);
    let result = stage_action(
        &mut tx,
        &model,
        schema,
        ActionDraft {
            root: root("0YvctVUKbD0xjK5xJ8Jg61"),
            applied_load: linear,
            coordinate_system: CoordinateSystem::Global,
            destabilizing_load: None,
            caused_by: None,
            kind: ActionDraftKind::Point,
        },
    );
    assert!(matches!(
        result,
        Err(StructuralError::WrongReferenceType { .. })
    ));
    assert!(tx.is_empty());
}

#[test]
fn entity_specific_authoring_rules_are_independently_enforced() {
    let schema = ifc4();
    let mut model = model("IFC4");
    let axis = model.push(named(schema, "IfcDirection", &[]));
    let linear = model.push(named(schema, "IfcStructuralLoadLinearForce", &[]));
    let activity = model.push(named(schema, "IfcStructuralPointAction", &[]));

    let mut tx = Transaction::new(&model);
    let user_defined = stage_member(
        &mut tx,
        &model,
        schema,
        MemberDraft {
            root: root("0YvctVUKbD0xjK5xJ8Jg81"),
            kind: MemberDraftKind::Curve {
                predefined_type: MemberPredefinedType::UserDefined,
                axis: Some(axis),
            },
        },
    );
    assert!(matches!(
        user_defined,
        Err(StructuralError::SemanticViolation { .. })
    ));
    assert!(tx.is_empty());

    let mut tx = Transaction::new(&model);
    let wrong_enum = stage_member(
        &mut tx,
        &model,
        schema,
        MemberDraft {
            root: root("0YvctVUKbD0xjK5xJ8Jg82"),
            kind: MemberDraftKind::Curve {
                predefined_type: MemberPredefinedType::Shell,
                axis: Some(axis),
            },
        },
    );
    assert!(matches!(
        wrong_enum,
        Err(StructuralError::InvalidDraftValue {
            attribute: "PredefinedType",
            ..
        })
    ));
    assert!(tx.is_empty());

    let mut tx = Transaction::new(&model);
    let missing_connection_axis = stage_connection(
        &mut tx,
        &model,
        schema,
        ConnectionDraft {
            root: root("0YvctVUKbD0xjK5xJ8Jg83"),
            kind: ConnectionDraftKind::Curve {
                applied_condition: None,
                axis: None,
            },
        },
    );
    assert!(matches!(
        missing_connection_axis,
        Err(StructuralError::MissingRequired { .. })
    ));
    assert!(tx.is_empty());

    let mut tx = Transaction::new(&model);
    let projected_local = stage_action(
        &mut tx,
        &model,
        schema,
        ActionDraft {
            root: root("0YvctVUKbD0xjK5xJ8Jg84"),
            applied_load: linear,
            coordinate_system: CoordinateSystem::Local,
            destabilizing_load: None,
            caused_by: None,
            kind: ActionDraftKind::Linear {
                projected_or_true: Some(ProjectedOrTrue::ProjectedLength),
            },
        },
    );
    assert!(matches!(
        projected_local,
        Err(StructuralError::SemanticViolation { .. })
    ));
    assert!(tx.is_empty());

    let mut tx = Transaction::new(&model);
    let wrong_select = stage_activity_assignment(
        &mut tx,
        &model,
        schema,
        ActivityAssignmentDraft {
            root: relation_root("0YvctVUKbD0xjK5xJ8Jg85"),
            relating_element: axis,
            activity,
        },
    );
    assert!(matches!(
        wrong_select,
        Err(StructuralError::WrongReferenceType { .. })
    ));
    assert!(tx.is_empty());
}

#[test]
fn ifc2x3_linear_action_requires_projected_or_true() {
    let schema = ifc2x3();
    let mut model = model("IFC2X3");
    let owner = model.push(named(schema, "IfcOwnerHistory", &[]));
    let load = model.push(named(schema, "IfcStructuralLoadLinearForce", &[]));
    let mut action_root = root("0YvctVUKbD0xjK5xJ8Jg86");
    action_root.owner_history = Some(owner);
    let mut tx = Transaction::new(&model);
    let result = stage_action(
        &mut tx,
        &model,
        schema,
        ActionDraft {
            root: action_root,
            applied_load: load,
            coordinate_system: CoordinateSystem::Global,
            destabilizing_load: Some(false),
            caused_by: None,
            kind: ActionDraftKind::Linear {
                projected_or_true: None,
            },
        },
    );
    assert!(
        matches!(result, Err(StructuralError::MissingRequired { attribute, .. }) if attribute == "ProjectedOrTrue")
    );
    assert!(tx.is_empty());
}
