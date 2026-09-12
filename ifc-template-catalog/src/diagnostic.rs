//! Structural and schema-aware catalog diagnostics.

mod schema;

pub use schema::CatalogSchema;

use std::collections::BTreeSet;

use crate::catalog::Catalog;
use crate::definition::{PropertyKind, PropertyTemplate, SetTemplateKind};

/// Category of a structural or schema-aware catalog defect.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum DiagnosticCode {
    /// A property's official `DataType` element carried no `type` attribute.
    MissingPropertyDataType,
    /// An `EnumeratedValue` property has neither lexical values nor documented constants.
    EmptyEnumeration,
    /// A set template declares no applicable IFC entity.
    EmptyApplicability,
    /// Two members of the same set template (or nested `Complex` scope) share a name.
    DuplicateMember,
    /// [`Catalog::schema_diagnostics`] found an applicability entity the schema does not declare.
    UnknownApplicableEntity,
    /// [`Catalog::schema_diagnostics`] found a property value type the schema does not declare.
    UnknownPropertyDataType,
    /// [`Catalog::schema_diagnostics`] found a `ReferenceValue` entity the schema does not declare.
    UnknownReferenceEntity,
}

/// How serious a [`CatalogDiagnostic`] is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum DiagnosticSeverity {
    /// Worth flagging but does not indicate malformed catalog data.
    Warning,
    /// Indicates malformed or self-inconsistent catalog data.
    Error,
}

/// One structural or schema-aware defect found in a catalog snapshot.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CatalogDiagnostic {
    /// Category of the defect.
    pub code: DiagnosticCode,
    /// How serious the defect is.
    pub severity: DiagnosticSeverity,
    /// `Name` of the set template the defect was found in.
    pub template: String,
    /// Dotted member path within the template, absent for template-level defects.
    pub member: Option<String>,
    /// Human-readable description of the defect.
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
