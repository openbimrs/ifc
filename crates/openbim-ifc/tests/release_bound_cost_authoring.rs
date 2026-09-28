//! Cost records authored in IFC2X3, IFC4 and IFC4X3 validate against their
//! own release (#202, #203).
//!
//! Each release is authored through the `*_with_owner_history` writers of
//! `ifc-cost` (IFC2X3 requires `IfcRoot.OwnerHistory`), written to STEP,
//! read back with `ifc-step`, and checked by `ifc-validate` against the
//! declared release's table. No record this test wrote may carry an error
//! finding. In IFC4 and IFC4X3 the cost item also carries quantities,
//! `IfcQuantityNumber` included in IFC4X3 (#203). In IFC2X3 the schedule's
//! `SubmittedOn` and `UpdateDate` are `IfcDateTimeSelect` records the writer
//! stages (#214); in IFC4 and IFC4X3 they are `IfcDateTime` text.

#![cfg(all(
    feature = "validate",
    feature = "cost",
    feature = "schema",
    feature = "step"
))]

use ifc::cost::mutation::{
    assign_cost_quantities, assign_schedule_items_with_owner_history,
    create_cost_item_with_owner_history, create_cost_schedule_with_owner_history, create_quantity,
    nest_cost_items_with_owner_history, CostItemDraft, CostItemType, CostScheduleDraft,
    CostScheduleType, NestingDraft, QuantityDraft, QuantityKind, ScheduleAssignmentDraft,
};
use ifc::cost::{CalendarDate, CostView, DateTimeValue, LocalTime};
use ifc::schema::{for_version, SchemaVersion};
use ifc::{Codec, Model, StepCodec};
use ifc_model::{EntityId, Transaction};

const OWNER: EntityId = EntityId(5);

/// Actors and an owner history (`#5`) in `schema`.
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
    StepCodec.read_bytes(text.as_bytes()).expect("parses")
}

fn quantity(kind: QuantityKind) -> QuantityDraft<'static> {
    QuantityDraft::new(kind, "Measured", 3.0)
}

/// A schedule, two nested items and the assignment, with what `version`
/// can hold.
fn author(model: &mut Model, version: SchemaVersion) -> Vec<EntityId> {
    let ifc4 = version != SchemaVersion::Ifc2x3;
    let mut tx = Transaction::new(model);
    let (submitted, updated): (DateTimeValue<'_>, DateTimeValue<'_>) = if ifc4 {
        ("2026-09-28T00:00:00".into(), "2026-09-29T12:30:00".into())
    } else {
        let time = LocalTime::new(12).minute(30).second(15.5);
        (
            CalendarDate::new(2024, 2, 29).into(),
            DateTimeValue::DateAndTime(CalendarDate::new(2026, 9, 29), time),
        )
    };
    let schedule = CostScheduleDraft::new("0YvctVUKr0kugbFTf53O08")
        .name("Estimate")
        .identification("CS-1")
        .predefined_type(CostScheduleType::Estimate)
        .status("DRAFT")
        .submitted_on(submitted)
        .update_date(updated);
    let schedule =
        create_cost_schedule_with_owner_history(&mut tx, model, schedule, OWNER).expect("plan");
    let item = |global_id| {
        let mut draft = CostItemDraft::new(global_id).name("Excavation");
        draft.identification = ifc4.then_some("1.1");
        draft.predefined_type = ifc4.then_some(CostItemType::NotDefined);
        draft
    };
    let parent =
        create_cost_item_with_owner_history(&mut tx, model, item("0YvctVUKr0kugbFTf53O09"), OWNER)
            .expect("item");
    let child =
        create_cost_item_with_owner_history(&mut tx, model, item("0YvctVUKr0kugbFTf53O0A"), OWNER)
            .expect("item");
    let children = [child];
    let nesting = NestingDraft::new("0YvctVUKr0kugbFTf53O0B", parent, &children);
    let nest = nest_cost_items_with_owner_history(&mut tx, model, nesting, OWNER).expect("nest");
    let items = [parent];
    let assignment = ScheduleAssignmentDraft::new("0YvctVUKr0kugbFTf53O0C", schedule, &items);
    let assign = assign_schedule_items_with_owner_history(&mut tx, model, assignment, OWNER)
        .expect("assign");
    let mut written = vec![schedule, parent, child, nest, assign];
    if ifc4 {
        let mut quantities =
            vec![create_quantity(&mut tx, model, quantity(QuantityKind::Area)).expect("area")];
        if version == SchemaVersion::Ifc4x3 {
            quantities.push(
                create_quantity(&mut tx, model, quantity(QuantityKind::Number)).expect("number"),
            );
        }
        assign_cost_quantities(&mut tx, model, parent, &quantities).expect("quantities");
        written.extend(quantities);
    }
    tx.commit(model).expect("commit");
    // The IFC2X3 date records the schedule references, and theirs.
    let mut dates: Vec<EntityId> = model
        .get(schedule)
        .expect("schedule")
        .attributes
        .iter()
        .filter_map(|value| match value {
            ifc::Value::Ref(id) if *id != OWNER => Some(*id),
            _ => None,
        })
        .collect();
    let mut at = 0;
    while at < dates.len() {
        let nested: Vec<EntityId> = model
            .get(dates[at])
            .expect("date")
            .attributes
            .iter()
            .filter_map(|value| match value {
                ifc::Value::Ref(id) => Some(*id),
                _ => None,
            })
            .collect();
        dates.extend(nested);
        at += 1;
    }
    written.extend(dates);
    written
}

/// Error findings `ifc-validate` reports on `ids`, against `version`.
fn errors(model: &Model, version: SchemaVersion, ids: &[EntityId]) -> Vec<String> {
    let report = ifc_validate::validate(model, for_version(version).expect("bundled"));
    report
        .findings()
        .iter()
        .filter(|finding| finding.severity == ifc_validate::Severity::Error)
        .filter(|finding| match &finding.path {
            ifc_validate::Path::Entity(id) | ifc_validate::Path::Attribute { entity: id, .. } => {
                ids.contains(id)
            }
            ifc_validate::Path::File => false,
        })
        .map(|finding| format!("{} at {}: {}", finding.rule, finding.path, finding.message))
        .collect()
}

#[test]
fn authored_cost_records_validate_in_their_release() {
    for (schema, version) in [
        ("IFC2X3", SchemaVersion::Ifc2x3),
        ("IFC4", SchemaVersion::Ifc4),
        ("IFC4X3_ADD2", SchemaVersion::Ifc4x3),
    ] {
        let mut model = base(schema);
        let written = author(&mut model, version);
        let bytes = StepCodec.write_bytes(&model).expect("written");
        let back = StepCodec.read_bytes(&bytes).expect("read back");
        assert!(back.diagnostics().is_empty(), "{:?}", back.diagnostics());
        let found = errors(&back, version, &written);
        assert!(found.is_empty(), "{schema}:\n  {}", found.join("\n  "));
        let plan = CostView::new(&back).schedules().next().expect("schedule");
        assert_eq!(plan.name(), Some("Estimate"), "{schema}");
        if version == SchemaVersion::Ifc2x3 {
            // SubmittedOn (7) and UpdateDate (10) reference their records.
            let record = back.get(plan.id()).expect("schedule");
            let referenced = |slot: usize| match &record.attributes[slot] {
                ifc::Value::Ref(id) => back.get(*id).expect("date"),
                other => panic!("slot {slot}: {other:?}"),
            };
            let date = referenced(7);
            assert_eq!(&*date.type_name, "IFCCALENDARDATE");
            let int = ifc::Value::Integer;
            assert_eq!(date.attributes, vec![int(29), int(2), int(2024)]);
            let both = referenced(10);
            assert_eq!(&*both.type_name, "IFCDATEANDTIME");
            let ifc::Value::Ref(time) = both.attributes[1] else {
                panic!("TimeComponent");
            };
            let time = back.get(time).expect("time");
            assert_eq!(&*time.type_name, "IFCLOCALTIME");
            assert_eq!(
                time.attributes[..3],
                [int(12), int(30), ifc::Value::Real(15.5)]
            );
            assert_eq!(written.len(), 9, "the three date records are validated");
        } else {
            assert_eq!(plan.submitted_on(), Some("2026-09-28T00:00:00"), "{schema}");
            assert_eq!(plan.update_date(), Some("2026-09-29T12:30:00"), "{schema}");
        }
    }
}

/// The oracle is trusted because it fails when it should: the IFC4
/// nine-attribute `IfcCostItem` with `$` owner history, as the writer
/// produced before #202, is an error finding in IFC2X3.
#[test]
fn the_validator_catches_an_ifc4_cost_item_in_ifc2x3() {
    let mut model = base("IFC2X3");
    let mut tx = Transaction::new(&model);
    let mut attributes = vec![ifc::Value::Null; 9];
    attributes[0] = ifc::Value::Text("0YvctVUKr0kugbFTf53O09".into());
    let item = tx.create(ifc::Entity::new("IFCCOSTITEM", attributes));
    tx.commit(&mut model).expect("commit");
    assert!(
        !errors(&model, SchemaVersion::Ifc2x3, &[item]).is_empty(),
        "a nine-attribute unowned IfcCostItem is not IFC2X3"
    );
}
