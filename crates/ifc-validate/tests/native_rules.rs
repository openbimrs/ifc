//! Native rules.

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
            "IfcRelAssignsToGroup.NoSelfReference",
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

/// `NoSelfReference` is declared on `IfcRelAssignsToGroup`, so it binds the
/// plain relation and its `ByFactor` subtype alike, under the declaring
/// entity's id, in both releases that declare it.
#[test]
fn group_assignment_self_reference_is_checked_on_the_declaring_entity_and_its_subtype() {
    let rule = "IfcRelAssignsToGroup.NoSelfReference";
    for schema in [ifc_schema::ifc4(), ifc_schema::ifc4x3()] {
        for relation in ["IFCRELASSIGNSTOGROUP", "IFCRELASSIGNSTOGROUPBYFACTOR"] {
            let mut model = Model::new();
            let group = model.push(entity(schema, "IFCGROUP", &[]));
            model.push(entity(
                schema,
                relation,
                &[
                    ("RelatingGroup", Value::Ref(group)),
                    ("RelatedObjects", Value::List(vec![Value::Ref(group)])),
                ],
            ));
            let report = validate(&model, schema);
            assert!(has(&report, rule), "{relation} in {}", schema.name());
            assert!(
                !has(&report, "IfcRelAssignsToGroupByFactor.NoSelfReference"),
                "the subtype declares no rule of its own"
            );
        }
    }
}

/// `IfcMaterialLayerWithOffsets` inherits `NormalizedPriority`.
#[test]
fn material_layer_with_offsets_priority_is_bounded() {
    for schema in [ifc_schema::ifc4(), ifc_schema::ifc4x3()] {
        for (priority, fires) in [(101, true), (-1, true), (50, false)] {
            let mut model = Model::new();
            model.push(entity(
                schema,
                "IFCMATERIALLAYERWITHOFFSETS",
                &[
                    ("LayerThickness", Value::Real(1.0)),
                    ("Priority", Value::Integer(priority)),
                ],
            ));
            assert_eq!(
                has(
                    &validate(&model, schema),
                    "IfcMaterialLayer.NormalizedPriority"
                ),
                fires,
                "priority {priority} in {}",
                schema.name()
            );
        }
    }
}

/// IFC2X3 declares neither rule, so a model that breaks both is silent
/// under IFC2X3 tables.
#[test]
fn ifc4_only_rules_do_not_run_under_ifc2x3() {
    let schema = ifc_schema::ifc2x3();
    let mut model = Model::new();
    let wall_type = model.push(entity(schema, "IFCWALLTYPE", &[]));
    model.push(entity(
        schema,
        "IFCRELDEFINESBYPROPERTIES",
        &[("RelatedObjects", Value::List(vec![Value::Ref(wall_type)]))],
    ));
    let group = model.push(entity(schema, "IFCGROUP", &[]));
    model.push(entity(
        schema,
        "IFCRELASSIGNSTOGROUP",
        &[
            ("RelatingGroup", Value::Ref(group)),
            ("RelatedObjects", Value::List(vec![Value::Ref(group)])),
        ],
    ));
    let report = validate(&model, schema);
    for rule in [
        "IfcRelDefinesByProperties.NoRelatedTypeObject",
        "IfcRelAssignsToGroup.NoSelfReference",
    ] {
        assert!(!has(&report, rule), "{rule} is not declared by IFC2X3");
    }
}

/// Every name shared by two property sets of one object is its own
/// finding, and a definition the file lacks is an evaluation error rather
/// than a pass (#215).
#[test]
fn unique_property_set_names_reports_each_shared_name() {
    let schema = ifc_schema::ifc4();
    let mut model = Model::new();
    model.insert(
        EntityId(1),
        entity(
            schema,
            "IFCWALL",
            &[("GlobalId", Value::Text("0hMOPMBpTAoOL$IPqA1$xY".into()))],
        ),
    );
    for (id, name) in [(10, "A"), (11, "A"), (12, "B"), (13, "B"), (14, "C")] {
        model.insert(
            EntityId(id),
            entity(
                schema,
                "IFCPROPERTYSET",
                &[("Name", Value::Text(name.into()))],
            ),
        );
    }
    let mut definitions: Vec<Value> = (10..=14).map(|id| Value::Ref(EntityId(id))).collect();
    definitions.push(Value::Ref(EntityId(99)));
    for definition in definitions {
        model.push(entity(
            schema,
            "IFCRELDEFINESBYPROPERTIES",
            &[
                ("RelatedObjects", Value::List(vec![Value::Ref(EntityId(1))])),
                ("RelatingPropertyDefinition", definition),
            ],
        ));
    }
    let report = validate(&model, schema);
    let findings: Vec<_> = report
        .findings()
        .iter()
        .filter(|finding| finding.rule == "IfcObject.UniquePropertySetNames")
        .collect();
    let errors = findings
        .iter()
        .filter(|finding| finding.severity == ifc_validate::Severity::Error)
        .count();
    let undecided = findings
        .iter()
        .filter(|finding| finding.severity == ifc_validate::Severity::EvaluationError)
        .count();
    assert_eq!((errors, undecided), (2, 1), "{findings:?}");
}
