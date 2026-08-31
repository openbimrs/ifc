mod support;

use ifc_model::{EntityId, Value};
use ifc_schema::ifc4;
use ifc_structural::{StructuralError, StructuralView};

use support::{enumeration, model, named, refs, text, GUID};

#[test]
fn analysis_items_follow_group_assignments_in_file_order() {
    let schema = ifc4();
    let mut model = model("IFC4");
    let member = model.push(named(
        schema,
        "IfcStructuralCurveMember",
        &[
            ("GlobalId", text(GUID)),
            ("PredefinedType", enumeration("RIGID_JOINED_MEMBER")),
        ],
    ));
    let connection = model.push(named(
        schema,
        "IfcStructuralPointConnection",
        &[("GlobalId", text("1O2Fr$t4X7Zf8NOew3FLOH"))],
    ));
    let analysis = model.push(named(
        schema,
        "IfcStructuralAnalysisModel",
        &[
            ("GlobalId", text("0O2Fr$t4X7Zf8NOew3FLOH")),
            ("PredefinedType", enumeration("LOADING_3D")),
        ],
    ));
    model.push(named(
        schema,
        "IfcRelAssignsToGroup",
        &[
            ("GlobalId", text("3O2Fr$t4X7Zf8NOew3FLOH")),
            ("RelatedObjects", refs(&[member, connection])),
            ("RelatingGroup", Value::Ref(analysis)),
        ],
    ));

    assert_eq!(
        StructuralView::new(&model, schema)
            .analysis_items(analysis)
            .unwrap(),
        vec![member, connection]
    );
}

#[test]
fn analysis_items_follow_relation_records_in_file_order() {
    let schema = ifc4();
    let mut model = model("IFC4");
    let first_member = model.push(named(
        schema,
        "IfcStructuralCurveMember",
        &[
            ("GlobalId", text(GUID)),
            ("PredefinedType", enumeration("RIGID_JOINED_MEMBER")),
        ],
    ));
    let second_member = model.push(named(
        schema,
        "IfcStructuralCurveMember",
        &[
            ("GlobalId", text("1O2Fr$t4X7Zf8NOew3FLOH")),
            ("PredefinedType", enumeration("RIGID_JOINED_MEMBER")),
        ],
    ));
    let analysis = model.push(named(
        schema,
        "IfcStructuralAnalysisModel",
        &[
            ("GlobalId", text("0O2Fr$t4X7Zf8NOew3FLOH")),
            ("PredefinedType", enumeration("LOADING_3D")),
        ],
    ));
    for (relation_id, global_id, member) in [
        (EntityId(100), "3O2Fr$t4X7Zf8NOew3FLOH", first_member),
        (EntityId(99), "4O2Fr$t4X7Zf8NOew3FLOH", second_member),
    ] {
        model.insert(
            relation_id,
            named(
                schema,
                "IfcRelAssignsToGroup",
                &[
                    ("GlobalId", text(global_id)),
                    ("RelatedObjects", refs(&[member])),
                    ("RelatingGroup", Value::Ref(analysis)),
                ],
            ),
        );
    }

    assert_eq!(
        StructuralView::new(&model, schema)
            .analysis_items(analysis)
            .unwrap(),
        vec![first_member, second_member]
    );
}

#[test]
fn member_connections_validate_both_structural_endpoints() {
    let schema = ifc4();
    let mut model = model("IFC4");
    let member = model.push(named(
        schema,
        "IfcStructuralCurveMember",
        &[
            ("GlobalId", text(GUID)),
            ("PredefinedType", enumeration("RIGID_JOINED_MEMBER")),
        ],
    ));
    let connection = model.push(named(
        schema,
        "IfcStructuralPointConnection",
        &[("GlobalId", text("1O2Fr$t4X7Zf8NOew3FLOH"))],
    ));
    let relation = model.push(named(
        schema,
        "IfcRelConnectsStructuralMember",
        &[
            ("GlobalId", text("0O2Fr$t4X7Zf8NOew3FLOH")),
            ("RelatingStructuralMember", Value::Ref(member)),
            ("RelatedStructuralConnection", Value::Ref(connection)),
            ("SupportedLength", Value::Real(2.5)),
        ],
    ));
    let edges = StructuralView::new(&model, schema)
        .member_connections(member)
        .unwrap();
    assert_eq!(edges.len(), 1);
    assert_eq!(edges[0].relation_id(), relation);
    assert_eq!(edges[0].member().unwrap(), member);
    assert_eq!(edges[0].connection().unwrap(), connection);
    assert_eq!(edges[0].supported_length().unwrap(), Some(2.5));
}

#[test]
fn activity_assignments_enforce_the_element_or_structural_item_select() {
    let schema = ifc4();
    let mut model = model("IFC4");
    let load = model.push(named(schema, "IfcStructuralLoadSingleForce", &[]));
    let activity = model.push(named(
        schema,
        "IfcStructuralPointAction",
        &[
            ("GlobalId", text(GUID)),
            ("AppliedLoad", Value::Ref(load)),
            ("GlobalOrLocal", enumeration("GLOBAL_COORDS")),
        ],
    ));
    let invalid_target = model.push(named(
        schema,
        "IfcStructuralAnalysisModel",
        &[
            ("GlobalId", text("1O2Fr$t4X7Zf8NOew3FLOH")),
            ("PredefinedType", enumeration("LOADING_3D")),
        ],
    ));
    let valid_target = model.push(named(
        schema,
        "IfcStructuralPointConnection",
        &[("GlobalId", text("2O2Fr$t4X7Zf8NOew3FLOH"))],
    ));
    model.push(named(
        schema,
        "IfcRelConnectsStructuralActivity",
        &[
            ("GlobalId", text("0O2Fr$t4X7Zf8NOew3FLOH")),
            ("RelatingElement", Value::Ref(invalid_target)),
            ("RelatedStructuralActivity", Value::Ref(activity)),
        ],
    ));
    assert!(matches!(
        StructuralView::new(&model, schema).activities_for(valid_target),
        Err(StructuralError::WrongReferenceType { .. })
    ));
}

#[test]
fn analysis_group_assignment_rejects_self_reference() {
    let schema = ifc4();
    let mut model = model("IFC4");
    let analysis = model.push(named(
        schema,
        "IfcStructuralAnalysisModel",
        &[
            ("GlobalId", text(GUID)),
            ("PredefinedType", enumeration("LOADING_3D")),
        ],
    ));
    model.push(named(
        schema,
        "IfcRelAssignsToGroup",
        &[
            ("GlobalId", text("1O2Fr$t4X7Zf8NOew3FLOH")),
            ("RelatedObjects", refs(&[analysis])),
            ("RelatingGroup", Value::Ref(analysis)),
        ],
    ));
    assert!(matches!(
        StructuralView::new(&model, schema).analysis_items(analysis),
        Err(StructuralError::SemanticViolation { .. })
    ));
}

#[test]
fn malformed_related_objects_aggregate_is_not_silently_ignored() {
    let schema = ifc4();
    let mut model = model("IFC4");
    let analysis = model.push(named(
        schema,
        "IfcStructuralAnalysisModel",
        &[
            ("GlobalId", text(GUID)),
            ("PredefinedType", enumeration("LOADING_3D")),
        ],
    ));
    model.push(named(
        schema,
        "IfcRelAssignsToGroup",
        &[
            ("GlobalId", text("1O2Fr$t4X7Zf8NOew3FLOH")),
            ("RelatedObjects", Value::Ref(EntityId(999))),
            ("RelatingGroup", Value::Ref(analysis)),
        ],
    ));
    assert!(matches!(
        StructuralView::new(&model, schema).analysis_items(analysis),
        Err(StructuralError::InvalidValue { .. })
    ));
}
