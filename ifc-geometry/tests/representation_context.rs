//! Reading representation contexts, including DERIVED inheritance.

use ifc_geometry::{
    all_contexts, context_of, plan_contexts, select_plan_representation,
    select_shape_representation, RepresentationContext, TargetView,
};
use ifc_model::{Entity, EntityId, Model, Value};

fn put(model: &mut Model, id: u64, type_name: &str, attributes: Vec<Value>) -> EntityId {
    let entity_id = EntityId(id);
    model.insert(entity_id, Entity::new(type_name, attributes));
    entity_id
}

fn text(value: &str) -> Value {
    Value::Text(value.into())
}

/// A root context carrying real values, plus a sub-context that redeclares the
/// inherited six as `*` — exactly the shape real exporters write.
fn model_with_contexts() -> (Model, EntityId, EntityId) {
    let mut model = Model::new();
    let placement = put(&mut model, 1, "IFCAXIS2PLACEMENT3D", vec![]);
    let root = put(
        &mut model,
        2,
        "IFCGEOMETRICREPRESENTATIONCONTEXT",
        vec![
            Value::Null,
            text("Model"),
            Value::Integer(3),
            Value::Real(1.0e-5),
            Value::Ref(placement),
            Value::Null,
        ],
    );
    let plan = put(
        &mut model,
        3,
        "IFCGEOMETRICREPRESENTATIONSUBCONTEXT",
        vec![
            text("Plan"),
            text("Plan"),
            Value::Derived,
            Value::Derived,
            Value::Derived,
            Value::Derived,
            Value::Ref(root),
            Value::Real(0.01),
            Value::Enum("PLAN_VIEW".into()),
            Value::Null,
        ],
    );
    (model, root, plan)
}

#[test]
fn a_sub_context_reports_its_own_attributes() {
    let (model, root, plan) = model_with_contexts();
    let entity = model.get(plan).unwrap();
    let context = RepresentationContext::new(plan, entity);

    assert!(context.is_sub_context());
    assert_eq!(context.identifier().as_deref(), Some("Plan"));
    assert_eq!(context.parent(), Some(root));
    assert_eq!(context.target_scale(), Some(0.01));
    assert_eq!(context.target_view(), Some(TargetView::PlanView));
    assert!(context.is_plan_view());
}

/// The `*` trap: these six live on the parent, not on the sub-context.
#[test]
fn derived_attributes_resolve_through_the_parent_context() {
    let (model, _, plan) = model_with_contexts();
    let entity = model.get(plan).unwrap();
    let context = RepresentationContext::new(plan, entity);

    assert_eq!(
        context.precision(&model),
        Some(1.0e-5),
        "precision is written as * and lives on the parent"
    );
    assert_eq!(context.coordinate_space_dimension(&model), Some(3));
    assert_eq!(
        context.world_coordinate_system(&model),
        Some(EntityId(1)),
        "without this a sub-context appears to have no placement"
    );
}

/// `$` is not `*`: an explicitly unset value on the CHILD must not trigger a
/// walk to the parent, even when the parent holds a value.
///
/// The distinction only shows when the parent has something to steal: if the
/// child says `$` and inherits anyway, it wrongly reports the parent's number.
#[test]
fn an_explicitly_null_attribute_is_not_inherited() {
    let mut model = Model::new();
    let root = put(
        &mut model,
        1,
        "IFCGEOMETRICREPRESENTATIONCONTEXT",
        vec![
            Value::Null,
            text("Model"),
            Value::Integer(3),
            Value::Real(1.0e-5),
            Value::Null,
            Value::Null,
        ],
    );
    // Precision is `$` here, not `*`: the author stated "unset", and the
    // parent's 1.0e-5 must not leak in.
    let child = put(
        &mut model,
        2,
        "IFCGEOMETRICREPRESENTATIONSUBCONTEXT",
        vec![
            Value::Null,
            text("Plan"),
            Value::Derived,
            Value::Null,
            Value::Derived,
            Value::Derived,
            Value::Ref(root),
            Value::Null,
            Value::Enum("PLAN_VIEW".into()),
            Value::Null,
        ],
    );
    let context = RepresentationContext::new(child, model.get(child).unwrap());

    assert_eq!(
        context.precision(&model),
        None,
        "`$` means unset; inheriting the parent's 1.0e-5 would be wrong"
    );
    assert_eq!(
        context.coordinate_space_dimension(&model),
        Some(3),
        "but `*` in the neighbouring slot still inherits"
    );
}

#[test]
fn a_root_context_has_no_target_view_or_parent() {
    let (model, root, _) = model_with_contexts();
    let entity = model.get(root).unwrap();
    let context = RepresentationContext::new(root, entity);

    assert!(!context.is_sub_context());
    assert_eq!(context.parent(), None);
    assert_eq!(context.target_view(), None);
    assert!(!context.is_plan_view());
    assert_eq!(context.precision(&model), Some(1.0e-5), "read directly");
}

/// The depth bound is the single termination mechanism, so pin both sides of
/// it: a chain within the bound must still resolve, and one past it must give
/// up rather than loop. A bound nobody tests at its edge is a guess.
#[test]
fn a_deep_but_finite_chain_still_resolves_within_the_bound() {
    let mut model = Model::new();
    // Root holds the real value.
    let root = put(
        &mut model,
        1,
        "IFCGEOMETRICREPRESENTATIONCONTEXT",
        vec![
            Value::Null,
            text("Model"),
            Value::Integer(3),
            Value::Real(1.0e-5),
            Value::Null,
            Value::Null,
        ],
    );
    // Four sub-contexts chained onto it, each deferring with `*`.
    let mut parent = root;
    for id in 2u64..=5 {
        parent = put(
            &mut model,
            id,
            "IFCGEOMETRICREPRESENTATIONSUBCONTEXT",
            vec![
                Value::Null,
                Value::Null,
                Value::Derived,
                Value::Derived,
                Value::Derived,
                Value::Derived,
                Value::Ref(parent),
            ],
        );
    }
    let deepest = RepresentationContext::new(parent, model.get(parent).unwrap());
    assert_eq!(
        deepest.precision(&model),
        Some(1.0e-5),
        "a legal chain must resolve, not hit the ceiling"
    );
}

/// A cycle: the walk must terminate and report the value as unresolved.
#[test]
fn a_context_cycle_gives_up_instead_of_looping() {
    let mut model = Model::new();
    let a = EntityId(1);
    let b = EntityId(2);
    let attrs = |parent: EntityId| {
        vec![
            Value::Null,
            Value::Null,
            Value::Derived,
            Value::Derived,
            Value::Derived,
            Value::Derived,
            Value::Ref(parent),
        ]
    };
    model.insert(
        a,
        Entity::new("IFCGEOMETRICREPRESENTATIONSUBCONTEXT", attrs(b)),
    );
    model.insert(
        b,
        Entity::new("IFCGEOMETRICREPRESENTATIONSUBCONTEXT", attrs(a)),
    );

    for id in [a, b] {
        let view = RepresentationContext::new(id, model.get(id).unwrap());
        assert_eq!(view.precision(&model), None, "terminates on a cycle");
        assert_eq!(view.world_coordinate_system(&model), None);
    }
}

/// The 3D selector must keep REJECTING 2D identifiers. Without this, widening
/// SOLID_IDENTIFIERS would silently hand a viewer a centreline instead of a
/// solid -- the exact failure the original selector was written to prevent.
#[test]
fn the_solid_selector_refuses_2d_only_products() {
    let (mut model, root, _) = model_with_contexts();

    for (id, identifier) in [(10u64, "Axis"), (11, "FootPrint")] {
        put(
            &mut model,
            id,
            "IFCSHAPEREPRESENTATION",
            vec![
                Value::Ref(root),
                text(identifier),
                text("Curve2D"),
                Value::List(vec![]),
            ],
        );
    }
    let shape = put(
        &mut model,
        12,
        "IFCPRODUCTDEFINITIONSHAPE",
        vec![
            Value::Null,
            Value::Null,
            Value::List(vec![Value::Ref(EntityId(10)), Value::Ref(EntityId(11))]),
        ],
    );
    let wall = put(
        &mut model,
        13,
        "IFCWALL",
        vec![
            text("3vB2YO$MX4xv5uCqZZG05x"),
            Value::Null,
            Value::Null,
            Value::Null,
            Value::Null,
            Value::Null,
            Value::Ref(shape),
        ],
    );

    assert_eq!(
        select_shape_representation(&model, wall).unwrap(),
        None,
        "a curve must never be returned as a body"
    );
    assert_eq!(
        select_plan_representation(&model, wall).unwrap(),
        Some(EntityId(11)),
        "but the plan selector finds the FootPrint"
    );
}

/// A product carrying Axis, FootPrint and Body representations: the shape any
/// Revit-authored wall has.
fn model_with_three_representations() -> (Model, EntityId, EntityId, EntityId, EntityId) {
    let (mut model, root, _plan) = model_with_contexts();

    let curve = |identifier: &str| {
        vec![
            Value::Ref(root),
            text(identifier),
            text("Curve2D"),
            Value::List(vec![]),
        ]
    };
    let axis = put(&mut model, 10, "IFCSHAPEREPRESENTATION", curve("Axis"));
    let footprint = put(&mut model, 11, "IFCSHAPEREPRESENTATION", curve("FootPrint"));
    let body = put(
        &mut model,
        12,
        "IFCSHAPEREPRESENTATION",
        vec![
            Value::Ref(root),
            text("Body"),
            text("SweptSolid"),
            Value::List(vec![]),
        ],
    );
    let shape = put(
        &mut model,
        13,
        "IFCPRODUCTDEFINITIONSHAPE",
        vec![
            Value::Null,
            Value::Null,
            Value::List(vec![
                Value::Ref(axis),
                Value::Ref(footprint),
                Value::Ref(body),
            ]),
        ],
    );
    let wall = put(
        &mut model,
        14,
        "IFCWALL",
        vec![
            text("3vB2YO$MX4xv5uCqZZG05x"),
            Value::Null,
            Value::Null,
            Value::Null,
            Value::Null,
            Value::Null,
            Value::Ref(shape),
        ],
    );
    (model, wall, axis, footprint, body)
}

/// The two selectors must disagree: that is the whole point of adding one.
#[test]
fn the_solid_and_plan_selectors_choose_differently() {
    let (model, wall, _, footprint, body) = model_with_three_representations();

    assert_eq!(
        select_shape_representation(&model, wall).unwrap(),
        Some(body),
        "a 3D viewer wants the Body"
    );
    assert_eq!(
        select_plan_representation(&model, wall).unwrap(),
        Some(footprint),
        "a drawing wants the FootPrint, not the Body"
    );
}

#[test]
fn the_plan_selector_prefers_footprint_over_axis() {
    let (model, wall, axis, footprint, _) = model_with_three_representations();
    let chosen = select_plan_representation(&model, wall).unwrap();
    assert_eq!(chosen, Some(footprint));
    assert_ne!(
        chosen,
        Some(axis),
        "Axis is the least specific 2D identifier"
    );
}

/// An explicit PLAN_VIEW context beats any identifier heuristic.
#[test]
fn an_explicit_plan_context_wins_over_the_identifier_order() {
    let (mut model, root, plan) = model_with_contexts();

    // An Axis representation, but authored into the PLAN_VIEW sub-context.
    let axis_in_plan = put(
        &mut model,
        10,
        "IFCSHAPEREPRESENTATION",
        vec![
            Value::Ref(plan),
            text("Axis"),
            text("Curve2D"),
            Value::List(vec![]),
        ],
    );
    // A FootPrint in the plain model context, which the identifier order
    // would otherwise prefer.
    let footprint = put(
        &mut model,
        11,
        "IFCSHAPEREPRESENTATION",
        vec![
            Value::Ref(root),
            text("FootPrint"),
            text("Curve2D"),
            Value::List(vec![]),
        ],
    );
    let shape = put(
        &mut model,
        12,
        "IFCPRODUCTDEFINITIONSHAPE",
        vec![
            Value::Null,
            Value::Null,
            Value::List(vec![Value::Ref(axis_in_plan), Value::Ref(footprint)]),
        ],
    );
    let wall = put(
        &mut model,
        13,
        "IFCWALL",
        vec![
            text("3vB2YO$MX4xv5uCqZZG05x"),
            Value::Null,
            Value::Null,
            Value::Null,
            Value::Null,
            Value::Null,
            Value::Ref(shape),
        ],
    );

    assert_eq!(
        select_plan_representation(&model, wall).unwrap(),
        Some(axis_in_plan),
        "stated authorial intent beats the identifier heuristic"
    );
}

/// A product with only a Body has no plan geometry. Deriving one needs
/// sectioning, which this crate does not do, so None is the honest answer.
#[test]
fn a_solid_only_product_has_no_plan_representation() {
    let (mut model, root, _) = model_with_contexts();
    let body = put(
        &mut model,
        10,
        "IFCSHAPEREPRESENTATION",
        vec![
            Value::Ref(root),
            text("Body"),
            text("SweptSolid"),
            Value::List(vec![]),
        ],
    );
    let shape = put(
        &mut model,
        11,
        "IFCPRODUCTDEFINITIONSHAPE",
        vec![
            Value::Null,
            Value::Null,
            Value::List(vec![Value::Ref(body)]),
        ],
    );
    let wall = put(
        &mut model,
        12,
        "IFCWALL",
        vec![
            text("3vB2YO$MX4xv5uCqZZG05x"),
            Value::Null,
            Value::Null,
            Value::Null,
            Value::Null,
            Value::Null,
            Value::Ref(shape),
        ],
    );

    assert_eq!(select_plan_representation(&model, wall).unwrap(), None);
    assert_eq!(
        select_shape_representation(&model, wall).unwrap(),
        Some(body),
        "the 3D selector still finds it"
    );
}

#[test]
fn contexts_are_discoverable_from_the_model() {
    let (model, root, plan) = model_with_contexts();

    let all: Vec<_> = all_contexts(&model).iter().map(|c| c.id()).collect();
    assert_eq!(all, [root, plan], "root and sub-context, in id order");

    let plans: Vec<_> = plan_contexts(&model).iter().map(|c| c.id()).collect();
    assert_eq!(plans, [plan], "only the PLAN_VIEW sub-context");
}

#[test]
fn a_representation_reports_the_context_it_was_authored_into() {
    let (model, _, _, footprint, _) = model_with_three_representations();
    let context = context_of(&model, footprint).expect("has a context");

    assert_eq!(context.id(), EntityId(2), "the root model context");
    assert_eq!(context.context_type().as_deref(), Some("Model"));
    assert_eq!(context.identifier(), None, "this fixture leaves it unset");
    assert!(!context.is_plan_view());
}

#[test]
fn unknown_target_views_are_preserved_not_flattened() {
    let mut model = Model::new();
    let id = put(
        &mut model,
        1,
        "IFCGEOMETRICREPRESENTATIONSUBCONTEXT",
        vec![
            Value::Null,
            Value::Null,
            Value::Derived,
            Value::Derived,
            Value::Derived,
            Value::Derived,
            Value::Null,
            Value::Null,
            Value::Enum("SOME_FUTURE_VIEW".into()),
            Value::Null,
        ],
    );
    let context = RepresentationContext::new(id, model.get(id).unwrap());
    assert_eq!(
        context.target_view(),
        Some(TargetView::Other("SOME_FUTURE_VIEW".into()))
    );
    assert!(!context.is_plan_view());
}

#[test]
fn a_user_defined_view_carries_its_label() {
    let mut model = Model::new();
    let id = put(
        &mut model,
        1,
        "IFCGEOMETRICREPRESENTATIONSUBCONTEXT",
        vec![
            Value::Null,
            Value::Null,
            Value::Derived,
            Value::Derived,
            Value::Derived,
            Value::Derived,
            Value::Null,
            Value::Null,
            Value::Enum("USERDEFINED".into()),
            text("Baugenehmigung"),
        ],
    );
    let context = RepresentationContext::new(id, model.get(id).unwrap());
    assert_eq!(
        context.target_view(),
        Some(TargetView::UserDefined(Some("Baugenehmigung".into())))
    );
}

#[test]
fn a_reflected_plan_counts_as_a_plan() {
    assert!(TargetView::ReflectedPlanView.is_plan());
    assert!(TargetView::PlanView.is_plan());
    assert!(!TargetView::ModelView.is_plan());
    assert!(!TargetView::GraphView.is_plan());
}
