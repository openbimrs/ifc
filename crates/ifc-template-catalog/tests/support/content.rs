//! An encoding-independent fingerprint of a catalog's whole content.
//!
//! Every field of every template, the manifest, the profile, the applied
//! patches and the advisories feed one SHA-256, each value length-prefixed
//! and each absent value tagged, in declaration order. It reads the public
//! types only, so it pins *what* a snapshot decodes to, never how it is
//! stored: a format change that loses or alters one character changes it.

#![allow(dead_code)]

use ifc_template_catalog::catalog::Catalog;
use ifc_template_catalog::definition::{
    Applicability, LocalizedText, PropertyDataType, PropertyKind, PropertyTemplate, SetTemplate,
    SetTemplateKind,
};
use sha2::{Digest, Sha256};

/// Lowercase hexadecimal SHA-256 over the catalog's full content.
pub fn content_digest(catalog: &Catalog) -> String {
    let mut h = Fingerprint(Sha256::new());
    let manifest = catalog.manifest();
    h.debug(&manifest.edition);
    h.text(&manifest.source_label);
    h.text(&manifest.source_url);
    h.text(&manifest.sha256);
    h.count(manifest.property_set_count);
    h.count(manifest.quantity_set_count);
    h.debug(&catalog.profile());
    h.count(catalog.len());
    for set in catalog.iter() {
        h.set(set);
        // Advisories are reachable per template only.
        let advisories = catalog.advisories_for(&set.name);
        h.count(advisories.len());
        for advisory in advisories {
            h.debug(advisory);
        }
    }
    h.count(catalog.applied_patches().len());
    for patch in catalog.applied_patches() {
        h.debug(patch);
    }
    h.0.finalize()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

struct Fingerprint(Sha256);

impl Fingerprint {
    fn count(&mut self, n: usize) {
        self.0.update((n as u64).to_le_bytes());
    }

    fn text(&mut self, s: &str) {
        self.count(s.len());
        self.0.update(s.as_bytes());
    }

    fn optional(&mut self, s: Option<&String>) {
        match s {
            None => self.0.update([0]),
            Some(s) => {
                self.0.update([1]);
                self.text(s);
            }
        }
    }

    /// Field-less enums and the overlay records, by their derived `Debug`.
    fn debug(&mut self, value: &impl std::fmt::Debug) {
        self.text(&format!("{value:?}"));
    }

    fn aliases(&mut self, aliases: &[LocalizedText]) {
        self.count(aliases.len());
        for alias in aliases {
            self.optional(alias.language.as_ref());
            self.text(&alias.text);
        }
    }

    fn data_type(&mut self, data_type: &PropertyDataType) {
        self.optional(data_type.type_name.as_ref());
        self.optional(data_type.unit_type.as_ref());
    }

    fn applicability(&mut self, applicability: &Applicability) {
        self.text(&applicability.raw);
        self.text(&applicability.entity);
        self.optional(applicability.predefined_type.as_ref());
    }

    fn property(&mut self, property: &PropertyTemplate) {
        self.text(&property.name);
        self.optional(property.guid.as_ref());
        self.optional(property.definition.as_ref());
        self.aliases(&property.name_aliases);
        self.aliases(&property.definition_aliases);
        match &property.kind {
            PropertyKind::SingleValue { data_type } => {
                self.text("single");
                self.data_type(data_type);
            }
            PropertyKind::BoundedValue { data_type } => {
                self.text("bounded");
                self.data_type(data_type);
            }
            PropertyKind::EnumeratedValue {
                enumeration_name,
                data_type,
                values,
                constants,
            } => {
                self.text("enumerated");
                self.optional(enumeration_name.as_ref());
                match data_type {
                    None => self.0.update([0]),
                    Some(data_type) => {
                        self.0.update([1]);
                        self.data_type(data_type);
                    }
                }
                self.count(values.len());
                for value in values {
                    self.text(value);
                }
                self.count(constants.len());
                for constant in constants {
                    self.text(&constant.name);
                    self.optional(constant.definition.as_ref());
                    self.aliases(&constant.name_aliases);
                    self.aliases(&constant.definition_aliases);
                }
            }
            PropertyKind::ListValue { data_type } => {
                self.text("list");
                self.data_type(data_type);
            }
            PropertyKind::ReferenceValue { reference_type } => {
                self.text("reference");
                self.text(reference_type);
            }
            PropertyKind::TableValue {
                defining_type,
                defined_type,
                expression,
            } => {
                self.text("table");
                self.data_type(defining_type);
                self.data_type(defined_type);
                self.optional(expression.as_ref());
            }
            PropertyKind::Complex {
                usage_name,
                properties,
            } => {
                self.text("complex");
                self.text(usage_name);
                self.count(properties.len());
                for child in properties {
                    self.property(child);
                }
            }
            other => panic!("the fingerprint does not know {other:?}"),
        }
    }

    fn set(&mut self, set: &SetTemplate) {
        self.text(&set.name);
        self.optional(set.guid.as_ref());
        self.optional(set.definition.as_ref());
        self.aliases(&set.name_aliases);
        self.aliases(&set.definition_aliases);
        match &set.source {
            None => self.0.update([0]),
            Some(source) => {
                self.0.update([1]);
                self.text(&source.relative_path);
                self.text(&source.sha256);
            }
        }
        self.optional(set.raw_applicability.as_ref());
        self.count(set.applicability.len());
        for applicability in &set.applicability {
            self.applicability(applicability);
        }
        match &set.kind {
            SetTemplateKind::Property {
                set_type,
                properties,
            } => {
                self.text("property-set");
                self.debug(set_type);
                self.count(properties.len());
                for property in properties {
                    self.property(property);
                }
            }
            SetTemplateKind::Quantity {
                set_type,
                method_of_measurement,
                quantities,
            } => {
                self.text("quantity-set");
                self.debug(set_type);
                self.optional(method_of_measurement.as_ref());
                self.count(quantities.len());
                for quantity in quantities {
                    self.text(&quantity.name);
                    self.optional(quantity.definition.as_ref());
                    self.aliases(&quantity.name_aliases);
                    self.aliases(&quantity.definition_aliases);
                    self.debug(&quantity.kind);
                }
            }
            other => panic!("the fingerprint does not know {other:?}"),
        }
    }
}
