//! #212: metrics and objectives are read and written by name in the
//! declared release.
//!
//! From the EXPRESS sources: IFC2X3 TC1 declares `IfcMetric` with ten
//! attributes (`DataValue` required, no `ReferencePath`, `CreationTime` an
//! `IfcDateTimeSelect`) and `IfcObjective` with a single `IfcMetric` as
//! `BenchmarkValues` and `ResultValues` where IFC4 has `LogicalAggregator`;
//! it declares no `IfcResourceConstraintRelationship` or `IfcReference`.
//! IFC4 ADD2 TC1 and IFC4X3 ADD2 declare eleven and eleven. Each release is
//! authored, written as STEP, read back and re-read through the view.

use ifc_constraint::{
    associate_constraint_with_owner_history, create_metric, create_objective, create_reference,
    relate_resource_constraint, Benchmark, ConstraintAssociationDraft, ConstraintBaseDraft,
    ConstraintError, ConstraintGrade, ConstraintView, LogicalOperator, Metric, MetricDraft,
    MetricValue, MetricValueDraft, ObjectiveDraft, ObjectiveQualifier, ReferenceDraft,
    ResourceConstraintDraft, SchemaVersion,
};
use ifc_model::{Codec, Entity, EntityId, Model, Transaction, Value};
use ifc_step::StepCodec;

const OWNER: EntityId = EntityId(5);
const WALL: EntityId = EntityId(10);
const DATE: EntityId = EntityId(21);
const G1: &str = "0YvctVUKr0kugbFTf53O08";

/// `#5` an owner history, `#10` a wall and, in IFC2X3, `#21` a calendar
/// date, all in the layout of `schema`.
fn model(schema: &str) -> Model {
    let wall = if schema == "IFC2X3" {
        ",$,$,$,$,$"
    } else {
        ",$,$,$,$,$,$"
    };
    let date = if schema == "IFC2X3" {
        "#21=IFCCALENDARDATE(28,9,2026);\n"
    } else {
        ""
    };
    let text = format!(
        "ISO-10303-21;\nHEADER;\nFILE_DESCRIPTION((''),'2;1');\n\
         FILE_NAME('','',(''),(''),'','','');\nFILE_SCHEMA(('{schema}'));\nENDSEC;\nDATA;\n\
         #1=IFCPERSON($,'Doe','Jane',$,$,$,$,$);\n\
         #2=IFCORGANIZATION($,'Acme',$,$,$);\n\
         #3=IFCPERSONANDORGANIZATION(#1,#2,$);\n\
         #4=IFCAPPLICATION(#2,'1.0','Test','test');\n\
         #5=IFCOWNERHISTORY(#3,#4,$,.NOCHANGE.,$,$,$,1700000000);\n\
         #10=IFCWALL('1xS3BCk291UvhgP2dvNsgp',#5,'W1'{wall});\n\
         {date}ENDSEC;\nEND-ISO-10303-21;\n"
    );
    let model = StepCodec.read_bytes(text.as_bytes()).expect("parses");
    assert!(model.diagnostics().is_empty(), "{:?}", model.diagnostics());
    model
}

fn round_trip(model: &Model) -> Model {
    let bytes = StepCodec.write_bytes(model).expect("written");
    let back = StepCodec.read_bytes(&bytes).expect("read back");
    assert!(back.diagnostics().is_empty(), "{:?}", back.diagnostics());
    back
}

fn text(s: &str) -> Value {
    Value::Text(s.into())
}

fn base(version: SchemaVersion, name: &'static str) -> ConstraintBaseDraft<'static> {
    let draft = ConstraintBaseDraft::new(name, ConstraintGrade::Hard)
        .description("Fire rating")
        .source("Code")
        .creating_actor(EntityId(3));
    if version == SchemaVersion::Ifc2x3 {
        draft.creation_time(DATE)
    } else {
        draft.creation_time("2026-09-28T10:00:00")
    }
}

#[test]
fn every_release_round_trips_through_the_view() {
    let data = Value::Text("REI 90".into());
    for (schema, version) in [
        ("IFC2X3", SchemaVersion::Ifc2x3),
        ("IFC4", SchemaVersion::Ifc4),
        ("IFC4X3_ADD2", SchemaVersion::Ifc4x3),
    ] {
        let ifc2x3 = version == SchemaVersion::Ifc2x3;
        let mut model = model(schema);
        let m = model.clone();
        let mut tx = Transaction::new(&model);
        let metric = create_metric(
            &mut tx,
            &m,
            MetricDraft::new(base(version, "Rating"), Benchmark::EqualTo)
                .value_source("Test")
                .data_value(MetricValueDraft::Typed {
                    type_name: "IfcText",
                    value: &data,
                }),
        )
        .expect("metric");
        let benchmarks = [metric];
        let mut objective =
            ObjectiveDraft::new(base(version, "Objective"), ObjectiveQualifier::DesignIntent)
                .benchmark_values(&benchmarks);
        if !ifc2x3 {
            objective = objective.logical_aggregator(LogicalOperator::LogicalAnd);
        }
        let objective = create_objective(&mut tx, &m, objective).expect("objective");
        let objects = [WALL];
        let association = associate_constraint_with_owner_history(
            &mut tx,
            &m,
            ConstraintAssociationDraft::new(G1, &objects, metric).intent("Design"),
            OWNER,
        )
        .expect("association");
        tx.commit(&mut model).expect("commit");
        let back = round_trip(&model);
        let record = |id: EntityId| back.get(id).expect("record").attributes.clone();
        let created = if ifc2x3 {
            Value::Ref(DATE)
        } else {
            text("2026-09-28T10:00:00")
        };
        let mut expected = vec![
            text("Rating"),
            text("Fire rating"),
            Value::Enum("HARD".into()),
            text("Code"),
            Value::Ref(EntityId(3)),
            created.clone(),
            Value::Null,
            Value::Enum("EQUALTO".into()),
            text("Test"),
            Value::Typed {
                type_name: "IFCTEXT".into(),
                value: Box::new(data.clone()),
            },
        ];
        if !ifc2x3 {
            expected.push(Value::Null); // ReferencePath
        }
        assert_eq!(record(metric), expected, "{schema}");
        let objective_record = record(objective);
        assert_eq!(objective_record.len(), 11, "{schema}");
        if ifc2x3 {
            // BenchmarkValues: one IfcMetric; ResultValues unset.
            assert_eq!(objective_record[7], Value::Ref(metric));
            assert_eq!(objective_record[8], Value::Null);
        } else {
            assert_eq!(objective_record[7], Value::List(vec![Value::Ref(metric)]));
            assert_eq!(objective_record[8], Value::Enum("LOGICALAND".into()));
        }
        let view = ConstraintView::new(&back);
        let read = view.metric(metric).expect("metric");
        assert_eq!(read.release(), version);
        assert_eq!(read.name().unwrap(), "Rating");
        assert_eq!(read.grade().unwrap(), ConstraintGrade::Hard);
        assert_eq!(read.benchmark().unwrap(), Benchmark::EqualTo);
        assert_eq!(read.value_source().unwrap(), Some("Test"));
        assert!(matches!(
            read.data_value().unwrap(),
            Some(MetricValue::Typed {
                type_name: "IFCTEXT",
                ..
            })
        ));
        let read_objective = view.objective(objective).expect("objective");
        assert_eq!(
            read_objective.benchmark_values().unwrap(),
            Some(vec![metric])
        );
        assert_eq!(
            read_objective.qualifier().unwrap(),
            ObjectiveQualifier::DesignIntent
        );
        if ifc2x3 {
            assert_eq!(
                read.creation_time(),
                Err(ConstraintError::StructuredValue {
                    entity: "IFCMETRIC",
                    id: metric,
                    attribute: "CreationTime",
                    target: DATE,
                })
            );
            assert_eq!(
                read.reference_path(),
                Err(ConstraintError::NotInSchema {
                    entity: "IFCMETRIC",
                    id: metric,
                    attribute: "ReferencePath",
                    schema: version,
                })
            );
            assert!(matches!(
                read_objective.logical_aggregator(),
                Err(ConstraintError::NotInSchema {
                    attribute: "LogicalAggregator",
                    ..
                })
            ));
        } else {
            assert_eq!(read.creation_time().unwrap(), Some("2026-09-28T10:00:00"));
            assert_eq!(read.reference_path().unwrap(), None);
            assert_eq!(
                read_objective.logical_aggregator().unwrap(),
                Some(LogicalOperator::LogicalAnd)
            );
        }
        assert_eq!(
            view.constraint_assignment(association)
                .unwrap()
                .intent()
                .unwrap(),
            Some("Design")
        );
        assert_eq!(view.objects_constrained_by(metric).unwrap(), vec![WALL]);
        assert!(view.resources_constrained_by(metric).unwrap().is_empty());
    }
}

fn refused(
    model: &Model,
    author: impl Fn(&mut Transaction) -> Result<EntityId, ConstraintError>,
) -> ConstraintError {
    let mut tx = Transaction::new(model);
    let error = author(&mut tx).expect_err("refused");
    assert!(tx.is_empty(), "a refusal staged {:?}", tx.edits());
    error
}

#[test]
fn ifc2x3_refuses_what_it_cannot_hold() {
    let v = SchemaVersion::Ifc2x3;
    let model = model("IFC2X3");
    let data = Value::Text("x".into());
    let metric = |draft: ConstraintBaseDraft<'static>| {
        MetricDraft::new(draft, Benchmark::EqualTo).data_value(MetricValueDraft::Typed {
            type_name: "IfcText",
            value: Box::leak(Box::new(data.clone())),
        })
    };
    assert_eq!(
        refused(&model, |tx| create_metric(
            tx,
            &model,
            MetricDraft::new(base(v, "M"), Benchmark::EqualTo)
        )),
        ConstraintError::AuthoringRequired {
            entity: "IFCMETRIC",
            attribute: "DataValue",
            schema: v
        }
    );
    assert!(matches!(
        refused(&model, |tx| create_metric(
            tx,
            &model,
            metric(base(v, "M").creation_time("2026-09-28T10:00:00"))
        )),
        ConstraintError::AuthoringValueType {
            attribute: "CreationTime",
            ..
        }
    ));
    assert_eq!(
        refused(&model, |tx| create_metric(
            tx,
            &model,
            metric(base(v, "M")).reference_path(EntityId(3))
        )),
        ConstraintError::AuthoringNotInSchema {
            entity: "IFCMETRIC",
            attribute: "ReferencePath",
            schema: v
        }
    );
    // IFC4 added INCLUDES to IfcBenchmarkEnum.
    assert!(matches!(
        refused(&model, |tx| create_metric(
            tx,
            &model,
            MetricDraft::new(base(v, "M"), Benchmark::Includes).data_value(
                MetricValueDraft::Typed {
                    type_name: "IfcText",
                    value: Box::leak(Box::new(data.clone())),
                }
            )
        )),
        ConstraintError::AuthoringValueType {
            attribute: "Benchmark",
            ..
        }
    ));
    let mut staged = Transaction::new(&model);
    let a = create_metric(&mut staged, &model, metric(base(v, "A"))).unwrap();
    let b = create_metric(&mut staged, &model, metric(base(v, "B"))).unwrap();
    let before = staged.edits().len();
    let two = [a, b];
    assert!(matches!(
        create_objective(
            &mut staged,
            &model,
            ObjectiveDraft::new(base(v, "O"), ObjectiveQualifier::DesignIntent)
                .benchmark_values(&two)
        ),
        Err(ConstraintError::AuthoringValueType {
            attribute: "BenchmarkValues",
            ..
        })
    ));
    let one = [a];
    assert_eq!(
        create_objective(
            &mut staged,
            &model,
            ObjectiveDraft::new(base(v, "O"), ObjectiveQualifier::DesignIntent)
                .benchmark_values(&one)
                .logical_aggregator(LogicalOperator::LogicalAnd)
        ),
        Err(ConstraintError::AuthoringNotInSchema {
            entity: "IFCOBJECTIVE",
            attribute: "LogicalAggregator",
            schema: v
        })
    );
    // IFC4 added MODELVIEW to IfcObjectiveEnum.
    assert!(matches!(
        create_objective(
            &mut staged,
            &model,
            ObjectiveDraft::new(base(v, "O"), ObjectiveQualifier::ModelView)
        ),
        Err(ConstraintError::AuthoringValueType {
            attribute: "ObjectiveQualifier",
            ..
        })
    ));
    let resources = [EntityId(2)];
    assert_eq!(
        relate_resource_constraint(
            &mut staged,
            &model,
            ResourceConstraintDraft::new(a, &resources)
        ),
        Err(ConstraintError::EntityNotInSchema {
            entity: "IFCRESOURCECONSTRAINTRELATIONSHIP",
            schema: v
        })
    );
    assert_eq!(
        create_reference(
            &mut staged,
            &model,
            ReferenceDraft::new().attribute_identifier("Name")
        ),
        Err(ConstraintError::EntityNotInSchema {
            entity: "IFCREFERENCE",
            schema: v
        })
    );
    assert_eq!(staged.edits().len(), before, "nothing staged");
}

/// IFC4 and IFC4X3 records are the positional ones the writers staged
/// before binding, slot for slot.
#[test]
fn ifc4_and_ifc4x3_output_is_unchanged() {
    for schema in ["IFC4", "IFC4X3_ADD2"] {
        let model = model(schema);
        let mut tx = Transaction::new(&model);
        let reference = create_reference(
            &mut tx,
            &model,
            ReferenceDraft::new().attribute_identifier("Name"),
        )
        .unwrap();
        let metric = create_metric(
            &mut tx,
            &model,
            MetricDraft::new(base(SchemaVersion::Ifc4, "M"), Benchmark::Includes)
                .reference_path(reference),
        )
        .unwrap();
        let benchmarks = [metric];
        let objective = create_objective(
            &mut tx,
            &model,
            ObjectiveDraft::new(
                base(SchemaVersion::Ifc4, "O"),
                ObjectiveQualifier::ModelView,
            )
            .benchmark_values(&benchmarks)
            .logical_aggregator(LogicalOperator::LogicalOr),
        )
        .unwrap();
        let resources = [EntityId(2)];
        let relationship = relate_resource_constraint(
            &mut tx,
            &model,
            ResourceConstraintDraft::new(metric, &resources).name("R"),
        )
        .unwrap();
        let staged = |id: EntityId| {
            tx.edits()
                .iter()
                .find_map(|edit| match edit {
                    ifc_model::Edit::Create { id: e, entity } if *e == id => {
                        Some(entity.attributes.clone())
                    }
                    _ => None,
                })
                .unwrap()
        };
        let head = |name: &str| {
            vec![
                text(name),
                text("Fire rating"),
                Value::Enum("HARD".into()),
                text("Code"),
                Value::Ref(EntityId(3)),
                text("2026-09-28T10:00:00"),
                Value::Null,
            ]
        };
        let mut expected = head("M");
        expected.extend([
            Value::Enum("INCLUDES".into()),
            Value::Null,
            Value::Null,
            Value::Ref(reference),
        ]);
        assert_eq!(staged(metric), expected, "{schema}");
        let mut expected = head("O");
        expected.extend([
            Value::List(vec![Value::Ref(metric)]),
            Value::Enum("LOGICALOR".into()),
            Value::Enum("MODELVIEW".into()),
            Value::Null,
        ]);
        assert_eq!(staged(objective), expected, "{schema}");
        assert_eq!(
            staged(relationship),
            vec![
                text("R"),
                Value::Null,
                Value::Ref(metric),
                Value::List(vec![Value::Ref(EntityId(2))])
            ]
        );
    }
}

/// Unverified or ambiguous headers are refused by the writers and the view,
/// never read or written as IFC4.
#[test]
fn unverified_releases_are_refused() {
    for schema in ["IFC4X1", "IFC4X2", "IFC5"] {
        let model = model(schema);
        let unsupported = ConstraintError::UnsupportedSchema {
            schema: schema.into(),
        };
        assert_eq!(
            refused(&model, |tx| create_metric(
                tx,
                &model,
                MetricDraft::new(
                    ConstraintBaseDraft::new("M", ConstraintGrade::Hard),
                    Benchmark::EqualTo
                )
            )),
            unsupported
        );
        let mut staged = model.clone();
        let mut tx = Transaction::new(&model);
        let id = tx.create(Entity::new("IFCMETRIC", vec![Value::Null; 11]));
        tx.commit(&mut staged).unwrap();
        assert_eq!(
            ConstraintView::new(&staged).metric(id).err(),
            Some(unsupported)
        );
    }
    let entity = Entity::new("IFCMETRIC", vec![Value::Null; 11]);
    for version in [SchemaVersion::Ifc4x1, SchemaVersion::Ifc4x2] {
        assert!(matches!(
            Metric::try_new(EntityId(1), &entity, version),
            Err(ConstraintError::UnsupportedSchema { .. })
        ));
    }
}

/// The binding is what makes the answers right: an IFC2X3 objective's
/// `ResultValues` metric read through the IFC4 layout is a malformed
/// logical aggregator.
#[test]
fn reading_an_ifc2x3_record_as_ifc4_would_misread_it() {
    let v = SchemaVersion::Ifc2x3;
    let mut model = model("IFC2X3");
    let m = model.clone();
    let data = Value::Text("x".into());
    let mut tx = Transaction::new(&model);
    let metric = create_metric(
        &mut tx,
        &m,
        MetricDraft::new(base(v, "M"), Benchmark::EqualTo).data_value(MetricValueDraft::Typed {
            type_name: "IfcText",
            value: &data,
        }),
    )
    .unwrap();
    tx.commit(&mut model).unwrap();
    let entity = model.get(metric).unwrap();
    let as_ifc4 = Metric::try_new(metric, entity, SchemaVersion::Ifc4).unwrap();
    assert!(as_ifc4.creation_time().is_err(), "a record is not text");
    assert_eq!(as_ifc4.reference_path().unwrap(), None);
    let bound = ConstraintView::new(&model).metric(metric).unwrap();
    assert!(matches!(
        bound.reference_path(),
        Err(ConstraintError::NotInSchema { .. })
    ));
}
