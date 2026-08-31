mod support;

use ifc_model::{EntityId, Value};
use ifc_schema::ifc4;
use ifc_structural::{ActionKind, CoordinateSystem, StructuralError, StructuralView};

use support::{enumeration, model, named, text, GUID};

#[test]
fn load_and_result_groups_expose_declared_semantics() {
    let schema = ifc4();
    let mut model = model("IFC4");
    let load_group = model.push(named(
        schema,
        "IfcStructuralLoadGroup",
        &[
            ("GlobalId", text(GUID)),
            ("PredefinedType", enumeration("LOAD_CASE")),
            ("ActionType", enumeration("VARIABLE_Q")),
            ("ActionSource", enumeration("WIND_W")),
            ("Coefficient", Value::Real(1.35)),
            ("Purpose", text("ULS")),
        ],
    ));
    let result_group = model.push(named(
        schema,
        "IfcStructuralResultGroup",
        &[
            ("GlobalId", text("1O2Fr$t4X7Zf8NOew3FLOH")),
            ("TheoryType", enumeration("FIRST_ORDER_THEORY")),
            ("ResultForLoadGroup", Value::Ref(load_group)),
            ("IsLinear", Value::Bool(true)),
        ],
    ));

    let view = StructuralView::new(&model, schema);
    let load = view.load_group(load_group).unwrap();
    assert_eq!(load.predefined_type().unwrap(), "LOAD_CASE");
    assert_eq!(load.action_type().unwrap(), "VARIABLE_Q");
    assert_eq!(load.action_source().unwrap(), "WIND_W");
    assert_eq!(load.coefficient().unwrap(), Some(1.35));
    assert_eq!(load.purpose().unwrap(), Some("ULS"));

    let no_coefficient = model.push(named(
        schema,
        "IfcStructuralLoadGroup",
        &[
            ("GlobalId", text("0kVyZQY5P1M8Q8tb8gMCvI")),
            ("PredefinedType", enumeration("LOAD_GROUP")),
            ("ActionType", enumeration("PERMANENT_G")),
            ("ActionSource", enumeration("DEAD_LOAD_G")),
        ],
    ));
    assert_eq!(
        StructuralView::new(&model, schema)
            .load_group(no_coefficient)
            .unwrap()
            .coefficient()
            .unwrap(),
        None
    );

    let result = StructuralView::new(&model, schema)
        .result_group(result_group)
        .unwrap();
    assert_eq!(result.theory_type().unwrap(), "FIRST_ORDER_THEORY");
    assert_eq!(result.result_for_load_group().unwrap(), Some(load_group));
    assert!(result.is_linear().unwrap());
}

#[test]
fn point_action_validates_applied_load_and_coordinate_semantics() {
    let schema = ifc4();
    let mut model = model("IFC4");
    let load = model.push(named(
        schema,
        "IfcStructuralLoadSingleForce",
        &[("ForceZ", Value::Real(-12.0))],
    ));
    let action = model.push(named(
        schema,
        "IfcStructuralPointAction",
        &[
            ("GlobalId", text(GUID)),
            ("AppliedLoad", Value::Ref(load)),
            ("GlobalOrLocal", enumeration("GLOBAL_COORDS")),
            ("DestabilizingLoad", Value::Bool(false)),
        ],
    ));

    let view = StructuralView::new(&model, schema);
    let action = view.action(action).unwrap();
    assert_eq!(action.kind(), ActionKind::Point);
    assert_eq!(action.applied_load().unwrap(), load);
    assert_eq!(
        action.coordinate_system().unwrap(),
        CoordinateSystem::Global
    );
    assert_eq!(action.destabilizing_load().unwrap(), Some(false));
    assert_eq!(action.projected_or_true().unwrap(), None);

    let curve = model.push(named(
        schema,
        "IfcStructuralCurveAction",
        &[
            ("GlobalId", text("1O2Fr$t4X7Zf8NOew3FLOH")),
            ("AppliedLoad", Value::Ref(load)),
            ("GlobalOrLocal", enumeration("LOCAL_COORDS")),
            ("ProjectedOrTrue", enumeration("TRUE_LENGTH")),
            ("PredefinedType", enumeration("CONST")),
        ],
    ));
    let curve = StructuralView::new(&model, schema).action(curve).unwrap();
    assert_eq!(curve.kind(), ActionKind::Curve);
    assert_eq!(curve.projected_or_true().unwrap(), Some("TRUE_LENGTH"));
    assert_eq!(curve.predefined_type().unwrap(), Some("CONST"));
}

#[test]
fn forged_enum_tokens_are_rejected_against_the_canonical_schema() {
    let schema = ifc4();
    let mut model = model("IFC4");
    let group = model.push(named(
        schema,
        "IfcStructuralLoadGroup",
        &[
            ("GlobalId", text(GUID)),
            ("PredefinedType", enumeration("BOGUS")),
            ("ActionType", enumeration("PERMANENT_G")),
            ("ActionSource", enumeration("DEAD_LOAD_G")),
        ],
    ));
    assert!(StructuralView::new(&model, schema)
        .load_group(group)
        .unwrap()
        .predefined_type()
        .is_err());

    let load = model.push(named(
        schema,
        "IfcStructuralLoadLinearForce",
        &[("LinearForceX", Value::Real(1.0))],
    ));
    let action = model.push(named(
        schema,
        "IfcStructuralCurveAction",
        &[
            ("GlobalId", text(GUID)),
            ("AppliedLoad", Value::Ref(load)),
            ("GlobalOrLocal", enumeration("GLOBAL_COORDS")),
            ("ProjectedOrTrue", enumeration("BOGUS")),
            ("PredefinedType", enumeration("CONST")),
        ],
    ));
    assert!(StructuralView::new(&model, schema)
        .action(action)
        .unwrap()
        .projected_or_true()
        .is_err());
}

#[test]
fn userdefined_groups_require_object_type_in_ifc4_but_not_ifc2x3() {
    for (schema, token, must_reject) in [
        (ifc_schema::ifc2x3(), "IFC2X3", false),
        (ifc4(), "IFC4", true),
    ] {
        let mut model = model(token);
        let group = model.push(named(
            schema,
            "IfcStructuralLoadGroup",
            &[
                ("GlobalId", text(GUID)),
                ("PredefinedType", enumeration("USERDEFINED")),
                ("ActionType", enumeration("VARIABLE_Q")),
                ("ActionSource", enumeration("WIND_W")),
            ],
        ));
        let result = StructuralView::new(&model, schema)
            .load_group(group)
            .unwrap()
            .predefined_type();
        assert_eq!(result.is_err(), must_reject);
    }

    let schema = ifc4();
    let mut model = model("IFC4");
    let result = model.push(named(
        schema,
        "IfcStructuralResultGroup",
        &[
            ("GlobalId", text(GUID)),
            ("TheoryType", enumeration("USERDEFINED")),
            ("IsLinear", Value::Bool(true)),
        ],
    ));
    assert!(matches!(
        StructuralView::new(&model, schema)
            .result_group(result)
            .unwrap()
            .theory_type(),
        Err(StructuralError::SemanticViolation { .. })
    ));
}

fn curve_action_is_rejected(global_or_local: &str, projected: &str, predefined: &str) -> bool {
    let schema = ifc4();
    let mut model = model("IFC4");
    let load = model.push(named(
        schema,
        "IfcStructuralLoadLinearForce",
        &[("LinearForceX", Value::Real(1.0))],
    ));
    let action = model.push(named(
        schema,
        "IfcStructuralCurveAction",
        &[
            ("GlobalId", text(GUID)),
            ("AppliedLoad", Value::Ref(load)),
            ("GlobalOrLocal", enumeration(global_or_local)),
            ("ProjectedOrTrue", enumeration(projected)),
            ("PredefinedType", enumeration(predefined)),
        ],
    ));
    StructuralView::new(&model, schema)
        .action(action)
        .unwrap()
        .predefined_type()
        .is_err()
}

#[test]
fn projected_curve_action_requires_global_coordinates() {
    assert!(curve_action_is_rejected(
        "LOCAL_COORDS",
        "PROJECTED_LENGTH",
        "CONST"
    ));
}

#[test]
fn equidistant_curve_action_is_rejected() {
    assert!(curve_action_is_rejected(
        "GLOBAL_COORDS",
        "TRUE_LENGTH",
        "EQUIDISTANT"
    ));
}

#[test]
fn userdefined_curve_action_requires_object_type() {
    assert!(curve_action_is_rejected(
        "GLOBAL_COORDS",
        "TRUE_LENGTH",
        "USERDEFINED"
    ));
}

#[test]
fn destabilizing_load_requiredness_tracks_schema_version() {
    for (schema, token, required) in [
        (ifc_schema::ifc2x3(), "IFC2X3", true),
        (ifc4(), "IFC4", false),
    ] {
        let mut model = model(token);
        let load = model.push(named(
            schema,
            "IfcStructuralLoadSingleForce",
            &[("ForceX", Value::Real(1.0))],
        ));
        let action = model.push(named(
            schema,
            "IfcStructuralPointAction",
            &[
                ("GlobalId", text(GUID)),
                ("AppliedLoad", Value::Ref(load)),
                ("GlobalOrLocal", enumeration("GLOBAL_COORDS")),
            ],
        ));
        let result = StructuralView::new(&model, schema)
            .action(action)
            .unwrap()
            .destabilizing_load();
        if required {
            assert!(result.is_err());
        } else {
            assert_eq!(result.unwrap(), None);
        }
    }
}

#[test]
fn action_rejects_dangling_applied_load() {
    let schema = ifc4();
    let mut model = model("IFC4");
    let action = model.push(named(
        schema,
        "IfcStructuralPointAction",
        &[
            ("GlobalId", text(GUID)),
            ("AppliedLoad", Value::Ref(EntityId(999))),
            ("GlobalOrLocal", enumeration("LOCAL_COORDS")),
        ],
    ));
    let error = StructuralView::new(&model, schema)
        .action(action)
        .unwrap()
        .applied_load()
        .unwrap_err();
    assert!(matches!(error, StructuralError::DanglingReference { .. }));
}
