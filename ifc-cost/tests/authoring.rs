use ifc_cost::{
    assign_schedule_items, children_of, controlled_by, controls_of, create_cost_item,
    create_cost_schedule, create_cost_value, nest_cost_items, ArithmeticOperator,
    CostAuthoringError, CostItemDraft, CostItemType, CostScheduleDraft, CostScheduleType,
    CostValueDraft, CostValueKind, NestingDraft, ScheduleAssignmentDraft,
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
        CostItemDraft {
            global_id,
            predefined_type: Some(CostItemType::NotDefined),
            ..Default::default()
        },
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
        CostValueDraft {
            name: Some("Labour"),
            category: Some("Labour"),
            kind: CostValueKind::Monetary(125.5),
            ..Default::default()
        },
    )
    .unwrap();
    let root = create_cost_item(
        &mut tx,
        &model,
        CostItemDraft {
            global_id: ROOT_GUID,
            name: Some("Root"),
            identification: Some("1"),
            predefined_type: Some(CostItemType::NotDefined),
            cost_values: &[value],
            ..Default::default()
        },
    )
    .unwrap();
    let child = create_cost_item(
        &mut tx,
        &model,
        CostItemDraft {
            global_id: CHILD_GUID,
            name: Some("Child"),
            predefined_type: Some(CostItemType::NotDefined),
            ..Default::default()
        },
    )
    .unwrap();
    let schedule = create_cost_schedule(
        &mut tx,
        &model,
        CostScheduleDraft {
            global_id: SCHEDULE_GUID,
            name: Some("Estimate"),
            predefined_type: Some(CostScheduleType::Estimate),
            status: Some("Draft"),
            ..Default::default()
        },
    )
    .unwrap();
    nest_cost_items(
        &mut tx,
        &model,
        NestingDraft {
            global_id: NEST_GUID,
            parent: root,
            children: &[child],
        },
    )
    .unwrap();
    let assignment = assign_schedule_items(
        &mut tx,
        &model,
        ScheduleAssignmentDraft {
            global_id: ASSIGN_GUID,
            schedule,
            items: &[root],
        },
    )
    .unwrap();

    assert_eq!(model.len(), 0, "staging must not mutate the model");
    tx.commit(&mut model).unwrap();
    assert!(matches!(
        model.get(assignment).unwrap().attribute(5),
        Some(Value::Enum(token)) if token.as_ref() == "CONTROL"
    ));
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
        CostValueDraft {
            kind: CostValueKind::Components {
                operator: ArithmeticOperator::Add,
                components: &[a, b],
            },
            ..Default::default()
        },
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
        create_cost_item(
            &mut tx,
            &model,
            CostItemDraft {
                global_id: "invalid",
                ..Default::default()
            },
        ),
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
            CostItemDraft {
                global_id: ROOT_GUID,
                cost_values: &[EntityId(999)],
                ..Default::default()
            },
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
    let item = create_cost_item(
        &mut tx,
        &model,
        CostItemDraft {
            global_id: ROOT_GUID,
            ..Default::default()
        },
    )
    .unwrap();
    let before = tx.len();
    for draft in [
        NestingDraft {
            global_id: NEST_GUID,
            parent: item,
            children: &[item],
        },
        NestingDraft {
            global_id: NEST_GUID,
            parent: item,
            children: &[not_item],
        },
    ] {
        assert!(nest_cost_items(&mut tx, &model, draft).is_err());
        assert_eq!(tx.len(), before);
    }
}

#[test]
fn refuses_second_parent_cycles_and_duplicate_global_ids_before_staging() {
    let model = Model::new();
    let mut tx = Transaction::new(&model);
    let a = create_cost_item(
        &mut tx,
        &model,
        CostItemDraft {
            global_id: ROOT_GUID,
            ..Default::default()
        },
    )
    .unwrap();
    let b = create_cost_item(
        &mut tx,
        &model,
        CostItemDraft {
            global_id: CHILD_GUID,
            ..Default::default()
        },
    )
    .unwrap();
    let c = create_cost_item(
        &mut tx,
        &model,
        CostItemDraft {
            global_id: "0JH8Y2dTv1LhX9ZzQqFbca",
            ..Default::default()
        },
    )
    .unwrap();
    nest_cost_items(
        &mut tx,
        &model,
        NestingDraft {
            global_id: NEST_GUID,
            parent: a,
            children: &[b],
        },
    )
    .unwrap();
    let before = tx.len();
    assert!(matches!(
        nest_cost_items(&mut tx, &model, NestingDraft { global_id: "1JH8Y2dTv1LhX9ZzQqFbca", parent: c, children: &[b] }),
        Err(CostAuthoringError::MultipleParents { child, existing_parent }) if child == b && existing_parent == a
    ));
    assert_eq!(tx.len(), before);
    assert!(matches!(
        nest_cost_items(&mut tx, &model, NestingDraft { global_id: "2JH8Y2dTv1LhX9ZzQqFbca", parent: b, children: &[a] }),
        Err(CostAuthoringError::NestingCycle { item }) if item == a
    ));
    assert_eq!(tx.len(), before);
    assert!(matches!(
        create_cost_schedule(
            &mut tx,
            &model,
            CostScheduleDraft {
                global_id: ROOT_GUID,
                ..Default::default()
            }
        ),
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
        NestingDraft {
            global_id: "00D0000000000000000034",
            parent: left,
            children: &[child],
        },
    )
    .unwrap();
    tx.commit(&mut model).unwrap();

    let mut tx = Transaction::new(&model);
    tx.remove(relation);
    nest_cost_items(
        &mut tx,
        &model,
        NestingDraft {
            global_id: "00D0000000000000000035",
            parent: right,
            children: &[child],
        },
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
        create_cost_item(
            &mut tx,
            &model,
            CostItemDraft {
                global_id: COLLISION,
                ..Default::default()
            },
        ),
        Err(CostAuthoringError::InvalidValue {
            attribute: "GlobalId",
            ..
        })
    ));
    assert_eq!(tx.len(), before);
}
