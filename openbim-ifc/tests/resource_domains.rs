#![cfg(all(
    feature = "step",
    feature = "classification",
    feature = "approval",
    feature = "constraint"
))]

use ifc::approval::{
    create_approval, relate_resource_approval, ApprovalDraft, ApprovalView, ResourceApprovalDraft,
};
use ifc::classification::{
    create_classification_reference, create_external_reference_relationship,
    ClassificationReferenceDraft, ClassificationView, ExternalReferenceRelationshipDraft,
};
use ifc::constraint::{
    create_metric, relate_resource_constraint, Benchmark, ConstraintBaseDraft, ConstraintGrade,
    ConstraintView, MetricDraft, ResourceConstraintDraft,
};
use ifc::{Codec, Model, StepCodec};
use ifc_model::Transaction;

fn author_resource_graph() -> (Model, [ifc::EntityId; 5]) {
    let mut model = Model::new();
    model.header_mut().schema.push("IFC4".into());
    let mut tx = Transaction::new(&model);
    let reference = create_classification_reference(
        &mut tx,
        &model,
        ClassificationReferenceDraft {
            location: Some("https://example/requirement"),
            identification: Some("REQ-1"),
            name: None,
            referenced_source: None,
            description: None,
            sort: None,
        },
    )
    .unwrap();
    let approval = create_approval(
        &mut tx,
        &model,
        ApprovalDraft {
            identifier: Some("APP-1"),
            status: Some("APPROVED"),
            ..Default::default()
        },
    )
    .unwrap();
    let metric = create_metric(
        &mut tx,
        &model,
        MetricDraft {
            base: ConstraintBaseDraft {
                name: "Tolerance",
                description: None,
                grade: ConstraintGrade::Hard,
                source: None,
                creating_actor: None,
                creation_time: None,
                user_defined_grade: None,
            },
            benchmark: Benchmark::LessThanOrEqualTo,
            value_source: None,
            data_value: None,
            reference_path: None,
        },
    )
    .unwrap();
    let external = create_external_reference_relationship(
        &mut tx,
        &model,
        ExternalReferenceRelationshipDraft {
            name: Some("approval evidence"),
            description: None,
            relating_reference: reference,
            related_resources: &[approval],
        },
    )
    .unwrap();
    let approved_metric = relate_resource_approval(
        &mut tx,
        &model,
        ResourceApprovalDraft {
            name: None,
            description: None,
            related_resources: &[metric],
            relating_approval: approval,
        },
    )
    .unwrap();
    relate_resource_constraint(
        &mut tx,
        &model,
        ResourceConstraintDraft {
            name: None,
            description: None,
            relating_constraint: metric,
            related_resources: &[approval],
        },
    )
    .unwrap();
    tx.commit(&mut model).unwrap();
    (
        model,
        [reference, approval, metric, external, approved_metric],
    )
}

fn assert_joined(model: &Model, ids: [ifc::EntityId; 5]) {
    let [reference, approval, metric, external, approved_metric] = ids;
    assert_eq!(
        ApprovalView::new(model).approval(approval).unwrap().id(),
        approval
    );
    assert_eq!(
        ConstraintView::new(model).metric(metric).unwrap().id(),
        metric
    );
    let external = ClassificationView::new(model)
        .external_reference_relationship(external)
        .unwrap();
    assert_eq!(external.relating_reference().unwrap(), reference);
    assert_eq!(external.related_resources().unwrap(), [approval]);
    let approval_relation = ApprovalView::new(model)
        .resource_approval_relationship(approved_metric)
        .unwrap();
    assert_eq!(approval_relation.relating_approval().unwrap(), approval);
    assert_eq!(approval_relation.related_resources().unwrap(), [metric]);
    assert_eq!(
        ConstraintView::new(model)
            .resources_constrained_by(metric)
            .unwrap(),
        [approval]
    );
}

#[test]
fn resource_domains_join_by_entity_id_before_and_after_step_round_trip() {
    let (model, ids) = author_resource_graph();
    assert_joined(&model, ids);
    let bytes = StepCodec.write_bytes(&model).unwrap();
    let decoded = StepCodec.read_bytes(&bytes).unwrap();
    assert_joined(&decoded, ids);
}
