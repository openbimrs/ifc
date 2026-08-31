mod support;

use ifc_model::{Entity, EntityId, Value};
use ifc_schema::{ifc2x3, ifc4, ifc4x3};
use ifc_structural::{
    AnalysisModelType, ConnectionKind, LoadKind, MemberKind, StructuralError, StructuralView,
};

use support::{enumeration, model, named, refs, text, GUID};

#[test]
fn selects_the_canonical_schema_from_the_file_header() {
    let model = model("IFC4X3_ADD2");
    let view = StructuralView::for_model(&model).unwrap();
    assert_eq!(view.schema().name(), "IFC4X3_ADD2");
}

#[test]
fn rejects_missing_ambiguous_and_unknown_schema_headers() {
    let empty = ifc_model::Model::new();
    assert!(matches!(
        StructuralView::for_model(&empty),
        Err(StructuralError::MissingSchema)
    ));

    let mut ambiguous = model("IFC4");
    ambiguous.header_mut().schema.push("IFC2X3".into());
    assert!(matches!(
        StructuralView::for_model(&ambiguous),
        Err(StructuralError::AmbiguousSchema { .. })
    ));

    let unknown = model("IFC5");
    assert!(matches!(
        StructuralView::for_model(&unknown),
        Err(StructuralError::UnsupportedSchema { .. })
    ));
}

#[test]
fn analysis_model_resolves_groups_and_version_specific_shared_placement() {
    let schema = ifc4();
    let mut model = model("IFC4");
    let load = model.push(named(
        schema,
        "IfcStructuralLoadGroup",
        &[
            ("GlobalId", text(GUID)),
            ("PredefinedType", enumeration("LOAD_CASE")),
            ("ActionType", enumeration("VARIABLE_Q")),
            ("ActionSource", enumeration("LIVE_LOAD_Q")),
            ("Coefficient", Value::Real(1.5)),
        ],
    ));
    let result = model.push(named(
        schema,
        "IfcStructuralResultGroup",
        &[
            ("GlobalId", text("1O2Fr$t4X7Zf8NOew3FLOH")),
            ("TheoryType", enumeration("FIRST_ORDER_THEORY")),
            ("IsLinear", Value::Bool(true)),
        ],
    ));
    let placement = model.push(named(schema, "IfcLocalPlacement", &[]));
    let analysis = model.push(named(
        schema,
        "IfcStructuralAnalysisModel",
        &[
            ("GlobalId", text("0O2Fr$t4X7Zf8NOew3FLOH")),
            ("Name", text("Frame")),
            ("PredefinedType", enumeration("LOADING_3D")),
            ("LoadedBy", refs(&[load])),
            ("HasResults", refs(&[result])),
            ("SharedPlacement", Value::Ref(placement)),
        ],
    ));

    let projection = StructuralView::new(&model, schema)
        .analysis_model(analysis)
        .unwrap();
    assert_eq!(projection.name().unwrap(), Some("Frame"));
    assert_eq!(
        projection.predefined_type().unwrap(),
        AnalysisModelType::Loading3d
    );
    assert_eq!(projection.loaded_by().unwrap(), vec![load]);
    assert_eq!(projection.result_groups().unwrap(), vec![result]);
    assert_eq!(projection.shared_placement().unwrap(), Some(placement));
}

#[test]
fn ifc2x3_omits_shared_placement_without_shifting_other_fields() {
    let schema = ifc2x3();
    let mut model = model("IFC2X3");
    let analysis = model.push(named(
        schema,
        "IfcStructuralAnalysisModel",
        &[
            ("GlobalId", text(GUID)),
            ("OwnerHistory", Value::Ref(EntityId(99))),
            ("PredefinedType", enumeration("LOADING_3D")),
        ],
    ));
    let projection = StructuralView::new(&model, schema)
        .analysis_model(analysis)
        .unwrap();
    assert_eq!(projection.shared_placement().unwrap(), None);
}

#[test]
fn analysis_model_rejects_wrong_group_reference_types() {
    let schema = ifc4();
    let mut model = model("IFC4");
    let wall = model.push(named(schema, "IfcWall", &[("GlobalId", text(GUID))]));
    let analysis = model.push(named(
        schema,
        "IfcStructuralAnalysisModel",
        &[
            ("GlobalId", text("1O2Fr$t4X7Zf8NOew3FLOH")),
            ("PredefinedType", enumeration("LOADING_3D")),
            ("LoadedBy", refs(&[wall])),
        ],
    ));
    let err = StructuralView::new(&model, schema)
        .analysis_model(analysis)
        .unwrap()
        .loaded_by()
        .unwrap_err();
    assert!(matches!(err, StructuralError::WrongReferenceType { .. }));
}

#[test]
fn user_defined_analysis_model_requires_nonblank_object_type() {
    let schema = ifc4x3();
    let mut model = model("IFC4X3_ADD2");
    let analysis = model.push(named(
        schema,
        "IfcStructuralAnalysisModel",
        &[
            ("GlobalId", text(GUID)),
            ("ObjectType", text("  ")),
            ("PredefinedType", enumeration("USERDEFINED")),
        ],
    ));
    assert!(matches!(
        StructuralView::new(&model, schema)
            .analysis_model(analysis)
            .unwrap()
            .predefined_type(),
        Err(StructuralError::SemanticViolation { .. })
    ));
}

#[test]
fn member_and_connection_views_resolve_schema_drift() {
    let schema = ifc4x3();
    let mut model = model("IFC4X3_ADD2");
    let axis = model.push(named(
        schema,
        "IfcDirection",
        &[(
            "DirectionRatios",
            Value::List(vec![Value::Real(1.0), Value::Real(0.0), Value::Real(0.0)]),
        )],
    ));
    let curve = model.push(named(
        schema,
        "IfcStructuralCurveMember",
        &[
            ("GlobalId", text(GUID)),
            ("PredefinedType", enumeration("RIGID_JOINED_MEMBER")),
            ("Axis", Value::Ref(axis)),
        ],
    ));
    let connection = model.push(named(
        schema,
        "IfcStructuralCurveConnection",
        &[
            ("GlobalId", text("1O2Fr$t4X7Zf8NOew3FLOH")),
            ("AxisDirection", Value::Ref(axis)),
        ],
    ));
    let view = StructuralView::new(&model, schema);
    let member = view.member(curve).unwrap();
    assert_eq!(member.kind(), MemberKind::Curve);
    assert_eq!(member.axis().unwrap(), Some(axis));
    let connection = view.connection(connection).unwrap();
    assert_eq!(connection.kind(), ConnectionKind::Curve);
    assert_eq!(connection.axis().unwrap(), Some(axis));
}

#[test]
fn member_schema_conditions_are_enforced() {
    let schema = ifc4();

    let mut missing_axis_model = model("IFC4");
    let missing_axis = missing_axis_model.push(named(
        schema,
        "IfcStructuralCurveMember",
        &[
            ("GlobalId", text(GUID)),
            ("PredefinedType", enumeration("RIGID_JOINED_MEMBER")),
        ],
    ));
    assert!(StructuralView::new(&missing_axis_model, schema)
        .member(missing_axis)
        .unwrap()
        .axis()
        .is_err());

    let mut user_defined_model = model("IFC4");
    let axis = user_defined_model.push(Entity::new(
        "IFCDIRECTION",
        vec![Value::List(vec![Value::Real(1.0), Value::Real(0.0)])],
    ));
    let user_defined = user_defined_model.push(named(
        schema,
        "IfcStructuralCurveMember",
        &[
            ("GlobalId", text(GUID)),
            ("PredefinedType", enumeration("USERDEFINED")),
            ("Axis", Value::Ref(axis)),
        ],
    ));
    assert!(matches!(
        StructuralView::new(&user_defined_model, schema)
            .member(user_defined)
            .unwrap()
            .predefined_type(),
        Err(StructuralError::SemanticViolation { .. })
    ));

    let mut thickness_model = model("IFC4");
    let surface = thickness_model.push(named(
        schema,
        "IfcStructuralSurfaceMember",
        &[
            ("GlobalId", text(GUID)),
            ("PredefinedType", enumeration("SHELL")),
            ("Thickness", Value::Real(0.0)),
        ],
    ));
    assert!(matches!(
        StructuralView::new(&thickness_model, schema)
            .member(surface)
            .unwrap()
            .thickness(),
        Err(StructuralError::SemanticViolation { .. })
    ));
}

#[test]
fn curve_connection_axis_and_numeric_measures_are_strict() {
    let schema = ifc4();
    let mut model = model("IFC4");
    let connection = model.push(named(
        schema,
        "IfcStructuralCurveConnection",
        &[("GlobalId", text(GUID))],
    ));
    assert!(StructuralView::new(&model, schema)
        .connection(connection)
        .unwrap()
        .axis()
        .is_err());

    let load = model.push(named(
        schema,
        "IfcStructuralLoadSingleForce",
        &[("ForceX", Value::Real(f64::NAN))],
    ));
    assert!(StructuralView::new(&model, schema)
        .load(load)
        .unwrap()
        .components()
        .is_err());
}

#[test]
fn core_static_loads_expose_components_and_temperature_name_drift() {
    for (schema, token, names) in [
        (
            ifc2x3(),
            "IFC2X3",
            ["DeltaT_Constant", "DeltaT_Y", "DeltaT_Z"],
        ),
        (ifc4(), "IFC4", ["DeltaTConstant", "DeltaTY", "DeltaTZ"]),
        (
            ifc4x3(),
            "IFC4X3_ADD2",
            ["DeltaTConstant", "DeltaTY", "DeltaTZ"],
        ),
    ] {
        let mut model = model(token);
        let temperature = model.push(named(
            schema,
            "IfcStructuralLoadTemperature",
            &[
                (names[0], Value::Real(10.0)),
                (names[1], Value::Real(2.0)),
                (names[2], Value::Real(-1.0)),
            ],
        ));
        let load = StructuralView::new(&model, schema)
            .load(temperature)
            .unwrap();
        assert_eq!(load.kind(), LoadKind::Temperature);
        assert_eq!(
            load.components().unwrap(),
            vec![Some(10.0), Some(2.0), Some(-1.0)]
        );
    }
}

#[test]
fn rejects_forged_entity_type_even_if_slots_look_compatible() {
    let schema = ifc4();
    let mut model = model("IFC4");
    let fake = model.push(Entity::new("IFCWALL", vec![Value::Null; 9]));
    assert!(matches!(
        StructuralView::new(&model, schema).member(fake),
        Err(StructuralError::WrongType { .. })
    ));
}

#[test]
fn member_predefined_type_is_required() {
    let schema = ifc4();
    let mut model = model("IFC4");
    let member = model.push(named(
        schema,
        "IfcStructuralCurveMember",
        &[("GlobalId", text(GUID))],
    ));
    let error = StructuralView::new(&model, schema)
        .member(member)
        .unwrap()
        .predefined_type()
        .unwrap_err();
    assert!(matches!(
        error,
        StructuralError::InvalidValue {
            attribute: "PredefinedType",
            ..
        }
    ));
}
