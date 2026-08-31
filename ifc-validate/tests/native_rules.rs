use ifc_model::{Entity, Model, Value};
use ifc_schema::Schema;
use ifc_validate::{validate, Report};

fn entity(schema: &Schema, type_name: &str, values: &[(&str, Value)]) -> Entity {
    let names = schema.attribute_names(type_name);
    let mut attributes = vec![Value::Null; names.len()];
    for (name, value) in values {
        let index = names
            .iter()
            .position(|candidate| candidate.eq_ignore_ascii_case(name))
            .unwrap_or_else(|| panic!("{type_name} has no {name} in {}", schema.name()));
        attributes[index] = value.clone();
    }
    Entity::new(type_name, attributes)
}

fn has(report: &Report, rule: &str) -> bool {
    report.findings().iter().any(|finding| finding.rule == rule)
}

#[test]
fn external_references_require_identity_in_every_bundled_schema() {
    for schema in [
        ifc_schema::ifc2x3(),
        ifc_schema::ifc4(),
        ifc_schema::ifc4x3(),
    ] {
        let mut invalid = Model::new();
        invalid.push(entity(schema, "IFCCLASSIFICATIONREFERENCE", &[]));
        assert!(has(&validate(&invalid, schema), "IfcExternalReference.WR1"));

        let mut valid = Model::new();
        valid.push(entity(
            schema,
            "IFCCLASSIFICATIONREFERENCE",
            &[("Location", Value::Text("urn:classification".into()))],
        ));
        assert!(!has(&validate(&valid, schema), "IfcExternalReference.WR1"));
    }
}

#[test]
fn sequence_endpoints_must_differ_in_every_bundled_schema() {
    for schema in [
        ifc_schema::ifc2x3(),
        ifc_schema::ifc4(),
        ifc_schema::ifc4x3(),
    ] {
        let rule = if schema.version() == Some(ifc_schema::SchemaVersion::Ifc2x3) {
            "IfcRelSequence.WR1"
        } else {
            "IfcRelSequence.AvoidInconsistentSequence"
        };
        let mut invalid = Model::new();
        let first = invalid.push(entity(schema, "IFCTASK", &[]));
        invalid.push(entity(
            schema,
            "IFCRELSEQUENCE",
            &[
                ("RelatingProcess", Value::Ref(first)),
                ("RelatedProcess", Value::Ref(first)),
            ],
        ));
        assert!(has(&validate(&invalid, schema), rule));

        let mut valid = Model::new();
        let first = valid.push(entity(schema, "IFCTASK", &[]));
        let second = valid.push(entity(schema, "IFCTASK", &[]));
        valid.push(entity(
            schema,
            "IFCRELSEQUENCE",
            &[
                ("RelatingProcess", Value::Ref(first)),
                ("RelatedProcess", Value::Ref(second)),
            ],
        ));
        assert!(!has(&validate(&valid, schema), rule));
    }
}

#[test]
fn decomposition_self_references_are_rejected_only_where_declared() {
    for schema in [ifc_schema::ifc4(), ifc_schema::ifc4x3()] {
        for (relation, rule) in [
            ("IFCRELAGGREGATES", "IfcRelAggregates.NoSelfReference"),
            ("IFCRELNESTS", "IfcRelNests.NoSelfReference"),
        ] {
            let mut model = Model::new();
            let object = model.push(entity(schema, "IFCPROJECT", &[]));
            model.push(entity(
                schema,
                relation,
                &[
                    ("RelatingObject", Value::Ref(object)),
                    ("RelatedObjects", Value::List(vec![Value::Ref(object)])),
                ],
            ));
            assert!(has(&validate(&model, schema), rule));

            let mut model = Model::new();
            let parent = model.push(entity(schema, "IFCPROJECT", &[]));
            let child = model.push(entity(schema, "IFCPROJECT", &[]));
            model.push(entity(
                schema,
                relation,
                &[
                    ("RelatingObject", Value::Ref(parent)),
                    ("RelatedObjects", Value::List(vec![Value::Ref(child)])),
                ],
            ));
            assert!(!has(&validate(&model, schema), rule));
        }
    }

    let schema = ifc_schema::ifc2x3();
    let mut model = Model::new();
    let object = model.push(entity(schema, "IFCPROJECT", &[]));
    model.push(entity(
        schema,
        "IFCRELAGGREGATES",
        &[
            ("RelatingObject", Value::Ref(object)),
            ("RelatedObjects", Value::List(vec![Value::Ref(object)])),
        ],
    ));
    assert!(!has(
        &validate(&model, schema),
        "IfcRelAggregates.NoSelfReference"
    ));
}

#[test]
fn material_layer_priority_is_bounded_in_ifc4_and_ifc4x3() {
    for schema in [ifc_schema::ifc4(), ifc_schema::ifc4x3()] {
        for priority in [-1, 101] {
            let mut model = Model::new();
            model.push(entity(
                schema,
                "IFCMATERIALLAYER",
                &[
                    ("LayerThickness", Value::Real(1.0)),
                    ("Priority", Value::Integer(priority)),
                ],
            ));
            assert!(has(
                &validate(&model, schema),
                "IfcMaterialLayer.NormalizedPriority"
            ));
        }
        for priority in [0, 100] {
            let mut model = Model::new();
            model.push(entity(
                schema,
                "IFCMATERIALLAYER",
                &[
                    ("LayerThickness", Value::Real(1.0)),
                    ("Priority", Value::Integer(priority)),
                ],
            ));
            assert!(!has(
                &validate(&model, schema),
                "IfcMaterialLayer.NormalizedPriority"
            ));
        }
    }
}
