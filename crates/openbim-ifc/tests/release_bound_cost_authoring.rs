//! Cost records authored in IFC2X3, IFC4 and IFC4X3 validate against their
//! own release (#202, #203).
//!
//! Each release is authored through the `*_with_owner_history` writers of
//! `ifc-cost` (IFC2X3 requires `IfcRoot.OwnerHistory`), written to STEP,
//! read back with `ifc-step`, and checked by `ifc-validate` against the
//! declared release's table. No record this test wrote may carry an error
//! finding. In IFC4 and IFC4X3 the cost item also carries quantities,
//! `IfcQuantityNumber` included in IFC4X3 (#203).

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
    QuantityDraft {
        kind,
        name: "Measured",
        description: None,
        unit: None,
        value: 3.0,
        formula: None,
    }
}

/// A schedule, two nested items and the assignment, with what `version`
/// can hold.
fn author(model: &mut Model, version: SchemaVersion) -> Vec<EntityId> {
    let ifc4 = version != SchemaVersion::Ifc2x3;
    let mut tx = Transaction::new(model);
    let schedule = CostScheduleDraft {
        global_id: "0YvctVUKr0kugbFTf53O08",
        name: Some("Estimate"),
        description: None,
        object_type: None,
        identification: Some("CS-1"),
        predefined_type: Some(CostScheduleType::Estimate),
        status: Some("DRAFT"),
        submitted_on: ifc4.then_some("2026-09-28T00:00:00"),
        update_date: None,
    };
    let schedule =
        create_cost_schedule_with_owner_history(&mut tx, model, schedule, OWNER).expect("plan");
    let item = |global_id| CostItemDraft {
        global_id,
        name: Some("Excavation"),
        description: None,
        object_type: None,
        identification: ifc4.then_some("1.1"),
        predefined_type: ifc4.then_some(CostItemType::NotDefined),
        cost_values: &[],
    };
    let parent =
        create_cost_item_with_owner_history(&mut tx, model, item("0YvctVUKr0kugbFTf53O09"), OWNER)
            .expect("item");
    let child =
        create_cost_item_with_owner_history(&mut tx, model, item("0YvctVUKr0kugbFTf53O0A"), OWNER)
            .expect("item");
    let nesting = NestingDraft {
        global_id: "0YvctVUKr0kugbFTf53O0B",
        parent,
        children: &[child],
    };
    let nest = nest_cost_items_with_owner_history(&mut tx, model, nesting, OWNER).expect("nest");
    let assignment = ScheduleAssignmentDraft {
        global_id: "0YvctVUKr0kugbFTf53O0C",
        schedule,
        items: &[parent],
    };
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
