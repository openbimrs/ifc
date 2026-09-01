mod support;

use ifc_model::{Budget, Value};
use ifc_resource::{ResourceError, ResourceView};
use ifc_schema::ifc4;

use support::{model, named, refs, text, GUID_A, GUID_B, GUID_C};

#[test]
fn allocations_preserve_relation_and_related_object_order() {
    let schema = ifc4();
    let mut model = model("IFC4");
    let labor = model.push(named(
        schema,
        "IfcLaborResource",
        &[("GlobalId", text(GUID_A))],
    ));
    let task_a = model.push(named(schema, "IfcTask", &[("GlobalId", text(GUID_B))]));
    let task_b = model.push(named(schema, "IfcTask", &[("GlobalId", text(GUID_C))]));
    let relation = model.push(named(
        schema,
        "IfcRelAssignsToResource",
        &[
            ("GlobalId", text("3O2Fr$t4X7Zf8NOew3FLOH")),
            ("RelatedObjects", refs(&[task_b, task_a])),
            ("RelatingResource", Value::Ref(labor)),
        ],
    ));

    let view = ResourceView::for_model(&model).unwrap();
    let allocation = view.allocation(relation).unwrap();
    assert_eq!(allocation.relation_id(), relation);
    assert_eq!(allocation.resource_id(), labor);
    assert_eq!(allocation.related_objects(), &[task_b, task_a]);
    assert_eq!(view.allocations_for(labor).unwrap(), vec![allocation]);
}

#[test]
fn allocation_rejects_empty_duplicate_wrong_select_and_self_reference() {
    let schema = ifc4();
    let mut model = model("IFC4");
    let labor = model.push(named(
        schema,
        "IfcLaborResource",
        &[("GlobalId", text(GUID_A))],
    ));
    let task = model.push(named(schema, "IfcTask", &[("GlobalId", text(GUID_B))]));
    let wall = model.push(named(schema, "IfcWall", &[("GlobalId", text(GUID_C))]));

    let empty = model.push(named(
        schema,
        "IfcRelAssignsToResource",
        &[
            ("GlobalId", text("3O2Fr$t4X7Zf8NOew3FLOH")),
            ("RelatedObjects", Value::List(Vec::new())),
            ("RelatingResource", Value::Ref(labor)),
        ],
    ));
    let duplicate = model.push(named(
        schema,
        "IfcRelAssignsToResource",
        &[
            ("GlobalId", text("4O2Fr$t4X7Zf8NOew3FLOH")),
            ("RelatedObjects", refs(&[task, task])),
            ("RelatingResource", Value::Ref(labor)),
        ],
    ));
    let wrong_select = model.push(named(
        schema,
        "IfcRelAssignsToResource",
        &[
            ("GlobalId", text("5O2Fr$t4X7Zf8NOew3FLOH")),
            ("RelatedObjects", refs(&[task])),
            ("RelatingResource", Value::Ref(wall)),
        ],
    ));
    let self_reference = model.push(named(
        schema,
        "IfcRelAssignsToResource",
        &[
            ("GlobalId", text("6O2Fr$t4X7Zf8NOew3FLOH")),
            ("RelatedObjects", refs(&[labor])),
            ("RelatingResource", Value::Ref(labor)),
        ],
    ));

    let view = ResourceView::for_model(&model).unwrap();
    assert!(matches!(
        view.allocation(empty),
        Err(ResourceError::InvalidCardinality { .. })
    ));
    assert!(matches!(
        view.allocation(duplicate),
        Err(ResourceError::DuplicateReference { .. })
    ));
    assert!(matches!(
        view.allocation(wrong_select),
        Err(ResourceError::WrongReferenceType { .. })
    ));
    assert!(matches!(
        view.allocation(self_reference),
        Err(ResourceError::SemanticViolation { .. })
    ));
}

#[test]
fn allocation_enforces_related_objects_type_where_rule() {
    let schema = ifc4();
    for (category, entity_type) in [
        ("PRODUCT", "IfcWall"),
        ("PROCESS", "IfcTask"),
        ("CONTROL", "IfcWorkCalendar"),
        ("RESOURCE", "IfcLaborResource"),
        ("ACTOR", "IfcActor"),
        ("GROUP", "IfcGroup"),
        ("PROJECT", "IfcProject"),
    ] {
        let mut model = model("IFC4");
        let labor = model.push(named(
            schema,
            "IfcLaborResource",
            &[("GlobalId", text(GUID_A))],
        ));
        let related = model.push(named(schema, entity_type, &[("GlobalId", text(GUID_B))]));
        let relation = model.push(named(
            schema,
            "IfcRelAssignsToResource",
            &[
                ("GlobalId", text(GUID_C)),
                ("RelatedObjects", refs(&[related])),
                ("RelatedObjectsType", Value::Enum(category.into())),
                ("RelatingResource", Value::Ref(labor)),
            ],
        ));
        let allocation = ResourceView::for_model(&model)
            .unwrap()
            .allocation(relation)
            .unwrap();
        assert_eq!(allocation.related_objects_type(), Some(category));
    }

    for category in [None, Some("NOTDEFINED")] {
        let mut model = model("IFC4");
        let labor = model.push(named(
            schema,
            "IfcLaborResource",
            &[("GlobalId", text(GUID_A))],
        ));
        let task = model.push(named(schema, "IfcTask", &[("GlobalId", text(GUID_B))]));
        let mut values = vec![
            ("GlobalId", text(GUID_C)),
            ("RelatedObjects", refs(&[task])),
            ("RelatingResource", Value::Ref(labor)),
        ];
        if let Some(category) = category {
            values.push(("RelatedObjectsType", Value::Enum(category.into())));
        }
        let relation = model.push(named(schema, "IfcRelAssignsToResource", &values));
        ResourceView::for_model(&model)
            .unwrap()
            .allocation(relation)
            .unwrap();
    }

    let mut model = model("IFC4");
    let labor = model.push(named(
        schema,
        "IfcLaborResource",
        &[("GlobalId", text(GUID_A))],
    ));
    let task = model.push(named(schema, "IfcTask", &[("GlobalId", text(GUID_B))]));
    let relation = model.push(named(
        schema,
        "IfcRelAssignsToResource",
        &[
            ("GlobalId", text(GUID_C)),
            ("RelatedObjects", refs(&[task])),
            ("RelatedObjectsType", Value::Enum("PRODUCT".into())),
            ("RelatingResource", Value::Ref(labor)),
        ],
    ));
    assert!(matches!(
        ResourceView::for_model(&model)
            .unwrap()
            .allocation(relation),
        Err(ResourceError::SemanticViolation { .. })
    ));
}

#[test]
fn resource_composition_is_ordered_budgeted_and_cycle_checked() {
    let schema = ifc4();
    let mut model = model("IFC4");
    let crew = model.push(named(
        schema,
        "IfcCrewResource",
        &[("GlobalId", text(GUID_A))],
    ));
    let labor_a = model.push(named(
        schema,
        "IfcLaborResource",
        &[("GlobalId", text(GUID_B))],
    ));
    let labor_b = model.push(named(
        schema,
        "IfcLaborResource",
        &[("GlobalId", text(GUID_C))],
    ));
    model.push(named(
        schema,
        "IfcRelNests",
        &[
            ("GlobalId", text("3O2Fr$t4X7Zf8NOew3FLOH")),
            ("RelatingObject", Value::Ref(crew)),
            ("RelatedObjects", refs(&[labor_b, labor_a])),
        ],
    ));

    let view = ResourceView::for_model(&model).unwrap();
    assert_eq!(view.direct_members(crew).unwrap(), vec![labor_b, labor_a]);
    assert_eq!(view.parent_resource(crew).unwrap(), None);
    assert_eq!(view.parent_resource(labor_a).unwrap(), Some(crew));
    assert_eq!(view.parent_resource(labor_b).unwrap(), Some(crew));
    assert_eq!(
        view.descendants(crew, Budget::DEFAULT).unwrap(),
        vec![labor_b, labor_a]
    );
    assert!(matches!(
        view.descendants(
            crew,
            Budget {
                max_depth: 64,
                max_nodes: 1,
            },
        ),
        Err(ResourceError::BudgetExceeded { .. })
    ));

    model.push(named(
        schema,
        "IfcRelNests",
        &[
            ("GlobalId", text("4O2Fr$t4X7Zf8NOew3FLOH")),
            ("RelatingObject", Value::Ref(labor_a)),
            ("RelatedObjects", refs(&[crew])),
        ],
    ));
    assert!(matches!(
        ResourceView::for_model(&model)
            .unwrap()
            .descendants(crew, Budget::DEFAULT),
        Err(ResourceError::Cycle { .. })
    ));

    model.push(named(
        schema,
        "IfcRelNests",
        &[
            ("GlobalId", text("0P2Fr$t4X7Zf8NOew3FLOH")),
            ("RelatingObject", Value::Ref(labor_b)),
            ("RelatedObjects", refs(&[labor_a])),
        ],
    ));
    assert!(matches!(
        ResourceView::for_model(&model)
            .unwrap()
            .parent_resource(labor_a),
        Err(ResourceError::SemanticViolation { .. })
    ));
}

#[test]
fn nesting_rejects_non_resource_members_and_duplicate_members() {
    let schema = ifc4();
    let mut model = model("IFC4");
    let crew = model.push(named(
        schema,
        "IfcCrewResource",
        &[("GlobalId", text(GUID_A))],
    ));
    let labor = model.push(named(
        schema,
        "IfcLaborResource",
        &[("GlobalId", text(GUID_B))],
    ));
    let _wall = model.push(named(schema, "IfcWall", &[("GlobalId", text(GUID_C))]));
    model.push(named(
        schema,
        "IfcRelNests",
        &[
            ("GlobalId", text("3O2Fr$t4X7Zf8NOew3FLOH")),
            ("RelatingObject", Value::Ref(crew)),
            ("RelatedObjects", refs(&[labor, labor])),
        ],
    ));
    assert!(matches!(
        ResourceView::for_model(&model)
            .unwrap()
            .direct_members(crew),
        Err(ResourceError::DuplicateReference { .. })
    ));

    let mut wrong = support::model("IFC4");
    let crew = wrong.push(named(
        schema,
        "IfcCrewResource",
        &[("GlobalId", text(GUID_A))],
    ));
    let wall = wrong.push(named(schema, "IfcWall", &[("GlobalId", text(GUID_C))]));
    wrong.push(named(
        schema,
        "IfcRelNests",
        &[
            ("GlobalId", text("4O2Fr$t4X7Zf8NOew3FLOH")),
            ("RelatingObject", Value::Ref(crew)),
            ("RelatedObjects", refs(&[wall])),
        ],
    ));
    assert!(matches!(
        ResourceView::for_model(&wrong)
            .unwrap()
            .direct_members(crew),
        Err(ResourceError::WrongReferenceType { .. })
    ));
}
