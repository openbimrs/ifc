//! Cost values, monetary units, currency relationships and the cost
//! schedule reader, bound to the declared release (#212, #213).
//!
//! From the EXPRESS sources: IFC2X3 TC1 declares `IfcCostValue` with eight
//! attributes (`CostType` required, dates as `IfcDateTimeSelect` records),
//! `IfcMonetaryUnit.Currency` as `IfcCurrencyEnum` and
//! `IfcCurrencyRelationship` with five (`RateDateTime` a required
//! `IfcDateAndTime`). IFC4 ADD2 TC1 and IFC4X3 ADD2 declare ten, an
//! `IfcLabel` and seven.

use ifc_cost::mutation::{
    create_cost_schedule_with_owner_history, create_cost_value, create_currency_relationship,
    create_monetary_unit, CostAuthoringError, CostScheduleDraft, CostScheduleType, CostValueDraft,
    CostValueKind,
};
use ifc_cost::{
    ArithmeticOperator, AuthoredDateTime, CalendarDate, CostError, CostSchedule, CostView,
    DateTimeValue, LocalTime, SchemaVersion,
};
use ifc_model::{Codec, Entity, EntityId, Model, Transaction, Value};
use ifc_step::StepCodec;

const OWNER: EntityId = EntityId(5);

/// An owner history (`#5`) and its actors, in `schema`.
fn base(schema: &str) -> Model {
    let text = format!(
        "ISO-10303-21;\nHEADER;\nFILE_DESCRIPTION((''),'2;1');\n\
         FILE_NAME('','',(''),(''),'','','');\nFILE_SCHEMA(('{schema}'));\nENDSEC;\nDATA;\n\
         #1=IFCPERSON($,'Doe','Jane',$,$,$,$,$);\n\
         #2=IFCORGANIZATION($,'Acme',$,$,$);\n\
         #3=IFCPERSONANDORGANIZATION(#1,#2,$);\n\
         #4=IFCAPPLICATION(#2,'1.0','Test','test');\n\
         #5=IFCOWNERHISTORY(#3,#4,$,.NOCHANGE.,$,$,$,1700000000);\n\
         ENDSEC;\nEND-ISO-10303-21;\n"
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

fn money(amount: f64) -> Value {
    Value::Typed {
        type_name: "IFCMONETARYMEASURE".into(),
        value: Box::new(Value::Real(amount)),
    }
}

fn text(s: &str) -> Value {
    Value::Text(s.into())
}

/// The three writers, each release: arity and slots by the release's own
/// layout, written as STEP and read back, the value through `CostView`.
#[test]
fn value_records_round_trip_in_their_release() {
    for (schema, version) in [
        ("IFC2X3", SchemaVersion::Ifc2x3),
        ("IFC4", SchemaVersion::Ifc4),
        ("IFC4X3_ADD2", SchemaVersion::Ifc4x3),
    ] {
        let ifc2x3 = version == SchemaVersion::Ifc2x3;
        let mut model = base(schema);
        let m = model.clone();
        let mut tx = Transaction::new(&model);
        let (applicable, rate_date): (DateTimeValue<'_>, DateTimeValue<'_>) = if ifc2x3 {
            (
                CalendarDate::new(2026, 9, 28).into(),
                DateTimeValue::DateAndTime(CalendarDate::new(2026, 9, 22), LocalTime::new(9)),
            )
        } else {
            ("2026-09-28".into(), "2026-09-22T09:00:00".into())
        };
        let value = create_cost_value(
            &mut tx,
            &m,
            CostValueDraft::monetary(125.5)
                .name("Labour")
                .category("Labour")
                .condition("Day rate")
                .applicable_date(applicable),
        )
        .expect("cost value");
        let eur = create_monetary_unit(&mut tx, &m, "EUR").expect("eur");
        let gbp = create_monetary_unit(&mut tx, &m, "GBP").expect("gbp");
        let rate = create_currency_relationship(&mut tx, &m, eur, gbp, 0.85, Some(rate_date))
            .expect("rate");
        tx.commit(&mut model).expect("commit");
        let back = round_trip(&model);
        let record = |id: EntityId| back.get(id).expect("record").attributes.clone();
        let referenced = |id: EntityId, slot: usize| match &record(id)[slot] {
            Value::Ref(target) => back.get(*target).expect("date").type_name.to_string(),
            other => panic!("{schema} #{} slot {slot}: {other:?}", id.0),
        };
        if ifc2x3 {
            // Name, Description, AppliedValue, UnitBasis, ApplicableDate,
            // FixedUntilDate, CostType, Condition.
            let cost = record(value);
            assert_eq!(cost.len(), 8);
            assert_eq!(cost[2], money(125.5));
            assert_eq!(referenced(value, 4), "IFCCALENDARDATE");
            assert_eq!(cost[6], text("Labour"), "CostType");
            assert_eq!(cost[7], text("Day rate"), "Condition");
            assert_eq!(record(eur), vec![Value::Enum("EUR".into())]);
            // RelatingMonetaryUnit, RelatedMonetaryUnit, ExchangeRate,
            // RateDateTime, RateSource.
            let relationship = record(rate);
            assert_eq!(relationship.len(), 5);
            assert_eq!(
                relationship[..3],
                [Value::Ref(eur), Value::Ref(gbp), Value::Real(0.85)]
            );
            assert_eq!(referenced(rate, 3), "IFCDATEANDTIME");
        } else {
            assert_eq!(
                record(value),
                vec![
                    text("Labour"),
                    Value::Null,
                    money(125.5),
                    Value::Null,
                    text("2026-09-28"),
                    Value::Null,
                    text("Labour"),
                    text("Day rate"),
                    Value::Null,
                    Value::Null,
                ],
                "{schema}"
            );
            assert_eq!(record(eur), vec![text("EUR")]);
            assert_eq!(
                record(rate),
                vec![
                    Value::Null,
                    Value::Null,
                    Value::Ref(eur),
                    Value::Ref(gbp),
                    Value::Real(0.85),
                    text("2026-09-22T09:00:00"),
                    Value::Null,
                ],
                "{schema}"
            );
        }
        let view = CostView::new(&back);
        let read = ifc_cost::value::CostValue::new(value, back.get(value).unwrap());
        assert_eq!(read.amount(), Some(125.5), "{schema}");
        assert_eq!(view.model().ids_of_type("IFCMONETARYUNIT").len(), 2);
    }
}

fn refused(tx: &Transaction, result: Result<EntityId, CostAuthoringError>) -> CostAuthoringError {
    assert!(tx.is_empty(), "a refusal staged {:?}", tx.edits());
    result.expect_err("refused")
}

/// What IFC2X3 cannot hold is refused, never dropped or aliased, and
/// nothing is staged.
#[test]
fn ifc2x3_refuses_what_it_cannot_hold() {
    let v = SchemaVersion::Ifc2x3;
    let model = base("IFC2X3");
    let tx = &mut Transaction::new(&model);
    let date = CalendarDate::new(2026, 9, 28);
    let result = create_cost_value(tx, &model, CostValueDraft::monetary(1.0));
    assert_eq!(
        refused(tx, result),
        CostAuthoringError::AuthoringRequired {
            entity: "IFCCOSTVALUE",
            attribute: "CostType",
            schema: v
        }
    );
    let result = create_cost_value(
        tx,
        &model,
        CostValueDraft::monetary(1.0)
            .category("Labour")
            .applicable_date("2026-09-28"),
    );
    assert!(matches!(
        refused(tx, result),
        CostAuthoringError::AuthoringValueType {
            attribute: "ApplicableDate",
            ..
        }
    ));
    // A real component, so only the release can refuse the composition.
    let mut staging = Transaction::new(&model);
    let part = create_cost_value(
        &mut staging,
        &model,
        CostValueDraft::monetary(1.0).category("Part"),
    )
    .expect("part");
    let parts = [part];
    let composed = CostValueDraft::default()
        .category("Sum")
        .kind(CostValueKind::Components {
            operator: ArithmeticOperator::Add,
            components: &parts,
        });
    let before = staging.edits().len();
    let error = create_cost_value(&mut staging, &model, composed).expect_err("refused");
    assert_eq!(staging.edits().len(), before);
    assert!(matches!(
        error,
        CostAuthoringError::AuthoringNotInSchema {
            attribute: "ArithmeticOperator" | "Components",
            ..
        }
    ));
    let result = create_monetary_unit(tx, &model, "Bitcoin");
    assert!(matches!(
        refused(tx, result),
        CostAuthoringError::AuthoringValueType {
            attribute: "Currency",
            ..
        }
    ));
    let mut units = Transaction::new(&model);
    let eur = create_monetary_unit(&mut units, &model, "eur").expect("eur");
    let gbp = create_monetary_unit(&mut units, &model, "GBP").expect("gbp");
    let before = units.edits().len();
    let missing = create_currency_relationship(&mut units, &model, eur, gbp, 1.1, None);
    assert_eq!(
        missing.expect_err("refused"),
        CostAuthoringError::AuthoringRequired {
            entity: "IFCCURRENCYRELATIONSHIP",
            attribute: "RateDateTime",
            schema: v
        }
    );
    // IfcDateAndTime, not any IfcDateTimeSelect: a bare date is refused.
    let bare = create_currency_relationship(&mut units, &model, eur, gbp, 1.1, Some(date.into()));
    assert!(matches!(
        bare.expect_err("refused"),
        CostAuthoringError::AuthoringValueType {
            attribute: "RateDateTime",
            ..
        }
    ));
    assert_eq!(units.edits().len(), before, "nothing staged");
    // The enumerator is written as the release spells it.
    let Some(ifc_model::Edit::Create { entity, .. }) = units.edits().first() else {
        panic!("eur staged");
    };
    assert_eq!(entity.attributes, vec![Value::Enum("EUR".into())]);
}

/// IFC4 and IFC4X3 refuse IFC2X3 date records, and every writer refuses a
/// release the crate is not verified against.
#[test]
fn record_dates_and_unverified_releases_are_refused() {
    let model = base("IFC4");
    let tx = &mut Transaction::new(&model);
    let result = create_cost_value(
        tx,
        &model,
        CostValueDraft::monetary(1.0).applicable_date(CalendarDate::new(2026, 1, 1)),
    );
    assert!(matches!(
        refused(tx, result),
        CostAuthoringError::AuthoringValueType {
            attribute: "ApplicableDate",
            schema: SchemaVersion::Ifc4,
            ..
        }
    ));
    for schema in ["IFC4X1", "IFC4X2", "IFC5"] {
        let model = base(schema);
        let tx = &mut Transaction::new(&model);
        let unsupported = CostAuthoringError::UnsupportedSchema {
            schema: schema.into(),
        };
        let result = create_cost_value(tx, &model, CostValueDraft::monetary(1.0));
        assert_eq!(refused(tx, result), unsupported);
        let result = create_monetary_unit(tx, &model, "EUR");
        assert_eq!(refused(tx, result), unsupported);
    }
}

/// `CostSchedule` reads by name in the declared release: IFC2X3's `ID` at
/// 11, `PredefinedType` at 12, `Status` at 8 and record dates, where the
/// IFC4 positions hold `SubmittedBy`, `PreparedBy`, a date and `Status`.
#[test]
fn the_schedule_reader_binds_the_declared_release() {
    for (schema, version) in [
        ("IFC2X3", SchemaVersion::Ifc2x3),
        ("IFC4", SchemaVersion::Ifc4),
        ("IFC4X3_ADD2", SchemaVersion::Ifc4x3),
    ] {
        let ifc2x3 = version == SchemaVersion::Ifc2x3;
        let mut model = base(schema);
        let m = model.clone();
        let mut tx = Transaction::new(&model);
        let submitted: DateTimeValue<'_> = if ifc2x3 {
            CalendarDate::new(2026, 9, 28).into()
        } else {
            "2026-09-28T00:00:00".into()
        };
        let mut draft = CostScheduleDraft::new("0YvctVUKr0kugbFTf53O08")
            .name("Estimate")
            .identification("CS-1")
            .predefined_type(CostScheduleType::Estimate)
            .status("DRAFT");
        draft.submitted_on = Some(submitted);
        let id =
            create_cost_schedule_with_owner_history(&mut tx, &m, draft, OWNER).expect("schedule");
        tx.commit(&mut model).expect("commit");
        let back = round_trip(&model);
        let view = CostView::new(&back);
        let schedule = view.schedules().expect("bound").next().expect("schedule");
        assert_eq!(schedule.id(), id);
        assert_eq!(schedule.release(), version);
        assert_eq!(schedule.name(), Some("Estimate"), "{schema}");
        assert_eq!(schedule.identification(), Some("CS-1"), "{schema}");
        assert_eq!(schedule.predefined_type(), Some("ESTIMATE"), "{schema}");
        assert_eq!(schedule.status(), Some("DRAFT"), "{schema}");
        assert_eq!(schedule.update_date(), None, "{schema}");
        match schedule.submitted_on().expect("submitted") {
            AuthoredDateTime::Record(record) => {
                assert!(ifc2x3, "{schema}");
                assert_eq!(&*back.get(record).unwrap().type_name, "IFCCALENDARDATE");
            }
            AuthoredDateTime::Text(text) => {
                assert!(!ifc2x3, "{schema}");
                assert_eq!(text, "2026-09-28T00:00:00");
            }
            other => panic!("{other:?}"),
        }
        let entity = back.get(id).unwrap();
        let explicit = CostSchedule::new(id, entity, version).expect("verified");
        assert_eq!(explicit.identification(), Some("CS-1"));
        // The same record read through the other layout is not the same
        // schedule: bound reading is what makes the answers right.
        let other = if ifc2x3 {
            SchemaVersion::Ifc4
        } else {
            SchemaVersion::Ifc2x3
        };
        let misread = CostSchedule::new(id, entity, other).expect("verified");
        assert_ne!(misread.status(), Some("DRAFT"), "{schema}");
    }
}

/// A header the reader cannot bind is a typed refusal, never IFC4.
#[test]
fn the_schedule_reader_refuses_unverified_releases() {
    for schema in ["IFC4X1", "IFC4X2", "IFC5"] {
        let model = base(schema);
        assert!(
            matches!(
                CostView::new(&model).schedules().err(),
                Some(CostError::UnsupportedSchema { schema: s }) if s == schema
            ),
            "{schema}"
        );
    }
    let mut model = base("IFC4");
    model.header_mut().schema.push("IFC2X3".into());
    assert!(matches!(
        CostView::new(&model).schedules().err(),
        Some(CostError::MultipleSchemas { schemas: 2 })
    ));
    let entity = Entity::new("IFCCOSTSCHEDULE", vec![Value::Null; 10]);
    for version in [SchemaVersion::Ifc4x1, SchemaVersion::Ifc4x2] {
        assert!(matches!(
            CostSchedule::new(EntityId(1), &entity, version),
            Err(CostError::UnsupportedSchema { .. })
        ));
    }
}
