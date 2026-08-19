use std::fs;
use std::path::{Path, PathBuf};

use ifc_template_catalog::definition::{
    CatalogEdition, PropertySetType, QuantitySetType, SetTemplate, SetTemplateKind, SourceManifest,
    TemplateSource,
};
use ifc_template_catalog::xml::parse_template;
use sha2::{Digest, Sha256};

pub struct ImportedCatalog {
    pub manifest: SourceManifest,
    pub templates: Vec<SetTemplate>,
}

pub fn import(source: &Path) -> Result<ImportedCatalog, String> {
    let mut inputs = Vec::new();
    for kind in ["psd", "qto"] {
        let directory = source.join(kind);
        let entries = fs::read_dir(&directory)
            .map_err(|error| format!("read {}: {error}", directory.display()))?;
        for entry in entries {
            let path = entry.map_err(|error| error.to_string())?.path();
            if path.extension().and_then(|value| value.to_str()) == Some("xml") {
                let name = path
                    .file_name()
                    .and_then(|value| value.to_str())
                    .ok_or_else(|| format!("non-UTF8 path: {}", path.display()))?;
                inputs.push((format!("{kind}/{name}"), path));
            }
        }
    }
    inputs.sort_by(|left, right| left.0.cmp(&right.0));

    let mut hasher = Sha256::new();
    let mut templates = Vec::with_capacity(inputs.len());
    for (relative, path) in inputs {
        let bytes = fs::read(&path).map_err(|error| format!("read {}: {error}", path.display()))?;
        hasher.update(relative.as_bytes());
        hasher.update([0]);
        hasher.update(&bytes);
        hasher.update([0]);
        let xml = std::str::from_utf8(&bytes)
            .map_err(|error| format!("{} is not UTF-8: {error}", path.display()))?;
        let mut template =
            parse_template(xml).map_err(|error| format!("import {}: {error}", path.display()))?;
        let file_sha256 = Sha256::digest(&bytes)
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect();
        template.source = Some(TemplateSource {
            relative_path: relative,
            sha256: file_sha256,
        });
        templates.push(template);
    }

    let (property_sets, quantity_sets, properties, quantities) = counts(&templates);
    let actual = (property_sets, quantity_sets, properties, quantities);
    let expected = (420, 93, 2_550, 257);
    if actual != expected {
        return Err(format!(
            "IFC4 catalog counts {actual:?}, expected {expected:?}"
        ));
    }
    let actual_classifications = classification_counts(&templates)?;
    let expected_classifications = ([353, 0, 16, 42, 9], [0, 0, 0, 93]);
    if actual_classifications != expected_classifications {
        return Err(format!(
            "IFC4 set classifications {actual_classifications:?}, expected {expected_classifications:?}"
        ));
    }
    let sha256 = hasher
        .finalize()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect();
    Ok(ImportedCatalog {
        manifest: SourceManifest {
            edition: CatalogEdition::Ifc4Add2Tc1,
            source_label: "IFC4 ADD2 TC1 PSD/QTO XML".into(),
            source_url: "https://standards.buildingsmart.org/IFC/RELEASE/IFC4/ADD2_TC1/HTML/"
                .into(),
            sha256,
            property_set_count: property_sets,
            quantity_set_count: quantity_sets,
        },
        templates,
    })
}

fn classification_counts(templates: &[SetTemplate]) -> Result<([usize; 5], [usize; 4]), String> {
    let mut property = [0; 5];
    let mut quantity = [0; 4];
    for template in templates {
        match &template.kind {
            SetTemplateKind::Property { set_type, .. } => {
                let index = match set_type {
                    PropertySetType::TypeDrivenOverride => 0,
                    PropertySetType::TypeDrivenOnly => 1,
                    PropertySetType::OccurrenceDriven => 2,
                    PropertySetType::PerformanceDriven => 3,
                    PropertySetType::Unspecified => 4,
                    _ => return Err("generator does not classify a property-set type".into()),
                };
                property[index] += 1;
            }
            SetTemplateKind::Quantity { set_type, .. } => {
                let index = match set_type {
                    QuantitySetType::TypeDrivenOverride => 0,
                    QuantitySetType::TypeDrivenOnly => 1,
                    QuantitySetType::OccurrenceDriven => 2,
                    QuantitySetType::Unspecified => 3,
                    _ => return Err("generator does not classify a quantity-set type".into()),
                };
                quantity[index] += 1;
            }
            _ => return Err("generator and catalog model versions differ".into()),
        }
    }
    Ok((property, quantity))
}

fn counts(templates: &[SetTemplate]) -> (usize, usize, usize, usize) {
    let mut counts = (0, 0, 0, 0);
    for template in templates {
        match &template.kind {
            SetTemplateKind::Property { properties, .. } => {
                counts.0 += 1;
                counts.2 += property_count(properties);
            }
            SetTemplateKind::Quantity { quantities, .. } => {
                counts.1 += 1;
                counts.3 += quantities.len();
            }
            _ => unreachable!("generator and catalog model versions differ"),
        }
    }
    counts
}

fn property_count(properties: &[ifc_template_catalog::definition::PropertyTemplate]) -> usize {
    properties
        .iter()
        .map(|property| {
            1 + match &property.kind {
                ifc_template_catalog::definition::PropertyKind::Complex { properties, .. } => {
                    property_count(properties)
                }
                _ => 0,
            }
        })
        .sum()
}

pub fn default_output(manifest_dir: &Path) -> PathBuf {
    manifest_dir.join("data/ifc4-add2-tc1.bin")
}
