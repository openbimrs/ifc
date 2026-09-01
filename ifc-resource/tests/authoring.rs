mod support;

use ifc_resource::{
    AllocationDraft, NestingDraft, ResourceDraft, ResourceEditor, ResourceError, ResourceKind,
    ResourceTimeDraft, ResourceView,
};

use support::{model, named, text, GUID_A, GUID_B, GUID_C, GUID_D, GUID_E, GUID_F, GUID_G};

#[test]
fn resource_authoring_round_trips_usage_allocation_and_nesting() {
    let schema = ifc_schema::ifc4();
    let mut model = model("IFC4");
    let task = model.push(named(schema, "IfcTask", &[("GlobalId", text(GUID_D))]));

    let (labor, crew, equipment, allocation, nesting) = {
        let mut editor = ResourceEditor::for_model(&mut model).unwrap();
        let usage = editor
            .create_time(
                ResourceTimeDraft::new()
                    .name("Day shift")
                    .schedule_work("PT8H")
                    .schedule_usage(1.0)
                    .completion(0.5),
            )
            .unwrap();
        let crew = editor
            .create_resource(
                ResourceDraft::new(ResourceKind::Crew, GUID_A)
                    .name("Envelope crew")
                    .identification("CREW-01")
                    .predefined_type("OFFICE")
                    .usage(usage),
            )
            .unwrap();
        let labor = editor
            .create_resource(
                ResourceDraft::new(ResourceKind::Labor, GUID_B)
                    .name("Carpenter")
                    .predefined_type("CARPENTRY"),
            )
            .unwrap();
        let equipment = editor
            .create_resource(
                ResourceDraft::new(ResourceKind::Equipment, GUID_C)
                    .name("Lift")
                    .predefined_type("ERECTING"),
            )
            .unwrap();
        let allocation = editor
            .create_allocation(AllocationDraft::new(GUID_E, labor, vec![task]).name("Task labour"))
            .unwrap();
        let nesting = editor
            .create_nesting(NestingDraft::new(GUID_F, crew, vec![labor, equipment]))
            .unwrap();
        (labor, crew, equipment, allocation, nesting)
    };

    let view = ResourceView::for_model(&model).unwrap();
    assert_eq!(
        view.resource(labor).unwrap().name().unwrap(),
        Some("Carpenter")
    );
    assert_eq!(
        view.resource(crew)
            .unwrap()
            .usage()
            .unwrap()
            .unwrap()
            .schedule_work()
            .unwrap(),
        Some("PT8H")
    );
    assert_eq!(
        view.allocation(allocation).unwrap().related_objects(),
        &[task]
    );
    assert_eq!(view.direct_members(crew).unwrap(), vec![labor, equipment]);
    assert!(model.get(nesting).is_some());
}

#[test]
fn rejected_resource_and_time_drafts_are_atomic() {
    let schema = ifc_schema::ifc4();
    let mut model = model("IFC4");
    let wall = model.push(named(schema, "IfcWall", &[("GlobalId", text(GUID_D))]));

    for invalid in [
        ResourceDraft::new(ResourceKind::Labor, "not-an-ifc-guid"),
        ResourceDraft::new(ResourceKind::Labor, "4O2Fr$t4X7Zf8NOew3FLOH"),
        ResourceDraft::new(ResourceKind::Labor, GUID_B).predefined_type("NOT_DECLARED"),
        ResourceDraft::new(ResourceKind::Labor, GUID_B)
            .predefined_type("USERDEFINED")
            .object_type(""),
        ResourceDraft::new(ResourceKind::Labor, GUID_B).usage(wall),
    ] {
        let len = model.len();
        let revision = model.revision();
        let error = ResourceEditor::for_model(&mut model)
            .unwrap()
            .create_resource(invalid)
            .unwrap_err();
        assert!(matches!(
            error,
            ResourceError::InvalidGlobalId
                | ResourceError::InvalidEnumeration { .. }
                | ResourceError::SemanticViolation { .. }
                | ResourceError::WrongReferenceType { .. }
        ));
        assert_eq!(model.len(), len);
        assert_eq!(model.revision(), revision);
    }

    let len = model.len();
    let revision = model.revision();
    let duplicate_id = ResourceEditor::for_model(&mut model)
        .unwrap()
        .create_resource(ResourceDraft::new(ResourceKind::Labor, GUID_D));
    assert!(
        matches!(duplicate_id, Err(ResourceError::SemanticViolation { .. })),
        "unexpected duplicate GlobalId result: {duplicate_id:?}"
    );
    assert_eq!(model.len(), len);
    assert_eq!(model.revision(), revision);

    let len = model.len();
    let revision = model.revision();
    let error = ResourceEditor::for_model(&mut model)
        .unwrap()
        .create_time(ResourceTimeDraft::new().schedule_usage(f64::INFINITY))
        .unwrap_err();
    assert!(matches!(error, ResourceError::InvalidDraft { .. }));
    assert_eq!(model.len(), len);
    assert_eq!(model.revision(), revision);
}

#[test]
fn rejected_relation_and_reference_drafts_are_atomic() {
    let schema = ifc_schema::ifc4();
    let mut model = model("IFC4");
    let wall = model.push(named(schema, "IfcWall", &[("GlobalId", text(GUID_D))]));
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
    let equipment = model.push(named(
        schema,
        "IfcConstructionEquipmentResource",
        &[("GlobalId", text(GUID_C))],
    ));
    let len = model.len();
    let revision = model.revision();

    let empty_allocation = ResourceEditor::for_model(&mut model)
        .unwrap()
        .create_allocation(AllocationDraft::new(GUID_F, labor, vec![]));
    assert!(matches!(
        empty_allocation,
        Err(ResourceError::InvalidCardinality { .. })
    ));

    let duplicate_allocation = ResourceEditor::for_model(&mut model)
        .unwrap()
        .create_allocation(AllocationDraft::new(GUID_F, labor, vec![wall, wall]));
    assert!(matches!(
        duplicate_allocation,
        Err(ResourceError::DuplicateReference { .. })
    ));

    let mismatched_allocation = ResourceEditor::for_model(&mut model)
        .unwrap()
        .create_allocation(
            AllocationDraft::new(GUID_F, labor, vec![wall]).related_objects_type("PROCESS"),
        );
    assert!(matches!(
        mismatched_allocation,
        Err(ResourceError::SemanticViolation { .. })
    ));

    let wrong_nesting = ResourceEditor::for_model(&mut model)
        .unwrap()
        .create_nesting(NestingDraft::new(GUID_F, crew, vec![wall]));
    assert!(matches!(
        wrong_nesting,
        Err(ResourceError::WrongReferenceType { .. })
    ));

    let self_nesting = ResourceEditor::for_model(&mut model)
        .unwrap()
        .create_nesting(NestingDraft::new(GUID_F, crew, vec![crew]));
    assert!(matches!(
        self_nesting,
        Err(ResourceError::SemanticViolation { .. })
    ));

    assert_eq!(model.len(), len);
    assert_eq!(model.revision(), revision);

    ResourceEditor::for_model(&mut model)
        .unwrap()
        .create_nesting(NestingDraft::new(GUID_E, crew, vec![labor]))
        .unwrap();
    let nested_len = model.len();
    let nested_revision = model.revision();

    let cycle = ResourceEditor::for_model(&mut model)
        .unwrap()
        .create_nesting(NestingDraft::new(GUID_F, labor, vec![crew]));
    assert!(matches!(
        cycle,
        Err(ResourceError::SemanticViolation { .. })
    ));

    let second_parent = ResourceEditor::for_model(&mut model)
        .unwrap()
        .create_nesting(NestingDraft::new(GUID_G, equipment, vec![labor]));
    assert!(matches!(
        second_parent,
        Err(ResourceError::SemanticViolation { .. })
    ));
    assert_eq!(model.len(), nested_len);
    assert_eq!(model.revision(), nested_revision);
}
