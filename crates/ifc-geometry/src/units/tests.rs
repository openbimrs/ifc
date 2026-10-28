use super::*;
use ifc_model::Entity;

fn id(n: u32) -> EntityId {
    EntityId(n.into())
}

fn si(unit_type: &str, prefix: Option<&str>, name: &str) -> Entity {
    Entity::new(
        "IFCSIUNIT",
        vec![
            Value::Derived,
            Value::Enum(unit_type.into()),
            prefix.map_or(Value::Null, |p| Value::Enum(p.into())),
            Value::Enum(name.into()),
        ],
    )
}

fn measure(value: Value, component: Value) -> Entity {
    Entity::new("IFCMEASUREWITHUNIT", vec![value, component])
}

fn real(type_name: &str, value: f64) -> Value {
    Value::Typed {
        type_name: type_name.into(),
        value: Box::new(Value::Real(value)),
    }
}

fn conversion(type_name: &str, unit_type: &str, name: &str, factor: u32) -> Entity {
    Entity::new(
        type_name,
        vec![
            Value::Null,
            Value::Enum(unit_type.into()),
            Value::Text(name.into()),
            Value::Ref(id(factor)),
        ],
    )
}

fn assignment(units: &[u32]) -> Entity {
    Entity::new(
        "IFCUNITASSIGNMENT",
        vec![Value::List(
            units.iter().map(|u| Value::Ref(id(*u))).collect(),
        )],
    )
}

fn project(assignment: Option<u32>) -> Entity {
    let mut slots = vec![Value::Null; 9];
    if let Some(a) = assignment {
        slots[8] = Value::Ref(id(a));
    }
    Entity::new("IFCPROJECT", slots)
}

/// `#10` is the unit assignment, `#11` the project.
fn model_with(entities: Vec<(u32, Entity)>, units: &[u32]) -> Model {
    let mut model = Model::new();
    for (n, entity) in entities {
        model.insert(id(n), entity);
    }
    model.insert(id(10), assignment(units));
    model.insert(id(11), project(Some(10)));
    model
}

fn model_with_length_unit(prefix: Option<&str>) -> Model {
    model_with(vec![(1, si("LENGTHUNIT", prefix, "METRE"))], &[1])
}

#[test]
fn metres_are_the_identity() {
    let scale = resolve(&model_with_length_unit(None)).expect("units");
    assert_eq!(scale.length_to_metres, 1.0);
    assert_eq!(scale.angle_to_radians, None);
    assert!(scale.is_metric_identity());
}

/// The case that silently breaks viewers: a 3000 mm wall is 3 m.
#[test]
fn millimetres_scale_coordinates_down_by_a_thousand() {
    let scale = resolve(&model_with_length_unit(Some("MILLI"))).expect("units");
    assert_eq!(scale.length_to_metres, 1e-3);
    assert_eq!(scale.length(3000.0), 3.0);
    assert!(!scale.is_metric_identity());
}

#[test]
fn handles_the_whole_si_prefix_table() {
    assert_eq!(prefix_factor("KILO"), Some(1e3));
    assert_eq!(prefix_factor("CENTI"), Some(1e-2));
    assert_eq!(prefix_factor("MICRO"), Some(1e-6));
    assert_eq!(prefix_factor("UNRECOGNIZED"), None);
}

/// Degrees arrive as a conversion-based unit of pi/180 radians.
#[test]
fn resolves_conversion_based_angle_units() {
    let model = model_with(
        vec![
            (1, si("LENGTHUNIT", None, "METRE")),
            (2, si("PLANEANGLEUNIT", None, "RADIAN")),
            (
                3,
                measure(
                    real("IFCPLANEANGLEMEASURE", 0.017453292519943295),
                    Value::Ref(id(2)),
                ),
            ),
            (
                4,
                conversion("IFCCONVERSIONBASEDUNIT", "PLANEANGLEUNIT", "DEGREE", 3),
            ),
        ],
        &[1, 4],
    );
    let scale = resolve(&model).expect("units");
    let radians = scale.angle(id(99), 90.0).expect("angle unit");
    assert!((radians - std::f64::consts::FRAC_PI_2).abs() < 1e-12);
}

#[test]
fn no_project_refuses() {
    assert_eq!(
        resolve(&Model::new()),
        Err(GeometryError::LengthUnitUndeclared {
            project: None,
            assignment: None
        })
    );
}

#[test]
fn project_without_units_in_context_refuses() {
    let mut model = Model::new();
    model.insert(id(1), project(None));
    assert_eq!(
        resolve(&model),
        Err(GeometryError::LengthUnitUndeclared {
            project: Some(id(1)),
            assignment: None
        })
    );
}

#[test]
fn assignment_without_length_unit_refuses() {
    let model = model_with(vec![(1, si("PLANEANGLEUNIT", None, "RADIAN"))], &[1]);
    assert_eq!(
        resolve(&model),
        Err(GeometryError::LengthUnitUndeclared {
            project: Some(id(11)),
            assignment: Some(id(10))
        })
    );
}

#[test]
fn two_projects_refuse() {
    let mut model = model_with_length_unit(None);
    model.insert(id(12), project(Some(10)));
    assert_eq!(
        resolve(&model),
        Err(GeometryError::UnitsAmbiguous {
            first: id(11),
            second: id(12),
            kind: "IFCPROJECT"
        })
    );
}

#[test]
fn two_length_units_refuse() {
    let model = model_with(
        vec![
            (1, si("LENGTHUNIT", None, "METRE")),
            (2, si("LENGTHUNIT", Some("MILLI"), "METRE")),
        ],
        &[1, 2],
    );
    assert_eq!(
        resolve(&model),
        Err(GeometryError::UnitsAmbiguous {
            first: id(1),
            second: id(2),
            kind: "LENGTHUNIT"
        })
    );
}

#[test]
fn unknown_prefix_refuses() {
    let model = model_with_length_unit(Some("BOGUS"));
    assert!(matches!(
        resolve(&model),
        Err(GeometryError::UnitUnreadable {
            unit,
            unit_type: "LENGTHUNIT",
            ..
        }) if unit == id(1)
    ));
}

#[test]
fn si_length_unit_not_named_metre_refuses() {
    let model = model_with(vec![(1, si("LENGTHUNIT", None, "GRAM"))], &[1]);
    match resolve(&model) {
        Err(GeometryError::UnitUnreadable { unit, detail, .. }) => {
            assert_eq!(unit, id(1));
            assert!(detail.contains("WR1"), "{detail}");
        }
        other => panic!("expected UnitUnreadable, got {other:?}"),
    }
}

#[test]
fn conversion_factor_without_a_number_refuses() {
    let model = model_with(
        vec![
            (
                2,
                measure(
                    Value::Typed {
                        type_name: "IFCLABEL".into(),
                        value: Box::new(Value::Text("x".into())),
                    },
                    Value::Null,
                ),
            ),
            (
                3,
                conversion("IFCCONVERSIONBASEDUNIT", "LENGTHUNIT", "INCH", 2),
            ),
        ],
        &[3],
    );
    assert!(matches!(
        resolve(&model),
        Err(GeometryError::UnitUnreadable {
            unit_type: "LENGTHUNIT",
            ..
        })
    ));
}

/// The factor is stated in its unit component: 25.4 millimetres.
#[test]
fn inch_stated_in_millimetres_reads_the_unit_component() {
    let model = model_with(
        vec![
            (1, si("LENGTHUNIT", Some("MILLI"), "METRE")),
            (
                2,
                measure(real("IFCLENGTHMEASURE", 25.4), Value::Ref(id(1))),
            ),
            (
                3,
                conversion("IFCCONVERSIONBASEDUNIT", "LENGTHUNIT", "INCH", 2),
            ),
        ],
        &[3],
    );
    let scale = resolve(&model).expect("units");
    assert!((scale.length_to_metres - 25.4 * 1e-3).abs() < 1e-15);
}

#[test]
fn nested_conversion_unit_is_unsupported() {
    let model = model_with(
        vec![
            (1, si("LENGTHUNIT", None, "METRE")),
            (
                2,
                measure(real("IFCLENGTHMEASURE", 0.0254), Value::Ref(id(1))),
            ),
            (
                3,
                conversion("IFCCONVERSIONBASEDUNIT", "LENGTHUNIT", "INCH", 2),
            ),
            (
                4,
                measure(real("IFCLENGTHMEASURE", 12.0), Value::Ref(id(3))),
            ),
            (
                5,
                conversion("IFCCONVERSIONBASEDUNIT", "LENGTHUNIT", "FOOT", 4),
            ),
        ],
        &[5],
    );
    match resolve(&model) {
        Err(error @ GeometryError::Unsupported { .. }) => {
            assert_eq!(error.entity(), Some(id(3)));
            assert!(error.is_unsupported());
        }
        other => panic!("expected Unsupported, got {other:?}"),
    }
}

#[test]
fn context_dependent_length_unit_refuses() {
    let model = model_with(
        vec![(
            1,
            Entity::new(
                "IFCCONTEXTDEPENDENTUNIT",
                vec![
                    Value::Null,
                    Value::Enum("LENGTHUNIT".into()),
                    Value::Text("PIXEL".into()),
                ],
            ),
        )],
        &[1],
    );
    assert!(matches!(
        resolve(&model),
        Err(GeometryError::UnitUnreadable {
            unit_type: "LENGTHUNIT",
            ..
        })
    ));
}

#[test]
fn offset_length_unit_refuses() {
    let model = model_with(
        vec![
            (1, si("LENGTHUNIT", None, "METRE")),
            (2, measure(real("IFCLENGTHMEASURE", 1.0), Value::Ref(id(1)))),
            (
                3,
                Entity::new(
                    "IFCCONVERSIONBASEDUNITWITHOFFSET",
                    vec![
                        Value::Null,
                        Value::Enum("LENGTHUNIT".into()),
                        Value::Text("SHIFTED".into()),
                        Value::Ref(id(2)),
                        Value::Real(32.0),
                    ],
                ),
            ),
        ],
        &[3],
    );
    assert!(matches!(
        resolve(&model),
        Err(GeometryError::UnitUnreadable {
            unit_type: "LENGTHUNIT",
            ..
        })
    ));
}

#[test]
fn dangling_unit_reference_refuses() {
    let model = model_with(vec![(1, si("LENGTHUNIT", None, "METRE"))], &[1, 77]);
    assert_eq!(
        resolve(&model),
        Err(GeometryError::MissingEntity {
            referrer: id(10),
            missing: id(77)
        })
    );
}

#[test]
fn missing_angle_unit_is_none_and_reading_an_angle_refuses() {
    let scale = resolve(&model_with_length_unit(None)).expect("units");
    assert_eq!(scale.angle_to_radians, None);
    assert_eq!(
        scale.angle(id(5), 1.0),
        Err(GeometryError::AngleUnitUndeclared { entity: id(5) })
    );
    assert_eq!(
        scale.radians_per_unit(id(6)),
        Err(GeometryError::AngleUnitUndeclared { entity: id(6) })
    );
}

#[test]
fn unreadable_angle_unit_refuses_resolve() {
    let model = model_with(
        vec![
            (1, si("LENGTHUNIT", None, "METRE")),
            (2, si("PLANEANGLEUNIT", None, "METRE")),
        ],
        &[1, 2],
    );
    assert!(matches!(
        resolve(&model),
        Err(GeometryError::UnitUnreadable {
            unit,
            unit_type: "PLANEANGLEUNIT",
            ..
        }) if unit == id(2)
    ));
}

#[test]
fn error_entity_names_the_unit() {
    let model = model_with_length_unit(Some("BOGUS"));
    let error = resolve(&model).expect_err("unknown prefix");
    assert_eq!(error.entity(), Some(id(1)));
    assert!(error.to_string().contains("#1"));
}

#[test]
fn si_constant_is_metres_and_radians() {
    assert_eq!(UnitScale::SI.length(2.0), 2.0);
    assert_eq!(UnitScale::SI.angle(id(1), 2.0), Ok(2.0));
}
