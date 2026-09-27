//! Shared PSD/QTO decoding helpers.

use crate::definition::{Applicability, LocalizedText, PropertyDataType};

use super::{Node, XmlImportError};

pub(super) fn required_text<'a>(
    node: &'a Node,
    field: &str,
    path: &str,
) -> Result<&'a str, XmlImportError> {
    node.child_text(field)
        .ok_or_else(|| XmlImportError::MissingField {
            path: path.to_owned(),
            field: field.to_owned(),
        })
}

pub(super) fn applicability(
    root: &Node,
) -> Result<(Option<String>, Vec<Applicability>), XmlImportError> {
    let raw = root.child_text("ApplicableTypeValue").map(str::to_owned);
    let values: Vec<String> = if let Some(raw) = &raw {
        raw.split(',')
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_owned)
            .collect()
    } else {
        root.child("ApplicableClasses")
            .into_iter()
            .flat_map(|classes| classes.children_named("ClassName"))
            .filter_map(Node::text_trimmed)
            .map(str::to_owned)
            .collect()
    };

    let selectors = values
        .into_iter()
        .map(|value| Applicability::parse(value).map_err(XmlImportError::Applicability))
        .collect::<Result<Vec<_>, _>>()?;
    Ok((raw, selectors))
}

pub(super) fn localized(node: &Node, container: &str, item: &str) -> Vec<LocalizedText> {
    node.child(container)
        .into_iter()
        .flat_map(|aliases| aliases.children_named(item))
        .map(|alias| LocalizedText {
            language: alias.attribute("lang").map(str::to_owned),
            text: alias.text.trim().to_owned(),
        })
        .collect()
}

pub(super) fn data_type(node: &Node, path: &str) -> Result<PropertyDataType, XmlImportError> {
    let data_type = node
        .child("DataType")
        .ok_or_else(|| XmlImportError::MissingField {
            path: path.to_owned(),
            field: "DataType".into(),
        })?;
    let type_name = data_type.attribute("type").map(str::to_owned);
    Ok(PropertyDataType {
        type_name,
        unit_type: node
            .child("UnitType")
            .and_then(|unit| unit.attribute("type"))
            .map(str::to_owned),
    })
}
