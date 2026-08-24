//! PSD set decoding.

use crate::definition::{PropertySetType, SetTemplate, SetTemplateKind};

use super::common::{applicability, localized, required_text};
use super::property::parse_property;
use super::{Node, XmlImportError};

pub(super) fn parse(root: &Node) -> Result<SetTemplate, XmlImportError> {
    let name = required_text(root, "Name", "PropertySetDef")?.to_owned();
    let set_type = match root.attribute("templatetype").unwrap_or_default() {
        "" => PropertySetType::Unspecified,
        "PSET_TYPEDRIVENOVERRIDE" => PropertySetType::TypeDrivenOverride,
        "PSET_OCCURRENCEDRIVEN" => PropertySetType::OccurrenceDriven,
        "PSET_PERFORMANCEDRIVEN" => PropertySetType::PerformanceDriven,
        value => {
            return Err(XmlImportError::UnsupportedSetType {
                set: name,
                value: value.to_owned(),
            });
        }
    };
    let mut properties = Vec::new();
    if let Some(definitions) = root.child("PropertyDefs") {
        for definition in definitions.children_named("PropertyDef") {
            properties.push(parse_property(definition, &name)?);
        }
    }
    let (raw_applicability, applicability) = applicability(root)?;
    let mut definition_aliases = localized(root, "PsetDefinitionAliases", "PsetDefinitionAlias");
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
        kind: SetTemplateKind::Property {
            set_type,
            properties,
        },
    })
}
