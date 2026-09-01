mod support;

use std::sync::Arc;

use ifc_model::Value;
use ifc_schema::{ifc2x3, ifc4, ifc4x3, Schema, SchemaVersion};
use ifc_structural::{
    AxisValues, BoundaryConditionKind, ConnectionConditionKind, MemberKind, StiffnessValue,
    StructuralError, StructuralView,
};

use support::{model, named, text};

fn typed(type_name: &str, value: Value) -> Value {
    Value::Typed {
        type_name: Arc::from(type_name),
        value: Box::new(value),
    }
}

fn has(schema: &Schema, entity: &str, attribute: &str) -> bool {
    schema
        .attribute_names(entity)
        .iter()
        .any(|candidate| candidate.eq_ignore_ascii_case(attribute))
}

#[test]
fn condition_and_varying_layouts_are_pinned_in_all_supported_schemas() {
    for schema in [ifc2x3(), ifc4(), ifc4x3()] {
        for entity in [
            "IfcBoundaryEdgeCondition",
            "IfcBoundaryFaceCondition",
            "IfcBoundaryNodeCondition",
            "IfcBoundaryNodeConditionWarping",
            "IfcFailureConnectionCondition",
            "IfcSlippageConnectionCondition",
            "IfcStructuralCurveMemberVarying",
            "IfcStructuralSurfaceMemberVarying",
        ] {
            assert!(!schema.attribute_names(entity).is_empty(), "{entity}");
        }
        for attribute in [
            "TensionFailureX",
            "TensionFailureY",
            "TensionFailureZ",
            "CompressionFailureX",
            "CompressionFailureY",
            "CompressionFailureZ",
        ] {
            assert!(has(schema, "IfcFailureConnectionCondition", attribute));
        }
        for attribute in ["SlippageX", "SlippageY", "SlippageZ"] {
            assert!(has(schema, "IfcSlippageConnectionCondition", attribute));
        }
    }

    assert!(has(
        ifc2x3(),
        "IfcBoundaryNodeCondition",
        "LinearStiffnessX"
    ));
    assert!(has(
        ifc4(),
        "IfcBoundaryNodeCondition",
        "TranslationalStiffnessX"
    ));
    assert!(has(
        ifc4x3(),
        "IfcBoundaryNodeCondition",
        "TranslationalStiffnessX"
    ));
    assert!(has(
        ifc2x3(),
        "IfcStructuralSurfaceMemberVarying",
        "SubsequentThickness"
    ));
    assert!(!has(
        ifc4(),
        "IfcStructuralSurfaceMemberVarying",
        "SubsequentThickness"
    ));
    assert!(!has(
        ifc4x3(),
        "IfcStructuralSurfaceMemberVarying",
        "SubsequentThickness"
    ));
}

#[test]
fn boundary_conditions_preserve_boolean_and_measure_selects() {
    for (schema, token) in [(ifc4(), "IFC4"), (ifc4x3(), "IFC4X3_ADD2")] {
        let mut model = model(token);
        let condition = model.push(named(
            schema,
            "IfcBoundaryNodeConditionWarping",
            &[
                ("Name", text("Pinned base")),
                ("TranslationalStiffnessX", Value::Bool(true)),
                (
                    "TranslationalStiffnessY",
                    typed("IFCLINEARSTIFFNESSMEASURE", Value::Real(12.5)),
                ),
                ("RotationalStiffnessZ", Value::Bool(false)),
                (
                    "WarpingStiffness",
                    typed("IFCWARPINGMOMENTMEASURE", Value::Real(7.0)),
                ),
            ],
        ));
        let condition = StructuralView::new(&model, schema)
            .boundary_condition(condition)
            .unwrap();
        assert_eq!(condition.kind(), BoundaryConditionKind::NodeWarping);
        assert_eq!(condition.name().unwrap(), Some("Pinned base"));
        assert_eq!(
            condition.translational_stiffnesses().unwrap(),
            AxisValues {
                x: Some(StiffnessValue::Boolean(true)),
                y: Some(StiffnessValue::Measure(12.5)),
                z: None,
            }
        );
        assert_eq!(
            condition.rotational_stiffnesses().unwrap(),
            AxisValues {
                x: None,
                y: None,
                z: Some(StiffnessValue::Boolean(false)),
            }
        );
        assert_eq!(
            condition.warping_stiffness().unwrap(),
            Some(StiffnessValue::Measure(7.0))
        );
    }
}

#[test]
fn ifc2x3_condition_names_are_normalized_but_booleans_are_refused() {
    let schema = ifc2x3();
    let mut model = model("IFC2X3");
    let edge = model.push(named(
        schema,
        "IfcBoundaryEdgeCondition",
        &[
            ("LinearStiffnessByLengthX", Value::Real(2.0)),
            ("RotationalStiffnessByLengthY", Value::Real(3.0)),
        ],
    ));
    let edge = StructuralView::new(&model, schema)
        .boundary_condition(edge)
        .unwrap();
    assert_eq!(edge.kind(), BoundaryConditionKind::Edge);
    assert_eq!(
        edge.translational_stiffnesses().unwrap().x,
        Some(StiffnessValue::Measure(2.0))
    );
    assert_eq!(
        edge.rotational_stiffnesses().unwrap().y,
        Some(StiffnessValue::Measure(3.0))
    );

    let invalid = model.push(named(
        schema,
        "IfcBoundaryNodeCondition",
        &[("LinearStiffnessX", Value::Bool(true))],
    ));
    assert!(matches!(
        StructuralView::new(&model, schema)
            .boundary_condition(invalid)
            .unwrap()
            .translational_stiffnesses(),
        Err(StructuralError::InvalidValue { .. })
    ));
}

#[test]
fn connection_conditions_project_failure_and_slippage_families() {
    for (schema, token) in [
        (ifc2x3(), "IFC2X3"),
        (ifc4(), "IFC4"),
        (ifc4x3(), "IFC4X3_ADD2"),
    ] {
        let mut model = model(token);
        let failure = model.push(named(
            schema,
            "IfcFailureConnectionCondition",
            &[
                ("TensionFailureX", Value::Real(100.0)),
                ("CompressionFailureZ", Value::Real(-80.0)),
            ],
        ));
        let slippage = model.push(named(
            schema,
            "IfcSlippageConnectionCondition",
            &[("SlippageY", Value::Real(0.002))],
        ));
        let view = StructuralView::new(&model, schema);
        let failure = view.connection_condition(failure).unwrap();
        assert_eq!(failure.kind(), ConnectionConditionKind::Failure);
        let limits = failure.failure_limits().unwrap().unwrap();
        assert_eq!(limits.tension.x, Some(100.0));
        assert_eq!(limits.compression.z, Some(-80.0));
        assert_eq!(failure.slippages().unwrap(), None);

        let slippage = view.connection_condition(slippage).unwrap();
        assert_eq!(slippage.kind(), ConnectionConditionKind::Slippage);
        assert_eq!(slippage.failure_limits().unwrap(), None);
        assert_eq!(slippage.slippages().unwrap().unwrap().y, Some(0.002));
    }
}

#[test]
fn varying_members_are_typed_and_ifc2x3_thickness_data_is_preserved() {
    for (schema, token) in [(ifc4(), "IFC4"), (ifc4x3(), "IFC4X3_ADD2")] {
        let mut model = model(token);
        let curve = model.push(named(
            schema,
            "IfcStructuralCurveMemberVarying",
            &[(
                "PredefinedType",
                Value::Enum(Arc::from("RIGID_JOINED_MEMBER")),
            )],
        ));
        let surface = model.push(named(
            schema,
            "IfcStructuralSurfaceMemberVarying",
            &[("PredefinedType", Value::Enum(Arc::from("SHELL")))],
        ));
        let view = StructuralView::new(&model, schema);
        assert_eq!(view.member(curve).unwrap().kind(), MemberKind::CurveVarying);
        let surface = view.member(surface).unwrap();
        assert_eq!(surface.kind(), MemberKind::SurfaceVarying);
        assert_eq!(surface.subsequent_thicknesses().unwrap(), None);
        assert_eq!(surface.varying_thickness_location().unwrap(), None);
    }

    let schema = ifc2x3();
    let mut model = model("IFC2X3");
    let location = model.push(named(schema, "IfcShapeAspect", &[]));
    let surface = model.push(named(
        schema,
        "IfcStructuralSurfaceMemberVarying",
        &[
            ("Thickness", Value::Real(0.2)),
            (
                "SubsequentThickness",
                Value::List(vec![Value::Real(0.25), Value::Real(0.3)]),
            ),
            ("VaryingThicknessLocation", Value::Ref(location)),
        ],
    ));
    let surface = StructuralView::new(&model, schema).member(surface).unwrap();
    assert_eq!(surface.kind(), MemberKind::SurfaceVarying);
    assert_eq!(
        surface.subsequent_thicknesses().unwrap(),
        Some(vec![0.25, 0.3])
    );
    assert_eq!(
        surface.varying_thickness_location().unwrap(),
        Some(location)
    );
}

#[test]
fn ifc2x3_varying_surface_enforces_required_positive_thickness_sequence() {
    let schema = ifc2x3();
    let mut model = model("IFC2X3");
    let location = model.push(named(schema, "IfcShapeAspect", &[]));

    let missing_thickness = model.push(named(
        schema,
        "IfcStructuralSurfaceMemberVarying",
        &[
            (
                "SubsequentThickness",
                Value::List(vec![Value::Real(0.25), Value::Real(0.3)]),
            ),
            ("VaryingThicknessLocation", Value::Ref(location)),
        ],
    ));
    assert!(matches!(
        StructuralView::new(&model, schema)
            .member(missing_thickness)
            .unwrap()
            .subsequent_thicknesses(),
        Err(StructuralError::SemanticViolation {
            rule: "IfcStructuralSurfaceMemberVarying.WR61",
            ..
        })
    ));

    let too_short = model.push(named(
        schema,
        "IfcStructuralSurfaceMemberVarying",
        &[
            ("Thickness", Value::Real(0.2)),
            ("SubsequentThickness", Value::List(vec![Value::Real(0.25)])),
            ("VaryingThicknessLocation", Value::Ref(location)),
        ],
    ));
    assert!(matches!(
        StructuralView::new(&model, schema)
            .member(too_short)
            .unwrap()
            .subsequent_thicknesses(),
        Err(StructuralError::InvalidCardinality { minimum: 2, .. })
    ));

    let non_positive = model.push(named(
        schema,
        "IfcStructuralSurfaceMemberVarying",
        &[
            ("Thickness", Value::Real(0.2)),
            (
                "SubsequentThickness",
                Value::List(vec![Value::Real(0.25), Value::Real(-0.1)]),
            ),
            ("VaryingThicknessLocation", Value::Ref(location)),
        ],
    ));
    assert!(matches!(
        StructuralView::new(&model, schema)
            .member(non_positive)
            .unwrap()
            .subsequent_thicknesses(),
        Err(StructuralError::InvalidValue { .. })
    ));
}

#[test]
fn selected_schema_version_stays_explicit() {
    assert_eq!(ifc2x3().version(), Some(SchemaVersion::Ifc2x3));
    assert_eq!(ifc4().version(), Some(SchemaVersion::Ifc4));
    assert_eq!(ifc4x3().version(), Some(SchemaVersion::Ifc4x3));
}
