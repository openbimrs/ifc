//! #212: the task, work-control and sequence readers bind the declared
//! release and read by attribute name.
//!
//! From the EXPRESS sources: IFC2X3 TC1 declares `IfcTask` with ten
//! attributes (`TaskId`, `Status`, `WorkMethod`, `IsMilestone`, `Priority`
//! after the five inherited ones), `IfcWorkPlan` and `IfcWorkSchedule` with
//! fifteen (`Identifier`, `IfcDateTimeSelect` dates, `IfcTimeMeasure`
//! durations, `WorkControlType` and `UserDefinedControlType` last) and
//! `IfcRelSequence` with eight (`TimeLag` an `IfcTimeMeasure`). IFC4 ADD2 TC1
//! and IFC4X3 ADD2 declare thirteen, fourteen and nine. Each release is
//! authored, written as STEP, read back with `ifc-step` and re-read through
//! the readers; a hand-written IFC2X3 file pins the positions the IFC4
//! layout would misread.

mod common;

use ifc_model::{Codec, Entity, EntityId, Model, Transaction, Value};
use ifc_schedule::{
    create_sequence_with_owner_history, create_task_with_owner_history,
    create_work_control_with_owner_history, end_tasks, execution_order, find_cycle,
    predecessors_of, sequences, start_tasks, subtasks_of, successors_of, tasks, tasks_of_schedule,
    work_plans, work_schedules, AuthoredDateTime, AuthoredDuration, CalendarDate, DateTimeValue,
    LocalTime, ScheduleReadError, SchemaVersion, Task, TaskDraft, TimeLag, WorkControl,
    WorkControlDraft, WorkControlKind,
};
use ifc_step::StepCodec;

use common::OWNER;
const G: [&str; 5] = [
    "0YvctVUKr0kugbFTf53O08",
    "0YvctVUKr0kugbFTf53O09",
    "0YvctVUKr0kugbFTf53O0A",
    "0YvctVUKr0kugbFTf53O0B",
    "0YvctVUKr0kugbFTf53O0C",
];

fn round_trip(model: &Model) -> Model {
    let bytes = StepCodec.write_bytes(model).expect("written");
    let back = StepCodec.read_bytes(&bytes).expect("read back");
    assert!(back.diagnostics().is_empty(), "{:?}", back.diagnostics());
    back
}

fn read(text: &str) -> Model {
    let model = StepCodec.read_bytes(text.as_bytes()).expect("parses");
    assert!(model.diagnostics().is_empty(), "{:?}", model.diagnostics());
    model
}

#[test]
fn every_release_round_trips_through_the_readers() {
    for (schema, version) in [
        ("IFC2X3", SchemaVersion::Ifc2x3),
        ("IFC4", SchemaVersion::Ifc4),
        ("IFC4X3_ADD2", SchemaVersion::Ifc4x3),
    ] {
        let ifc2x3 = version == SchemaVersion::Ifc2x3;
        let mut model = common::base(schema, version);
        let m = model.clone();
        let mut tx = Transaction::new(&model);
        let mut draft = TaskDraft::new(G[0])
            .name("Slab")
            .identification("T-1")
            .status("PLANNED")
            .work_method("Pump")
            .is_milestone(true)
            .priority(7);
        if !ifc2x3 {
            draft = draft
                .long_description("Long")
                .predefined_type("CONSTRUCTION");
        }
        let first = create_task_with_owner_history(&mut tx, &m, draft, OWNER).expect("task");
        let second = create_task_with_owner_history(
            &mut tx,
            &m,
            TaskDraft::new(G[1]).name("Walls").identification("T-2"),
            OWNER,
        )
        .expect("task");
        let (created, start): (DateTimeValue<'_>, DateTimeValue<'_>) = if ifc2x3 {
            (
                CalendarDate::new(2026, 9, 28).into(),
                LocalTime::new(8).minute(0).into(),
            )
        } else {
            ("2026-09-28T00:00:00".into(), "2026-10-01T08:00:00".into())
        };
        let mut control = WorkControlDraft::new(G[2], created, start)
            .name("Programme")
            .identification("WS-1")
            .purpose("Build");
        if !ifc2x3 {
            control = control
                .duration("P60D")
                .total_float("P5D")
                .predefined_type("PLANNED");
        }
        let schedule = create_work_control_with_owner_history(
            &mut tx,
            &m,
            WorkControlKind::Schedule,
            control,
            OWNER,
        )
        .expect("schedule");
        let lag = if ifc2x3 {
            Some(TimeLag::Seconds(3600.0))
        } else {
            let lag = tx.create(Entity::new(
                "IFCLAGTIME",
                vec![
                    Value::Null,
                    Value::Null,
                    Value::Null,
                    Value::Typed {
                        type_name: "IFCDURATION".into(),
                        value: Box::new(Value::Text("P1D".into())),
                    },
                    Value::Enum("WORKTIME".into()),
                ],
            ));
            Some(TimeLag::LagTime(lag))
        };
        let link = create_sequence_with_owner_history(
            &mut tx,
            &m,
            G[3],
            first,
            second,
            Some("FINISH_START"),
            lag,
            OWNER,
        )
        .expect("sequence");
        tx.commit(&mut model).expect("commit");
        let back = round_trip(&model);

        let found = tasks(&back).expect("bound");
        let task = found.iter().find(|t| t.id() == first).expect("task");
        assert_eq!(task.release(), version);
        assert_eq!(task.name(), Some("Slab"), "{schema}");
        assert_eq!(task.identification(), Some("T-1"), "{schema}");
        assert_eq!(task.status(), Some("PLANNED"), "{schema}");
        assert_eq!(task.work_method(), Some("Pump"), "{schema}");
        assert_eq!(task.is_milestone(), Some(true), "{schema}");
        assert_eq!(task.priority(), Some(7), "{schema}");
        if ifc2x3 {
            assert_eq!(task.long_description(), None);
            assert_eq!(task.predefined_type(), None);
            assert_eq!(task.task_time_ref(), None);
        } else {
            assert_eq!(task.long_description(), Some("Long"));
            assert_eq!(task.predefined_type(), Some("CONSTRUCTION"));
        }

        let read = work_schedules(&back).expect("bound");
        let control = read.iter().find(|c| c.id() == schedule).expect("schedule");
        assert_eq!(control.identification(), Some("WS-1"), "{schema}");
        assert_eq!(control.purpose(), Some("Build"), "{schema}");
        if ifc2x3 {
            let Some(AuthoredDateTime::Record(date)) = control.creation_date() else {
                panic!("{:?}", control.creation_date());
            };
            assert_eq!(&*back.get(date).unwrap().type_name, "IFCCALENDARDATE");
            let Some(AuthoredDateTime::Record(time)) = control.start_time() else {
                panic!("{:?}", control.start_time());
            };
            assert_eq!(&*back.get(time).unwrap().type_name, "IFCLOCALTIME");
            assert_eq!(control.predefined_type(), None);
        } else {
            assert_eq!(
                control.creation_date(),
                Some(AuthoredDateTime::Text("2026-09-28T00:00:00"))
            );
            assert_eq!(control.duration(), Some(AuthoredDuration::Text("P60D")));
            assert_eq!(control.total_float(), Some(AuthoredDuration::Text("P5D")));
            assert_eq!(control.predefined_type(), Some("PLANNED"));
            assert_eq!(control.work_control_type(), None);
        }
        assert!(work_plans(&back).expect("bound").is_empty());

        let links = sequences(&back).expect("bound");
        let read = links.iter().find(|s| s.id == link).expect("sequence");
        assert_eq!((read.predecessor, read.successor), (first, second));
        if ifc2x3 {
            assert_eq!(read.time_lag_measure, Some(3600.0));
            assert!(read.lag.is_none());
        } else {
            assert_eq!(read.time_lag_measure, None);
            let lag = read.lag.as_ref().expect("lag");
            assert_eq!(lag.duration.as_deref(), Some("P1D"));
        }
        assert_eq!(successors_of(&back, first).unwrap(), vec![second]);
        assert_eq!(predecessors_of(&back, second).unwrap(), vec![first]);
        assert_eq!(start_tasks(&back).unwrap(), vec![first]);
        assert_eq!(end_tasks(&back).unwrap(), vec![second]);
        assert_eq!(execution_order(&back).unwrap(), vec![first, second]);
        assert_eq!(find_cycle(&back).unwrap(), None);
    }
}

/// A hand-written IFC2X3 file: the IFC4 positions would read `Status` as
/// the long description, `Priority` as the milestone flag,
/// `WorkControlType` as the predefined type and the lag measure as nothing.
#[test]
fn an_ifc2x3_file_is_read_by_name() {
    let model = read(
        "ISO-10303-21;\nHEADER;\nFILE_DESCRIPTION((''),'2;1');\n\
         FILE_NAME('','',(''),(''),'','','');\nFILE_SCHEMA(('IFC2X3'));\nENDSEC;\nDATA;\n\
         #1=IFCPERSON($,'Doe','Jane',$,$,$,$,$);\n\
         #2=IFCORGANIZATION($,'Acme',$,$,$);\n\
         #3=IFCPERSONANDORGANIZATION(#1,#2,$);\n\
         #4=IFCAPPLICATION(#2,'1.0','Test','test');\n\
         #5=IFCOWNERHISTORY(#3,#4,$,.NOCHANGE.,$,$,$,1700000000);\n\
         #10=IFCTASK('0YvctVUKr0kugbFTf53O08',#5,'Dig',$,$,'T-9','ACTIVE','Excavator',.F.,3);\n\
         #11=IFCTASK('0YvctVUKr0kugbFTf53O09',#5,'Pour',$,$,'T-10',$,$,.T.,$);\n\
         #12=IFCCALENDARDATE(28,9,2026);\n\
         #13=IFCWORKPLAN('0YvctVUKr0kugbFTf53O0A',#5,'Plan',$,$,'WP-9',#12,$,'Why',\
         86400.,3600.,#12,$,.BASELINE.,$);\n\
         #14=IFCRELSEQUENCE('0YvctVUKr0kugbFTf53O0B',#5,$,$,#10,#11,7200.,.START_START.);\n\
         #15=IFCRELASSIGNSTOCONTROL('0YvctVUKr0kugbFTf53O0C',#5,$,$,(#10,#11),$,#13);\n\
         ENDSEC;\nEND-ISO-10303-21;\n",
    );
    let found = tasks(&model).expect("bound");
    let dig = &found[0];
    assert_eq!(dig.release(), SchemaVersion::Ifc2x3);
    assert_eq!(dig.identification(), Some("T-9"), "TaskId");
    assert_eq!(dig.status(), Some("ACTIVE"), "slot 6, not 7");
    assert_eq!(dig.work_method(), Some("Excavator"), "slot 7, not 8");
    assert_eq!(dig.is_milestone(), Some(false), "slot 8, not 9");
    assert_eq!(dig.priority(), Some(3), "slot 9, not 10");
    assert_eq!(dig.long_description(), None, "IFC2X3 declares none");
    assert_eq!(found[1].is_milestone(), Some(true));
    // The same record read as IFC4 is a different task.
    let misread =
        Task::new(dig.id(), model.get(dig.id()).unwrap(), SchemaVersion::Ifc4).expect("verified");
    assert_eq!(misread.long_description(), Some("ACTIVE"));
    assert_eq!(misread.is_milestone(), None);

    let plan = &work_plans(&model).expect("bound")[0];
    assert_eq!(plan.identification(), Some("WP-9"), "Identifier");
    assert_eq!(plan.purpose(), Some("Why"));
    assert_eq!(
        plan.creation_date(),
        Some(AuthoredDateTime::Record(EntityId(12)))
    );
    assert_eq!(
        plan.start_time(),
        Some(AuthoredDateTime::Record(EntityId(12)))
    );
    assert_eq!(plan.finish_time(), None);
    assert_eq!(
        plan.duration(),
        Some(AuthoredDuration::TimeMeasure(86400.0))
    );
    assert_eq!(
        plan.total_float(),
        Some(AuthoredDuration::TimeMeasure(3600.0))
    );
    assert_eq!(plan.work_control_type(), Some("BASELINE"));
    assert_eq!(
        plan.predefined_type(),
        None,
        "not aliased to WorkControlType"
    );
    let misread = WorkControl::new(
        plan.id(),
        model.get(plan.id()).unwrap(),
        SchemaVersion::Ifc4,
    )
    .expect("verified")
    .expect("a plan");
    assert_eq!(misread.predefined_type(), Some("BASELINE"));

    let link = &sequences(&model).expect("bound")[0];
    assert_eq!(link.time_lag_measure, Some(7200.0));
    assert_eq!(
        link.sequence_type,
        Some(ifc_schedule::SequenceType::StartStart)
    );
    assert_eq!(
        tasks_of_schedule(&model, EntityId(13)).unwrap(),
        vec![EntityId(10), EntityId(11)]
    );
    assert!(subtasks_of(&model, EntityId(10)).unwrap().is_empty());
}

/// A header the readers cannot bind is a typed refusal, never IFC4.
#[test]
fn unverified_and_multiple_releases_are_refused() {
    for schema in ["IFC4X1", "IFC4X2", "IFC5"] {
        let model = read(&format!(
            "ISO-10303-21;\nHEADER;\nFILE_DESCRIPTION((''),'2;1');\n\
             FILE_NAME('','',(''),(''),'','','');\nFILE_SCHEMA(('{schema}'));\nENDSEC;\nDATA;\n\
             #10=IFCTASK('0YvctVUKr0kugbFTf53O08',$,'Dig',$,$,$,$,$,$,.F.,$,$,$);\n\
             ENDSEC;\nEND-ISO-10303-21;\n"
        ));
        let refused = ScheduleReadError::UnsupportedSchema {
            schema: schema.into(),
        };
        assert_eq!(tasks(&model).err(), Some(refused.clone()), "{schema}");
        assert_eq!(work_plans(&model).err(), Some(refused.clone()));
        assert_eq!(work_schedules(&model).err(), Some(refused.clone()));
        assert_eq!(sequences(&model).err(), Some(refused.clone()));
        assert_eq!(
            successors_of(&model, EntityId(10)).err(),
            Some(refused.clone())
        );
        assert_eq!(execution_order(&model).err(), Some(refused.clone()));
        assert_eq!(find_cycle(&model).err(), Some(refused.clone()));
        assert_eq!(
            tasks_of_schedule(&model, EntityId(1)).err(),
            Some(refused.clone())
        );
        assert_eq!(subtasks_of(&model, EntityId(10)).err(), Some(refused));
    }
    let mut model = Model::new();
    model.header_mut().schema = vec!["IFC4".into(), "IFC2X3".into()];
    assert_eq!(
        tasks(&model).err(),
        Some(ScheduleReadError::MultipleSchemas { schemas: 2 })
    );
    let entity = Entity::new("IFCTASK", vec![Value::Null; 13]);
    for version in [SchemaVersion::Ifc4x1, SchemaVersion::Ifc4x2] {
        assert!(matches!(
            Task::new(EntityId(1), &entity, version),
            Err(ScheduleReadError::UnsupportedSchema { .. })
        ));
        assert!(matches!(
            WorkControl::new(EntityId(1), &entity, version),
            Err(ScheduleReadError::UnsupportedSchema { .. })
        ));
    }
}
