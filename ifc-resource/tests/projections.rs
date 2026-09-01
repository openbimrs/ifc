mod support;

use ifc_model::Value;
use ifc_resource::{ResourceError, ResourceKind, ResourceView};
use ifc_schema::ifc4;

use support::{enumeration, model, named, refs, text, GUID_A};

#[test]
fn labor_projection_resolves_identity_usage_costs_and_quantity() {
    let schema = ifc4();
    let mut model = model("IFC4");
    let usage_id = model.push(named(
        schema,
        "IfcResourceTime",
        &[
            ("Name", text("Day shift")),
            ("ScheduleWork", text("PT8H")),
            ("ScheduleUsage", Value::Real(1.0)),
            ("ScheduleStart", text("2026-09-01T07:00:00")),
            ("ScheduleFinish", text("2026-09-01T15:00:00")),
            ("IsOverAllocated", Value::Bool(false)),
            ("Completion", Value::Real(0.5)),
        ],
    ));
    let cost = model.push(named(schema, "IfcAppliedValue", &[]));
    let quantity = model.push(named(
        schema,
        "IfcQuantityCount",
        &[
            ("Name", text("Crew count")),
            ("CountValue", Value::Real(4.0)),
        ],
    ));
    let labor = model.push(named(
        schema,
        "IfcLaborResource",
        &[
            ("GlobalId", text(GUID_A)),
            ("Name", text("Carpenter")),
            ("Identification", text("LAB-001")),
            ("LongDescription", text("Certified carpenter")),
            ("Usage", Value::Ref(usage_id)),
            ("BaseCosts", refs(&[cost])),
            ("BaseQuantity", Value::Ref(quantity)),
            ("PredefinedType", enumeration("CARPENTRY")),
        ],
    ));

    let view = ResourceView::for_model(&model).unwrap();
    let resource = view.resource(labor).unwrap();
    assert_eq!(resource.kind(), ResourceKind::Labor);
    assert_eq!(resource.name().unwrap(), Some("Carpenter"));
    assert_eq!(resource.identification().unwrap(), Some("LAB-001"));
    assert_eq!(
        resource.long_description().unwrap(),
        Some("Certified carpenter")
    );
    assert_eq!(resource.predefined_type().unwrap(), Some("CARPENTRY"));
    assert_eq!(resource.base_costs().unwrap(), vec![cost]);
    assert_eq!(resource.base_quantity().unwrap(), Some(quantity));

    let usage = resource.usage().unwrap().expect("usage");
    assert_eq!(usage.id(), usage_id);
    assert_eq!(usage.name().unwrap(), Some("Day shift"));
    assert_eq!(usage.schedule_work().unwrap(), Some("PT8H"));
    assert_eq!(usage.schedule_usage().unwrap(), Some(1.0));
    assert_eq!(usage.schedule_start().unwrap(), Some("2026-09-01T07:00:00"));
    assert_eq!(
        usage.schedule_finish().unwrap(),
        Some("2026-09-01T15:00:00")
    );
    assert_eq!(usage.is_over_allocated().unwrap(), Some(false));
    assert_eq!(usage.completion().unwrap(), Some(0.5));
}

#[test]
fn specializations_are_classified_without_claiming_other_resource_semantics() {
    let schema = ifc4();
    let mut model = model("IFC4");
    for (entity_type, expected) in [
        ("IfcConstructionEquipmentResource", ResourceKind::Equipment),
        ("IfcCrewResource", ResourceKind::Crew),
        ("IfcConstructionMaterialResource", ResourceKind::Material),
        ("IfcConstructionProductResource", ResourceKind::Product),
        ("IfcSubContractResource", ResourceKind::Subcontract),
    ] {
        let id = model.push(named(schema, entity_type, &[("GlobalId", text(GUID_A))]));
        assert_eq!(
            ResourceView::for_model(&model)
                .unwrap()
                .resource(id)
                .unwrap()
                .kind(),
            expected
        );
    }
}

#[test]
fn malformed_usage_and_predefined_values_are_typed_refusals() {
    let schema = ifc4();

    let mut bad_ratio = model("IFC4");
    let usage = bad_ratio.push(named(
        schema,
        "IfcResourceTime",
        &[("ScheduleUsage", Value::Real(f64::NAN))],
    ));
    assert!(matches!(
        ResourceView::for_model(&bad_ratio)
            .unwrap()
            .resource_time(usage)
            .unwrap()
            .schedule_usage(),
        Err(ResourceError::InvalidValue { .. })
    ));

    let mut unknown_enum = model("IFC4");
    let labor = unknown_enum.push(named(
        schema,
        "IfcLaborResource",
        &[
            ("GlobalId", text(GUID_A)),
            ("PredefinedType", enumeration("MAGIC")),
        ],
    ));
    assert!(matches!(
        ResourceView::for_model(&unknown_enum)
            .unwrap()
            .resource(labor)
            .unwrap()
            .predefined_type(),
        Err(ResourceError::InvalidEnumeration { .. })
    ));

    let mut user_defined = model("IFC4");
    let labor = user_defined.push(named(
        schema,
        "IfcLaborResource",
        &[
            ("GlobalId", text(GUID_A)),
            ("ObjectType", text("  ")),
            ("PredefinedType", enumeration("USERDEFINED")),
        ],
    ));
    assert!(matches!(
        ResourceView::for_model(&user_defined)
            .unwrap()
            .resource(labor)
            .unwrap()
            .predefined_type(),
        Err(ResourceError::SemanticViolation { .. })
    ));

    let mut wrong_usage = model("IFC4");
    let wall = wrong_usage.push(named(schema, "IfcWall", &[("GlobalId", text(GUID_A))]));
    let labor = wrong_usage.push(named(
        schema,
        "IfcLaborResource",
        &[("GlobalId", text(GUID_A)), ("Usage", Value::Ref(wall))],
    ));
    assert!(matches!(
        ResourceView::for_model(&wrong_usage)
            .unwrap()
            .resource(labor)
            .unwrap()
            .usage(),
        Err(ResourceError::WrongReferenceType { .. })
    ));
}
