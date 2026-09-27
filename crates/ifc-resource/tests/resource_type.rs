//! `IfcConstructionResourceType` projection and type-assignment checks.

mod support;

use ifc_resource::{
    ResourceDraft, ResourceEditor, ResourceError, ResourceKind, ResourceTypeKind, ResourceView,
};
use ifc_schema::ifc4;

use support::{
    enumeration, model, named, refs, text, GUID_A, GUID_B, GUID_C, GUID_D, GUID_E, GUID_F,
};

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

/// Authored resource types project through the same reader.
///
/// The existing cases in this file hand-build the entity; this one
/// authors it, so a wrong attribute name would surface as a projection
/// mismatch rather than passing unnoticed.
#[test]
fn authored_resource_types_project_through_the_reader() {
    let schema = ifc4();
    let mut model = model("IFC4");
    let cost = model.push(named(schema, "IfcAppliedValue", &[]));

    let mut editor = ResourceEditor::for_model(&mut model).unwrap();
    let ty = editor
        .create_resource_type(
            ResourceDraft::new(ResourceKind::Labor, GUID_A)
                .name("Carpentry crew type")
                .identification("TYP-LAB-1")
                .base_costs(vec![cost])
                .predefined_type("CARPENTRY"),
        )
        .expect("authored resource type");

    let view = ResourceView::for_model(&model).unwrap();
    let projected = view.resource_type(ty).unwrap();
    assert_eq!(projected.kind(), ResourceTypeKind::Labor);
    assert_eq!(projected.name().unwrap(), "Carpentry crew type");
    assert_eq!(projected.identification().unwrap(), Some("TYP-LAB-1"));
    assert_eq!(projected.predefined_type().unwrap(), Some("CARPENTRY"));
    assert_eq!(projected.base_costs().unwrap(), vec![cost]);
}

/// The rules that differ between a resource type and an occurrence.
///
/// PredefinedType is optional on the occurrence and required on the
/// type; Usage belongs to the occurrence only. Both would otherwise
/// parse and quietly mean something else.
#[test]
fn resource_type_authoring_enforces_its_own_rules() {
    let schema = ifc4();
    let mut model = model("IFC4");
    let time = model.push(named(schema, "IfcResourceTime", &[]));
    let mut editor = ResourceEditor::for_model(&mut model).unwrap();

    assert!(
        editor
            .create_resource_type(ResourceDraft::new(ResourceKind::Crew, GUID_A))
            .is_err(),
        "a resource type without a PredefinedType is refused",
    );
    assert!(
        editor
            .create_resource_type(
                ResourceDraft::new(ResourceKind::Crew, GUID_A).predefined_type("USERDEFINED"),
            )
            .is_err(),
        "USERDEFINED without an ObjectType names nothing",
    );
    assert!(
        editor
            .create_resource_type(
                ResourceDraft::new(ResourceKind::Crew, GUID_A)
                    .predefined_type("OFFICE")
                    .usage(time),
            )
            .is_err(),
        "Usage belongs to the occurrence, not the type",
    );
    assert!(
        editor
            .create_resource_type(
                ResourceDraft::new(ResourceKind::Crew, "not-a-guid").predefined_type("OFFICE"),
            )
            .is_err(),
        "a malformed GlobalId is refused",
    );
    assert!(
        editor
            .create_resource_type(
                ResourceDraft::new(ResourceKind::Crew, GUID_B).predefined_type("NOT_A_REAL_TOKEN"),
            )
            .is_err(),
        "an unknown enumeration token is refused",
    );
}

/// Every resource kind maps to its own type entity.
///
/// One staging path serves all six, so a mapping mistake would send
/// a crew type to the labor entity and still project a valid-looking
/// resource type. Each kind is checked against the name it must write,
/// and ApplicableOccurrence against its own slot.
#[test]
fn each_resource_kind_writes_its_own_type_entity() {
    let mut model = model("IFC4");
    let mut editor = ResourceEditor::for_model(&mut model).unwrap();
    let cases = [
        (
            ResourceKind::Labor,
            "IFCLABORRESOURCETYPE",
            "CARPENTRY",
            GUID_A,
        ),
        (
            ResourceKind::Equipment,
            "IFCCONSTRUCTIONEQUIPMENTRESOURCETYPE",
            "DEMOLISHING",
            GUID_B,
        ),
        (ResourceKind::Crew, "IFCCREWRESOURCETYPE", "OFFICE", GUID_C),
        (
            ResourceKind::Material,
            "IFCCONSTRUCTIONMATERIALRESOURCETYPE",
            "CONCRETE",
            GUID_D,
        ),
        (
            ResourceKind::Product,
            "IFCCONSTRUCTIONPRODUCTRESOURCETYPE",
            "ASSEMBLY",
            GUID_E,
        ),
        (
            ResourceKind::Subcontract,
            "IFCSUBCONTRACTRESOURCETYPE",
            "PURCHASE",
            GUID_F,
        ),
    ];
    let mut authored = Vec::new();
    for (kind, expected, predefined, guid) in cases {
        let id = editor
            .create_resource_type(
                ResourceDraft::new(kind, guid)
                    .object_type("OnSite")
                    .predefined_type(predefined),
            )
            .unwrap_or_else(|e| panic!("{expected} authored: {e:?}"));
        authored.push((id, expected));
    }

    for (id, expected) in authored {
        let entity = model.get(id).expect("written");
        assert!(
            entity.type_name.eq_ignore_ascii_case(expected),
            "expected {expected}, got {}",
            entity.type_name,
        );
        // ApplicableOccurrence is slot 4 on every IfcTypeObject.
        assert_eq!(
            entity.attributes[4],
            ifc_model::Value::Text("OnSite".into()),
            "{expected}: ApplicableOccurrence at slot 4",
        );
    }
}
