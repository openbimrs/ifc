//! `IfcConstructionResourceType` projection and type-assignment checks.

mod support;

use ifc_resource::{ResourceError, ResourceTypeKind, ResourceView};
use ifc_schema::ifc4;

use support::{enumeration, model, named, refs, text, GUID_A, GUID_B};

#[test]
fn labor_type_projects_identity_and_userdefined_rule() {
    let schema = ifc4();
    let mut model = model("IFC4");
    let cost = model.push(named(schema, "IfcAppliedValue", &[]));
    let quantity = model.push(named(
        schema,
        "IfcQuantityCount",
        &[
            ("Name", text("Crew count")),
            ("CountValue", ifc_model::Value::Real(4.0)),
        ],
    ));
    let ty = model.push(named(
        schema,
        "IfcLaborResourceType",
        &[
            ("Name", text("Carpentry crew type")),
            ("Identification", text("TYP-LAB-1")),
            ("LongDescription", text("Standard carpentry crew")),
            ("ResourceType", text("Skilled")),
            ("BaseCosts", refs(&[cost])),
            ("BaseQuantity", ifc_model::Value::Ref(quantity)),
            ("PredefinedType", enumeration("CARPENTRY")),
        ],
    ));

    let view = ResourceView::for_model(&model).unwrap();
    let projected = view.resource_type(ty).unwrap();
    assert_eq!(projected.kind(), ResourceTypeKind::Labor);
    assert_eq!(projected.name().unwrap(), "Carpentry crew type");
    assert_eq!(projected.identification().unwrap(), Some("TYP-LAB-1"));
    assert_eq!(
        projected.long_description().unwrap(),
        Some("Standard carpentry crew")
    );
    assert_eq!(projected.resource_type().unwrap(), Some("Skilled"));
    assert_eq!(projected.predefined_type().unwrap(), Some("CARPENTRY"));
    assert_eq!(projected.base_costs().unwrap(), vec![cost]);
    assert_eq!(projected.base_quantity().unwrap(), Some(quantity));
}

#[test]
fn all_six_type_kinds_are_classified() {
    let schema = ifc4();
    let mut model = model("IFC4");
    for (entity_type, expected) in [
        (
            "IfcConstructionEquipmentResourceType",
            ResourceTypeKind::Equipment,
        ),
        ("IfcCrewResourceType", ResourceTypeKind::Crew),
        (
            "IfcConstructionMaterialResourceType",
            ResourceTypeKind::Material,
        ),
        (
            "IfcConstructionProductResourceType",
            ResourceTypeKind::Product,
        ),
        ("IfcSubContractResourceType", ResourceTypeKind::Subcontract),
    ] {
        let id = model.push(named(schema, entity_type, &[("Name", text("T"))]));
        assert_eq!(
            ResourceView::for_model(&model)
                .unwrap()
                .resource_type(id)
                .unwrap()
                .kind(),
            expected
        );
    }
}

#[test]
fn userdefined_without_resource_type_label_is_a_typed_refusal() {
    let schema = ifc4();
    let mut model = model("IFC4");
    let ty = model.push(named(
        schema,
        "IfcLaborResourceType",
        &[
            ("Name", text("T")),
            ("PredefinedType", enumeration("USERDEFINED")),
        ],
    ));
    assert!(matches!(
        ResourceView::for_model(&model)
            .unwrap()
            .resource_type(ty)
            .unwrap()
            .predefined_type(),
        Err(ResourceError::SemanticViolation { .. })
    ));
}

#[test]
fn assigned_resource_type_resolves_via_rel_defines_by_type() {
    let schema = ifc4();
    let mut model = model("IFC4");
    let ty = model.push(named(
        schema,
        "IfcLaborResourceType",
        &[("Name", text("T"))],
    ));
    let labor = model.push(named(
        schema,
        "IfcLaborResource",
        &[("GlobalId", text(GUID_A))],
    ));
    model.push(named(
        schema,
        "IfcRelDefinesByType",
        &[
            ("RelatedObjects", refs(&[labor])),
            ("RelatingType", ifc_model::Value::Ref(ty)),
        ],
    ));

    let view = ResourceView::for_model(&model).unwrap();
    assert_eq!(view.assigned_resource_type(labor).unwrap(), Some(ty));

    let unassigned = model.push(named(
        schema,
        "IfcLaborResource",
        &[("GlobalId", text(GUID_B))],
    ));
    let view = ResourceView::for_model(&model).unwrap();
    assert_eq!(view.assigned_resource_type(unassigned).unwrap(), None);
}

#[test]
fn two_relations_naming_the_same_type_for_the_same_occurrence_is_accepted() {
    let schema = ifc4();
    let mut model = model("IFC4");
    let ty = model.push(named(
        schema,
        "IfcLaborResourceType",
        &[("Name", text("T"))],
    ));
    let labor = model.push(named(
        schema,
        "IfcLaborResource",
        &[("GlobalId", text(GUID_A))],
    ));
    model.push(named(
        schema,
        "IfcRelDefinesByType",
        &[
            ("RelatedObjects", refs(&[labor])),
            ("RelatingType", ifc_model::Value::Ref(ty)),
        ],
    ));
    model.push(named(
        schema,
        "IfcRelDefinesByType",
        &[
            ("RelatedObjects", refs(&[labor])),
            ("RelatingType", ifc_model::Value::Ref(ty)),
        ],
    ));

    let view = ResourceView::for_model(&model).unwrap();
    assert_eq!(view.assigned_resource_type(labor).unwrap(), Some(ty));
}

#[test]
fn two_relations_naming_different_types_for_the_same_occurrence_is_a_typed_refusal() {
    let schema = ifc4();
    let mut model = model("IFC4");
    let ty_a = model.push(named(
        schema,
        "IfcLaborResourceType",
        &[("Name", text("A"))],
    ));
    let ty_b = model.push(named(
        schema,
        "IfcLaborResourceType",
        &[("Name", text("B"))],
    ));
    let labor = model.push(named(
        schema,
        "IfcLaborResource",
        &[("GlobalId", text(GUID_A))],
    ));
    model.push(named(
        schema,
        "IfcRelDefinesByType",
        &[
            ("RelatedObjects", refs(&[labor])),
            ("RelatingType", ifc_model::Value::Ref(ty_a)),
        ],
    ));
    model.push(named(
        schema,
        "IfcRelDefinesByType",
        &[
            ("RelatedObjects", refs(&[labor])),
            ("RelatingType", ifc_model::Value::Ref(ty_b)),
        ],
    ));

    assert!(matches!(
        ResourceView::for_model(&model)
            .unwrap()
            .assigned_resource_type(labor),
        Err(ResourceError::SemanticViolation { .. })
    ));
}
