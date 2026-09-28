//! Authoring.

use ifc_cost::{
    assign_schedule_items, children_of, controlled_by, controls_of, create_cost_item,
    create_cost_schedule, create_cost_value, create_monetary_unit, monetary_units, nest_cost_items,
    project_currency, ArithmeticOperator, CostAuthoringError, CostItemDraft, CostItemType,
    CostScheduleDraft, CostScheduleType, CostValueDraft, CostValueKind, NestingDraft,
    ScheduleAssignmentDraft,
};
use ifc_model::{Entity, EntityId, Model, Transaction, Value};

const SCHEDULE_GUID: &str = "0O2Fr$t4X7Zf8NOew3FLOH";
const ROOT_GUID: &str = "1O2Fr$t4X7Zf8NOew3FLOH";
const CHILD_GUID: &str = "2O2Fr$t4X7Zf8NOew3FLOH";
const NEST_GUID: &str = "3O2Fr$t4X7Zf8NOew3FLOH";
const ASSIGN_GUID: &str = "0P2Fr$t4X7Zf8NOew3FLOH";

fn item(tx: &mut Transaction, model: &Model, global_id: &str) -> EntityId {
    create_cost_item(
        tx,
        model,
        CostItemDraft::new(global_id).predefined_type(CostItemType::NotDefined),
    )
    .unwrap()
}

#[test]
fn stages_a_queryable_cost_schedule_tree_atomically() {
    let mut model = Model::new();
    let mut tx = Transaction::new(&model);
    let value = create_cost_value(
        &mut tx,
        &model,
        CostValueDraft::default()
            .name("Labour")
            .category("Labour")
            .kind(CostValueKind::Monetary(125.5)),
    )
    .unwrap();
    let root = create_cost_item(
        &mut tx,
        &model,
        CostItemDraft::new(ROOT_GUID)
            .name("Root")
            .identification("1")
            .predefined_type(CostItemType::NotDefined)
            .cost_values(&[value]),
    )
    .unwrap();
    let child = create_cost_item(
        &mut tx,
        &model,
        CostItemDraft::new(CHILD_GUID)
            .name("Child")
            .predefined_type(CostItemType::NotDefined),
    )
    .unwrap();
    let schedule = create_cost_schedule(
        &mut tx,
        &model,
        CostScheduleDraft::new(SCHEDULE_GUID)
            .name("Estimate")
            .predefined_type(CostScheduleType::Estimate)
            .status("Draft"),
    )
    .unwrap();
    nest_cost_items(
        &mut tx,
        &model,
        NestingDraft::new(NEST_GUID, root, &[child]),
    )
    .unwrap();
    let assignment = assign_schedule_items(
        &mut tx,
        &model,
        ScheduleAssignmentDraft::new(ASSIGN_GUID, schedule, &[root]),
    )
    .unwrap();

    assert_eq!(model.len(), 0, "staging must not mutate the model");
    tx.commit(&mut model).unwrap();
    // RelatedObjectsType is OPTIONAL IfcStrippedOptional in IFC4: a
    // BOOLEAN kept only for backward parsing. It is left unset rather
    // than carrying an enumeration token the slot cannot hold.
    assert!(
        matches!(
            model.get(assignment).unwrap().attribute(5),
            None | Some(Value::Null)
        ),
        "a stripped optional is not authored: {:?}",
        model.get(assignment).unwrap().attribute(5),
    );
    assert_eq!(children_of(&model, root), [child]);
    assert_eq!(controlled_by(&model, schedule), [root]);
    assert_eq!(controls_of(&model, root), [schedule]);
    let view = ifc_cost::CostView::new(&model);
    assert_eq!(
        view.items()
            .find(|item| item.id() == root)
            .unwrap()
            .value_refs(),
        [value]
    );
    assert_eq!(
        view.schedules().next().unwrap().predefined_type(),
        Some("ESTIMATE")
    );
}

#[test]
fn composed_values_round_trip_in_authored_order() {
    let mut model = Model::new();
    let mut tx = Transaction::new(&model);
    let a = create_cost_value(&mut tx, &model, CostValueDraft::monetary(10.0)).unwrap();
    let b = create_cost_value(&mut tx, &model, CostValueDraft::monetary(20.0)).unwrap();
    let sum = create_cost_value(
        &mut tx,
        &model,
        CostValueDraft::default().kind(CostValueKind::Components {
            operator: ArithmeticOperator::Add,
            components: &[a, b],
        }),
    )
    .unwrap();
    tx.commit(&mut model).unwrap();
    let value = ifc_cost::CostValue::new(sum, model.get(sum).unwrap());
    assert_eq!(value.component_refs(), [a, b]);
    assert_eq!(value.operator(), Some(ArithmeticOperator::Add));
}

#[test]
fn invalid_drafts_stage_nothing_and_failed_commit_is_atomic() {
    let model = Model::new();
    let mut tx = Transaction::new(&model);
    let before = tx.len();
    assert!(matches!(
        create_cost_value(&mut tx, &model, CostValueDraft::monetary(f64::NAN)),
        Err(CostAuthoringError::InvalidValue {
            attribute: "AppliedValue",
            ..
        })
    ));
    assert_eq!(tx.len(), before);

    assert!(matches!(
        create_cost_item(&mut tx, &model, CostItemDraft::new("invalid"),),
        Err(CostAuthoringError::InvalidValue {
            attribute: "GlobalId",
            ..
        })
    ));
    assert_eq!(tx.len(), before);

    assert!(matches!(
        create_cost_item(
            &mut tx,
            &model,
            CostItemDraft::new(ROOT_GUID).cost_values(&[EntityId(999)]),
        ),
        Err(CostAuthoringError::MissingReference {
            target: EntityId(999),
            ..
        })
    ));
    assert_eq!(tx.len(), before);

    let mut broken = Model::new();
    let revision = broken.revision();
    let mut bad_tx = Transaction::new(&broken);
    bad_tx.create(Entity::new("IFCCOSTITEM", vec![Value::Ref(EntityId(999))]));
    assert!(bad_tx.commit(&mut broken).is_err());
    assert_eq!(broken.len(), 0);
    assert_eq!(broken.revision(), revision);
}

#[test]
fn nesting_refuses_self_duplicates_and_wrong_kinds_before_staging() {
    let mut model = Model::new();
    let not_item = model.push(Entity::new("IFCWALL", vec![]));
    let mut tx = Transaction::new(&model);
    let item = create_cost_item(&mut tx, &model, CostItemDraft::new(ROOT_GUID)).unwrap();
    let before = tx.len();
    for draft in [
        NestingDraft::new(NEST_GUID, item, &[item]),
        NestingDraft::new(NEST_GUID, item, &[not_item]),
    ] {
        assert!(nest_cost_items(&mut tx, &model, draft).is_err());
        assert_eq!(tx.len(), before);
    }
}

#[test]
fn refuses_second_parent_cycles_and_duplicate_global_ids_before_staging() {
    let model = Model::new();
    let mut tx = Transaction::new(&model);
    let a = create_cost_item(&mut tx, &model, CostItemDraft::new(ROOT_GUID)).unwrap();
    let b = create_cost_item(&mut tx, &model, CostItemDraft::new(CHILD_GUID)).unwrap();
    let c = create_cost_item(
        &mut tx,
        &model,
        CostItemDraft::new("0JH8Y2dTv1LhX9ZzQqFbca"),
    )
    .unwrap();
    nest_cost_items(&mut tx, &model, NestingDraft::new(NEST_GUID, a, &[b])).unwrap();
    let before = tx.len();
    assert!(matches!(
        nest_cost_items(&mut tx, &model, NestingDraft::new("1JH8Y2dTv1LhX9ZzQqFbca", c, &[b])),
        Err(CostAuthoringError::MultipleParents { child, existing_parent }) if child == b && existing_parent == a
    ));
    assert_eq!(tx.len(), before);
    assert!(matches!(
        nest_cost_items(&mut tx, &model, NestingDraft::new("2JH8Y2dTv1LhX9ZzQqFbca", b, &[a])),
        Err(CostAuthoringError::NestingCycle { item }) if item == a
    ));
    assert_eq!(tx.len(), before);
    assert!(matches!(
        create_cost_schedule(&mut tx, &model, CostScheduleDraft::new(ROOT_GUID)),
        Err(CostAuthoringError::InvalidValue {
            attribute: "GlobalId",
            ..
        })
    ));
    assert_eq!(tx.len(), before);
}

#[test]
fn staged_relation_removal_allows_reparenting() {
    let mut model = Model::new();
    let mut tx = Transaction::new(&model);
    let left = item(&mut tx, &model, "00D0000000000000000031");
    let right = item(&mut tx, &model, "00D0000000000000000032");
    let child = item(&mut tx, &model, "00D0000000000000000033");
    let relation = nest_cost_items(
        &mut tx,
        &model,
        NestingDraft::new("00D0000000000000000034", left, &[child]),
    )
    .unwrap();
    tx.commit(&mut model).unwrap();

    let mut tx = Transaction::new(&model);
    tx.remove(relation);
    nest_cost_items(
        &mut tx,
        &model,
        NestingDraft::new("00D0000000000000000035", right, &[child]),
    )
    .unwrap();
    tx.commit(&mut model).unwrap();
    assert_eq!(children_of(&model, right), [child]);
}

#[test]
fn removed_global_id_can_be_reused_in_the_projected_model() {
    const REUSED: &str = "00E0000000000000000041";
    let mut model = Model::new();
    let mut initial = Transaction::new(&model);
    let removed = item(&mut initial, &model, REUSED);
    initial.commit(&mut model).unwrap();

    let mut replacement = Transaction::new(&model);
    replacement.remove(removed);
    let created = item(&mut replacement, &model, REUSED);
    replacement.commit(&mut model).unwrap();

    assert!(model.get(removed).is_none());
    assert_eq!(
        model.get(created).and_then(|entity| entity.text(0)),
        Some(REUSED)
    );
}

#[test]
fn staged_global_id_changes_participate_in_duplicate_validation() {
    const ORIGINAL: &str = "00E0000000000000000042";
    const COLLISION: &str = "00E0000000000000000043";
    let mut model = Model::new();
    let mut initial = Transaction::new(&model);
    let changed = item(&mut initial, &model, ORIGINAL);
    initial.commit(&mut model).unwrap();

    let mut tx = Transaction::new(&model);
    tx.set_attribute(changed, 0, Value::Text(COLLISION.into()));
    let before = tx.len();
    assert!(matches!(
        create_cost_item(&mut tx, &model, CostItemDraft::new(COLLISION),),
        Err(CostAuthoringError::InvalidValue {
            attribute: "GlobalId",
            ..
        })
    ));
    assert_eq!(tx.len(), before);
}

/// An authored currency is what project_currency resolves.
///
/// Every cost value in a file is denominated by this one entity, so a
/// blank currency is refused rather than written: it would leave the
/// reader unable to state what the numbers mean.
#[test]
fn an_authored_currency_reads_back() {
    let mut model = Model::default();
    let mut tx = Transaction::new(&model);
    create_monetary_unit(&mut tx, "EUR").expect("currency");
    tx.commit(&mut model).expect("commit");

    assert_eq!(project_currency(&model).as_deref(), Ok("EUR"));
    assert_eq!(monetary_units(&model).len(), 1);

    let mut tx = Transaction::new(&model);
    assert!(create_monetary_unit(&mut tx, "   ").is_err());
    assert!(create_monetary_unit(&mut tx, "").is_err());
}
