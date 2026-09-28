//! Cost `IfcRoot` authoring bound to the declared release (#202).
//!
//! From the EXPRESS sources: IFC2X3 TC1 declares `IfcCostItem` with the
//! five `IfcControl` attributes only and `IfcCostSchedule` with thirteen
//! (`ID` and `PredefinedType` required, dates as `IfcDateTimeSelect`
//! records); `IfcRoot.OwnerHistory` is required. IFC4 ADD2 TC1 and IFC4X3
//! ADD2 declare nine and ten, `OwnerHistory` optional.

use ifc_cost::mutation::{
    assign_schedule_items, assign_schedule_items_with_owner_history, create_cost_item,
    create_cost_item_with_owner_history, create_cost_schedule,
    create_cost_schedule_with_owner_history, create_cost_value, nest_cost_items,
    nest_cost_items_with_owner_history, CostAuthoringError, CostItemDraft, CostItemType,
    CostScheduleDraft, CostScheduleType, CostValueDraft, NestingDraft, ScheduleAssignmentDraft,
};
use ifc_cost::{children_of, controlled_by, CostView, SchemaVersion};
use ifc_model::{Codec, Edit, Entity, EntityId, Model, Transaction, Value};
use ifc_step::StepCodec;

const OWNER: EntityId = EntityId(5);
const WALL: EntityId = EntityId(10);
const G: [&str; 6] = [
    "0YvctVUKr0kugbFTf53O08",
    "0YvctVUKr0kugbFTf53O09",
    "0YvctVUKr0kugbFTf53O0A",
    "0YvctVUKr0kugbFTf53O0B",
    "0YvctVUKr0kugbFTf53O0C",
    "0YvctVUKr0kugbFTf53O0D",
];

const RELEASES: [(&str, SchemaVersion); 3] = [
    ("IFC2X3", SchemaVersion::Ifc2x3),
    ("IFC4", SchemaVersion::Ifc4),
    ("IFC4X3_ADD2", SchemaVersion::Ifc4x3),
];

/// An owner history (`#5`) with its actors and a wall (`#10`).
fn base(schema: &str) -> Model {
    let wall = if schema == "IFC2X3" {
        ",$,$,$,$,$"
    } else {
        ",$,$,$,$,$,$"
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
         ENDSEC;\nEND-ISO-10303-21;\n"
    );
    let model = StepCodec.read_bytes(text.as_bytes()).expect("parses");
    assert!(model.diagnostics().is_empty(), "{:?}", model.diagnostics());
    model
}

fn item(global_id: &str, ifc4: bool) -> CostItemDraft<'_> {
    CostItemDraft {
        global_id,
        name: Some("Excavation"),
        description: Some("Bulk dig"),
        object_type: None,
        identification: ifc4.then_some("1.1"),
        predefined_type: ifc4.then_some(CostItemType::NotDefined),
        cost_values: &[],
    }
}

fn schedule(ifc4: bool) -> CostScheduleDraft<'static> {
    CostScheduleDraft {
        global_id: G[0],
        name: Some("Estimate"),
        description: None,
        object_type: None,
        identification: Some("CS-1"),
        predefined_type: Some(CostScheduleType::Estimate),
        status: Some("DRAFT"),
        submitted_on: ifc4.then_some("2026-09-28T00:00:00"),
        update_date: None,
    }
}

fn staged(tx: &Transaction, id: EntityId) -> Entity {
    tx.edits()
        .iter()
        .find_map(|edit| match edit {
            Edit::Create { id: staged, entity } if *staged == id => Some(entity.clone()),
            _ => None,
        })
        .expect("staged")
}

/// Each release round-trips through STEP and reads back through the crate's
/// relationship views; the records have the release's own arity.
#[test]
fn cost_records_round_trip_in_their_release() {
    for (schema, version) in RELEASES {
        let ifc4 = version != SchemaVersion::Ifc2x3;
        let mut model = base(schema);
        let mut tx = Transaction::new(&model);
        let plan = create_cost_schedule_with_owner_history(&mut tx, &model, schedule(ifc4), OWNER)
            .expect(schema);
        let parent = create_cost_item_with_owner_history(&mut tx, &model, item(G[1], ifc4), OWNER)
            .expect(schema);
        let child = create_cost_item_with_owner_history(&mut tx, &model, item(G[2], ifc4), OWNER)
            .expect(schema);
        let nests = NestingDraft {
            global_id: G[3],
            parent,
            children: &[child],
        };
        let nest = nest_cost_items_with_owner_history(&mut tx, &model, nests, OWNER).expect(schema);
        let assignment = ScheduleAssignmentDraft {
            global_id: G[4],
            schedule: plan,
            items: &[parent],
        };
        let assign = assign_schedule_items_with_owner_history(&mut tx, &model, assignment, OWNER)
            .expect(schema);
        tx.commit(&mut model).expect("commit");

        let bytes = StepCodec.write_bytes(&model).expect("written");
        let back = StepCodec.read_bytes(&bytes).expect("read back");
        assert!(back.diagnostics().is_empty(), "{:?}", back.diagnostics());
        let arity = |id| back.get(id).expect("read back").attributes.len();
        let (schedule_arity, item_arity) = if ifc4 { (10, 9) } else { (13, 5) };
        assert_eq!(arity(plan), schedule_arity, "{schema}");
        assert_eq!(arity(parent), item_arity, "{schema}");
        for id in [plan, parent, child, nest, assign] {
            assert_eq!(back.get(id).unwrap().attributes[1], Value::Ref(OWNER));
        }
        if !ifc4 {
            // IFC2X3 `ID` (slot 11) and `PredefinedType` (slot 12).
            let plan = back.get(plan).unwrap();
            assert_eq!(plan.attributes[11], Value::Text("CS-1".into()));
            assert_eq!(plan.attributes[12], Value::Enum("ESTIMATE".into()));
            assert_eq!(plan.attributes[8], Value::Text("DRAFT".into()), "Status");
        }
        assert_eq!(children_of(&back, parent), vec![child], "{schema}");
        assert_eq!(controlled_by(&back, plan), vec![parent], "{schema}");
        let view = CostView::new(&back);
        let names: Vec<_> = view.items().map(|i| i.name()).collect();
        assert_eq!(names, vec![Some("Excavation"); 2], "{schema}");
    }
}

/// IFC4 and IFC4X3 output is the pre-#202 positional record, slot for slot,
/// and the new variants write the same record with the reference.
#[test]
fn ifc4_and_ifc4x3_records_are_unchanged() {
    let t = |s: &str| Value::Text(s.into());
    let e = |s: &str| Value::Enum(s.into());
    for (schema, _) in &RELEASES[1..] {
        let model = base(schema);
        let mut tx = Transaction::new(&model);
        let value = create_cost_value(&mut tx, &model, CostValueDraft::monetary(10.0)).unwrap();
        let draft = CostItemDraft {
            cost_values: &[value],
            ..item(G[1], true)
        };
        let plain = create_cost_item(&mut tx, &model, draft).unwrap();
        let draft = CostItemDraft {
            global_id: G[2],
            ..draft
        };
        let with = create_cost_item_with_owner_history(&mut tx, &model, draft, OWNER).unwrap();
        let mut expected = vec![
            t(G[1]),
            Value::Null,
            t("Excavation"),
            t("Bulk dig"),
            Value::Null,
            t("1.1"),
            e("NOTDEFINED"),
            Value::List(vec![Value::Ref(value)]),
            Value::Null,
        ];
        assert_eq!(staged(&tx, plain).attributes, expected, "{schema}");
        expected[0] = t(G[2]);
        expected[1] = Value::Ref(OWNER);
        assert_eq!(staged(&tx, with).attributes, expected, "{schema}");

        let plain = create_cost_schedule(&mut tx, &model, schedule(true)).unwrap();
        let draft = CostScheduleDraft {
            global_id: G[5],
            ..schedule(true)
        };
        let with = create_cost_schedule_with_owner_history(&mut tx, &model, draft, OWNER).unwrap();
        let mut expected = vec![
            t(G[0]),
            Value::Null,
            t("Estimate"),
            Value::Null,
            Value::Null,
            t("CS-1"),
            e("ESTIMATE"),
            t("DRAFT"),
            t("2026-09-28T00:00:00"),
            Value::Null,
        ];
        assert_eq!(staged(&tx, plain).attributes, expected, "{schema}");
        expected[0] = t(G[5]);
        expected[1] = Value::Ref(OWNER);
        assert_eq!(staged(&tx, with).attributes, expected, "{schema}");
    }
}

/// The two relationships, in IFC4 and IFC4X3, before and after.
#[test]
fn ifc4_and_ifc4x3_relationships_are_unchanged() {
    let t = |s: &str| Value::Text(s.into());
    for (schema, _) in &RELEASES[1..] {
        let model = base(schema);
        let mut tx = Transaction::new(&model);
        let plan = create_cost_schedule(&mut tx, &model, schedule(true)).unwrap();
        let a = create_cost_item(&mut tx, &model, item(G[1], true)).unwrap();
        let b = create_cost_item(&mut tx, &model, item(G[2], true)).unwrap();
        let c = create_cost_item(&mut tx, &model, item(G[3], true)).unwrap();
        let plain = nest_cost_items(
            &mut tx,
            &model,
            NestingDraft {
                global_id: G[4],
                parent: a,
                children: &[b],
            },
        )
        .unwrap();
        let with = nest_cost_items_with_owner_history(
            &mut tx,
            &model,
            NestingDraft {
                global_id: G[5],
                parent: a,
                children: &[c],
            },
            OWNER,
        )
        .unwrap();
        let rel = |guid, owner, list: EntityId| {
            vec![
                t(guid),
                owner,
                Value::Null,
                Value::Null,
                Value::Ref(a),
                Value::List(vec![Value::Ref(list)]),
            ]
        };
        assert_eq!(staged(&tx, plain).attributes, rel(G[4], Value::Null, b));
        assert_eq!(
            staged(&tx, with).attributes,
            rel(G[5], Value::Ref(OWNER), c)
        );

        let items = [a];
        let assignment = |global_id| ScheduleAssignmentDraft {
            global_id,
            schedule: plan,
            items: &items,
        };
        let guid = "1kTvXnbbzCWw8lcMd1dR4o";
        let plain = assign_schedule_items(&mut tx, &model, assignment(guid)).unwrap();
        let guid2 = "1kTvXnbbzCWw8lcMd1dR4p";
        let with =
            assign_schedule_items_with_owner_history(&mut tx, &model, assignment(guid2), OWNER)
                .unwrap();
        let rel = |guid, owner| {
            vec![
                t(guid),
                owner,
                Value::Null,
                Value::Null,
                Value::List(vec![Value::Ref(a)]),
                Value::Null,
                Value::Ref(plan),
            ]
        };
        assert_eq!(staged(&tx, plain).attributes, rel(guid, Value::Null));
        assert_eq!(staged(&tx, with).attributes, rel(guid2, Value::Ref(OWNER)));
    }
}

fn refused(tx: &Transaction, result: Result<EntityId, CostAuthoringError>) -> CostAuthoringError {
    assert!(tx.is_empty(), "a refusal staged {:?}", tx.edits());
    result.expect_err("refused")
}

/// IFC2X3 refuses what it cannot hold, with a typed error, staging nothing.
#[test]
fn ifc2x3_refuses_what_it_cannot_hold() {
    let v = SchemaVersion::Ifc2x3;
    let model = base("IFC2X3");
    let mut tx = Transaction::new(&model);
    let required = |entity, attribute| CostAuthoringError::AuthoringRequired {
        entity,
        attribute,
        schema: v,
    };
    let result = create_cost_item(&mut tx, &model, item(G[1], false));
    assert_eq!(
        refused(&tx, result),
        required("IFCCOSTITEM", "OwnerHistory")
    );
    let result = create_cost_schedule(&mut tx, &model, schedule(false));
    assert_eq!(
        refused(&tx, result),
        required("IFCCOSTSCHEDULE", "OwnerHistory")
    );
    let result = create_cost_item_with_owner_history(&mut tx, &model, item(G[1], true), OWNER);
    assert_eq!(
        refused(&tx, result),
        CostAuthoringError::AuthoringNotInSchema {
            entity: "IFCCOSTITEM",
            attribute: "Identification",
            schema: v,
        }
    );
    let result = create_cost_schedule_with_owner_history(&mut tx, &model, schedule(true), OWNER);
    assert_eq!(
        refused(&tx, result),
        CostAuthoringError::AuthoringValueType {
            entity: "IFCCOSTSCHEDULE",
            attribute: "SubmittedOn",
            declared: "IfcDateTimeSelect",
            schema: v,
        }
    );
    let unidentified = CostScheduleDraft {
        identification: None,
        ..schedule(false)
    };
    let result = create_cost_schedule_with_owner_history(&mut tx, &model, unidentified, OWNER);
    assert_eq!(refused(&tx, result), required("IFCCOSTSCHEDULE", "ID"));
    let untyped = CostScheduleDraft {
        predefined_type: None,
        ..schedule(false)
    };
    let result = create_cost_schedule_with_owner_history(&mut tx, &model, untyped, OWNER);
    assert_eq!(
        refused(&tx, result),
        required("IFCCOSTSCHEDULE", "PredefinedType")
    );
}

/// The owner history must exist and be an `IfcOwnerHistory`; a header must
/// bind exactly one known release.
#[test]
fn owner_history_and_binding_are_checked() {
    let model = base("IFC2X3");
    let mut tx = Transaction::new(&model);
    let result = create_cost_item_with_owner_history(&mut tx, &model, item(G[1], false), WALL);
    assert_eq!(
        refused(&tx, result),
        CostAuthoringError::WrongReferenceType {
            entity: "IFCCOSTITEM",
            attribute: "OwnerHistory",
            target: WALL,
            actual: "IFCWALL".into(),
            expected: "IFCOWNERHISTORY",
        }
    );
    let missing = EntityId(99);
    let result = create_cost_item_with_owner_history(&mut tx, &model, item(G[1], false), missing);
    assert_eq!(
        refused(&tx, result),
        CostAuthoringError::MissingReference {
            entity: "IFCCOSTITEM",
            attribute: "OwnerHistory",
            target: missing,
        }
    );
    let mut several = base("IFC2X3");
    several.header_mut().schema = vec!["IFC4".into(), "IFC2X3".into()];
    let result = create_cost_item_with_owner_history(&mut tx, &several, item(G[1], false), OWNER);
    assert_eq!(
        refused(&tx, result),
        CostAuthoringError::MultipleSchemas { schemas: 2 }
    );
    let mut unknown = base("IFC2X3");
    unknown.header_mut().schema = vec!["IFC5".into()];
    let result = create_cost_item(&mut tx, &unknown, item(G[1], true));
    assert_eq!(
        refused(&tx, result),
        CostAuthoringError::UnsupportedSchema {
            schema: "IFC5".into()
        }
    );
}
