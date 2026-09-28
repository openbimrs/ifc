//! The writers #212 and #213 fixed, authored in IFC2X3, IFC4 and IFC4X3,
//! validate against their own release.
//!
//! `ifc-approval`'s `create_approval`, `relate_approvals` and
//! `relate_resource_approval`, `ifc-constraint`'s `create_metric`,
//! `create_objective`, `relate_resource_constraint` and `create_reference`,
//! `ifc-cost`'s `create_cost_value`, `create_monetary_unit` and
//! `create_currency_relationship`, and `ifc-spatial`'s `assign_to_actor`,
//! `assign_to_process`, `connect_with_realizing_elements` and
//! `interfere_elements` each write every record the release can hold. The
//! result is written to STEP, read back with `ifc-step`, checked by
//! `ifc-validate` against the declared release's table (no record this
//! test wrote may carry an error finding), and read back through the
//! crates' views. In IFC2X3 the spatial relationships go through their
//! `*_with_owner_history` variants, since the plain writers refuse a model
//! that requires `OwnerHistory`.

#![cfg(all(
    feature = "validate",
    feature = "schema",
    feature = "step",
    feature = "approval",
    feature = "constraint",
    feature = "cost",
    feature = "spatial"
))]

use ifc::approval::{
    create_approval, relate_approvals, relate_resource_approval, ApprovalDraft,
    ApprovalRelationshipDraft, ApprovalView, ResourceApprovalDraft,
};
use ifc::constraint::{
    create_metric, create_objective, create_reference, relate_resource_constraint, Benchmark,
    ConstraintBaseDraft, ConstraintGrade, ConstraintView, MetricDraft, MetricValueDraft,
    ObjectiveDraft, ObjectiveQualifier, ReferenceDraft, ResourceConstraintDraft,
};
use ifc::cost::{
    create_cost_value, create_currency_relationship, create_monetary_unit, CalendarDate,
    CostValueDraft, DateTimeValue, LocalTime,
};
use ifc::schema::for_version;
use ifc::spatial::authoring::{
    assign_to_actor, assign_to_actor_with_owner_history, assign_to_process,
    assign_to_process_with_owner_history, connect_with_realizing_elements,
    connect_with_realizing_elements_with_owner_history, interfere_elements,
};
use ifc::{Codec, Model, SchemaVersion, StepCodec, Value};
use ifc_model::{EntityId, Transaction};

const RELEASES: [(&str, SchemaVersion); 3] = [
    ("IFC2X3", SchemaVersion::Ifc2x3),
    ("IFC4", SchemaVersion::Ifc4),
    ("IFC4X3_ADD2", SchemaVersion::Ifc4x3),
];
const OWNER: EntityId = EntityId(5);
const WALL: EntityId = EntityId(10);
const WALL2: EntityId = EntityId(11);
const PROXY: EntityId = EntityId(12);
const ACTOR: EntityId = EntityId(13);
const TASK: EntityId = EntityId(14);
const DATE: EntityId = EntityId(21);

/// Actors, an owner history (`#5`), two walls, a proxy, an actor, a task
/// and, in IFC2X3, a calendar date (`#21`), in `schema`.
fn base(schema: &str, version: SchemaVersion) -> Model {
    let table = for_version(version).unwrap();
    let tail = |entity: &str| ",$".repeat(table.attributes(entity).len() - 3);
    let (wall, proxy) = (tail("IFCWALL"), tail("IFCBUILDINGELEMENTPROXY"));
    let (task, date) = if version == SchemaVersion::Ifc2x3 {
        (
            "#14=IFCTASK('0YvctVUKr0kugbFTf53O0F',#5,'Task',$,$,'T-1',$,$,.F.,$);",
            "#21=IFCCALENDARDATE(28,9,2026);",
        )
    } else {
        (
            "#14=IFCTASK('0YvctVUKr0kugbFTf53O0F',#5,'Task',$,$,$,$,$,$,.F.,$,$,$);",
            "",
        )
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
         #11=IFCWALL('1xS3BCk291UvhgP2dvNsgq',#5,'W2'{wall});\n\
         #12=IFCBUILDINGELEMENTPROXY('1xS3BCk291UvhgP2dvNsgr',#5,'Bolt'{proxy});\n\
         #13=IFCACTOR('0YvctVUKr0kugbFTf53O0E',#5,'Owner',$,$,#2);\n\
         {task}\n{date}\n\
         ENDSEC;\nEND-ISO-10303-21;\n"
    );
    let model = StepCodec.read_bytes(text.as_bytes()).expect("parses");
    assert!(model.diagnostics().is_empty(), "{:?}", model.diagnostics());
    model
}

/// A fresh, unique GlobalId per record.
fn guid(n: usize) -> String {
    let mut bytes = [0u8; 16];
    bytes[..8].copy_from_slice(&(n as u64 + 1).to_le_bytes());
    ifc_model::guid::Guid::from_uuid(bytes).to_string()
}

/// Every writer of #212 and #213 `version` can hold; returns the records
/// written, date records the cost writers staged included.
#[allow(clippy::too_many_lines)]
fn author(model: &mut Model, version: SchemaVersion) -> Vec<EntityId> {
    let ifc2x3 = version == SchemaVersion::Ifc2x3;
    let m = model.clone();
    let mut tx = Transaction::new(model);
    let mut written = Vec::new();

    // ifc-approval
    let approval = |identifier| {
        let draft = ApprovalDraft::new()
            .identifier(identifier)
            .name("Review")
            .description("Stage 3");
        if ifc2x3 {
            draft.time_of_approval(DATE)
        } else {
            draft
                .time_of_approval("2026-09-28T10:00:00")
                .status("Approved")
                .level("Final")
                .giving_approval(EntityId(3))
        }
    };
    let a1 = create_approval(&mut tx, &m, approval("A-1")).expect("approval");
    let a2 = create_approval(&mut tx, &m, approval("A-2")).expect("approval");
    let related = [a2];
    let rel = relate_approvals(
        &mut tx,
        &m,
        ApprovalRelationshipDraft::new(a1, &related).name("Supersedes"),
    )
    .expect("approval relationship");
    written.extend([a1, a2, rel]);
    let resources = [EntityId(2)];
    let resource =
        relate_resource_approval(&mut tx, &m, ResourceApprovalDraft::new(&resources, a1));
    if ifc2x3 {
        assert!(resource.is_err(), "IFC2X3 has no resource approvals");
    } else {
        written.push(resource.expect("resource approval"));
    }

    // ifc-constraint
    let text = Value::Text("REI 90".into());
    let base = |name| {
        let draft = ConstraintBaseDraft::new(name, ConstraintGrade::Hard).source("Code");
        if ifc2x3 {
            draft.creation_time(DATE)
        } else {
            draft.creation_time("2026-09-28T10:00:00")
        }
    };
    let metric = create_metric(
        &mut tx,
        &m,
        MetricDraft::new(base("Rating"), Benchmark::EqualTo).data_value(MetricValueDraft::Typed {
            type_name: "IfcText",
            value: &text,
        }),
    )
    .expect("metric");
    let benchmarks = [metric];
    let objective = create_objective(
        &mut tx,
        &m,
        ObjectiveDraft::new(base("Objective"), ObjectiveQualifier::DesignIntent)
            .benchmark_values(&benchmarks),
    )
    .expect("objective");
    written.extend([metric, objective]);
    let reference = create_reference(
        &mut tx,
        &m,
        ReferenceDraft::new().attribute_identifier("Name"),
    );
    let constrained = relate_resource_constraint(
        &mut tx,
        &m,
        ResourceConstraintDraft::new(metric, &resources),
    );
    if ifc2x3 {
        assert!(reference.is_err() && constrained.is_err(), "IFC4 on only");
    } else {
        written.extend([
            reference.expect("reference"),
            constrained.expect("resource"),
        ]);
    }

    // ifc-cost
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
            .category("Labour")
            .applicable_date(applicable),
    )
    .expect("cost value");
    let eur = create_monetary_unit(&mut tx, &m, "EUR").expect("eur");
    let gbp = create_monetary_unit(&mut tx, &m, "GBP").expect("gbp");
    let rate =
        create_currency_relationship(&mut tx, &m, eur, gbp, 0.85, Some(rate_date)).expect("rate");
    written.extend([value, eur, gbp, rate]);

    // ifc-spatial
    let mut n = 0;
    let mut next = || {
        n += 1;
        guid(n)
    };
    if ifc2x3 {
        assert!(assign_to_actor(&mut tx, &m, &next(), ACTOR, &[WALL]).is_err());
        assert!(interfere_elements(&mut tx, &m, &next(), WALL, WALL2, None).is_err());
        written.extend([
            assign_to_actor_with_owner_history(&mut tx, &m, &next(), ACTOR, &[WALL], OWNER)
                .expect("actor"),
            assign_to_process_with_owner_history(&mut tx, &m, &next(), TASK, &[WALL], OWNER)
                .expect("process"),
            connect_with_realizing_elements_with_owner_history(
                &mut tx,
                &m,
                &next(),
                WALL,
                WALL2,
                &[PROXY],
                OWNER,
            )
            .expect("realizing"),
        ]);
    } else {
        written.extend([
            assign_to_actor(&mut tx, &m, &next(), ACTOR, &[WALL]).expect("actor"),
            assign_to_process(&mut tx, &m, &next(), TASK, &[WALL]).expect("process"),
            connect_with_realizing_elements(&mut tx, &m, &next(), WALL, WALL2, &[PROXY])
                .expect("realizing"),
            interfere_elements(&mut tx, &m, &next(), WALL, WALL2, Some(true)).expect("clash"),
        ]);
    }
    tx.commit(model).expect("commit");

    // The date records the cost writers staged, and theirs.
    let mut at = 0;
    while at < written.len() {
        let entity = model.get(written[at]).expect("written");
        if matches!(
            &*entity.type_name,
            "IFCCOSTVALUE" | "IFCCURRENCYRELATIONSHIP" | "IFCDATEANDTIME"
        ) {
            for value in &entity.attributes {
                if let Value::Ref(id) = value {
                    let staged = model.get(*id).expect("reference");
                    if staged.type_name.starts_with("IFCCALENDARDATE")
                        || staged.type_name.starts_with("IFCLOCALTIME")
                        || staged.type_name.starts_with("IFCDATEANDTIME")
                    {
                        if !written.contains(id) {
                            written.push(*id);
                        }
                    }
                }
            }
        }
        at += 1;
    }
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
            // A path kind a later ifc-validate adds names no authored record.
            _ => false,
        })
        .map(|finding| format!("{} at {}: {}", finding.rule, finding.path, finding.message))
        .collect()
}

#[test]
fn authored_records_validate_in_their_release() {
    for (schema, version) in RELEASES {
        let mut model = base(schema, version);
        let written = author(&mut model, version);
        let bytes = StepCodec.write_bytes(&model).expect("written");
        let back = StepCodec.read_bytes(&bytes).expect("read back");
        assert!(back.diagnostics().is_empty(), "{:?}", back.diagnostics());
        let found = errors(&back, version, &written);
        assert!(found.is_empty(), "{schema}:\n  {}", found.join("\n  "));
        // Every record has its release's arity.
        let table = for_version(version).unwrap();
        for id in &written {
            let entity = back.get(*id).expect("written");
            assert_eq!(
                entity.attributes.len(),
                table.attributes(&entity.type_name).len(),
                "{schema} #{} {}",
                id.0,
                entity.type_name
            );
        }
        // And reads back through the views.
        let approval = ApprovalView::new(&back)
            .approval(written[0])
            .expect("approval");
        assert_eq!(approval.identifier().unwrap(), Some("A-1"), "{schema}");
        assert_eq!(approval.name().unwrap(), Some("Review"), "{schema}");
        let metric = back
            .of_type("IFCMETRIC")
            .next()
            .map(|(id, _)| id)
            .expect("metric");
        let view = ConstraintView::new(&back);
        assert_eq!(view.metric(metric).unwrap().name().unwrap(), "Rating");
    }
}

/// The oracle is trusted because it fails when it should: the IFC4-layout
/// `IfcCostValue` the writer used to stage in IFC2X3 is an error there.
#[test]
fn the_validator_catches_an_ifc4_layout_in_ifc2x3() {
    let mut model = base("IFC2X3", SchemaVersion::Ifc2x3);
    let mut tx = Transaction::new(&model);
    let id = tx.create(ifc::Entity::new(
        "IFCCOSTVALUE",
        vec![
            Value::Null,
            Value::Null,
            Value::Null,
            Value::Null,
            Value::Null,
            Value::Null,
            Value::Text("Labour".into()),
            Value::Null,
            Value::Null,
            Value::Null,
        ],
    ));
    tx.commit(&mut model).expect("commit");
    assert!(
        !errors(&model, SchemaVersion::Ifc2x3, &[id]).is_empty(),
        "ten attributes where IFC2X3 declares eight"
    );
}
