//! Deterministic, diffable IFC template applicability export.

use std::io::{self, Write};

use crate::catalog::Catalog;
use crate::definition::{
    Applicability, CatalogEdition, PropertyDataType, PropertyKind, PropertySetType,
    PropertyTemplate, QuantityKind, QuantitySetType, SetTemplate, SetTemplateKind,
};

pub const TSV_HEADER: &str = "edition\tsource_digest\tset_kind\tset_name\tset_template_type\tapplicable_entity\tpredefined_type\tincludes_subtypes\tmember_path\tmember_kind\tvalue_type\tunit_type\tsource_path\tsource_file_digest";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExportSummary {
    pub set_count: usize,
    pub property_set_count: usize,
    pub quantity_set_count: usize,
    pub row_count: usize,
}

pub fn write_applicability_tsv(
    catalog: &Catalog,
    mut output: impl Write,
) -> io::Result<ExportSummary> {
    let manifest = catalog.manifest();
    let mut rows = Vec::new();
    let mut property_set_count = 0;
    let mut quantity_set_count = 0;
    for set in catalog.iter() {
        match &set.kind {
            SetTemplateKind::Property { .. } => property_set_count += 1,
            SetTemplateKind::Quantity { .. } => quantity_set_count += 1,
        }
        if set.applicability.is_empty() {
            append_set_rows(&mut rows, set, None, manifest.edition, &manifest.sha256);
        } else {
            for applicability in &set.applicability {
                append_set_rows(
                    &mut rows,
                    set,
                    Some(applicability),
                    manifest.edition,
                    &manifest.sha256,
                );
            }
        }
    }
    rows.sort_unstable();
    writeln!(output, "{TSV_HEADER}")?;
    for row in &rows {
        writeln!(output, "{row}")?;
    }
    Ok(ExportSummary {
        set_count: catalog.len(),
        property_set_count,
        quantity_set_count,
        row_count: rows.len(),
    })
}

fn append_set_rows(
    rows: &mut Vec<String>,
    set: &SetTemplate,
    applicability: Option<&Applicability>,
    edition: CatalogEdition,
    source_digest: &str,
) {
    match &set.kind {
        SetTemplateKind::Property {
            set_type,
            properties,
        } => {
            for property in properties {
                append_property_rows(
                    rows,
                    set,
                    applicability,
                    edition,
                    source_digest,
                    property_set_type(*set_type),
                    "",
                    property,
                );
            }
        }
        SetTemplateKind::Quantity {
            set_type,
            quantities,
            ..
        } => {
            for quantity in quantities {
                append_row(
                    rows,
                    set,
                    applicability,
                    edition,
                    source_digest,
                    "qto",
                    quantity_set_type(*set_type),
                    &quantity.name,
                    quantity_kind(quantity.kind),
                    quantity_value_type(quantity.kind),
                    "",
                );
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn append_property_rows(
    rows: &mut Vec<String>,
    set: &SetTemplate,
    applicability: Option<&Applicability>,
    edition: CatalogEdition,
    source_digest: &str,
    set_type: &str,
    parent_path: &str,
    property: &PropertyTemplate,
) {
    let path = if parent_path.is_empty() {
        property.name.clone()
    } else {
        format!("{parent_path}.{}", property.name)
    };
    let (kind, value_type, unit_type) = property_kind(&property.kind);
    append_row(
        rows,
        set,
        applicability,
        edition,
        source_digest,
        "psd",
        set_type,
        &path,
        kind,
        &value_type,
        &unit_type,
    );
    if let PropertyKind::Complex { properties, .. } = &property.kind {
        for nested in properties {
            append_property_rows(
                rows,
                set,
                applicability,
                edition,
                source_digest,
                set_type,
                &path,
                nested,
            );
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn append_row(
    rows: &mut Vec<String>,
    set: &SetTemplate,
    applicability: Option<&Applicability>,
    edition: CatalogEdition,
    source_digest: &str,
    set_kind: &str,
    set_type: &str,
    member_path: &str,
    member_kind: &str,
    value_type: &str,
    unit_type: &str,
) {
    let source = set.source.as_ref();
    let fields = [
        edition_name(edition),
        source_digest,
        set_kind,
        &set.name,
        set_type,
        applicability.map_or("", |value| value.entity.as_str()),
        applicability
            .and_then(|value| value.predefined_type.as_deref())
            .unwrap_or(""),
        "true",
        member_path,
        member_kind,
        value_type,
        unit_type,
        source.map_or("", |value| value.relative_path.as_str()),
        source.map_or("", |value| value.sha256.as_str()),
    ];
    rows.push(fields.map(escape).join("\t"));
}

fn property_kind(kind: &PropertyKind) -> (&'static str, String, String) {
    match kind {
        PropertyKind::SingleValue { data_type } => property_type("single", data_type),
        PropertyKind::BoundedValue { data_type } => property_type("bounded", data_type),
        PropertyKind::EnumeratedValue {
            enumeration_name,
            data_type,
            ..
        } => (
            "enumerated",
            data_type
                .as_ref()
                .and_then(|value| value.type_name.clone())
                .or_else(|| enumeration_name.clone())
                .unwrap_or_default(),
            data_type
                .as_ref()
                .and_then(|value| value.unit_type.clone())
                .unwrap_or_default(),
        ),
        PropertyKind::ListValue { data_type } => property_type("list", data_type),
        PropertyKind::ReferenceValue { reference_type } => {
            ("reference", reference_type.clone(), String::new())
        }
        PropertyKind::TableValue {
            defining_type,
            defined_type,
            ..
        } => (
            "table",
            format!(
                "{}->{}",
                defining_type.type_name.as_deref().unwrap_or(""),
                defined_type.type_name.as_deref().unwrap_or("")
            ),
            String::new(),
        ),
        PropertyKind::Complex { usage_name, .. } => ("complex", usage_name.clone(), String::new()),
    }
}

fn property_type(
    kind: &'static str,
    data_type: &PropertyDataType,
) -> (&'static str, String, String) {
    (
        kind,
        data_type.type_name.clone().unwrap_or_default(),
        data_type.unit_type.clone().unwrap_or_default(),
    )
}

fn escape(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('\t', "\\t")
        .replace('\r', "\\r")
        .replace('\n', "\\n")
}

fn edition_name(value: CatalogEdition) -> &'static str {
    match value {
        CatalogEdition::Ifc2x3Tc1 => "IFC2X3_TC1",
        CatalogEdition::Ifc4Add2Tc1 => "IFC4_ADD2_TC1",
        CatalogEdition::Ifc4x3Add2 => "IFC4X3_ADD2",
    }
}

fn property_set_type(value: PropertySetType) -> &'static str {
    match value {
        PropertySetType::TypeDrivenOverride => "type-driven-override",
        PropertySetType::TypeDrivenOnly => "type-driven-only",
        PropertySetType::OccurrenceDriven => "occurrence-driven",
        PropertySetType::PerformanceDriven => "performance-driven",
        PropertySetType::Unspecified => "unspecified",
    }
}

fn quantity_set_type(value: QuantitySetType) -> &'static str {
    match value {
        QuantitySetType::TypeDrivenOverride => "type-driven-override",
        QuantitySetType::TypeDrivenOnly => "type-driven-only",
        QuantitySetType::OccurrenceDriven => "occurrence-driven",
        QuantitySetType::Unspecified => "unspecified",
    }
}

fn quantity_kind(value: QuantityKind) -> &'static str {
    match value {
        QuantityKind::Length => "length",
        QuantityKind::Area => "area",
        QuantityKind::Volume => "volume",
        QuantityKind::Weight => "weight",
        QuantityKind::Time => "time",
        QuantityKind::Count => "count",
        QuantityKind::Number => "number",
    }
}

fn quantity_value_type(value: QuantityKind) -> &'static str {
    match value {
        QuantityKind::Length => "Q_LENGTH",
        QuantityKind::Area => "Q_AREA",
        QuantityKind::Volume => "Q_VOLUME",
        QuantityKind::Weight => "Q_WEIGHT",
        QuantityKind::Time => "Q_TIME",
        QuantityKind::Count => "Q_COUNT",
        QuantityKind::Number => "Q_NUMBER",
    }
}
