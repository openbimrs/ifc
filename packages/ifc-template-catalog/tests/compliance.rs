#[path = "support/mod.rs"]
mod support;

use ifc_template_catalog::compliance::{
    validate, MemberForm, ObservedMember, ObservedSet, ValidationCode, ValidationPolicy,
};
use ifc_template_catalog::definition::{
    PropertyDataType, PropertyKind, PropertyTemplate, SetTemplateKind,
};

#[test]
fn validation_checks_form_type_and_unexpected_members() {
    let mut template = support::property_set("Pset_Test");
    let SetTemplateKind::Property { properties, .. } = &mut template.kind else {
        panic!()
    };
    properties.push(PropertyTemplate {
        name: "Enabled".into(),
        guid: None,
        definition: None,
        name_aliases: vec![],
        definition_aliases: vec![],
        kind: PropertyKind::SingleValue {
            data_type: PropertyDataType::new("IfcBoolean"),
        },
    });
    let observed = ObservedSet::new("Pset_Test")
        .with_member(
            ObservedMember::property("Enabled", MemberForm::SingleValue).with_data_type("IfcLabel"),
        )
        .with_member(ObservedMember::property("Extra", MemberForm::SingleValue));
    let report = validate(&template, &observed, ValidationPolicy::default());
    assert!(!report.is_valid());
    assert!(report
        .issues
        .iter()
        .any(|issue| issue.code == ValidationCode::DataTypeMismatch));
    assert!(report
        .issues
        .iter()
        .any(|issue| issue.code == ValidationCode::UnexpectedMember));
}

#[test]
fn missing_official_type_is_diagnostic_not_a_false_mismatch() {
    let mut template = support::property_set("Pset_Test");
    let SetTemplateKind::Property { properties, .. } = &mut template.kind else {
        panic!()
    };
    properties.push(PropertyTemplate {
        name: "BrokenUpstreamType".into(),
        guid: None,
        definition: None,
        name_aliases: vec![],
        definition_aliases: vec![],
        kind: PropertyKind::SingleValue {
            data_type: PropertyDataType {
                type_name: None,
                unit_type: None,
            },
        },
    });
    let observed = ObservedSet::new("Pset_Test").with_member(
        ObservedMember::property("BrokenUpstreamType", MemberForm::SingleValue)
            .with_data_type("IfcLabel"),
    );
    assert!(validate(&template, &observed, ValidationPolicy::default()).is_valid());
}
