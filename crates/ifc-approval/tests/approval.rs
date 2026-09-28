//! Approval graph staging, querying, and fail-closed validation.

use ifc_approval::{
    associate_approval, create_approval, relate_approvals, relate_resource_approval,
    ApprovalAssociationDraft, ApprovalDraft, ApprovalError, ApprovalRelationshipDraft,
    ApprovalView, ResourceApprovalDraft,
};
use ifc_model::{Entity, EntityId, Model, Transaction, Value};

fn text(value: &str) -> Value {
    Value::Text(value.into())
}
fn refs(ids: &[EntityId]) -> Value {
    Value::List(ids.iter().copied().map(Value::Ref).collect())
}
fn gid(seed: u8) -> String {
    ifc_model::guid::Guid::from_uuid([seed; 16]).to_string()
}

#[test]
fn bundled_schema_pins_every_owned_ifc4_layout() {
    let schema = ifc_schema::ifc4();
    assert_eq!(
        schema.attribute_names("IFCAPPROVAL"),
        [
            "Identifier",
            "Name",
            "Description",
            "TimeOfApproval",
            "Status",
            "Level",
            "Qualifier",
            "RequestingApproval",
            "GivingApproval"
        ]
    );
    assert_eq!(
        schema.attribute_names("IFCAPPROVALRELATIONSHIP"),
        [
            "Name",
            "Description",
            "RelatingApproval",
            "RelatedApprovals"
        ]
    );
    assert_eq!(
        schema.attribute_names("IFCRESOURCEAPPROVALRELATIONSHIP"),
        [
            "Name",
            "Description",
            "RelatedResourceObjects",
            "RelatingApproval"
        ]
    );
    assert_eq!(
        schema.attribute_names("IFCRELASSOCIATESAPPROVAL"),
        [
            "GlobalId",
            "OwnerHistory",
            "Name",
            "Description",
            "RelatedObjects",
            "RelatingApproval"
        ]
    );
}

#[test]
fn stages_and_queries_the_complete_bounded_approval_graph() {
    let mut model = Model::new();
    let actor = model.push(Entity::new("IFCPERSON", vec![]));
    let wall = model.push(Entity::new("IFCWALL", vec![]));
    let metric = model.push(Entity::new(
        "IFCMETRIC",
        vec![
            text("Tolerance"),
            Value::Null,
            Value::Enum("HARD".into()),
            Value::Null,
            Value::Null,
            Value::Null,
            Value::Null,
            Value::Enum("EQUALTO".into()),
            Value::Null,
            Value::Null,
            Value::Null,
        ],
    ));
    let mut tx = Transaction::new(&model);
    let requested = create_approval(
        &mut tx,
        &model,
        ApprovalDraft::new()
            .identifier("REQ-1")
            .description("Requested")
            .status("PENDING")
            .requesting_approval(actor),
    )
    .unwrap();
    let approved = create_approval(
        &mut tx,
        &model,
        ApprovalDraft::new()
            .name("Accepted")
            .time_of_approval("2026-09-01T10:00:00")
            .status("APPROVED")
            .level("PROJECT")
            .giving_approval(actor),
    )
    .unwrap();
    let relation = relate_approvals(
        &mut tx,
        &model,
        ApprovalRelationshipDraft::new(approved, &[requested]).name("supersedes"),
    )
    .unwrap();
    let resource_relation = relate_resource_approval(
        &mut tx,
        &model,
        ResourceApprovalDraft::new(&[metric], approved),
    )
    .unwrap();
    let assignment = associate_approval(
        &mut tx,
        &model,
        ApprovalAssociationDraft::new(&gid(1), &[wall], approved),
    )
    .unwrap();
    tx.commit(&mut model).unwrap();

    let view = ApprovalView::new(&model);
    let projected = view.approval(approved).unwrap();
    assert_eq!(projected.name().unwrap(), Some("Accepted"));
    assert_eq!(projected.giving_approval().unwrap(), Some(actor));
    assert_eq!(
        view.approval_relationship(relation)
            .unwrap()
            .related_approvals()
            .unwrap(),
        [requested]
    );
    assert_eq!(view.resources_approved_by(approved).unwrap(), [metric]);
    assert_eq!(view.objects_approved_by(approved).unwrap(), [wall]);
    assert_eq!(
        view.resource_approval_relationship(resource_relation)
            .unwrap()
            .relating_approval()
            .unwrap(),
        approved
    );
    assert_eq!(
        view.approval_assignment(assignment)
            .unwrap()
            .global_id()
            .unwrap(),
        gid(1)
    );
}

#[test]
fn malformed_views_and_invalid_drafts_fail_closed_before_staging() {
    let mut model = Model::new();
    let wall = model.push(Entity::new("IFCWALL", vec![]));
    let empty = model.push(Entity::new("IFCAPPROVAL", vec![Value::Null; 9]));
    assert!(matches!(
        ApprovalView::new(&model).approval(empty),
        Err(ApprovalError::Semantic {
            rule: "HasIdentifierOrName",
            ..
        })
    ));

    let wrong_actor = model.push(Entity::new(
        "IFCAPPROVAL",
        vec![
            text("A"),
            Value::Null,
            Value::Null,
            Value::Null,
            Value::Null,
            Value::Null,
            Value::Null,
            Value::Ref(wall),
            Value::Null,
        ],
    ));
    assert!(matches!(
        ApprovalView::new(&model).approval(wrong_actor),
        Err(ApprovalError::ReferenceType { target, .. }) if target == wall
    ));

    let mut tx = Transaction::new(&model);
    let before = tx.len();
    assert!(matches!(
        create_approval(&mut tx, &model, ApprovalDraft::default()),
        Err(ApprovalError::AuthoringInvalid {
            attribute: "WR",
            ..
        })
    ));
    assert_eq!(tx.len(), before);
    assert!(matches!(
        create_approval(
            &mut tx,
            &model,
            ApprovalDraft::new().identifier("A").requesting_approval(wall)
        ),
        Err(ApprovalError::AuthoringReferenceType { target, .. }) if target == wall
    ));
    assert_eq!(tx.len(), before);
}

#[test]
fn relationship_sets_refuse_empty_duplicates_self_and_wrong_select_members() {
    let mut model = Model::new();
    let approval = model.push(Entity::new(
        "IFCAPPROVAL",
        vec![
            text("A"),
            Value::Null,
            Value::Null,
            Value::Null,
            Value::Null,
            Value::Null,
            Value::Null,
            Value::Null,
            Value::Null,
        ],
    ));
    let other_entity = model.get(approval).unwrap().clone();
    let other = model.push(other_entity);
    let wall = model.push(Entity::new("IFCWALL", vec![]));
    let mut tx = Transaction::new(&model);
    for related in [&[][..], &[other, other][..], &[approval][..]] {
        let result = relate_approvals(
            &mut tx,
            &model,
            ApprovalRelationshipDraft::new(approval, related),
        );
        assert!(result.is_err());
        assert_eq!(tx.len(), 0);
    }
    assert!(matches!(
        relate_resource_approval(
            &mut tx,
            &model,
            ResourceApprovalDraft::new(&[wall], approval)
        ),
        Err(ApprovalError::AuthoringReferenceType { target, .. }) if target == wall
    ));
    assert_eq!(tx.len(), 0);

    let malformed = model.push(Entity::new(
        "IFCAPPROVALRELATIONSHIP",
        vec![
            Value::Null,
            Value::Null,
            Value::Ref(approval),
            refs(&[approval, approval]),
        ],
    ));
    assert!(matches!(
        ApprovalView::new(&model).approval_relationship(malformed),
        Err(ApprovalError::InvalidValue {
            attribute: "RelatedApprovals",
            ..
        })
    ));
}
