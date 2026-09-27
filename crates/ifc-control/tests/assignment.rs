//! `IfcRelAssignsToControl` for the four controls this crate owns.
//!
//! The rule under test: the crate owning the relating control writes
//! the assignment. These tests prove the positive half for all four
//! controls in both shipped IFC4-family schemas, and the negative half
//! for every control another crate owns.
//!
//! The governed objects are bare records inserted into the model, not
//! staged: the assignment reads only their type, and staging stubs would
//! count them as authored in the coverage report.

use ifc_control::{
    assign_to_control, create_control, ControlAssignmentDraft, ControlDraft, ControlError,
    ControlKind,
};
use ifc_model::{Entity, EntityId, Model, Transaction, Value};
use ifc_schema::{ifc4, ifc4x3, Schema};

const CONTROL_GUID: &str = "0RSPnzHdf5hAmvCJDbRDzy";
const REL_GUID: &str = "1kTvXnbbzCWw8lcMd1dR4o";

/// Slots of `IfcRelAssignsToControl` in both IFC4 ADD2 TC1 and IFC4X3
/// ADD2: `IfcRoot` 0-3, `RelatedObjects` 4, `RelatedObjectsType` 5,
/// `RelatingControl` 6. `slot_constants_agree_with_both_schemas` checks.
const RELATED: usize = 4;
const RELATED_TYPE: usize = 5;
const RELATING: usize = 6;

/// A model holding one bare record per type name, at ids 1..=n.
fn model_with(types: &[&str]) -> (Model, Vec<EntityId>) {
    let mut model = Model::default();
    let ids = (1..)
        .zip(types)
        .map(|(n, type_name)| {
            let id = EntityId(n);
            model.insert(id, Entity::new(*type_name, vec![Value::Null]));
            id
        })
        .collect();
    (model, ids)
}

fn stage_control(tx: &mut Transaction, schema: &Schema, kind: ControlKind) -> EntityId {
    let draft = ControlDraft {
        name: Some("Control"),
        life_cycle_phase: (kind == ControlKind::PerformanceHistory).then_some("OPERATION"),
        ..ControlDraft::default()
    };
    create_control(tx, schema, kind, CONTROL_GUID, None, draft).expect("control")
}

fn draft(control: EntityId, related: &[EntityId]) -> ControlAssignmentDraft<'_> {
    ControlAssignmentDraft {
        global_id: REL_GUID,
        name: Some("Governed work"),
        description: None,
        control,
        related_objects: related,
    }
}

fn invalid_attribute(err: &ControlError) -> Option<&'static str> {
    match err {
        ControlError::AuthoringInvalid { attribute, .. } => Some(attribute),
        _ => None,
    }
}

/// The fixed slot constants above are what both schemas declare.
#[test]
fn slot_constants_agree_with_both_schemas() {
    for schema in [ifc4(), ifc4x3()] {
        let names = schema.attribute_names("IFCRELASSIGNSTOCONTROL");
        assert_eq!(names.len(), 7, "{}", schema.name());
        assert_eq!(names[RELATED], "RelatedObjects", "{}", schema.name());
        assert_eq!(
            names[RELATED_TYPE],
            "RelatedObjectsType",
            "{}",
            schema.name()
        );
        assert_eq!(names[RELATING], "RelatingControl", "{}", schema.name());
    }
}

/// Every owned control accepts an assignment in IFC4 and IFC4X3, and
/// the record carries the related list and the relating control.
#[test]
fn every_owned_control_is_assigned_in_both_schemas() {
    for schema in [ifc4(), ifc4x3()] {
        for kind in ControlKind::ALL {
            let (mut model, ids) = model_with(&["IFCTASK", "IFCWALL"]);
            let mut tx = Transaction::new(&model);
            let control = stage_control(&mut tx, schema, kind);

            let rel = assign_to_control(&mut tx, &model, schema, draft(control, &ids))
                .unwrap_or_else(|e| panic!("{kind:?} in {}: {e}", schema.name()));
            tx.commit(&mut model).expect("commit");

            let record = model.get(rel).expect("relationship");
            assert_eq!(record.type_name.as_ref(), "IFCRELASSIGNSTOCONTROL");
            assert_eq!(record.attributes.len(), 7);
            assert_eq!(record.attributes[0], Value::Text(REL_GUID.into()));
            assert_eq!(record.attributes[2], Value::Text("Governed work".into()));
            assert_eq!(
                record.attributes[RELATED],
                Value::List(vec![Value::Ref(ids[0]), Value::Ref(ids[1])])
            );
            assert_eq!(record.attributes[RELATING], Value::Ref(control));
        }
    }
}

/// A control already committed to the model is found as readily as
/// one staged in the same transaction.
#[test]
fn a_committed_control_is_assignable() {
    let (mut model, ids) = model_with(&["IFCTASK"]);
    let mut tx = Transaction::new(&model);
    let permit = stage_control(&mut tx, ifc4(), ControlKind::Permit);
    tx.commit(&mut model).expect("commit");

    let mut tx = Transaction::new(&model);
    assign_to_control(&mut tx, &model, ifc4(), draft(permit, &ids)).expect("assignment");
    assert_eq!(tx.len(), 1);
}

/// `RelatedObjectsType` is left unset: optional in IFC4, where its WR1
/// passes when absent, and a stripped BOOLEAN in IFC4X3.
#[test]
fn related_objects_type_is_left_unset() {
    for schema in [ifc4(), ifc4x3()] {
        let (mut model, ids) = model_with(&["IFCTASK"]);
        let mut tx = Transaction::new(&model);
        let order = stage_control(&mut tx, schema, ControlKind::ProjectOrder);
        let rel = assign_to_control(&mut tx, &model, schema, draft(order, &ids)).expect("ok");
        tx.commit(&mut model).expect("commit");
        assert_eq!(
            model.get(rel).expect("rel").attributes[RELATED_TYPE],
            Value::Null,
            "{}",
            schema.name()
        );
    }
}

/// Controls owned by `ifc-cost` and `ifc-schedule`, and a non-control,
/// are refused, and the refusal leaves the transaction unchanged.
#[test]
fn a_control_owned_elsewhere_is_refused() {
    let foreign = [
        "IFCCOSTSCHEDULE",
        "IFCCOSTITEM",
        "IFCWORKSCHEDULE",
        "IFCWORKPLAN",
        "IFCWORKCALENDAR",
        "IFCTASK",
    ];
    let (model, ids) = model_with(&[&foreign[..], &["IFCWALL"]].concat());
    let wall = [ids[foreign.len()]];
    for (control, type_name) in ids.iter().zip(foreign) {
        let mut tx = Transaction::new(&model);
        let err = assign_to_control(&mut tx, &model, ifc4(), draft(*control, &wall))
            .expect_err(type_name);
        assert_eq!(
            err,
            ControlError::ForeignControl {
                id: *control,
                actual: type_name.into(),
            }
        );
        assert!(tx.is_empty(), "{type_name}: refusal staged nothing");
    }
}

/// A relating control or related object that exists nowhere is a
/// missing reference, not a type error.
#[test]
fn missing_references_are_refused() {
    let (model, ids) = model_with(&["IFCTASK"]);
    let task = ids[0];
    let mut tx = Transaction::new(&model);
    let permit = stage_control(&mut tx, ifc4(), ControlKind::Permit);
    let ghost = EntityId(9_999);

    let err = assign_to_control(&mut tx, &model, ifc4(), draft(ghost, &[task])).expect_err("ghost");
    assert_eq!(err, ControlError::UnknownEntity { id: ghost });

    let err =
        assign_to_control(&mut tx, &model, ifc4(), draft(permit, &[task, ghost])).expect_err("x");
    assert_eq!(err, ControlError::UnknownEntity { id: ghost });

    // A removal staged in this transaction counts as absent.
    tx.remove(task);
    let err = assign_to_control(&mut tx, &model, ifc4(), draft(permit, &[task])).expect_err("rm");
    assert_eq!(err, ControlError::UnknownEntity { id: task });
}

/// `SET [1:?]`: an empty list and a repeated member are both refused.
#[test]
fn empty_or_duplicate_related_objects_are_refused() {
    let (model, ids) = model_with(&["IFCTASK"]);
    let task = ids[0];
    let mut tx = Transaction::new(&model);
    let request = stage_control(&mut tx, ifc4(), ControlKind::ActionRequest);
    let before = tx.len();

    let err = assign_to_control(&mut tx, &model, ifc4(), draft(request, &[])).expect_err("empty");
    assert_eq!(invalid_attribute(&err), Some("RelatedObjects"));

    let err =
        assign_to_control(&mut tx, &model, ifc4(), draft(request, &[task, task])).expect_err("dup");
    assert_eq!(invalid_attribute(&err), Some("RelatedObjects"));

    assert_eq!(tx.len(), before, "refusals staged nothing");
}

/// `NoSelfReference`: the control cannot govern itself.
#[test]
fn the_control_among_its_related_objects_is_refused() {
    let (model, ids) = model_with(&["IFCTASK"]);
    let mut tx = Transaction::new(&model);
    let history = stage_control(&mut tx, ifc4x3(), ControlKind::PerformanceHistory);
    let err = assign_to_control(
        &mut tx,
        &model,
        ifc4x3(),
        draft(history, &[ids[0], history]),
    )
    .expect_err("self");
    assert_eq!(invalid_attribute(&err), Some("RelatedObjects"));
}

/// `RelatedObjects` holds `IfcObjectDefinition`s; a geometry resource
/// is not one, a type object is.
#[test]
fn a_related_object_must_be_an_object_definition() {
    let (model, ids) = model_with(&["IFCCARTESIANPOINT", "IFCWALLTYPE"]);
    let mut tx = Transaction::new(&model);
    let permit = stage_control(&mut tx, ifc4(), ControlKind::Permit);

    let err =
        assign_to_control(&mut tx, &model, ifc4(), draft(permit, &[ids[0]])).expect_err("point");
    assert_eq!(invalid_attribute(&err), Some("RelatedObjects"));

    assign_to_control(&mut tx, &model, ifc4(), draft(permit, &[ids[1]])).expect("type object");
}

/// A malformed GlobalId is refused before anything is staged.
#[test]
fn a_malformed_guid_is_refused() {
    let (model, ids) = model_with(&["IFCTASK"]);
    let mut tx = Transaction::new(&model);
    let permit = stage_control(&mut tx, ifc4(), ControlKind::Permit);
    let mut bad = draft(permit, &ids);
    bad.global_id = "not-a-guid";
    let err = assign_to_control(&mut tx, &model, ifc4(), bad).expect_err("guid");
    assert_eq!(invalid_attribute(&err), Some("GlobalId"));
}
