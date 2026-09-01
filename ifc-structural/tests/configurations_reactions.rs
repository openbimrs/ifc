mod support;

use std::sync::Arc;

use ifc_model::Value;
use ifc_schema::{ifc2x3, ifc4, ifc4x3};
use ifc_structural::{CoordinateSystem, ReactionKind, StructuralError, StructuralView};

use support::{model, named};

#[test]
fn configuration_and_reaction_layouts_are_pinned() {
    for schema in [ifc4(), ifc4x3()] {
        let configuration = schema.attribute_names("IfcStructuralLoadConfiguration");
        assert!(configuration.ends_with(&["Name", "Values", "Locations"]));
        for entity in [
            "IfcStructuralPointReaction",
            "IfcStructuralCurveReaction",
            "IfcStructuralSurfaceReaction",
        ] {
            let attributes = schema.attribute_names(entity);
            assert!(attributes.contains(&"AppliedLoad"));
            assert!(attributes.contains(&"GlobalOrLocal"));
        }
    }
    assert!(ifc2x3().entity("IfcStructuralLoadConfiguration").is_none());
    assert!(ifc2x3().entity("IfcStructuralCurveReaction").is_none());
    assert!(ifc2x3().entity("IfcStructuralSurfaceReaction").is_none());
    assert!(ifc2x3().entity("IfcStructuralPointReaction").is_some());
}

#[test]
fn load_configuration_projects_ordered_values_and_locations() {
    for (schema, token) in [(ifc4(), "IFC4"), (ifc4x3(), "IFC4X3_ADD2")] {
        let mut model = model(token);
        let first = model.push(named(schema, "IfcStructuralLoadSingleForce", &[]));
        let second = model.push(named(schema, "IfcStructuralLoadTemperature", &[]));
        let configuration = model.push(named(
            schema,
            "IfcStructuralLoadConfiguration",
            &[
                ("Name", Value::Text(Arc::from("envelope"))),
                (
                    "Values",
                    Value::List(vec![Value::Ref(first), Value::Ref(second)]),
                ),
                (
                    "Locations",
                    Value::List(vec![
                        Value::List(vec![Value::Real(0.0)]),
                        Value::List(vec![Value::Real(1.0), Value::Real(2.0)]),
                    ]),
                ),
            ],
        ));
        let configuration = StructuralView::new(&model, schema)
            .load_configuration(configuration)
            .unwrap();
        assert_eq!(configuration.name().unwrap(), Some("envelope"));
        assert_eq!(configuration.values().unwrap(), vec![first, second]);
        assert_eq!(
            configuration.locations().unwrap(),
            Some(vec![vec![0.0], vec![1.0, 2.0]])
        );
    }
}

#[test]
fn load_configuration_refuses_wrong_values_locations_and_non_finite_numbers() {
    let schema = ifc4();
    let mut model = model("IFC4");
    let wrong = model.push(named(schema, "IfcWall", &[]));
    let load = model.push(named(schema, "IfcStructuralLoadSingleForce", &[]));
    let wrong_value = model.push(named(
        schema,
        "IfcStructuralLoadConfiguration",
        &[("Values", Value::List(vec![Value::Ref(wrong)]))],
    ));
    assert!(matches!(
        StructuralView::new(&model, schema)
            .load_configuration(wrong_value)
            .unwrap()
            .values(),
        Err(StructuralError::WrongType { .. })
    ));

    let empty = model.push(named(
        schema,
        "IfcStructuralLoadConfiguration",
        &[("Values", Value::List(Vec::new()))],
    ));
    assert!(matches!(
        StructuralView::new(&model, schema)
            .load_configuration(empty)
            .unwrap()
            .values(),
        Err(StructuralError::InvalidCardinality {
            attribute: "Values",
            ..
        })
    ));

    let mismatch = model.push(named(
        schema,
        "IfcStructuralLoadConfiguration",
        &[
            ("Values", Value::List(vec![Value::Ref(load)])),
            (
                "Locations",
                Value::List(vec![
                    Value::List(vec![Value::Real(0.0)]),
                    Value::List(vec![Value::Real(1.0)]),
                ]),
            ),
        ],
    ));
    assert!(matches!(
        StructuralView::new(&model, schema)
            .load_configuration(mismatch)
            .unwrap()
            .locations(),
        Err(StructuralError::SemanticViolation {
            rule: "IfcStructuralLoadConfiguration.ValidListSize",
            ..
        })
    ));

    let duplicate = model.push(named(
        schema,
        "IfcStructuralLoadConfiguration",
        &[
            (
                "Values",
                Value::List(vec![Value::Ref(load), Value::Ref(load)]),
            ),
            (
                "Locations",
                Value::List(vec![
                    Value::List(vec![Value::Real(0.0)]),
                    Value::List(vec![Value::Real(0.0)]),
                ]),
            ),
        ],
    ));
    assert!(matches!(
        StructuralView::new(&model, schema)
            .load_configuration(duplicate)
            .unwrap()
            .locations(),
        Err(StructuralError::SemanticViolation {
            rule: "IfcStructuralLoadConfiguration.UniqueLocations",
            ..
        })
    ));

    let invalid_location_cardinality = model.push(named(
        schema,
        "IfcStructuralLoadConfiguration",
        &[
            ("Values", Value::List(vec![Value::Ref(load)])),
            (
                "Locations",
                Value::List(vec![Value::List(vec![
                    Value::Real(0.0),
                    Value::Real(1.0),
                    Value::Real(2.0),
                ])]),
            ),
        ],
    ));
    assert!(matches!(
        StructuralView::new(&model, schema)
            .load_configuration(invalid_location_cardinality)
            .unwrap()
            .locations(),
        Err(StructuralError::InvalidCardinality {
            attribute: "Locations",
            ..
        })
    ));

    let non_finite = model.push(named(
        schema,
        "IfcStructuralLoadConfiguration",
        &[
            ("Values", Value::List(vec![Value::Ref(load)])),
            (
                "Locations",
                Value::List(vec![Value::List(vec![Value::Real(f64::NAN)])]),
            ),
        ],
    ));
    assert!(matches!(
        StructuralView::new(&model, schema)
            .load_configuration(non_finite)
            .unwrap()
            .locations(),
        Err(StructuralError::InvalidValue { .. })
    ));
}

#[test]
fn reactions_project_cross_version_metadata_without_computing_results() {
    for (schema, token) in [
        (ifc2x3(), "IFC2X3"),
        (ifc4(), "IFC4"),
        (ifc4x3(), "IFC4X3_ADD2"),
    ] {
        let mut model = model(token);
        let load = model.push(named(schema, "IfcStructuralLoadSingleDisplacement", &[]));
        let reaction = model.push(named(
            schema,
            "IfcStructuralPointReaction",
            &[
                ("Name", Value::Text(Arc::from("support reaction"))),
                ("AppliedLoad", Value::Ref(load)),
                ("GlobalOrLocal", Value::Enum(Arc::from("GLOBAL_COORDS"))),
            ],
        ));
        let reaction = StructuralView::new(&model, schema)
            .reaction(reaction)
            .unwrap();
        assert_eq!(reaction.kind(), ReactionKind::Point);
        assert_eq!(reaction.name().unwrap(), Some("support reaction"));
        assert_eq!(reaction.applied_load().unwrap(), load);
        assert_eq!(
            reaction.coordinate_system().unwrap(),
            CoordinateSystem::Global
        );
        assert_eq!(reaction.predefined_type().unwrap(), None);
    }
}

#[test]
fn ifc4_curve_and_surface_reactions_are_typed_and_semantically_checked() {
    for (schema, token) in [(ifc4(), "IFC4"), (ifc4x3(), "IFC4X3_ADD2")] {
        let mut model = model(token);
        let load = model.push(named(schema, "IfcStructuralLoadSingleForce", &[]));
        let curve = model.push(named(
            schema,
            "IfcStructuralCurveReaction",
            &[
                ("AppliedLoad", Value::Ref(load)),
                ("GlobalOrLocal", Value::Enum(Arc::from("LOCAL_COORDS"))),
                ("PredefinedType", Value::Enum(Arc::from("CONST"))),
            ],
        ));
        let surface = model.push(named(
            schema,
            "IfcStructuralSurfaceReaction",
            &[
                ("AppliedLoad", Value::Ref(load)),
                ("GlobalOrLocal", Value::Enum(Arc::from("GLOBAL_COORDS"))),
                ("PredefinedType", Value::Enum(Arc::from("CONST"))),
            ],
        ));
        let view = StructuralView::new(&model, schema);
        assert_eq!(view.reaction(curve).unwrap().kind(), ReactionKind::Curve);
        assert_eq!(
            view.reaction(curve).unwrap().predefined_type().unwrap(),
            Some("CONST")
        );
        assert_eq!(
            view.reaction(surface).unwrap().kind(),
            ReactionKind::Surface
        );

        let invalid = model.push(named(
            schema,
            "IfcStructuralCurveReaction",
            &[
                ("AppliedLoad", Value::Ref(load)),
                ("GlobalOrLocal", Value::Enum(Arc::from("GLOBAL_COORDS"))),
                ("PredefinedType", Value::Enum(Arc::from("SINUS"))),
            ],
        ));
        assert!(matches!(
            StructuralView::new(&model, schema)
                .reaction(invalid)
                .unwrap()
                .predefined_type(),
            Err(StructuralError::SemanticViolation {
                rule: "IfcStructuralCurveReaction.SuitablePredefinedType",
                ..
            })
        ));

        let user_defined = model.push(named(
            schema,
            "IfcStructuralSurfaceReaction",
            &[
                ("AppliedLoad", Value::Ref(load)),
                ("GlobalOrLocal", Value::Enum(Arc::from("GLOBAL_COORDS"))),
                ("PredefinedType", Value::Enum(Arc::from("USERDEFINED"))),
            ],
        ));
        assert!(matches!(
            StructuralView::new(&model, schema)
                .reaction(user_defined)
                .unwrap()
                .predefined_type(),
            Err(StructuralError::SemanticViolation { .. })
        ));
    }
}

#[test]
fn point_reaction_refuses_incompatible_applied_load() {
    let schema = ifc4();
    let mut model = model("IFC4");
    let linear = model.push(named(schema, "IfcStructuralLoadLinearForce", &[]));
    let reaction = model.push(named(
        schema,
        "IfcStructuralPointReaction",
        &[
            ("AppliedLoad", Value::Ref(linear)),
            ("GlobalOrLocal", Value::Enum(Arc::from("GLOBAL_COORDS"))),
        ],
    ));
    assert!(matches!(
        StructuralView::new(&model, schema)
            .reaction(reaction)
            .unwrap()
            .applied_load(),
        Err(StructuralError::WrongReferenceType { .. })
    ));
}

#[test]
fn result_group_reactions_preserve_relation_and_set_order() {
    let schema = ifc4();
    let mut model = model("IFC4");
    let load = model.push(named(schema, "IfcStructuralLoadSingleForce", &[]));
    let first = model.push(named(
        schema,
        "IfcStructuralPointReaction",
        &[
            ("AppliedLoad", Value::Ref(load)),
            ("GlobalOrLocal", Value::Enum(Arc::from("GLOBAL_COORDS"))),
        ],
    ));
    let second = model.push(named(
        schema,
        "IfcStructuralPointReaction",
        &[
            ("AppliedLoad", Value::Ref(load)),
            ("GlobalOrLocal", Value::Enum(Arc::from("LOCAL_COORDS"))),
        ],
    ));
    let group = model.push(named(
        schema,
        "IfcStructuralResultGroup",
        &[
            ("TheoryType", Value::Enum(Arc::from("FIRST_ORDER_THEORY"))),
            ("IsLinear", Value::Bool(true)),
        ],
    ));
    model.push(named(
        schema,
        "IfcRelAssignsToGroup",
        &[
            (
                "RelatedObjects",
                Value::List(vec![Value::Ref(second), Value::Ref(first)]),
            ),
            ("RelatingGroup", Value::Ref(group)),
        ],
    ));
    assert_eq!(
        StructuralView::new(&model, schema)
            .reactions_for_result_group(group)
            .unwrap(),
        vec![second, first]
    );
}

#[test]
fn result_group_reactions_refuse_non_reaction_members() {
    let schema = ifc4();
    let mut model = model("IFC4");
    let group = model.push(named(
        schema,
        "IfcStructuralResultGroup",
        &[
            ("TheoryType", Value::Enum(Arc::from("FIRST_ORDER_THEORY"))),
            ("IsLinear", Value::Bool(true)),
        ],
    ));
    let action = model.push(named(schema, "IfcStructuralPointAction", &[]));
    model.push(named(
        schema,
        "IfcRelAssignsToGroup",
        &[
            ("RelatedObjects", Value::List(vec![Value::Ref(action)])),
            ("RelatingGroup", Value::Ref(group)),
        ],
    ));
    assert!(matches!(
        StructuralView::new(&model, schema).reactions_for_result_group(group),
        Err(StructuralError::WrongReferenceType { .. })
    ));
}
