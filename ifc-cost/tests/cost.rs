//! `ifc-cost` over a real fixture: rates, nesting, currency and rollups.

use ifc_cost::{
    children_of, consistency, controlled_by, controls_of, descendants_of, direct_total, parent_of,
    project_currency, rolled_up_total, roots, ArithmeticOperator, CostItem, CostRelationError,
    CostView, CurrencyError,
};
use ifc_model::{Codec, Entity, Model, Value};

fn fixture() -> Model {
    let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../test/fixtures/synthetic-cost-schedule/synthetic_cost_schedule.ifc");
    ifc_step::StepCodec
        .read_path(&path)
        .expect("fixture parses")
}

fn item_named<'m>(view: &CostView<'m>, name: &str) -> CostItem<'m> {
    view.items()
        .find(|i| i.name() == Some(name))
        .expect("cost item in fixture")
}

// ---- slot correctness ----------------------------------------------------

/// The regression this crate shipped with: `IfcCostItem` slots were off by one.
///
/// `IfcControl` contributes `Identification` at 5, not 4, because `IfcObject`
/// adds `ObjectType` at 4. Reading `Identification` from slot 4 returns the
/// object type, and `CostValues` from 5 returns the identification string.
/// The fixture is written by IfcOpenShell, so it encodes the real layout and
/// cannot agree with a wrong reader.
#[test]
fn cost_item_slots_match_the_schema() {
    let model = fixture();
    let view = CostView::new(&model);
    let item = item_named(&view, "Excavation");

    assert_eq!(
        item.identification(),
        Some("A.1.1"),
        "Identification is slot 5"
    );
    assert_eq!(
        view.values_of(&item).len(),
        1,
        "CostValues is slot 7 and resolves"
    );
}

/// `IfcCostSchedule.PredefinedType` is slot 6, not 8.
///
/// Slot 8 is `SubmittedOn`, a date. A reader using it reports the date string
/// as the schedule's type.
#[test]
fn cost_schedule_predefined_type_is_not_the_submission_date() {
    let model = fixture();
    let view = CostView::new(&model);
    let schedule = view.schedules().next().expect("schedule in fixture");

    assert_eq!(schedule.predefined_type(), Some("BUDGET"));
    assert_eq!(schedule.name(), Some("Budget"));
}

// ---- COST-RATE -----------------------------------------------------------

/// A rate states what it is per: 45.50 EUR per cubic metre.
#[test]
fn a_rate_carries_its_unit_basis() {
    let model = fixture();
    let view = CostView::new(&model);
    let cladding = item_named(&view, "Cladding");
    let value = view.values_of(&cladding).into_iter().next().expect("value");

    assert_eq!(value.amount(), Some(45.50));
    assert_eq!(value.category(), Some("Material"));
    assert!(value.is_monetary(), "amount is IFCMONETARYMEASURE");

    let basis = value.unit_basis(&model).expect("rate states a basis");
    assert_eq!(basis.value, Some(1.0));
    assert_eq!(basis.measure, Some("IFCVOLUMEMEASURE"));
    assert!(basis.unit.is_some(), "basis names a unit entity");
}

/// A lump sum has no unit basis, which is how it differs from a rate.
#[test]
fn a_lump_sum_states_no_basis() {
    let model = fixture();
    let view = CostView::new(&model);
    let excavation = item_named(&view, "Excavation");
    let value = view
        .values_of(&excavation)
        .into_iter()
        .next()
        .expect("value");

    assert_eq!(value.amount(), Some(320.00));
    assert!(
        value.unit_basis(&model).is_none(),
        "a lump sum is not per anything"
    );
}

/// A composed value states components and an operator, and no amount of its own.
///
/// A reader that only looks at `AppliedValue` reports nothing here, which is
/// how composed rates silently vanish from a total.
#[test]
fn a_composed_value_states_components_not_an_amount() {
    let model = fixture();
    let view = CostView::new(&model);
    let prelim = item_named(&view, "Preliminaries");
    let value = view.values_of(&prelim).into_iter().next().expect("value");

    assert_eq!(value.amount(), None, "no directly stated amount");
    assert!(value.is_composed(), "it is composed from components");
    assert_eq!(value.operator(), Some(ArithmeticOperator::Add));
    assert_eq!(value.component_refs().len(), 2);
    assert!(
        !ArithmeticOperator::Add.is_order_sensitive(),
        "ADD folds safely in any order"
    );
    assert!(
        ArithmeticOperator::Subtract.is_order_sensitive(),
        "SUBTRACT does not, and IFC does not define the bracketing"
    );
}

// ---- COST-REL ------------------------------------------------------------

/// Nesting resolves in authored order, both directions.
#[test]
fn cost_items_nest_into_a_breakdown() {
    let model = fixture();
    let view = CostView::new(&model);
    let substructure = item_named(&view, "Substructure");
    let excavation = item_named(&view, "Excavation");

    let children = children_of(&model, substructure.id());
    assert_eq!(children.len(), 2, "excavation and concreting");
    assert_eq!(children[0], excavation.id(), "authored order preserved");

    assert_eq!(parent_of(&model, excavation.id()), Some(substructure.id()));
    assert_eq!(parent_of(&model, substructure.id()), None, "a root item");
}

/// Roots are the items no other item nests.
#[test]
fn breakdown_roots_are_the_unnested_items() {
    let model = fixture();
    let view = CostView::new(&model);
    let mut names: Vec<&str> = roots(&view)
        .into_iter()
        .filter_map(|id| model.get(id).and_then(|e| e.text(2)))
        .collect();
    names.sort_unstable();
    assert_eq!(
        names,
        vec!["Preliminaries", "Substructure", "Superstructure"]
    );
}

/// A cost item reaches the products it prices through `IfcRelAssignsToControl`.
///
/// `RelatingControl` is slot 6, not 5: `IfcRelAssigns` contributes
/// `RelatedObjects` and `RelatedObjectsType` first.
#[test]
fn a_cost_item_prices_a_product() {
    let model = fixture();
    let view = CostView::new(&model);
    let concreting = item_named(&view, "Concreting");

    let priced = controlled_by(&model, concreting.id());
    assert_eq!(priced.len(), 1, "concreting prices one element");
    let wall = model.get(priced[0]).expect("target resolves");
    assert_eq!(wall.type_name.as_ref(), "IFCWALL");

    assert!(
        controls_of(&model, priced[0]).contains(&concreting.id()),
        "the inverse holds"
    );
}

/// A nesting cycle is reported, not survived by recursion.
#[test]
fn a_nesting_cycle_is_reported() {
    let mut model = Model::new();
    let a = model.push(Entity::new(
        "IFCCOSTITEM",
        vec![
            Value::Text("a".into()),
            Value::Null,
            Value::Text("A".into()),
        ],
    ));
    let b = model.push(Entity::new(
        "IFCCOSTITEM",
        vec![
            Value::Text("b".into()),
            Value::Null,
            Value::Text("B".into()),
        ],
    ));
    // A nests B, and B nests A: legal per NoSelfReference, which only forbids
    // an item nesting itself directly.
    for (parent, child) in [(a, b), (b, a)] {
        model.push(Entity::new(
            "IFCRELNESTS",
            vec![
                Value::Text("rel".into()),
                Value::Null,
                Value::Null,
                Value::Null,
                Value::Ref(parent),
                Value::List(vec![Value::Ref(child)]),
            ],
        ));
    }

    match descendants_of(&model, a) {
        Err(CostRelationError::NestingCycle { repeated, path }) => {
            assert_eq!(repeated, a, "the walk returns to its start");
            assert!(path.len() >= 2, "the path is reported: {path:?}");
        }
        other => panic!("expected a cycle, got {other:?}"),
    }
}

// ---- COST-UNIT -----------------------------------------------------------

/// One currency stated, so totals are meaningful.
#[test]
fn a_single_currency_is_resolved() {
    let model = fixture();
    assert_eq!(project_currency(&model), Ok("EUR".to_string()));
}

/// Two currencies make a total meaningless, and that is reported.
///
/// The alternative -- adding EUR to GBP -- produces a number that looks
/// authoritative and is wrong, which is the worst failure a cost tool has.
#[test]
fn two_currencies_are_ambiguous_not_summed() {
    let mut model = fixture();
    model.push(Entity::new(
        "IFCMONETARYUNIT",
        vec![Value::Text("GBP".into())],
    ));

    match project_currency(&model) {
        Err(CurrencyError::Ambiguous { currencies }) => {
            assert_eq!(currencies, vec!["EUR".to_string(), "GBP".to_string()]);
        }
        other => panic!("expected ambiguity, got {other:?}"),
    }
}

/// A file with no monetary unit states amounts without a currency.
#[test]
fn a_missing_currency_is_reported_as_unstated() {
    let model = Model::new();
    assert_eq!(project_currency(&model), Err(CurrencyError::Unstated));
}

// ---- rollup --------------------------------------------------------------

/// Direct and rolled-up totals are different questions.
#[test]
fn direct_and_rolled_up_totals_are_distinct() {
    let model = fixture();
    let view = CostView::new(&model);
    let substructure = item_named(&view, "Substructure");

    assert_eq!(direct_total(&view, &substructure), 500.00, "its own value");
    assert_eq!(
        rolled_up_total(&view, &substructure).expect("no cycle"),
        500.00,
        "320 labour + 180 plant"
    );
}

/// When a parent's stated total disagrees with its children, that is reported.
///
/// The fixture states 900.00 for Superstructure whose only child totals 45.50.
/// Neither number is authoritative: IFC does not say whether a parent
/// summarises its children or adds to them.
#[test]
fn a_disagreeing_parent_total_is_surfaced() {
    let model = fixture();
    let view = CostView::new(&model);
    let superstructure = item_named(&view, "Superstructure");

    let check = consistency(&view, &superstructure).expect("no cycle");
    assert_eq!(check.direct, 900.00);
    assert_eq!(check.rolled_up, 45.50);
    assert!(check.has_children && check.states_own_value);
    assert!(!check.agrees(0.01), "the file contradicts itself");

    let substructure = item_named(&view, "Substructure");
    assert!(
        consistency(&view, &substructure)
            .expect("no cycle")
            .agrees(0.01),
        "the consistent branch agrees"
    );
}

/// A leaf with no children is trivially consistent.
#[test]
fn a_leaf_item_is_consistent_by_construction() {
    let model = fixture();
    let view = CostView::new(&model);
    let check = consistency(&view, &item_named(&view, "Cladding")).expect("no cycle");

    assert!(!check.has_children);
    assert!(check.agrees(0.0), "only one reading exists");
}
