//! Events, lags and recurrence patterns authored, then read back.
//!
//! Slot order for all four entities was verified against the bundled
//! IFC4X3_ADD2 EXPRESS schema before these helpers were written, not
//! recalled: `IfcLagTime.LagValue` and `DurationType` are REQUIRED, unlike
//! every other scheduling-time field, and `IfcEvent` carries a
//! `UserDefinedEventTriggerType` at slot 9 that the reader never exposed.

use ifc_model::codec::Codec;
use ifc_model::{Model, Transaction, Value};
use ifc_schedule::{
    create_event, create_event_time, create_lag_time, create_recurrence_pattern, create_sequence,
    create_task, events, sequences, EventDraft, EventTimeDraft, RecurrenceDraft, TaskDraft,
};
use ifc_step::StepCodec;

/// A task, since a sequence needs two of them.
fn task(tx: &mut Transaction, guid: &str, name: &str) -> ifc_model::EntityId {
    create_task(
        tx,
        TaskDraft {
            global_id: guid,
            name: Some(name),
            is_milestone: false,
            ..TaskDraft::default()
        },
    )
    .expect("authored task")
}

/// An authored event and its occurrence time read back through `events()`.
#[test]
fn an_authored_event_reads_back() {
    let mut model = Model::default();
    let mut tx = Transaction::new(&model);
    let time = create_event_time(
        &mut tx,
        EventTimeDraft {
            actual: Some("2026-03-02T08:00:00"),
            schedule: Some("2026-03-01T08:00:00"),
            ..EventTimeDraft::default()
        },
    )
    .expect("authored event time");
    create_event(
        &mut tx,
        EventDraft {
            global_id: "0aBcDeFgHiJkLmNoPqRsTu",
            name: Some("Concrete pour hold point"),
            identification: Some("EV-01"),
            predefined_type: Some("STARTEVENT"),
            trigger_type: Some("EVENTRULE"),
            occurence_time: Some(time),
            ..EventDraft::default()
        },
    )
    .expect("authored event");
    tx.commit(&mut model).expect("commit");

    let found = events(&model);
    assert_eq!(found.len(), 1, "one event");
    assert_eq!(found[0].name(), Some("Concrete pour hold point"));
    assert_eq!(found[0].identification(), Some("EV-01"));
    assert_eq!(found[0].predefined_type(), Some("STARTEVENT"));
    assert_eq!(found[0].trigger_type(), Some("EVENTRULE"));

    // The occurrence time resolves through the reference, so the slot the
    // author wrote is the slot the reader follows.
    let read = found[0].time(&model).expect("occurrence time");
    assert_eq!(read.actual.as_deref(), Some("2026-03-02T08:00:00"));
    assert_eq!(read.scheduled.as_deref(), Some("2026-03-01T08:00:00"));
    assert_eq!(read.early, None);
}

/// A lag reads back as a duration or a ratio, whichever was authored.
///
/// `IfcTimeOrRatioSelect` admits both and they mean different things: a
/// duration lag is "wait five days", a ratio lag is "start at 50% done".
/// The reader keeps them in separate fields, so a helper that collapsed
/// them would surface here.
#[test]
fn a_lag_reads_back_in_the_form_it_was_authored() {
    let mut model = Model::default();
    let mut tx = Transaction::new(&model);
    let a = task(&mut tx, "0aBcDeFgHiJkLmNoPqRsTu", "Strip formwork");
    let b = task(&mut tx, "1aBcDeFgHiJkLmNoPqRsTu", "Backfill");
    let duration_lag =
        create_lag_time(&mut tx, Some("cure"), Value::Text("P5D".into()), "WORKTIME")
            .expect("authored duration lag");
    create_sequence(
        &mut tx,
        "2aBcDeFgHiJkLmNoPqRsTu",
        a,
        b,
        Some("FINISH_START"),
        Some(duration_lag),
    )
    .expect("authored sequence");
    tx.commit(&mut model).expect("commit");

    let found = sequences(&model);
    assert_eq!(found.len(), 1, "one sequence");
    let lag = found[0].lag.as_ref().expect("lag present");
    assert_eq!(lag.duration.as_deref(), Some("P5D"));
    assert_eq!(lag.ratio, None, "a duration lag is not a ratio");
}

/// A recurrence pattern reads back through the calendar that owns it.
#[test]
fn a_recurrence_pattern_reads_back() {
    use ifc_schedule::{create_work_calendar, create_work_time, work_calendars};

    let mut model = Model::default();
    let mut tx = Transaction::new(&model);
    let pattern = create_recurrence_pattern(
        &mut tx,
        &RecurrenceDraft {
            recurrence_type: "WEEKLY",
            weekdays: vec![1, 2, 3, 4, 5],
            interval: Some(1),
            occurrences: Some(52),
            ..RecurrenceDraft::default()
        },
    )
    .expect("authored pattern");
    let working = create_work_time(
        &mut tx,
        Some("weekdays"),
        Some(pattern),
        Some("2026-01-05"),
        Some("2026-12-31"),
    )
    .expect("authored work time");
    create_work_calendar(
        &mut tx,
        "0aBcDeFgHiJkLmNoPqRsTu",
        Some("Site calendar"),
        &[working],
        &[],
        Some("FIRSTSHIFT"),
    )
    .expect("authored calendar");
    tx.commit(&mut model).expect("commit");

    let found = work_calendars(&model);
    let times = found[0].working_times(&model);
    let recurrence = times[0].recurrence.as_ref().expect("recurrence present");
    assert_eq!(recurrence.weekdays, vec![1, 2, 3, 4, 5]);
    assert_eq!(recurrence.interval, Some(1));
    assert_eq!(recurrence.occurrences, Some(52));
}

/// The schema's two USERDEFINED rules are enforced, not deferred.
///
/// Both cases produce a file that parses and validates structurally. What
/// is lost is meaning: the event reads back as user-defined with nothing
/// stating what the user defined it as.
#[test]
fn a_userdefined_discriminator_without_its_label_is_refused() {
    let model = Model::default();
    let mut tx = Transaction::new(&model);

    // CorrectPredefinedType: USERDEFINED needs IfcObject.ObjectType.
    assert!(create_event(
        &mut tx,
        EventDraft {
            global_id: "0aBcDeFgHiJkLmNoPqRsTu",
            predefined_type: Some("USERDEFINED"),
            ..EventDraft::default()
        },
    )
    .is_err());

    // CorrectTypeAssigned: USERDEFINED needs UserDefinedEventTriggerType.
    assert!(create_event(
        &mut tx,
        EventDraft {
            global_id: "0aBcDeFgHiJkLmNoPqRsTu",
            trigger_type: Some("USERDEFINED"),
            ..EventDraft::default()
        },
    )
    .is_err());

    // Whitespace satisfies EXISTS in the schema but carries no meaning.
    assert!(create_event(
        &mut tx,
        EventDraft {
            global_id: "0aBcDeFgHiJkLmNoPqRsTu",
            trigger_type: Some("USERDEFINED"),
            user_defined_trigger_type: Some("   "),
            ..EventDraft::default()
        },
    )
    .is_err());

    // Supplying the label makes the same event legal.
    assert!(create_event(
        &mut tx,
        EventDraft {
            global_id: "0aBcDeFgHiJkLmNoPqRsTu",
            trigger_type: Some("USERDEFINED"),
            user_defined_trigger_type: Some("ClientInstruction"),
            ..EventDraft::default()
        },
    )
    .is_ok());
}

/// Component values outside the schema's 1-based ranges are refused.
///
/// A 0 weekday is the classic off-by-one: it parses, it validates, and it
/// silently means a different day than the author intended. The schema
/// numbers Monday as 1, so 0 is not Sunday, it is nothing.
#[test]
fn out_of_range_recurrence_components_are_refused() {
    let model = Model::default();
    let mut tx = Transaction::new(&model);

    let zero_weekday = RecurrenceDraft {
        recurrence_type: "WEEKLY",
        weekdays: vec![0],
        ..RecurrenceDraft::default()
    };
    assert!(create_recurrence_pattern(&mut tx, &zero_weekday).is_err());

    let eighth_day = RecurrenceDraft {
        recurrence_type: "WEEKLY",
        weekdays: vec![8],
        ..RecurrenceDraft::default()
    };
    assert!(create_recurrence_pattern(&mut tx, &eighth_day).is_err());

    let month_zero = RecurrenceDraft {
        recurrence_type: "YEARLY_BY_DAY_OF_MONTH",
        months: vec![0],
        ..RecurrenceDraft::default()
    };
    assert!(create_recurrence_pattern(&mut tx, &month_zero).is_err());

    // "every 0 weeks" parses and describes nothing.
    let never = RecurrenceDraft {
        recurrence_type: "WEEKLY",
        interval: Some(0),
        ..RecurrenceDraft::default()
    };
    assert!(create_recurrence_pattern(&mut tx, &never).is_err());

    // A negative position is legal: it counts back from the period end.
    let last_friday = RecurrenceDraft {
        recurrence_type: "MONTHLY_BY_POSITION",
        weekdays: vec![5],
        position: Some(-1),
        ..RecurrenceDraft::default()
    };
    assert!(create_recurrence_pattern(&mut tx, &last_friday).is_ok());
}

/// A lag must state both how long and in what units.
#[test]
fn a_lag_without_its_required_fields_is_refused() {
    let model = Model::default();
    let mut tx = Transaction::new(&model);

    // DurationType is REQUIRED by the schema.
    assert!(create_lag_time(&mut tx, None, Value::Text("P5D".into()), "").is_err());

    // A reference is not an IfcTimeOrRatioSelect.
    assert!(create_lag_time(
        &mut tx,
        None,
        Value::Ref(ifc_model::EntityId(1)),
        "WORKTIME"
    )
    .is_err());

    // A ratio lag is legal: "start when the predecessor is 50% done".
    assert!(create_lag_time(&mut tx, None, Value::Real(0.5), "RATIO").is_ok());
}

/// The four records survive serialisation to STEP text and back.
///
/// Slot padding is where this would break: `IfcEvent` reserves ten
/// inherited positions before its own, and a writer that emitted a short
/// parameter list would still round-trip in memory while producing a file
/// whose trigger type lands in the wrong slot.
#[test]
fn the_records_survive_step_text() {
    let mut model = Model::default();
    *model.header_mut() = ifc_model::Header {
        schema: vec!["IFC4X3_ADD2".to_owned()],
        ..ifc_model::Header::default()
    };
    let mut tx = Transaction::new(&model);
    let time = create_event_time(
        &mut tx,
        EventTimeDraft {
            actual: Some("2026-03-02T08:00:00"),
            ..EventTimeDraft::default()
        },
    )
    .expect("authored event time");
    create_event(
        &mut tx,
        EventDraft {
            global_id: "0aBcDeFgHiJkLmNoPqRsTu",
            name: Some("Hold point"),
            trigger_type: Some("USERDEFINED"),
            user_defined_trigger_type: Some("ClientInstruction"),
            occurence_time: Some(time),
            ..EventDraft::default()
        },
    )
    .expect("authored event");
    tx.commit(&mut model).expect("commit");

    let mut bytes = Vec::new();
    Codec::write(&StepCodec, &model, &mut bytes).expect("written");
    let text = String::from_utf8(bytes.clone()).expect("utf-8");
    assert!(text.contains("IFCEVENT("), "the event reached the file");

    let reparsed = Codec::read_bytes(&StepCodec, &bytes).expect("reparsed");
    let found = events(&reparsed);
    assert_eq!(found.len(), 1, "one event after reparse");
    assert_eq!(found[0].name(), Some("Hold point"));
    assert_eq!(found[0].trigger_type(), Some("USERDEFINED"));

    // The reference survived, so slot padding held across the text seam.
    let read = found[0].time(&reparsed).expect("occurrence time");
    assert_eq!(read.actual.as_deref(), Some("2026-03-02T08:00:00"));
}

/// An omitted component set is absent, not an empty aggregate.
///
/// The schema types all three as `OPTIONAL SET [1:?]`, so an empty set is
/// invalid where omission is legal. Writing `()` instead of `$` produces a
/// file that parses and then fails schema validation for a reason the
/// author never expressed.
#[test]
fn an_omitted_component_set_is_written_as_absent() {
    let mut model = Model::default();
    *model.header_mut() = ifc_model::Header {
        schema: vec!["IFC4X3_ADD2".to_owned()],
        ..ifc_model::Header::default()
    };
    let mut tx = Transaction::new(&model);
    create_recurrence_pattern(
        &mut tx,
        &RecurrenceDraft {
            recurrence_type: "DAILY",
            interval: Some(2),
            ..RecurrenceDraft::default()
        },
    )
    .expect("authored pattern");
    tx.commit(&mut model).expect("commit");

    let mut bytes = Vec::new();
    Codec::write(&StepCodec, &model, &mut bytes).expect("written");
    let text = String::from_utf8(bytes).expect("utf-8");
    let line = text
        .lines()
        .find(|l| l.contains("IFCRECURRENCEPATTERN"))
        .expect("pattern reached the file");
    assert!(
        !line.contains("()"),
        "omitted sets must be $, not an empty aggregate: {line}"
    );
}
