use std::collections::{BTreeMap, BTreeSet};

use crate::definition::{PropertyKind, PropertyTemplate, SetTemplate, SetTemplateKind};

use super::{
    MemberForm, ObservedSet, UnexpectedMemberPolicy, ValidationCode, ValidationIssue,
    ValidationPolicy, ValidationReport, ValidationSeverity,
};

struct Expected {
    form: MemberForm,
    data_types: Vec<String>,
    values: Vec<String>,
}

pub fn validate(
    template: &SetTemplate,
    observed: &ObservedSet,
    policy: ValidationPolicy,
) -> ValidationReport {
    let mut report = ValidationReport::default();
    if template.name != observed.name {
        push(
            &mut report,
            ValidationCode::SetNameMismatch,
            ValidationSeverity::Error,
            None,
            format!(
                "observed `{}` does not match template `{}`",
                observed.name, template.name
            ),
        );
    }
    let expected = expected_members(template);
    let mut seen = BTreeSet::new();
    for member in &observed.members {
        if !seen.insert(member.name.as_str()) {
            push(
                &mut report,
                ValidationCode::DuplicateMember,
                ValidationSeverity::Error,
                Some(&member.name),
                "member appears more than once".into(),
            );
            continue;
        }
        let Some(spec) = expected.get(&member.name) else {
            let severity = match policy.unexpected_members {
                UnexpectedMemberPolicy::Ignore => continue,
                UnexpectedMemberPolicy::Warning => ValidationSeverity::Warning,
                UnexpectedMemberPolicy::Error => ValidationSeverity::Error,
            };
            push(
                &mut report,
                ValidationCode::UnexpectedMember,
                severity,
                Some(&member.name),
                "member is not declared by the template".into(),
            );
            continue;
        };
        if member.form != spec.form {
            push(
                &mut report,
                ValidationCode::FormMismatch,
                ValidationSeverity::Error,
                Some(&member.name),
                format!("observed {:?}, expected {:?}", member.form, spec.form),
            );
        }
        if !member.data_types.is_empty()
            && !spec.data_types.is_empty()
            && !same_types(&member.data_types, &spec.data_types)
        {
            push(
                &mut report,
                ValidationCode::DataTypeMismatch,
                ValidationSeverity::Error,
                Some(&member.name),
                format!(
                    "observed {:?}, expected {:?}",
                    member.data_types, spec.data_types
                ),
            );
        }
        if let Some(value) = &member.enumeration_value {
            if !spec.values.is_empty() && !spec.values.iter().any(|item| item == value) {
                push(
                    &mut report,
                    ValidationCode::InvalidEnumerationValue,
                    ValidationSeverity::Error,
                    Some(&member.name),
                    format!("`{value}` is not in the template enumeration"),
                );
            }
        }
    }
    if policy.require_all_members {
        for name in expected.keys().filter(|name| !seen.contains(name.as_str())) {
            push(
                &mut report,
                ValidationCode::MissingMember,
                ValidationSeverity::Error,
                Some(name),
                "required-by-policy member is absent".into(),
            );
        }
    }
    report
}

fn expected_members(template: &SetTemplate) -> BTreeMap<String, Expected> {
    let mut output = BTreeMap::new();
    match &template.kind {
        SetTemplateKind::Property { properties, .. } => flatten("", properties, &mut output),
        SetTemplateKind::Quantity { quantities, .. } => {
            for quantity in quantities {
                output.insert(
                    quantity.name.clone(),
                    Expected {
                        form: MemberForm::Quantity(quantity.kind),
                        data_types: vec![],
                        values: vec![],
                    },
                );
            }
        }
    }
    output
}

fn flatten(parent: &str, properties: &[PropertyTemplate], output: &mut BTreeMap<String, Expected>) {
    for property in properties {
        let path = if parent.is_empty() {
            property.name.clone()
        } else {
            format!("{parent}.{}", property.name)
        };
        let (form, data_types, values) = describe(&property.kind);
        output.insert(
            path.clone(),
            Expected {
                form,
                data_types,
                values,
            },
        );
        if let PropertyKind::Complex { properties, .. } = &property.kind {
            flatten(&path, properties, output);
        }
    }
}

fn describe(kind: &PropertyKind) -> (MemberForm, Vec<String>, Vec<String>) {
    let one = |name: &Option<String>| name.clone().into_iter().collect();
    match kind {
        PropertyKind::SingleValue { data_type } => {
            (MemberForm::SingleValue, one(&data_type.type_name), vec![])
        }
        PropertyKind::BoundedValue { data_type } => {
            (MemberForm::BoundedValue, one(&data_type.type_name), vec![])
        }
        PropertyKind::EnumeratedValue {
            data_type,
            values,
            constants,
            ..
        } => (
            MemberForm::EnumeratedValue,
            data_type
                .as_ref()
                .and_then(|item| item.type_name.clone())
                .into_iter()
                .collect(),
            values
                .iter()
                .cloned()
                .chain(constants.iter().map(|constant| constant.name.clone()))
                .collect(),
        ),
        PropertyKind::ListValue { data_type } => {
            (MemberForm::ListValue, one(&data_type.type_name), vec![])
        }
        PropertyKind::ReferenceValue { reference_type } => (
            MemberForm::ReferenceValue,
            vec![reference_type.clone()],
            vec![],
        ),
        PropertyKind::TableValue {
            defining_type,
            defined_type,
            ..
        } => (
            MemberForm::TableValue,
            [
                defining_type.type_name.clone(),
                defined_type.type_name.clone(),
            ]
            .into_iter()
            .flatten()
            .collect(),
            vec![],
        ),
        PropertyKind::Complex { .. } => (MemberForm::Complex, vec![], vec![]),
    }
}

fn same_types(actual: &[String], expected: &[String]) -> bool {
    actual.len() == expected.len()
        && actual
            .iter()
            .zip(expected)
            .all(|(left, right)| left.eq_ignore_ascii_case(right))
}
fn push(
    report: &mut ValidationReport,
    code: ValidationCode,
    severity: ValidationSeverity,
    member: Option<&str>,
    message: String,
) {
    report.issues.push(ValidationIssue {
        code,
        severity,
        member: member.map(str::to_owned),
        message,
    });
}
