//! QTO set decoding.

use crate::definition::{
    QuantityKind, QuantitySetType, QuantityTemplate, SetTemplate, SetTemplateKind,
};

use super::common::{applicability, localized, required_text};
use super::{Node, XmlImportError};

pub(super) fn parse(root: &Node) -> Result<SetTemplate, XmlImportError> {
    let name = required_text(root, "Name", "QtoSetDef")?.to_owned();
    let set_type = match root.attribute("templatetype").unwrap_or_default() {
        "" => QuantitySetType::Unspecified,
        "QTO_TYPEDRIVENOVERRIDE" => QuantitySetType::TypeDrivenOverride,
        "QTO_TYPEDRIVENONLY" => QuantitySetType::TypeDrivenOnly,
        "QTO_OCCURRENCEDRIVEN" => QuantitySetType::OccurrenceDriven,
        value => {
            return Err(XmlImportError::UnsupportedSetType {
                set: name,
                value: value.to_owned(),
            });
        }
    };
    let mut quantities = Vec::new();
    if let Some(definitions) = root.child("QtoDefs") {
        for definition in definitions.children_named("QtoDef") {
            quantities.push(parse_quantity(definition, &name)?);
        }
        for definition in definitions.children_named("QtoDefinition") {
            quantities.push(parse_quantity(definition, &name)?);
        }
    }
    let (raw_applicability, applicability) = applicability(root)?;
    let mut definition_aliases = localized(root, "QtoDefinitionAliases", "QtoDefinitionAlias");
    definition_aliases.extend(localized(root, "DefinitionAliases", "DefinitionAlias"));
    Ok(SetTemplate {
        name,
        guid: root.attribute("ifdguid").map(str::to_owned),
        definition: root.child_text("Definition").map(str::to_owned),
        name_aliases: localized(root, "NameAliases", "NameAlias"),
        definition_aliases,
        source: None,
        raw_applicability,
        applicability,
        kind: SetTemplateKind::Quantity {
            set_type,
            method_of_measurement: root.child_text("MethodOfMeasurement").map(str::to_owned),
            quantities,
        },
    })
}

fn parse_quantity(node: &Node, set_name: &str) -> Result<QuantityTemplate, XmlImportError> {
    let name = required_text(node, "Name", set_name)?.to_owned();
    let qto_type = required_text(node, "QtoType", &format!("{set_name}.{name}"))?;
    let kind = match qto_type {
        "Q_LENGTH" => QuantityKind::Length,
        "Q_AREA" => QuantityKind::Area,
        "Q_VOLUME" => QuantityKind::Volume,
        "Q_WEIGHT" => QuantityKind::Weight,
        "Q_TIME" => QuantityKind::Time,
        "Q_COUNT" => QuantityKind::Count,
        "Q_NUMBER" => QuantityKind::Number,
        value => {
            return Err(XmlImportError::UnsupportedQuantityType {
                set: set_name.to_owned(),
                quantity: name,
                value: value.to_owned(),
            });
        }
    };
    Ok(QuantityTemplate {
        name,
        definition: node.child_text("Definition").map(str::to_owned),
        name_aliases: localized(node, "NameAliases", "NameAlias"),
        definition_aliases: localized(node, "DefinitionAliases", "DefinitionAlias"),
        kind,
    })
}
