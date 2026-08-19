#![allow(dead_code)]

use ifc_template_catalog::definition::{
    CatalogEdition, PropertySetType, QuantitySetType, SetTemplate, SetTemplateKind, SourceManifest,
};

pub fn manifest(property_set_count: usize, quantity_set_count: usize) -> SourceManifest {
    SourceManifest {
        edition: CatalogEdition::Ifc4Add2Tc1,
        source_label: "IFC4 ADD2 TC1".into(),
        source_url: "https://standards.buildingsmart.org/IFC/RELEASE/IFC4/ADD2_TC1/".into(),
        sha256: "fixture".into(),
        property_set_count,
        quantity_set_count,
    }
}

pub fn quantity_set(name: &str, set_type: QuantitySetType) -> SetTemplate {
    SetTemplate {
        name: name.into(),
        guid: None,
        definition: None,
        name_aliases: Vec::new(),
        definition_aliases: Vec::new(),
        source: None,
        raw_applicability: None,
        applicability: Vec::new(),
        kind: SetTemplateKind::Quantity {
            set_type,
            method_of_measurement: None,
            quantities: Vec::new(),
        },
    }
}

pub fn property_set(name: &str) -> SetTemplate {
    SetTemplate {
        name: name.into(),
        guid: None,
        definition: None,
        name_aliases: Vec::new(),
        definition_aliases: Vec::new(),
        source: None,
        raw_applicability: None,
        applicability: Vec::new(),
        kind: SetTemplateKind::Property {
            set_type: PropertySetType::TypeDrivenOverride,
            properties: Vec::new(),
        },
    }
}
