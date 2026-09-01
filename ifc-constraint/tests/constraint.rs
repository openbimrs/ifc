use std::sync::Arc;

use ifc_constraint::{
    associate_constraint, create_metric, create_objective, relate_resource_constraint, Benchmark,
    ConstraintAssociationDraft, ConstraintBaseDraft, ConstraintError, ConstraintGrade,
    ConstraintView, LogicalOperator, MetricDraft, MetricValue, MetricValueDraft, ObjectiveDraft,
    ObjectiveQualifier, ResourceConstraintDraft,
};
use ifc_model::{Entity, Model, Transaction, Value};

fn text(value: &str) -> Value {
    Value::Text(value.into())
}
fn gid(seed: u8) -> String {
    ifc_model::guid::Guid::from_uuid([seed; 16]).to_string()
}

fn base<'a>(name: &'a str) -> ConstraintBaseDraft<'a> {
    ConstraintBaseDraft {
        name,
        description: None,
        grade: ConstraintGrade::Hard,
        source: None,
        creating_actor: None,
        creation_time: None,
        user_defined_grade: None,
    }
}

#[test]
fn bundled_schema_pins_all_owned_layouts() {
    let schema = ifc_schema::ifc4();
    assert_eq!(
        schema.attribute_names("IFCMETRIC"),
        [
            "Name",
            "Description",
            "ConstraintGrade",
            "ConstraintSource",
            "CreatingActor",
            "CreationTime",
            "UserDefinedGrade",
            "Benchmark",
            "ValueSource",
            "DataValue",
            "ReferencePath"
        ]
    );
    assert_eq!(
        schema.attribute_names("IFCOBJECTIVE"),
        [
            "Name",
            "Description",
            "ConstraintGrade",
            "ConstraintSource",
            "CreatingActor",
            "CreationTime",
            "UserDefinedGrade",
            "BenchmarkValues",
            "LogicalAggregator",
            "ObjectiveQualifier",
            "UserDefinedQualifier"
        ]
    );
    assert_eq!(
        schema.attribute_names("IFCRESOURCECONSTRAINTRELATIONSHIP"),
        [
            "Name",
            "Description",
            "RelatingConstraint",
            "RelatedResourceObjects"
        ]
    );
    assert_eq!(
        schema.attribute_names("IFCRELASSOCIATESCONSTRAINT"),
        [
            "GlobalId",
            "OwnerHistory",
            "Name",
            "Description",
            "RelatedObjects",
            "Intent",
            "RelatingConstraint"
        ]
    );
}

#[test]
fn stages_typed_metrics_objectives_and_both_relationship_families() {
    let mut model = Model::new();
    let actor = model.push(Entity::new("IFCPERSON", vec![]));
    let wall = model.push(Entity::new("IFCWALL", vec![]));
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
    let scalar = Value::Real(0.01);
    let mut tx = Transaction::new(&model);
    let metric = create_metric(
        &mut tx,
        &model,
        MetricDraft {
            base: ConstraintBaseDraft {
                creating_actor: Some(actor),
                ..base("Tolerance")
            },
            benchmark: Benchmark::LessThanOrEqualTo,
            value_source: Some("design"),
            data_value: Some(MetricValueDraft::Typed {
                type_name: "IfcLengthMeasure",
                value: &scalar,
            }),
            reference_path: None,
        },
    )
    .unwrap();
    let objective = create_objective(
        &mut tx,
        &model,
        ObjectiveDraft {
            base: base("Envelope"),
            benchmark_values: Some(&[metric]),
            logical_aggregator: Some(LogicalOperator::LogicalAnd),
            qualifier: ObjectiveQualifier::Requirement,
            user_defined_qualifier: None,
        },
    )
    .unwrap();
    let resource_relation = relate_resource_constraint(
        &mut tx,
        &model,
        ResourceConstraintDraft {
            name: None,
            description: None,
            relating_constraint: objective,
            related_resources: &[approval],
        },
    )
    .unwrap();
    let association = associate_constraint(
        &mut tx,
        &model,
        ConstraintAssociationDraft {
            global_id: &gid(4),
            name: None,
            description: None,
            related_objects: &[wall],
            intent: Some("design requirement"),
            relating_constraint: objective,
        },
    )
    .unwrap();
    tx.commit(&mut model).unwrap();

    let view = ConstraintView::new(&model);
    let metric_view = view.metric(metric).unwrap();
    assert_eq!(
        metric_view.benchmark().unwrap(),
        Benchmark::LessThanOrEqualTo
    );
    match metric_view.data_value().unwrap().unwrap() {
        MetricValue::Typed { type_name, value } => {
            assert_eq!(type_name, "IFCLENGTHMEASURE");
            assert_eq!(value, &Value::Real(0.01));
        }
        other => panic!("unexpected metric value: {other:?}"),
    }
    assert_eq!(
        view.objective(objective)
            .unwrap()
            .benchmark_values()
            .unwrap(),
        Some(vec![metric])
    );
    assert_eq!(
        view.resources_constrained_by(objective).unwrap(),
        [approval]
    );
    assert_eq!(view.objects_constrained_by(objective).unwrap(), [wall]);
    assert_eq!(
        view.resource_constraint_relationship(resource_relation)
            .unwrap()
            .relating_constraint()
            .unwrap(),
        objective
    );
    assert_eq!(
        view.constraint_assignment(association)
            .unwrap()
            .intent()
            .unwrap(),
        Some("design requirement")
    );
}

#[test]
fn entity_metric_values_are_preserved_without_evaluation() {
    let mut model = Model::new();
    let table = model.push(Entity::new(
        "IFCTABLE",
        vec![Value::Null, Value::Null, Value::Null],
    ));
    let metric = model.push(Entity::new(
        "IFCMETRIC",
        vec![
            text("Lookup"),
            Value::Null,
            Value::Enum("SOFT".into()),
            Value::Null,
            Value::Null,
            Value::Null,
            Value::Null,
            Value::Enum("INCLUDES".into()),
            Value::Null,
            Value::Ref(table),
            Value::Null,
        ],
    ));
    assert_eq!(
        ConstraintView::new(&model)
            .metric(metric)
            .unwrap()
            .data_value()
            .unwrap(),
        Some(MetricValue::Entity(table))
    );
}

#[test]
fn where_rules_selects_and_draft_atomicity_fail_closed() {
    let mut model = Model::new();
    let wall = model.push(Entity::new("IFCWALL", vec![]));
    let invalid = model.push(Entity::new(
        "IFCOBJECTIVE",
        vec![
            text("Custom"),
            Value::Null,
            Value::Enum("USERDEFINED".into()),
            Value::Null,
            Value::Null,
            Value::Null,
            Value::Null,
            Value::Null,
            Value::Null,
            Value::Enum("USERDEFINED".into()),
            Value::Null,
        ],
    ));
    assert!(matches!(
        ConstraintView::new(&model).objective(invalid),
        Err(ConstraintError::Semantic { rule: "WR11", .. })
            | Err(ConstraintError::Semantic { rule: "WR21", .. })
    ));

    let mut tx = Transaction::new(&model);
    let before = tx.len();
    assert!(matches!(
        create_metric(
            &mut tx,
            &model,
            MetricDraft {
                base: ConstraintBaseDraft {
                    grade: ConstraintGrade::UserDefined,
                    ..base("Custom")
                },
                benchmark: Benchmark::EqualTo,
                value_source: None,
                data_value: None,
                reference_path: None,
            }
        ),
        Err(ConstraintError::AuthoringInvalid {
            attribute: "WR11",
            ..
        })
    ));
    assert_eq!(tx.len(), before);
    assert!(matches!(
        create_objective(
            &mut tx,
            &model,
            ObjectiveDraft {
                base: base("Custom purpose"),
                benchmark_values: None,
                logical_aggregator: None,
                qualifier: ObjectiveQualifier::UserDefined,
                user_defined_qualifier: None,
            }
        ),
        Err(ConstraintError::AuthoringInvalid {
            attribute: "WR21",
            ..
        })
    ));
    assert_eq!(tx.len(), before);
    assert!(matches!(
        create_metric(
            &mut tx,
            &model,
            MetricDraft {
                base: base("Wrong"),
                benchmark: Benchmark::EqualTo,
                value_source: None,
                data_value: Some(MetricValueDraft::Entity(wall)),
                reference_path: None,
            }
        ),
        Err(ConstraintError::AuthoringReferenceType { target, .. }) if target == wall
    ));
    assert_eq!(tx.len(), before);

    let untyped = model.push(Entity::new(
        "IFCMETRIC",
        vec![
            text("Bad"),
            Value::Null,
            Value::Enum("HARD".into()),
            Value::Null,
            Value::Null,
            Value::Null,
            Value::Null,
            Value::Enum("EQUALTO".into()),
            Value::Null,
            Value::Real(1.0),
            Value::Null,
        ],
    ));
    assert!(matches!(
        ConstraintView::new(&model).metric(untyped),
        Err(ConstraintError::InvalidValue {
            attribute: "DataValue",
            ..
        })
    ));
}

#[test]
fn typed_metric_drafts_reject_types_outside_metric_value_select() {
    let model = Model::new();
    let scalar = Value::Text(Arc::from("not a metric value"));
    let mut tx = Transaction::new(&model);
    assert!(matches!(
        create_metric(
            &mut tx,
            &model,
            MetricDraft {
                base: base("Bad typed"),
                benchmark: Benchmark::EqualTo,
                value_source: None,
                data_value: Some(MetricValueDraft::Typed {
                    type_name: "IfcObjectiveEnum",
                    value: &scalar,
                }),
                reference_path: None,
            }
        ),
        Err(ConstraintError::AuthoringInvalid {
            attribute: "DataValue",
            ..
        })
    ));
    assert_eq!(tx.len(), 0);
}
