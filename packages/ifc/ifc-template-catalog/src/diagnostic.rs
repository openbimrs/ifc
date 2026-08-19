//! Structural and schema-aware catalog diagnostics.

mod schema;

pub use schema::CatalogSchema;

use std::collections::BTreeSet;

use crate::catalog::Catalog;
use crate::definition::{PropertyKind, PropertyTemplate, SetTemplateKind};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum DiagnosticCode {
    MissingPropertyDataType,
    EmptyEnumeration,
    EmptyApplicability,
    DuplicateMember,
    UnknownApplicableEntity,
    UnknownPropertyDataType,
    UnknownReferenceEntity,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum DiagnosticSeverity {
    Warning,
    Error,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CatalogDiagnostic {
    pub code: DiagnosticCode,
    pub severity: DiagnosticSeverity,
    pub template: String,
    pub member: Option<String>,
    pub message: String,
}

impl Catalog {
    /// Report source-shape defects without changing official definitions.
    pub fn diagnostics(&self) -> Vec<CatalogDiagnostic> {
        let mut output = Vec::new();
        for template in self.iter() {
            if template.applicability.is_empty() {
                output.push(issue(
                    DiagnosticCode::EmptyApplicability,
                    DiagnosticSeverity::Warning,
                    &template.name,
                    None,
                    "template declares no applicable IFC entity",
                ));
            }
            match &template.kind {
                SetTemplateKind::Property { properties, .. } => {
                    duplicate_members(&template.name, properties, &mut output);
                    inspect_properties(&template.name, "", properties, &mut output);
                }
                SetTemplateKind::Quantity { quantities, .. } => {
                    let mut names = BTreeSet::new();
                    for quantity in quantities {
                        if !names.insert(quantity.name.as_str()) {
                            output.push(issue(
                                DiagnosticCode::DuplicateMember,
                                DiagnosticSeverity::Error,
                                &template.name,
                                Some(&quantity.name),
                                "duplicate quantity template name",
                            ));
                        }
                    }
                }
            }
        }
        output
    }
}

fn inspect_properties(
    template: &str,
    parent: &str,
    properties: &[PropertyTemplate],
    output: &mut Vec<CatalogDiagnostic>,
) {
    for property in properties {
        let path = if parent.is_empty() {
            property.name.clone()
        } else {
            format!("{parent}.{}", property.name)
        };
        visit_data_types(&property.kind, &mut |type_name| {
            if type_name.is_none() {
                output.push(issue(
                    DiagnosticCode::MissingPropertyDataType,
                    DiagnosticSeverity::Error,
                    template,
                    Some(&path),
                    "official DataType element has no type attribute",
                ));
            }
        });
        match &property.kind {
            PropertyKind::EnumeratedValue {
                values, constants, ..
            } if values.is_empty() && constants.is_empty() => output.push(issue(
                DiagnosticCode::EmptyEnumeration,
                DiagnosticSeverity::Error,
                template,
                Some(&path),
                "enumerated property has no values",
            )),
            PropertyKind::Complex { properties, .. } => {
                duplicate_members(template, properties, output);
                inspect_properties(template, &path, properties, output);
            }
            _ => {}
        }
    }
}

pub(crate) fn visit_data_types(kind: &PropertyKind, visitor: &mut impl FnMut(&Option<String>)) {
    match kind {
        PropertyKind::SingleValue { data_type }
        | PropertyKind::BoundedValue { data_type }
        | PropertyKind::ListValue { data_type } => visitor(&data_type.type_name),
        PropertyKind::EnumeratedValue { data_type, .. } => {
            if let Some(data_type) = data_type {
                visitor(&data_type.type_name);
            }
        }
        PropertyKind::TableValue {
            defining_type,
            defined_type,
            ..
        } => {
            visitor(&defining_type.type_name);
            visitor(&defined_type.type_name);
        }
        PropertyKind::ReferenceValue { .. } | PropertyKind::Complex { .. } => {}
    }
}

fn duplicate_members(
    template: &str,
    properties: &[PropertyTemplate],
    output: &mut Vec<CatalogDiagnostic>,
) {
    let mut names = BTreeSet::new();
    for property in properties {
        if !names.insert(property.name.as_str()) {
            output.push(issue(
                DiagnosticCode::DuplicateMember,
                DiagnosticSeverity::Error,
                template,
                Some(&property.name),
                "duplicate property template name",
            ));
        }
    }
}

pub(crate) fn issue(
    code: DiagnosticCode,
    severity: DiagnosticSeverity,
    template: &str,
    member: Option<&str>,
    message: &str,
) -> CatalogDiagnostic {
    CatalogDiagnostic {
        code,
        severity,
        template: template.to_owned(),
        member: member.map(str::to_owned),
        message: message.to_owned(),
    }
}
