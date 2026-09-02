use std::fs;
use std::path::{Path, PathBuf};

use ifc_template_catalog::definition::{
    CatalogEdition, SetTemplate, SetTemplateKind, SourceManifest, TemplateSource,
};
use ifc_template_catalog::xml::parse_template;
use sha2::{Digest, Sha256};

pub struct ImportedCatalog {
    pub manifest: SourceManifest,
    pub templates: Vec<SetTemplate>,
}

struct EditionSpec {
    label: &'static str,
    url: &'static str,
    property_sets: usize,
    quantity_sets: usize,
    properties: usize,
    quantities: usize,
}

pub fn parse_edition(value: &str) -> Result<CatalogEdition, String> {
    match value {
        "ifc2x3-tc1" => Ok(CatalogEdition::Ifc2x3Tc1),
        "ifc4-add2-tc1" => Ok(CatalogEdition::Ifc4Add2Tc1),
        "ifc4x3-add2" => Ok(CatalogEdition::Ifc4x3Add2),
        _ => Err(format!("unsupported catalog edition `{value}`")),
    }
}

pub fn import(edition: CatalogEdition, source: &Path) -> Result<ImportedCatalog, String> {
    let spec = edition_spec(edition)?;
    let mut inputs = Vec::new();
    for directory in input_directories(source, edition)? {
        inputs.extend(walk_xml(&directory, source, edition)?);
    }
    inputs.sort_by(|left, right| left.0.cmp(&right.0));
    if inputs.is_empty() {
        return Err(format!("no XML files below {}", source.display()));
    }

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

    let actual = counts(&templates)?;
    let expected = (
        spec.property_sets,
        spec.quantity_sets,
        spec.properties,
        spec.quantities,
    );
    if actual != expected {
        return Err(format!(
            "{} catalog counts {actual:?}, expected {expected:?}",
            spec.label
        ));
    }
    let sha256 = hasher
        .finalize()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect();
    Ok(ImportedCatalog {
        manifest: SourceManifest {
            edition,
            source_label: spec.label.into(),
            source_url: spec.url.into(),
            sha256,
            property_set_count: actual.0,
            quantity_set_count: actual.1,
        },
        templates,
    })
}

fn input_directories(source: &Path, edition: CatalogEdition) -> Result<Vec<PathBuf>, String> {
    let directories = match edition {
        CatalogEdition::Ifc2x3Tc1 => vec![source.to_owned()],
        CatalogEdition::Ifc4Add2Tc1 => vec![source.join("psd"), source.join("qto")],
        CatalogEdition::Ifc4x3Add2 => vec![source.join("psd")],
        _ => return Err(format!("unsupported catalog edition {edition:?}")),
    };
    for directory in &directories {
        if !directory.is_dir() {
            return Err(format!(
                "missing XML input directory {}",
                directory.display()
            ));
        }
    }
    Ok(directories)
}

fn walk_xml(
    path: &Path,
    source_root: &Path,
    edition: CatalogEdition,
) -> Result<Vec<(String, PathBuf)>, String> {
    if path.is_file() {
        if path.extension().and_then(|value| value.to_str()) != Some("xml") {
            return Ok(Vec::new());
        }
        let relative = path
            .strip_prefix(source_root)
            .map_err(|error| format!("relative path for {}: {error}", path.display()))?
            .to_str()
            .ok_or_else(|| format!("non-UTF8 path: {}", path.display()))?
            .to_owned();
        let relative = match edition {
            CatalogEdition::Ifc2x3Tc1 => format!("psd/{relative}"),
            CatalogEdition::Ifc4Add2Tc1 | CatalogEdition::Ifc4x3Add2 => relative,
            _ => return Err(format!("unsupported catalog edition {edition:?}")),
        };
        return Ok(vec![(relative, path.to_owned())]);
    }
    let mut result = Vec::new();
    for entry in fs::read_dir(path).map_err(|error| format!("read {}: {error}", path.display()))? {
        result.extend(walk_xml(
            &entry.map_err(|error| error.to_string())?.path(),
            source_root,
            edition,
        )?);
    }
    Ok(result)
}

fn edition_spec(edition: CatalogEdition) -> Result<EditionSpec, String> {
    Ok(match edition {
        CatalogEdition::Ifc2x3Tc1 => EditionSpec {
            label: "IFC2X3 TC1 PSD XML",
            url: "https://standards.buildingsmart.org/IFC/RELEASE/IFC2x3/TC1/HTML/psd/",
            property_sets: 317,
            quantity_sets: 0,
            properties: 1_856,
            quantities: 0,
        },
        CatalogEdition::Ifc4Add2Tc1 => EditionSpec {
            label: "IFC4 ADD2 TC1 PSD/QTO XML",
            url: "https://standards.buildingsmart.org/IFC/RELEASE/IFC4/ADD2_TC1/HTML/",
            property_sets: 420,
            quantity_sets: 93,
            properties: 2_550,
            quantities: 257,
        },
        CatalogEdition::Ifc4x3Add2 => EditionSpec {
            label: "IFC4X3 ADD2 PSD/QTO XML",
            url: "https://github.com/buildingSMART/IFC4.x-development/tree/524daac53ca682e0649d240ace87f4cd7baff6e7/reference_schemas",
            property_sets: 502,
            quantity_sets: 110,
            properties: 2_918,
            quantities: 324,
        },
        _ => return Err(format!("unsupported catalog edition {edition:?}")),
    })
}

fn counts(templates: &[SetTemplate]) -> Result<(usize, usize, usize, usize), String> {
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
            _ => return Err("unsupported set template kind".into()),
        }
    }
    Ok(counts)
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

pub fn default_output(manifest_dir: &Path, edition: CatalogEdition) -> Result<PathBuf, String> {
    let filename = match edition {
        CatalogEdition::Ifc2x3Tc1 => "ifc2x3-tc1.bin",
        CatalogEdition::Ifc4Add2Tc1 => "ifc4-add2-tc1.bin",
        CatalogEdition::Ifc4x3Add2 => "ifc4x3-add2.bin",
        _ => return Err(format!("unsupported catalog edition {edition:?}")),
    };
    Ok(manifest_dir.join("data").join(filename))
}
