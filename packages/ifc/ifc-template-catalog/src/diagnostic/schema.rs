use crate::catalog::Catalog;
use crate::definition::{PropertyKind, PropertyTemplate, SetTemplateKind};

use super::{issue, visit_data_types, CatalogDiagnostic, DiagnosticCode, DiagnosticSeverity};

/// Minimum schema capability needed to validate external catalog names.
pub trait CatalogSchema {
    fn has_entity(&self, name: &str) -> bool;
    fn has_type(&self, name: &str) -> bool;
}

impl Catalog {
    /// Validate entity and value-type references against the selected IFC schema.
    pub fn schema_diagnostics(&self, schema: &impl CatalogSchema) -> Vec<CatalogDiagnostic> {
        let mut output = Vec::new();
        for template in self.iter() {
            for selector in &template.applicability {
                if !schema.has_entity(&selector.entity) {
                    output.push(issue(
                        DiagnosticCode::UnknownApplicableEntity,
                        DiagnosticSeverity::Error,
                        &template.name,
                        None,
                        &format!("unknown applicable entity `{}`", selector.entity),
                    ));
                }
            }
            if let SetTemplateKind::Property { properties, .. } = &template.kind {
                inspect_properties(&template.name, properties, schema, &mut output);
            }
        }
        output
    }
}

fn inspect_properties(
    template: &str,
    properties: &[PropertyTemplate],
    schema: &impl CatalogSchema,
    output: &mut Vec<CatalogDiagnostic>,
) {
    for property in properties {
        visit_data_types(&property.kind, &mut |type_name| {
            if let Some(type_name) = type_name {
                if !schema.has_type(type_name) {
                    output.push(issue(
                        DiagnosticCode::UnknownPropertyDataType,
                        DiagnosticSeverity::Error,
                        template,
                        Some(&property.name),
                        &format!("unknown property data type `{type_name}`"),
                    ));
                }
            }
        });
        match &property.kind {
            PropertyKind::ReferenceValue { reference_type }
                if !schema.has_entity(reference_type) =>
            {
                output.push(issue(
                    DiagnosticCode::UnknownReferenceEntity,
                    DiagnosticSeverity::Error,
                    template,
                    Some(&property.name),
                    &format!("unknown reference entity `{reference_type}`"),
                ));
            }
            PropertyKind::Complex { properties, .. } => {
                inspect_properties(template, properties, schema, output);
            }
            _ => {}
        }
    }
}

#[cfg(feature = "schema")]
impl CatalogSchema for ifc_schema::Schema {
    fn has_entity(&self, name: &str) -> bool {
        self.entity(name).is_some()
    }

    fn has_type(&self, name: &str) -> bool {
        self.type_def(name).is_some()
    }
}
