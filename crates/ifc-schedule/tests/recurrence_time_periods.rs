//! `IfcRecurrencePattern` carries all eight attributes and its time
//! periods (#233).
//!
//! IFC4 ADD2 TC1 and IFC4X3 ADD2 both declare `IfcRecurrencePattern` with
//! eight attributes, the last `TimePeriods : OPTIONAL LIST [1:?] OF
//! IfcTimePeriod`, and `IfcTimePeriod(StartTime, EndTime : IfcTime)`.
//! IFC2X3 TC1 declares neither.

use ifc_model::{Codec, Entity, EntityId, Model, Transaction, Value};
use ifc_schedule::calendar::recurrence_slot;
use ifc_schedule::error::ScheduleAuthoringError;
use ifc_schedule::{
    create_recurrence_pattern, create_recurrence_pattern_in, create_time_period,
    create_time_period_in, recurrence_pattern, RecurrenceDraft, RecurrenceType, SchemaVersion,
};
use ifc_schema::for_version;
use ifc_step::StepCodec;

fn model(schema: &str) -> Model {
    let mut model = Model::default();
    model.header_mut().schema = vec![schema.to_owned()];
    model
}

/// The schema tables agree with the record the writer lays out.
#[test]
fn the_pattern_is_eight_attributes_in_ifc4_and_ifc4x3() {
    for version in [SchemaVersion::Ifc4, SchemaVersion::Ifc4x3] {
        let schema = for_version(version).expect("bundled");
        let names = schema.attribute_names("IFCRECURRENCEPATTERN");
        assert_eq!(names.len(), 8, "{version:?}");
        assert_eq!(names[recurrence_slot::TIME_PERIODS], "TimePeriods");
        assert_eq!(
            schema.attribute_names("IFCTIMEPERIOD"),
            ["StartTime", "EndTime"]
        );
    }
    let mut tx = Transaction::new(&Model::default());
    let id = create_recurrence_pattern(&mut tx, &RecurrenceDraft::new("DAILY")).expect("pattern");
    let staged = tx
        .edits()
        .iter()
        .find_map(|edit| match edit {
            ifc_model::Edit::Create { id: staged, entity } if *staged == id => Some(entity),
            _ => None,
        })
        .expect("staged");
    assert_eq!(staged.attributes.len(), 8, "never a short record");
    assert_eq!(
        staged.attributes[recurrence_slot::TIME_PERIODS],
        Value::Null
    );
}

/// Days, months and periods survive a STEP round trip and are read back.
#[test]
fn a_pattern_with_periods_round_trips_in_ifc4_and_ifc4x3() {
    for schema in ["IFC4", "IFC4X3_ADD2"] {
        let mut model = model(schema);
        let mut tx = Transaction::new(&model);
        let morning = create_time_period_in(&mut tx, &model, "08:00:00", "12:00:00")
            .unwrap_or_else(|error| panic!("{schema}: {error}"));
        let afternoon =
            create_time_period_in(&mut tx, &model, "13:00:00", "17:00:00").expect("period");
        let draft = RecurrenceDraft::new("YEARLY_BY_DAY_OF_MONTH")
            .days(vec![1, 15])
            .months(vec![3, 9])
            .interval(1)
            .occurrences(4)
            .time_periods(vec![morning, afternoon]);
        let pattern = create_recurrence_pattern_in(&mut tx, &model, &draft)
            .unwrap_or_else(|error| panic!("{schema}: {error}"));
        tx.commit(&mut model).expect("commit");

        let bytes = StepCodec.write_bytes(&model).expect("writes");
        let text = String::from_utf8(bytes.clone()).expect("utf-8");
        let line = text
            .lines()
            .find(|line| line.contains("IFCRECURRENCEPATTERN("))
            .expect("pattern written");
        assert!(
            line.ends_with(&format!("(#{},#{}));", morning.0, afternoon.0)),
            "{schema}: TimePeriods written in slot 8: {line}"
        );

        let reread = StepCodec.read_bytes(&bytes).expect("reads back");
        assert!(
            reread.diagnostics().is_empty(),
            "{:?}",
            reread.diagnostics()
        );
        assert_eq!(reread.get(pattern).expect("present").attributes.len(), 8);
        let recurrence = recurrence_pattern(&reread, pattern).expect("a pattern");
        assert_eq!(
            recurrence.recurrence_type,
            Some(RecurrenceType::YearlyByDayOfMonth)
        );
        assert_eq!(recurrence.days, [1, 15], "{schema}");
        assert_eq!(recurrence.months, [3, 9], "{schema}");
        assert_eq!(recurrence.occurrences, Some(4));
        let periods: Vec<_> = recurrence
            .time_periods
            .iter()
            .map(|p| (p.id, p.start_time.as_deref(), p.end_time.as_deref()))
            .collect();
        assert_eq!(
            periods,
            [
                (morning, Some("08:00:00"), Some("12:00:00")),
                (afternoon, Some("13:00:00"), Some("17:00:00")),
            ],
            "{schema}"
        );
    }
}

/// The bound writers stage exactly what the unbound ones stage.
#[test]
fn bound_and_unbound_writers_agree() {
    for schema in ["IFC4", "IFC4X3_ADD2"] {
        let model = model(schema);
        let mut bound = Transaction::new(&model);
        let period = create_time_period_in(&mut bound, &model, "08:00", "17:00").expect("bound");
        let draft = RecurrenceDraft::new("WEEKLY")
            .weekdays(vec![1, 3])
            .time_periods(vec![period]);
        create_recurrence_pattern_in(&mut bound, &model, &draft).expect("bound");
        let mut unbound = Transaction::new(&model);
        create_time_period(&mut unbound, "08:00", "17:00").expect("unbound");
        create_recurrence_pattern(&mut unbound, &draft).expect("unbound");
        assert_eq!(bound.edits(), unbound.edits(), "{schema}");
    }
}

/// IFC2X3 declares neither entity: both writers refuse, nothing staged.
#[test]
fn ifc2x3_refuses_both_writers() {
    let model = model("IFC2X3");
    let mut tx = Transaction::new(&model);
    assert_eq!(
        create_time_period_in(&mut tx, &model, "08:00", "17:00"),
        Err(ScheduleAuthoringError::EntityNotInSchema {
            entity: "IFCTIMEPERIOD",
            schema: SchemaVersion::Ifc2x3,
        })
    );
    assert_eq!(
        create_recurrence_pattern_in(&mut tx, &model, &RecurrenceDraft::new("DAILY")),
        Err(ScheduleAuthoringError::EntityNotInSchema {
            entity: "IFCRECURRENCEPATTERN",
            schema: SchemaVersion::Ifc2x3,
        })
    );
    assert!(tx.is_empty(), "a refused write stages nothing");
}

/// A period reference that is absent or not an `IfcTimePeriod` is refused.
#[test]
fn a_bad_period_reference_is_refused() {
    let mut model = model("IFC4");
    let wrong = model.push(Entity::new("IFCTASKTIME", vec![Value::Null; 20]));
    let mut tx = Transaction::new(&model);

    let draft = RecurrenceDraft::new("DAILY").time_periods(vec![wrong]);
    assert_eq!(
        create_recurrence_pattern_in(&mut tx, &model, &draft),
        Err(ScheduleAuthoringError::WrongReferenceType {
            entity: "IFCRECURRENCEPATTERN",
            attribute: "TimePeriods",
            target: wrong,
            actual: "IFCTASKTIME".into(),
            expected: "IFCTIMEPERIOD",
        })
    );
    let missing = EntityId(999);
    let draft = RecurrenceDraft::new("DAILY").time_periods(vec![missing]);
    assert_eq!(
        create_recurrence_pattern_in(&mut tx, &model, &draft),
        Err(ScheduleAuthoringError::MissingReference {
            entity: "IFCRECURRENCEPATTERN",
            attribute: "TimePeriods",
            target: missing,
        })
    );
    assert!(tx.is_empty());
}

/// A pattern reached through a work calendar exposes the same components.
#[test]
fn a_calendar_pattern_reads_days_months_and_periods() {
    let text = "ISO-10303-21;\nHEADER;\nFILE_DESCRIPTION((''),'2;1');\n\
        FILE_NAME('','',(''),(''),'','','');\nFILE_SCHEMA(('IFC4'));\nENDSEC;\nDATA;\n\
        #1=IFCTIMEPERIOD('07:00:00','15:30:00');\n\
        #2=IFCRECURRENCEPATTERN(.MONTHLY_BY_DAY_OF_MONTH.,(1,2),$,(6),$,$,$,(#1));\n\
        #3=IFCWORKTIME('Shift',$,$,#2,$,$);\n\
        #4=IFCWORKCALENDAR('0YvctVUKr0kugbFTf53O9L',$,'Cal',$,$,$,(#3),$,$);\n\
        ENDSEC;\nEND-ISO-10303-21;\n";
    let model = StepCodec.read_bytes(text.as_bytes()).expect("parses");
    let calendars = ifc_schedule::work_calendars(&model);
    let recurrence = calendars[0].working_times(&model)[0]
        .recurrence
        .clone()
        .expect("a pattern");
    assert_eq!(recurrence.days, [1, 2]);
    assert_eq!(recurrence.months, [6]);
    assert_eq!(recurrence.time_periods.len(), 1);
    assert_eq!(
        recurrence.time_periods[0].start_time.as_deref(),
        Some("07:00:00")
    );
    assert_eq!(
        recurrence.time_periods[0].end_time.as_deref(),
        Some("15:30:00")
    );
}
