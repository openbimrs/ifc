use std::sync::Arc;

use ifc_model::{Codec, Diagnostic, Entity, EntityId, Model, Value};
use ifc_properties::{
    exact_property, ExactLogical, ExactPropertyError, ExactResolution, ExactSource, ExactValue,
};
use ifc_step::StepCodec;

fn wall_attributes(extra: usize) -> Vec<Value> {
    vec![Value::Null; ifc_schema::ifc4().attributes("IFCWALL").len() + extra]
}

fn assert_malformed_slots(result: Result<ExactResolution, ExactPropertyError>, expected: u64) {
    assert!(
        matches!(result, Err(ExactPropertyError::MalformedEntitySlots { entity, .. }) if entity == EntityId(expected))
    );
}

fn model() -> Model {
    let mut model = Model::new();
    model.header_mut().schema.push("IFC4".into());
    model.insert(EntityId(1), Entity::new("IFCWALL", wall_attributes(0)));
    model
}
fn pset(model: &mut Model, id: u64, name: &str, properties: Vec<u64>) {
    model.insert(
        EntityId(id),
        Entity::new(
            "IFCPROPERTYSET",
            vec![
                Value::Null,
                Value::Null,
                Value::Text(name.into()),
                Value::Null,
                Value::List(
                    properties
                        .into_iter()
                        .map(|id| Value::Ref(EntityId(id)))
                        .collect(),
                ),
            ],
        ),
    );
}

fn typed_value(type_name: &str, value: Value) -> Value {
    Value::Typed {
        type_name: Arc::from(type_name),
        value: Box::new(value),
    }
}

fn property(m: &mut Model, id: u64, name: &str, value: Value) {
    let value = match value {
        Value::Text(v) => typed_value("IFCTEXT", Value::Text(v)),
        Value::Integer(v) => typed_value("IFCINTEGER", Value::Integer(v)),
        Value::Real(v) => typed_value("IFCREAL", Value::Real(v)),
        Value::Bool(v) => typed_value("IFCBOOLEAN", Value::Bool(v)),
        other => other,
    };
    m.insert(
        EntityId(id),
        Entity::new(
            "IFCPROPERTYSINGLEVALUE",
            vec![Value::Text(name.into()), Value::Null, value, Value::Null],
        ),
    );
}
fn occurrence(model: &mut Model, id: u64, set: u64) {
    model.insert(
        EntityId(id),
        Entity::new(
            "IFCRELDEFINESBYPROPERTIES",
            vec![
                Value::Null,
                Value::Null,
                Value::Null,
                Value::Null,
                Value::List(vec![Value::Ref(EntityId(1))]),
                Value::Ref(EntityId(set)),
            ],
        ),
    );
}
fn type_attributes(has_property_sets: Value) -> Vec<Value> {
    let mut attributes = vec![Value::Null; ifc_schema::ifc4().attributes("IFCWALLTYPE").len()];
    attributes[5] = has_property_sets;
    attributes
}

fn typed(model: &mut Model, type_id: u64, set: u64) {
    let attributes = type_attributes(Value::List(vec![Value::Ref(EntityId(set))]));
    model.insert(EntityId(type_id), Entity::new("IFCWALLTYPE", attributes));
    model.insert(
        EntityId(type_id + 1),
        Entity::new(
            "IFCRELDEFINESBYTYPE",
            vec![
                Value::Null,
                Value::Null,
                Value::Null,
                Value::Null,
                Value::List(vec![Value::Ref(EntityId(1))]),
                Value::Ref(EntityId(type_id)),
            ],
        ),
    );
}

#[test]
fn occurrence_overrides_inherited_and_reports_ids() {
    let mut m = model();
    property(&mut m, 10, "FireRating", Value::Text("type".into()));
    pset(&mut m, 11, "Pset_WallCommon", vec![10]);
    typed(&mut m, 12, 11);
    property(&mut m, 20, "FireRating", Value::Text("occurrence".into()));
    pset(&mut m, 21, "Pset_WallCommon", vec![20]);
    occurrence(&mut m, 22, 21);
    assert_eq!(
        exact_property(&m, EntityId(1), Some("Pset_WallCommon"), "FireRating").unwrap(),
        ExactResolution::Present(ifc_properties::ExactProperty {
            source: ExactSource::Occurrence,
            property_set: Arc::from("Pset_WallCommon"),
            set_id: EntityId(21),
            property_id: EntityId(20),
            value_type: Some(Arc::from("IFCTEXT")),
            unit_id: None,
            value: ExactValue::Text(Arc::from("occurrence")),
        })
    );
}

#[test]
fn occurrence_set_without_property_preserves_inherited_property() {
    let mut m = model();
    property(&mut m, 10, "FireRating", Value::Text("type".into()));
    pset(&mut m, 11, "Pset_WallCommon", vec![10]);
    typed(&mut m, 12, 11);
    property(&mut m, 20, "Reference", Value::Text("occurrence".into()));
    pset(&mut m, 21, "Pset_WallCommon", vec![20]);
    occurrence(&mut m, 22, 21);

    assert!(matches!(
        exact_property(&m, EntityId(1), Some("Pset_WallCommon"), "FireRating"),
        Ok(ExactResolution::Present(p))
            if p.source == ExactSource::Type(EntityId(12))
                && p.value == ExactValue::Text(Arc::from("type"))
    ));
}

#[test]
fn inherited_absence_and_null_are_distinct() {
    let mut m = model();
    property(&mut m, 10, "Empty", Value::Null);
    pset(&mut m, 11, "Pset_Test", vec![10]);
    typed(&mut m, 12, 11);
    assert!(matches!(
        exact_property(&m, EntityId(1), Some("Pset_Test"), "Missing"),
        Ok(ExactResolution::Absent)
    ));
    assert!(
        matches!(exact_property(&m, EntityId(1), Some("Pset_Test"), "Empty"), Ok(ExactResolution::Present(p)) if p.source == ExactSource::Type(EntityId(12)) && p.value == ExactValue::Null)
    );
}

#[test]
fn malformed_and_duplicate_assignments_fail_closed() {
    let mut m = model();
    m.insert(
        EntityId(2),
        Entity::new("IFCRELDEFINESBYPROPERTIES", vec![]),
    );
    assert_malformed_slots(exact_property(&m, EntityId(1), None, "x"), 2);
    let mut m = model();
    property(&mut m, 10, "x", Value::Integer(1));
    pset(&mut m, 11, "s", vec![10]);
    typed(&mut m, 12, 11);
    typed(&mut m, 20, 11);
    assert!(matches!(
        exact_property(&m, EntityId(1), None, "x"),
        Err(ExactPropertyError::MultipleTypeAssignments { .. })
    ));
    let mut m = model();
    property(&mut m, 10, "x", Value::Integer(1));
    property(&mut m, 11, "x", Value::Integer(2));
    pset(&mut m, 12, "s", vec![10, 11]);
    occurrence(&mut m, 13, 12);
    assert!(matches!(
        exact_property(&m, EntityId(1), None, "x"),
        Err(ExactPropertyError::DuplicateMatchingProperties { .. })
    ));
    let mut m = model();
    property(&mut m, 10, "x", Value::Integer(1));
    pset(&mut m, 11, "s", vec![10]);
    occurrence(&mut m, 12, 11);
    property(&mut m, 20, "other", Value::Integer(2));
    pset(&mut m, 21, "s", vec![20]);
    occurrence(&mut m, 22, 21);
    assert!(matches!(
        exact_property(&m, EntityId(1), Some("s"), "x"),
        Err(ExactPropertyError::DuplicateMatchingSets { .. })
    ));
}

#[test]
fn empty_related_objects_is_not_complete_evidence() {
    let mut m = model();
    pset(&mut m, 10, "Pset_Test", vec![]);
    m.insert(
        EntityId(11),
        Entity::new(
            "IFCRELDEFINESBYPROPERTIES",
            vec![
                Value::Null,
                Value::Null,
                Value::Null,
                Value::Null,
                Value::List(vec![]),
                Value::Ref(EntityId(10)),
            ],
        ),
    );
    assert!(matches!(
        exact_property(&m, EntityId(1), None, "Missing"),
        Err(ExactPropertyError::MalformedAggregate {
            attribute: "RelatedObjects",
            ..
        })
    ));
}

#[test]
fn empty_has_properties_is_not_exact_absence() {
    let mut m = model();
    pset(&mut m, 10, "Pset_Test", vec![]);
    occurrence(&mut m, 11, 10);
    assert!(matches!(
        exact_property(&m, EntityId(1), Some("Pset_Test"), "Missing"),
        Err(ExactPropertyError::MalformedAggregate {
            attribute: "HasProperties",
            ..
        })
    ));
}

#[test]
fn type_object_cannot_receive_occurrence_property_assignment() {
    let mut m = model();
    m.insert(EntityId(2), Entity::new("IFCWALLTYPE", vec![]));
    property(&mut m, 10, "x", Value::Integer(1));
    pset(&mut m, 11, "Pset_Test", vec![10]);
    m.insert(
        EntityId(12),
        Entity::new(
            "IFCRELDEFINESBYPROPERTIES",
            vec![
                Value::Null,
                Value::Null,
                Value::Null,
                Value::Null,
                Value::List(vec![Value::Ref(EntityId(2))]),
                Value::Ref(EntityId(11)),
            ],
        ),
    );
    assert!(matches!(
        exact_property(&m, EntityId(1), Some("Pset_Test"), "x"),
        Err(ExactPropertyError::InvalidOccurrenceTarget {
            object: EntityId(2),
            ..
        })
    ));
    assert!(matches!(
        exact_property(&m, EntityId(2), None, "x"),
        Err(ExactPropertyError::InvalidQueryObject {
            object: EntityId(2),
            ..
        })
    ));
}

#[test]
fn property_definition_set_traverses_every_nonempty_member() {
    let mut m = model();
    property(&mut m, 10, "First", Value::Integer(1));
    property(&mut m, 11, "Second", Value::Integer(2));
    pset(&mut m, 12, "Pset_First", vec![10]);
    pset(&mut m, 13, "Pset_Second", vec![11]);
    m.insert(
        EntityId(14),
        Entity::new(
            "IFCRELDEFINESBYPROPERTIES",
            vec![
                Value::Null,
                Value::Null,
                Value::Null,
                Value::Null,
                Value::List(vec![Value::Ref(EntityId(1))]),
                Value::Typed {
                    type_name: "IFCPROPERTYSETDEFINITIONSET".into(),
                    value: Box::new(Value::List(vec![
                        Value::Ref(EntityId(12)),
                        Value::Ref(EntityId(13)),
                    ])),
                },
            ],
        ),
    );
    assert!(matches!(
        exact_property(&m, EntityId(1), Some("Pset_Second"), "Second"),
        Ok(ExactResolution::Present(p)) if p.set_id == EntityId(13) && p.value == ExactValue::Integer(2)
    ));
}

#[test]
fn empty_property_definition_set_is_incomplete() {
    let mut m = model();
    m.insert(
        EntityId(10),
        Entity::new(
            "IFCRELDEFINESBYPROPERTIES",
            vec![
                Value::Null,
                Value::Null,
                Value::Null,
                Value::Null,
                Value::List(vec![Value::Ref(EntityId(1))]),
                Value::List(vec![]),
            ],
        ),
    );
    assert!(matches!(
        exact_property(&m, EntityId(1), None, "Missing"),
        Err(ExactPropertyError::MalformedAggregate {
            attribute: "RelatingPropertyDefinition",
            ..
        })
    ));
}

#[test]
fn present_empty_type_property_sets_are_incomplete() {
    let mut m = model();
    m.insert(
        EntityId(10),
        Entity::new("IFCWALLTYPE", type_attributes(Value::List(vec![]))),
    );
    m.insert(
        EntityId(11),
        Entity::new(
            "IFCRELDEFINESBYTYPE",
            vec![
                Value::Null,
                Value::Null,
                Value::Null,
                Value::Null,
                Value::List(vec![Value::Ref(EntityId(1))]),
                Value::Ref(EntityId(10)),
            ],
        ),
    );
    assert!(matches!(
        exact_property(&m, EntityId(1), None, "Missing"),
        Err(ExactPropertyError::MalformedAggregate {
            entity: EntityId(10),
            attribute: "HasPropertySets"
        })
    ));
}

#[test]
fn absent_type_sets_and_quantities_preserve_exact_absence() {
    let mut m = model();
    m.insert(
        EntityId(10),
        Entity::new("IFCWALLTYPE", type_attributes(Value::Null)),
    );
    m.insert(
        EntityId(11),
        Entity::new(
            "IFCRELDEFINESBYTYPE",
            vec![
                Value::Null,
                Value::Null,
                Value::Null,
                Value::Null,
                Value::List(vec![Value::Ref(EntityId(1))]),
                Value::Ref(EntityId(10)),
            ],
        ),
    );
    m.insert(
        EntityId(20),
        Entity::new(
            "IFCELEMENTQUANTITY",
            vec![Value::Null; ifc_schema::ifc4().attributes("IFCELEMENTQUANTITY").len()],
        ),
    );
    occurrence(&mut m, 21, 20);

    assert_eq!(
        exact_property(&m, EntityId(1), Some("Pset_Test"), "Missing").unwrap(),
        ExactResolution::Absent
    );
}

#[test]
fn set_uniqueness_and_property_member_domains_fail_closed() {
    let mut m = model();
    property(&mut m, 10, "x", Value::Integer(1));
    pset(&mut m, 11, "s", vec![10, 10]);
    occurrence(&mut m, 12, 11);
    assert!(matches!(
        exact_property(&m, EntityId(1), Some("s"), "x"),
        Err(ExactPropertyError::DuplicateAggregateMember {
            entity: EntityId(11),
            attribute: "HasProperties",
            member: EntityId(10)
        })
    ));

    let mut m = model();
    pset(&mut m, 11, "s", vec![1]);
    occurrence(&mut m, 12, 11);
    assert!(matches!(
        exact_property(&m, EntityId(1), Some("s"), "missing"),
        Err(ExactPropertyError::UnsupportedProperty {
            entity: EntityId(1),
            ..
        })
    ));

    let mut m = model();
    m.insert(EntityId(2), Entity::new("IFCWALL", wall_attributes(0)));
    m.insert(
        EntityId(12),
        Entity::new(
            "IFCRELDEFINESBYPROPERTIES",
            vec![
                Value::Null,
                Value::Null,
                Value::Null,
                Value::Null,
                Value::List(vec![Value::Ref(EntityId(2))]),
                Value::Ref(EntityId(1)),
            ],
        ),
    );
    assert!(matches!(
        exact_property(&m, EntityId(1), None, "missing"),
        Err(ExactPropertyError::UnsupportedDefinition {
            entity: EntityId(1),
            ..
        })
    ));
}

#[test]
fn property_slots_value_select_and_unit_select_fail_closed() {
    let mut m = model();
    m.insert(
        EntityId(10),
        Entity::new(
            "IFCPROPERTYSINGLEVALUE",
            vec![
                Value::Text("x".into()),
                Value::Null,
                typed_value("IFCINTEGER", Value::Integer(1)),
            ],
        ),
    );
    pset(&mut m, 11, "s", vec![10]);
    occurrence(&mut m, 12, 11);
    assert_malformed_slots(exact_property(&m, EntityId(1), Some("s"), "x"), 10);

    for (value, unit, expected_unit_error) in [
        (
            typed_value("IFCOWNERHISTORY", Value::Integer(1)),
            Value::Null,
            false,
        ),
        (
            typed_value("IFCTEXT", Value::Integer(7)),
            Value::Null,
            false,
        ),
        (
            typed_value("IFCINTEGER", Value::Integer(1)),
            Value::Ref(EntityId(1)),
            true,
        ),
    ] {
        let mut m = model();
        m.insert(
            EntityId(10),
            Entity::new(
                "IFCPROPERTYSINGLEVALUE",
                vec![Value::Text("x".into()), Value::Null, value, unit],
            ),
        );
        pset(&mut m, 11, "s", vec![10]);
        occurrence(&mut m, 12, 11);
        let error = exact_property(&m, EntityId(1), Some("s"), "x").unwrap_err();
        assert_eq!(
            matches!(&error, ExactPropertyError::UnsupportedUnit { .. }),
            expected_unit_error
        );
        assert_eq!(
            matches!(&error, ExactPropertyError::UnsupportedValue { .. }),
            !expected_unit_error
        );
    }
}

#[test]
fn type_relationship_domains_and_optional_slot_presence_fail_closed() {
    let type_attrs = || vec![Value::Null; 6];
    let mut m = model();
    m.insert(EntityId(10), Entity::new("IFCWALLTYPE", type_attrs()));
    m.insert(EntityId(11), Entity::new("IFCWALLTYPE", type_attrs()));
    m.insert(
        EntityId(12),
        Entity::new(
            "IFCRELDEFINESBYTYPE",
            vec![
                Value::Null,
                Value::Null,
                Value::Null,
                Value::Null,
                Value::List(vec![Value::Ref(EntityId(10))]),
                Value::Ref(EntityId(11)),
            ],
        ),
    );
    assert!(matches!(
        exact_property(&m, EntityId(1), None, "x"),
        Err(ExactPropertyError::InvalidTypeTarget {
            relationship: EntityId(12),
            object: EntityId(10)
        })
    ));

    let mut m = model();
    m.insert(
        EntityId(10),
        Entity::new("IFCWALLTYPE", vec![Value::Null; 5]),
    );
    m.insert(
        EntityId(12),
        Entity::new(
            "IFCRELDEFINESBYTYPE",
            vec![
                Value::Null,
                Value::Null,
                Value::Null,
                Value::Null,
                Value::List(vec![Value::Ref(EntityId(1))]),
                Value::Ref(EntityId(10)),
            ],
        ),
    );
    assert_malformed_slots(exact_property(&m, EntityId(1), None, "x"), 10);
}

#[test]
fn declared_value_type_and_unit_identity_are_preserved() {
    let mut m = model();
    m.insert(
        EntityId(9),
        Entity::new(
            "IFCSIUNIT",
            vec![Value::Null; ifc_schema::ifc4().attributes("IFCSIUNIT").len()],
        ),
    );
    m.insert(
        EntityId(10),
        Entity::new(
            "IFCPROPERTYSINGLEVALUE",
            vec![
                Value::Text("Count".into()),
                Value::Null,
                typed_value("IFCINTEGER", Value::Integer(7)),
                Value::Ref(EntityId(9)),
            ],
        ),
    );
    pset(&mut m, 11, "Pset_Test", vec![10]);
    occurrence(&mut m, 12, 11);

    assert!(matches!(
        exact_property(&m, EntityId(1), Some("Pset_Test"), "Count"),
        Ok(ExactResolution::Present(property))
            if property.value_type.as_deref() == Some("IFCINTEGER")
                && property.unit_id == Some(EntityId(9))
                && property.value == ExactValue::Integer(7)
    ));
}

#[test]
fn surplus_slots_on_traversed_entities_fail_closed() {
    let mut m = model();
    m.insert(
        EntityId(11),
        Entity::new("IFCPROPERTYSET", vec![Value::Null; 6]),
    );
    occurrence(&mut m, 12, 11);
    assert_malformed_slots(exact_property(&m, EntityId(1), None, "x"), 11);

    let mut m = model();
    let mut relation = vec![Value::Null; 7];
    relation[4] = Value::List(vec![Value::Ref(EntityId(1))]);
    relation[5] = Value::Ref(EntityId(11));
    m.insert(
        EntityId(12),
        Entity::new("IFCRELDEFINESBYPROPERTIES", relation),
    );
    assert_malformed_slots(exact_property(&m, EntityId(1), None, "x"), 12);

    let mut m = model();
    let mut attributes = vec![Value::Null; ifc_schema::ifc4().attributes("IFCWALLTYPE").len() + 1];
    attributes[5] = Value::Null;
    m.insert(EntityId(10), Entity::new("IFCWALLTYPE", attributes));
    m.insert(
        EntityId(12),
        Entity::new(
            "IFCRELDEFINESBYTYPE",
            vec![
                Value::Null,
                Value::Null,
                Value::Null,
                Value::Null,
                Value::List(vec![Value::Ref(EntityId(1))]),
                Value::Ref(EntityId(10)),
            ],
        ),
    );
    assert_malformed_slots(exact_property(&m, EntityId(1), None, "x"), 10);
}

#[test]
fn malformed_query_object_slots_fail_closed() {
    let mut m = model();
    m.insert(EntityId(1), Entity::new("IFCWALL", wall_attributes(1)));

    assert_malformed_slots(exact_property(&m, EntityId(1), None, "x"), 1);
}

#[test]
fn malformed_occurrence_related_object_slots_fail_closed() {
    let mut m = model();
    m.insert(EntityId(6), Entity::new("IFCWALL", wall_attributes(1)));
    property(&mut m, 10, "x", Value::Integer(1));
    pset(&mut m, 11, "s", vec![10]);
    m.insert(
        EntityId(12),
        Entity::new(
            "IFCRELDEFINESBYPROPERTIES",
            vec![
                Value::Null,
                Value::Null,
                Value::Null,
                Value::Null,
                Value::List(vec![Value::Ref(EntityId(1)), Value::Ref(EntityId(6))]),
                Value::Ref(EntityId(11)),
            ],
        ),
    );

    assert_malformed_slots(exact_property(&m, EntityId(1), Some("s"), "x"), 6);
}

#[test]
fn malformed_type_related_object_slots_fail_closed() {
    let mut m = model();
    m.insert(EntityId(6), Entity::new("IFCWALL", wall_attributes(1)));
    m.insert(
        EntityId(10),
        Entity::new("IFCWALLTYPE", type_attributes(Value::Null)),
    );
    m.insert(
        EntityId(11),
        Entity::new(
            "IFCRELDEFINESBYTYPE",
            vec![
                Value::Null,
                Value::Null,
                Value::Null,
                Value::Null,
                Value::List(vec![Value::Ref(EntityId(1)), Value::Ref(EntityId(6))]),
                Value::Ref(EntityId(10)),
            ],
        ),
    );

    assert_malformed_slots(exact_property(&m, EntityId(1), None, "x"), 6);
}

#[test]
fn malformed_nonmatching_property_slots_fail_closed() {
    let mut m = model();
    property(&mut m, 10, "x", Value::Integer(1));
    m.insert(
        EntityId(6),
        Entity::new(
            "IFCPROPERTYSINGLEVALUE",
            vec![
                Value::Text("other".into()),
                Value::Null,
                typed_value("IFCINTEGER", Value::Integer(2)),
                Value::Null,
                Value::Null,
            ],
        ),
    );
    pset(&mut m, 11, "s", vec![10, 6]);
    occurrence(&mut m, 12, 11);

    assert_malformed_slots(exact_property(&m, EntityId(1), Some("s"), "x"), 6);
    assert_malformed_slots(exact_property(&m, EntityId(1), Some("s"), "missing"), 6);
}

#[test]
fn strict_step_typed_values_resolve_without_precision_loss() {
    let bytes = b"ISO-10303-21;\nHEADER;\nFILE_DESCRIPTION((''),'2;1');\nFILE_NAME('n','t',(''),(''),'p','o','a');\nFILE_SCHEMA(('IFC4'));\nENDSEC;\nDATA;\n#1=IFCWALL('g',$,$,$,$,$,$,$,$);\n#2=IFCPROPERTYSINGLEVALUE('Flag',$,IFCBOOLEAN(.T.),$);\n#3=IFCPROPERTYSINGLEVALUE('Big',$,IFCINTEGER(9007199254740993),$);\n#4=IFCPROPERTYSET('p',$,'Pset_Test',$,(#2,#3));\n#6=IFCPROPERTYSET('q',$,'Pset_Other',$,(#7,#8,#9,#10,#11));\n#7=IFCPROPERTYSINGLEVALUE('Other',$,IFCINTEGER(7),$);\n#8=IFCPROPERTYSINGLEVALUE('Logical',$,IFCLOGICAL(.U.),$);\n#9=IFCPROPERTYSINGLEVALUE('Bits',$,IFCBINARY(\"0101\"),$);\n#10=IFCPROPERTYSINGLEVALUE('LogicalTrue',$,IFCLOGICAL(.T.),$);\n#11=IFCPROPERTYSINGLEVALUE('LogicalFalse',$,IFCLOGICAL(.F.),$);\n#5=IFCRELDEFINESBYPROPERTIES('r',$,$,$,(#1),(#4,#6));\nENDSEC;\nEND-ISO-10303-21;\n";
    let model = StepCodec.read_bytes(bytes).unwrap();
    let result = exact_property(&model, EntityId(1), Some("Pset_Test"), "Flag");
    assert!(
        matches!(
            &result,
            Ok(ExactResolution::Present(p)) if p.value == ExactValue::Bool(true)
        ),
        "{result:?}"
    );
    assert!(matches!(
        exact_property(&model, EntityId(1), Some("Pset_Test"), "Big"),
        Ok(ExactResolution::Present(p)) if p.value == ExactValue::Integer(9_007_199_254_740_993)
    ));
    assert!(matches!(
        exact_property(&model, EntityId(1), Some("Pset_Other"), "Other"),
        Ok(ExactResolution::Present(p)) if p.set_id == EntityId(6) && p.value == ExactValue::Integer(7)
    ));
    assert!(matches!(
        exact_property(&model, EntityId(1), Some("Pset_Other"), "Logical"),
        Ok(ExactResolution::Present(p)) if p.value == ExactValue::Logical(ExactLogical::Unknown)
    ));
    assert!(matches!(
        exact_property(&model, EntityId(1), Some("Pset_Other"), "Bits"),
        Ok(ExactResolution::Present(p)) if p.value == ExactValue::Binary(Arc::from("0101"))
    ));
    assert!(matches!(
        exact_property(&model, EntityId(1), Some("Pset_Other"), "LogicalTrue"),
        Ok(ExactResolution::Present(p)) if p.value == ExactValue::Logical(ExactLogical::True)
    ));
    assert!(matches!(
        exact_property(&model, EntityId(1), Some("Pset_Other"), "LogicalFalse"),
        Ok(ExactResolution::Present(p)) if p.value == ExactValue::Logical(ExactLogical::False)
    ));
}

#[test]
fn unsupported_nonfinite_schema_and_diagnostics_fail_closed() {
    let mut m = model();
    property(&mut m, 10, "x", Value::List(vec![]));
    pset(&mut m, 11, "s", vec![10]);
    occurrence(&mut m, 12, 11);
    assert!(matches!(
        exact_property(&m, EntityId(1), None, "x"),
        Err(ExactPropertyError::UnsupportedValue { .. })
    ));
    let mut m = model();
    property(&mut m, 10, "x", Value::Real(f64::NAN));
    pset(&mut m, 11, "s", vec![10]);
    occurrence(&mut m, 12, 11);
    assert!(matches!(
        exact_property(&m, EntityId(1), None, "x"),
        Err(ExactPropertyError::NonFiniteReal { .. })
    ));
    let mut m = model();
    m.header_mut().schema.clear();
    assert!(matches!(
        exact_property(&m, EntityId(1), None, "x"),
        Err(ExactPropertyError::MissingSchema)
    ));
    let mut m = model();
    m.push_diagnostic(Diagnostic::unlocated("recovered"));
    assert!(matches!(
        exact_property(&m, EntityId(1), None, "x"),
        Err(ExactPropertyError::IncompleteModel { .. })
    ));
}
