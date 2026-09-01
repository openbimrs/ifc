use ifc_schema::ifc4;

#[test]
fn entity_membership_walks_selects_and_inheritance() {
    let schema = ifc4();
    assert!(schema.accepts_type("IfcResourceObjectSelect", "IfcApproval"));
    assert!(schema.accepts_type("IfcResourceObjectSelect", "IfcClassificationReference"));
    assert!(!schema.accepts_type("IfcResourceObjectSelect", "IfcWall"));
}

#[test]
fn nested_select_membership_covers_metric_values() {
    let schema = ifc4();
    assert!(schema.accepts_type("IfcMetricValueSelect", "IfcAppliedValue"));
    assert!(schema.accepts_type("IfcMetricValueSelect", "IfcLengthMeasure"));
    assert!(schema.accepts_type("IfcLengthMeasure", "IfcPositiveLengthMeasure"));
    assert!(schema.accepts_type("ifcmetricvalueselect", "ifclengthmeasure"));
    assert!(!schema.accepts_type("IfcMetricValueSelect", "IfcWall"));
}

#[test]
fn membership_refuses_unknown_and_enum_crossovers() {
    let schema = ifc4();
    assert!(!schema.accepts_type("IfcDoesNotExist", "IfcWall"));
    assert!(!schema.accepts_type("IfcDoesNotExist", "ifcdoesnotexist"));
    assert!(!schema.accepts_type("IfcConstraintEnum", "IfcObjectiveEnum"));
}
