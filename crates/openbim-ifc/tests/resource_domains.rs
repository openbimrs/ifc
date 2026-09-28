//! Facade wiring: construction resource domain re-exports.

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
        ClassificationReferenceDraft::new()
            .location("https://example/requirement")
            .identification("REQ-1"),
    )
    .unwrap();
    let approval = create_approval(
        &mut tx,
        &model,
        ApprovalDraft::new().identifier("APP-1").status("APPROVED"),
    )
    .unwrap();
    let metric = create_metric(
        &mut tx,
        &model,
        MetricDraft::new(
            ConstraintBaseDraft::new("Tolerance", ConstraintGrade::Hard),
            Benchmark::LessThanOrEqualTo,
        ),
    )
    .unwrap();
    let external = create_external_reference_relationship(
        &mut tx,
        &model,
        ExternalReferenceRelationshipDraft::new(reference, &[approval]).name("approval evidence"),
    )
    .unwrap();
    let approved_metric = relate_resource_approval(
        &mut tx,
        &model,
        ResourceApprovalDraft::new(&[metric], approval),
    )
    .unwrap();
    relate_resource_constraint(
        &mut tx,
        &model,
        ResourceConstraintDraft::new(metric, &[approval]),
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

/// `docs/guide/approvals-constraints.md` -- three domains, one graph.
#[test]
fn documented_domains_join_on_one_graph() -> Result<(), Box<dyn std::error::Error>> {
    let (model, [_, approval_id, metric_id, evidence_relationship_id, _]) = author_resource_graph();
    // docs:snippet domains-one-graph
    use ifc::approval::ApprovalView;
    use ifc::classification::ClassificationView;
    use ifc::constraint::ConstraintView;

    let approval = ApprovalView::new(&model).approval(approval_id)?;
    let metric = ConstraintView::new(&model).metric(metric_id)?;
    let evidence = ClassificationView::new(&model)
        .external_reference_relationship(evidence_relationship_id)?;

    assert_eq!(approval.id(), approval_id);
    assert_eq!(metric.id(), metric_id);
    assert_eq!(evidence.related_resources()?, vec![approval_id]);
    // docs:end
    Ok(())
}
