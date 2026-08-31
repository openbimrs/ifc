mod support;

use ifc_model::Value;
use ifc_schema::{ifc2x3, ifc4, ifc4x3, Schema};
use ifc_structural::{ActionKind, StructuralError, StructuralView};

use support::{enumeration, model, named, text, GUID};

fn action_fields(
    schema: &Schema,
    action_type: &str,
    applied_load: ifc_model::EntityId,
    predefined: Option<&str>,
) -> Vec<(&'static str, Value)> {
    let mut fields = vec![
        ("GlobalId", text(GUID)),
        ("AppliedLoad", Value::Ref(applied_load)),
        ("GlobalOrLocal", enumeration("GLOBAL_COORDS")),
    ];
    if schema
        .attribute_names(action_type)
        .iter()
        .any(|name| name.eq_ignore_ascii_case("DestabilizingLoad"))
        && schema.version() == Some(ifc_schema::SchemaVersion::Ifc2x3)
    {
        fields.push(("DestabilizingLoad", Value::Bool(false)));
    }
    if schema
        .attribute_names(action_type)
        .iter()
        .any(|name| name.eq_ignore_ascii_case("ProjectedOrTrue"))
    {
        fields.push(("ProjectedOrTrue", enumeration("TRUE_LENGTH")));
    }
    if let Some(predefined) = predefined {
        if schema
            .attribute_names(action_type)
            .iter()
            .any(|name| name.eq_ignore_ascii_case("PredefinedType"))
        {
            fields.push(("PredefinedType", enumeration(predefined)));
            if predefined.eq_ignore_ascii_case("USERDEFINED") {
                fields.push(("ObjectType", text("Custom action")));
            }
        }
    }
    fields
}

#[test]
fn ifc2x3_linear_and_planar_actions_have_no_predefined_type() {
    let schema = ifc2x3();
    let mut model = model("IFC2X3");
    for (action_type, load_type, component, expected_kind) in [
        (
            "IfcStructuralLinearAction",
            "IfcStructuralLoadLinearForce",
            "LinearForceX",
            ActionKind::Curve,
        ),
        (
            "IfcStructuralPlanarAction",
            "IfcStructuralLoadPlanarForce",
            "PlanarForceX",
            ActionKind::Surface,
        ),
    ] {
        let load = model.push(named(schema, load_type, &[(component, Value::Real(1.0))]));
        let action = model.push(named(
            schema,
            action_type,
            &action_fields(schema, action_type, load, None),
        ));
        let action = StructuralView::new(&model, schema).action(action).unwrap();
        assert_eq!(action.kind(), expected_kind);
        assert_eq!(action.predefined_type().unwrap(), None);
        assert_eq!(action.projected_or_true().unwrap(), Some("TRUE_LENGTH"));
        assert_eq!(action.applied_load().unwrap(), load);
    }
}

#[test]
fn action_suitable_load_type_is_enforced_in_all_schemas() {
    for (schema, token) in [
        (ifc2x3(), "IFC2X3"),
        (ifc4(), "IFC4"),
        (ifc4x3(), "IFC4X3_ADD2"),
    ] {
        let mut model = model(token);
        let single = model.push(named(
            schema,
            "IfcStructuralLoadSingleForce",
            &[("ForceX", Value::Real(1.0))],
        ));
        let planar = model.push(named(
            schema,
            "IfcStructuralLoadPlanarForce",
            &[("PlanarForceX", Value::Real(1.0))],
        ));
        let temperature = model.push(named(schema, "IfcStructuralLoadTemperature", &[]));

        let linear = model.push(named(
            schema,
            "IfcStructuralLinearAction",
            &action_fields(schema, "IfcStructuralLinearAction", single, Some("CONST")),
        ));
        let point = model.push(named(
            schema,
            "IfcStructuralPointAction",
            &action_fields(schema, "IfcStructuralPointAction", planar, None),
        ));
        assert!(matches!(
            StructuralView::new(&model, schema)
                .action(linear)
                .unwrap()
                .applied_load(),
            Err(StructuralError::WrongReferenceType { .. })
        ));
        assert!(matches!(
            StructuralView::new(&model, schema)
                .action(point)
                .unwrap()
                .applied_load(),
            Err(StructuralError::WrongReferenceType { .. })
        ));

        let valid = model.push(named(
            schema,
            "IfcStructuralLinearAction",
            &action_fields(
                schema,
                "IfcStructuralLinearAction",
                temperature,
                Some("CONST"),
            ),
        ));
        assert_eq!(
            StructuralView::new(&model, schema)
                .action(valid)
                .unwrap()
                .applied_load()
                .unwrap(),
            temperature
        );
    }
}

#[test]
fn ifc4_linear_and_planar_actions_require_const_predefined_type() {
    for (schema, token) in [(ifc4(), "IFC4"), (ifc4x3(), "IFC4X3_ADD2")] {
        let mut model = model(token);
        for (action_type, load_type) in [
            ("IfcStructuralLinearAction", "IfcStructuralLoadLinearForce"),
            ("IfcStructuralPlanarAction", "IfcStructuralLoadPlanarForce"),
        ] {
            let load = model.push(named(schema, load_type, &[]));
            let action = model.push(named(
                schema,
                action_type,
                &action_fields(schema, action_type, load, Some("USERDEFINED")),
            ));
            assert!(matches!(
                StructuralView::new(&model, schema)
                    .action(action)
                    .unwrap()
                    .predefined_type(),
                Err(StructuralError::SemanticViolation { .. })
            ));
        }
    }
}
