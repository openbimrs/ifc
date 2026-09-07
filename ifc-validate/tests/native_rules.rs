use ifc_model::{Entity, EntityId, Model, Value};
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

/// `NoSelfReference` on each assignment subtype: an object cannot be
/// assigned to itself. Each subtype names its relating end differently,
/// so all four are exercised.
#[test]
fn assignment_relations_reject_assigning_an_object_to_itself() {
    let schema = ifc_schema::ifc4();
    let cases = [
        (
            "IFCRELASSIGNSTOACTOR",
            "RelatingActor",
            "IfcRelAssignsToActor.NoSelfReference",
        ),
        (
            "IFCRELASSIGNSTOPROCESS",
            "RelatingProcess",
            "IfcRelAssignsToProcess.NoSelfReference",
        ),
        (
            "IFCRELASSIGNSTOPRODUCT",
            "RelatingProduct",
            "IfcRelAssignsToProduct.NoSelfReference",
        ),
        (
            "IFCRELASSIGNSTOGROUPBYFACTOR",
            "RelatingGroup",
            "IfcRelAssignsToGroupByFactor.NoSelfReference",
        ),
    ];
    for (relation, relating, rule) in cases {
        // The relating entity also appears in RelatedObjects: self-assignment.
        let mut invalid = Model::new();
        let subject = EntityId(1);
        invalid.insert(subject, entity(schema, "IFCWALL", &[]));
        invalid.push(entity(
            schema,
            relation,
            &[
                (relating, Value::Ref(subject)),
                ("RelatedObjects", Value::List(vec![Value::Ref(subject)])),
            ],
        ));
        assert!(has(&validate(&invalid, schema), rule), "{rule} must fire");

        // A distinct target is conformant.
        let mut valid = Model::new();
        let other = EntityId(2);
        valid.insert(subject, entity(schema, "IFCWALL", &[]));
        valid.insert(other, entity(schema, "IFCWALL", &[]));
        valid.push(entity(
            schema,
            relation,
            &[
                (relating, Value::Ref(subject)),
                ("RelatedObjects", Value::List(vec![Value::Ref(other)])),
            ],
        ));
        assert!(
            !has(&validate(&valid, schema), rule),
            "{rule} false positive"
        );
    }
}

/// Path-connection priorities are bounded 0..=100 inclusive, and an empty
/// list is explicitly conformant per the schema's OR clause.
#[test]
fn path_connection_priorities_are_bounded() {
    let schema = ifc_schema::ifc4();
    let rule = "IfcRelConnectsPathElements.NormalizedRelatingPriorities";

    let mut invalid = Model::new();
    invalid.push(entity(
        schema,
        "IFCRELCONNECTSPATHELEMENTS",
        &[("RelatingPriorities", Value::List(vec![Value::Integer(101)]))],
    ));
    assert!(has(&validate(&invalid, schema), rule));

    // 0 and 100 are inside the inclusive range.
    let mut edges = Model::new();
    edges.push(entity(
        schema,
        "IFCRELCONNECTSPATHELEMENTS",
        &[(
            "RelatingPriorities",
            Value::List(vec![Value::Integer(0), Value::Integer(100)]),
        )],
    ));
    assert!(!has(&validate(&edges, schema), rule), "0 and 100 are legal");

    // An empty list satisfies the rule's first disjunct.
    let mut empty = Model::new();
    empty.push(entity(
        schema,
        "IFCRELCONNECTSPATHELEMENTS",
        &[("RelatingPriorities", Value::List(Vec::new()))],
    ));
    assert!(!has(&validate(&empty, schema), rule), "empty is conformant");
}

/// `CorrectPhysOrVirt` across all three concrete boundary subtypes.
///
/// The 2nd-level form is the one real BEM exports write, and it is
/// invisible to a query for the supertype, so each is checked explicitly.
#[test]
fn space_boundary_physicality_matches_the_related_element() {
    let schema = ifc_schema::ifc4();
    let rule = "IfcRelSpaceBoundary.CorrectPhysOrVirt";
    let types = [
        "IFCRELSPACEBOUNDARY",
        "IFCRELSPACEBOUNDARY1STLEVEL",
        "IFCRELSPACEBOUNDARY2NDLEVEL",
    ];
    for boundary in types {
        // PHYSICAL against a virtual element contradicts the rule.
        let mut invalid = Model::new();
        let element = EntityId(1);
        invalid.insert(element, entity(schema, "IFCVIRTUALELEMENT", &[]));
        invalid.push(entity(
            schema,
            boundary,
            &[
                ("RelatedBuildingElement", Value::Ref(element)),
                ("PhysicalOrVirtualBoundary", Value::Enum("PHYSICAL".into())),
            ],
        ));
        assert!(
            has(&validate(&invalid, schema), rule),
            "{boundary} must fire"
        );
    }
}

/// The rule's three conformant shapes, including the one that is easy
/// to get wrong: VIRTUAL is legal against an opening, not only against an
/// IfcVirtualElement.
#[test]
fn space_boundary_physicality_accepts_the_legal_combinations() {
    let schema = ifc_schema::ifc4();
    let rule = "IfcRelSpaceBoundary.CorrectPhysOrVirt";
    let cases = [
        ("IFCWALL", "PHYSICAL"),
        ("IFCVIRTUALELEMENT", "VIRTUAL"),
        ("IFCOPENINGELEMENT", "VIRTUAL"),
        ("IFCVIRTUALELEMENT", "NOTDEFINED"),
    ];
    for (element_type, declared) in cases {
        let mut model = Model::new();
        let element = EntityId(1);
        model.insert(element, entity(schema, element_type, &[]));
        model.push(entity(
            schema,
            "IFCRELSPACEBOUNDARY2NDLEVEL",
            &[
                ("RelatedBuildingElement", Value::Ref(element)),
                ("PhysicalOrVirtualBoundary", Value::Enum(declared.into())),
            ],
        ));
        assert!(
            !has(&validate(&model, schema), rule),
            "{declared} against {element_type} is conformant"
        );
    }
}
