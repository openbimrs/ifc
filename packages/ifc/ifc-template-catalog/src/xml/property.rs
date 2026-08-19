//! PSD property decoding.

use crate::definition::{EnumerationConstant, PropertyKind, PropertyTemplate};

use super::common::{data_type, localized, required_text};
use super::{Node, XmlImportError};

pub(super) fn parse_property(
    node: &Node,
    set_name: &str,
) -> Result<PropertyTemplate, XmlImportError> {
    let name = required_text(node, "Name", set_name)?.to_owned();
    let path = format!("{set_name}.{name}");
    let property_type = node
        .child("PropertyType")
        .ok_or_else(|| XmlImportError::MissingField {
            path: path.clone(),
            field: "PropertyType".into(),
        })?;
    if property_type.children.len() != 1 {
        return Err(XmlImportError::AmbiguousPropertyType { path });
    }
    let kind = parse_kind(&property_type.children[0], set_name, &name)?;
    Ok(PropertyTemplate {
        name,
        guid: node.attribute("ifdguid").map(str::to_owned),
        definition: node.child_text("Definition").map(str::to_owned),
        name_aliases: localized(node, "NameAliases", "NameAlias"),
        definition_aliases: localized(node, "DefinitionAliases", "DefinitionAlias"),
        kind,
    })
}

fn parse_kind(node: &Node, set_name: &str, property: &str) -> Result<PropertyKind, XmlImportError> {
    let path = format!("{set_name}.{property}");
    match node.name.as_str() {
        "TypePropertySingleValue" => Ok(PropertyKind::SingleValue {
            data_type: data_type(node, &path)?,
        }),
        "TypePropertyBoundedValue" => Ok(PropertyKind::BoundedValue {
            data_type: data_type(node, &path)?,
        }),
        "TypePropertyEnumeratedValue" => parse_enumeration(node, &path),
        "TypePropertyListValue" => {
            let list = node.child("ListValue").unwrap_or(node);
            Ok(PropertyKind::ListValue {
                data_type: data_type(list, &path)?,
            })
        }
        "TypePropertyReferenceValue" => {
            let reference_type =
                node.attribute("reftype")
                    .ok_or_else(|| XmlImportError::MissingField {
                        path: path.clone(),
                        field: "TypePropertyReferenceValue@reftype".into(),
                    })?;
            Ok(PropertyKind::ReferenceValue {
                reference_type: reference_type.to_owned(),
            })
        }
        "TypePropertyTableValue" => {
            let defining =
                node.child("DefiningValue")
                    .ok_or_else(|| XmlImportError::MissingField {
                        path: path.clone(),
                        field: "DefiningValue".into(),
                    })?;
            let defined =
                node.child("DefinedValue")
                    .ok_or_else(|| XmlImportError::MissingField {
                        path: path.clone(),
                        field: "DefinedValue".into(),
                    })?;
            Ok(PropertyKind::TableValue {
                defining_type: data_type(defining, &path)?,
                defined_type: data_type(defined, &path)?,
                expression: node
                    .child("Expression")
                    .map(|expression| expression.text.trim().to_owned()),
            })
        }
        "TypeComplexProperty" => {
            let mut properties = Vec::new();
            for child in node.children_named("PropertyDef") {
                properties.push(parse_property(child, &path)?);
            }
            if let Some(children) = node.child("PropertyDefs") {
                for child in children.children_named("PropertyDef") {
                    properties.push(parse_property(child, &path)?);
                }
            }
            let usage_name =
                node.attribute("name")
                    .ok_or_else(|| XmlImportError::MissingField {
                        path: path.clone(),
                        field: "TypeComplexProperty@name".into(),
                    })?;
            Ok(PropertyKind::Complex {
                usage_name: usage_name.to_owned(),
                properties,
            })
        }
        element => Err(XmlImportError::UnsupportedPropertyType {
            set: set_name.to_owned(),
            property: property.to_owned(),
            element: element.to_owned(),
        }),
    }
}

fn parse_enumeration(node: &Node, path: &str) -> Result<PropertyKind, XmlImportError> {
    let enum_list = node.child("EnumList");
    let values: Vec<String> = enum_list
        .into_iter()
        .flat_map(|list| list.children_named("EnumItem"))
        .filter_map(Node::text_trimmed)
        .map(str::to_owned)
        .collect();
    let constants = node
        .child("ConstantList")
        .into_iter()
        .flat_map(|list| list.children_named("ConstantDef"))
        .map(|constant| {
            Ok(EnumerationConstant {
                name: required_text(constant, "Name", path)?.to_owned(),
                definition: constant.child_text("Definition").map(str::to_owned),
                name_aliases: localized(constant, "NameAliases", "NameAlias"),
                definition_aliases: localized(constant, "DefinitionAliases", "DefinitionAlias"),
            })
        })
        .collect::<Result<Vec<_>, XmlImportError>>()?;
    let data_type = node
        .child("DataType")
        .map(|_| data_type(node, path))
        .transpose()?;
    Ok(PropertyKind::EnumeratedValue {
        enumeration_name: enum_list
            .and_then(|list| list.attribute("name"))
            .map(str::to_owned),
        data_type,
        values,
        constants,
    })
}
