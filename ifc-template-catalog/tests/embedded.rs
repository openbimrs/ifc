use ifc_template_catalog::definition::{
    CatalogEdition, PropertyKind, PropertySetType, QuantitySetType, SetTemplateKind,
};
use ifc_template_catalog::embedded::official_catalog;

#[test]
fn embedded_all_editions_have_pinned_source_counts() {
    for (edition, property_sets, quantity_sets, properties, quantities, digest) in [
        (
            CatalogEdition::Ifc2x3Tc1,
            317,
            0,
            1_856,
            0,
            "395dcd1e8c6f8e5feeece08e8b46e211a3c35a7fc13a7a237485ea48b3c93d53",
        ),
        (
            CatalogEdition::Ifc4Add2Tc1,
            420,
            93,
            2_550,
            257,
            "57227d4c82f9903bc59cb5bade18a49f2c5f2c9363d0293ccb68fed8765d36e3",
        ),
        (
            CatalogEdition::Ifc4x3Add2,
            502,
            110,
            2_918,
            324,
            "b2f327638a844c8666d38dff90c5a48e12fdcec73da9efc4789e2dedd9239298",
        ),
    ] {
        let catalog = official_catalog(edition).unwrap();
        assert_eq!(catalog.manifest().edition, edition);
        assert_eq!(catalog.manifest().sha256, digest);
        assert_eq!(catalog.manifest().property_set_count, property_sets);
        assert_eq!(catalog.manifest().quantity_set_count, quantity_sets);
        let (actual_properties, actual_quantities) = catalog.iter().fold(
            (0, 0),
            |(properties, quantities), template| match &template.kind {
                SetTemplateKind::Property {
                    properties: items, ..
                } => (properties + property_count(items), quantities),
                SetTemplateKind::Quantity {
                    quantities: items, ..
                } => (properties, quantities + items.len()),
                _ => unreachable!("catalog model version differs from this corpus gate"),
            },
        );
        assert_eq!(actual_properties, properties);
        assert_eq!(actual_quantities, quantities);
    }
}

#[test]
fn ifc4x3_provenance_and_published_set_type_anomaly_are_explicit() {
    let catalog = official_catalog(CatalogEdition::Ifc4x3Add2).unwrap();
    assert_eq!(
        catalog.manifest().source_url,
        "https://github.com/buildingSMART/IFC4.x-development/tree/524daac53ca682e0649d240ace87f4cd7baff6e7/reference_schemas"
    );
    let set = catalog.get("Pset_Risk").unwrap();
    let ifc4 = official_catalog(CatalogEdition::Ifc4Add2Tc1).unwrap();
    assert_eq!(
        ifc4.get("Pset_Risk").unwrap().guid.as_deref(),
        Some("ff20d400d20011e1800000215ad4efdf")
    );
    assert_eq!(
        set.guid.as_deref(),
        Some("91e0ddf4d6ba45e3b689a10510bef4bc")
    );
    assert_ne!(
        set.guid.as_deref(),
        ifc4.get("Pset_Risk").unwrap().guid.as_deref()
    );

    let set = catalog.get("Pset_MarineVehicleCommon").unwrap();
    assert!(matches!(
        &set.kind,
        SetTemplateKind::Property {
            set_type: PropertySetType::TypeDrivenOverride,
            ..
        }
    ));
}

#[test]
fn embedded_ifc4_catalog_matches_source_manifest_and_typed_counts() {
    let catalog = official_catalog(CatalogEdition::Ifc4Add2Tc1).unwrap();
    assert_eq!(catalog.len(), 513);
    assert!(catalog.iter().all(
        |template| template.source.as_ref().is_some_and(
            |source| source.sha256.len() == 64 && source.relative_path.ends_with(".xml")
        )
    ));
    assert_eq!(catalog.manifest().property_set_count, 420);
    assert_eq!(catalog.manifest().quantity_set_count, 93);
    assert_eq!(
        catalog.manifest().sha256,
        "57227d4c82f9903bc59cb5bade18a49f2c5f2c9363d0293ccb68fed8765d36e3"
    );
    let mut properties = 0;
    let mut quantities = 0;
    for template in catalog.iter() {
        match &template.kind {
            SetTemplateKind::Property {
                properties: items, ..
            } => properties += property_count(items),
            SetTemplateKind::Quantity {
                quantities: items, ..
            } => quantities += items.len(),
            _ => unreachable!(),
        }
    }
    assert_eq!(properties, 2_550);
    assert_eq!(quantities, 257);
}

#[test]
fn embedded_catalog_preserves_published_alias_and_enumeration_structure() {
    let catalog = official_catalog(CatalogEdition::Ifc4Add2Tc1).unwrap();
    let set_definition_aliases: usize = catalog
        .iter()
        .map(|template| template.definition_aliases.len())
        .sum();
    let mut counts = GrammarCounts::default();
    for template in catalog.iter() {
        match &template.kind {
            SetTemplateKind::Property { properties, .. } => grammar_counts(properties, &mut counts),
            SetTemplateKind::Quantity {
                set_type,
                quantities,
                ..
            } => {
                counts.unspecified_quantity_sets +=
                    usize::from(*set_type == QuantitySetType::Unspecified);
                counts.quantity_name_aliases += quantities
                    .iter()
                    .map(|quantity| quantity.name_aliases.len())
                    .sum::<usize>();
                counts.quantity_definition_aliases += quantities
                    .iter()
                    .map(|quantity| quantity.definition_aliases.len())
                    .sum::<usize>();
            }
            _ => unreachable!("test and catalog model versions differ"),
        }
    }
    assert_eq!(set_definition_aliases, 896);
    assert_eq!(counts.enum_values, 2_380);
    assert_eq!(counts.constants, 2_638);
    assert_eq!(counts.constant_name_aliases, 2_614);
    assert_eq!(counts.constant_definition_aliases, 2_614);
    assert_eq!(counts.blank_table_expressions, 88);
    assert_eq!(counts.property_name_aliases, 5_802);
    assert_eq!(counts.property_definition_aliases, 5_802);
    assert_eq!(counts.quantity_name_aliases, 463);
    assert_eq!(counts.quantity_definition_aliases, 463);
    assert_eq!(counts.unspecified_quantity_sets, 93);
}

#[derive(Default)]
struct GrammarCounts {
    enum_values: usize,
    constants: usize,
    constant_name_aliases: usize,
    constant_definition_aliases: usize,
    blank_table_expressions: usize,
    property_name_aliases: usize,
    property_definition_aliases: usize,
    quantity_name_aliases: usize,
    quantity_definition_aliases: usize,
    unspecified_quantity_sets: usize,
}

fn grammar_counts(
    properties: &[ifc_template_catalog::definition::PropertyTemplate],
    counts: &mut GrammarCounts,
) {
    for property in properties {
        counts.property_name_aliases += property.name_aliases.len();
        counts.property_definition_aliases += property.definition_aliases.len();
        match &property.kind {
            PropertyKind::EnumeratedValue {
                values, constants, ..
            } => {
                counts.enum_values += values.len();
                counts.constants += constants.len();
                counts.constant_name_aliases += constants
                    .iter()
                    .map(|constant| constant.name_aliases.len())
                    .sum::<usize>();
                counts.constant_definition_aliases += constants
                    .iter()
                    .map(|constant| constant.definition_aliases.len())
                    .sum::<usize>();
            }
            PropertyKind::TableValue { expression, .. } if expression.as_deref() == Some("") => {
                counts.blank_table_expressions += 1;
            }
            PropertyKind::Complex { properties, .. } => grammar_counts(properties, counts),
            _ => {}
        }
    }
}

fn property_count(properties: &[ifc_template_catalog::definition::PropertyTemplate]) -> usize {
    properties
        .iter()
        .map(|property| {
            1 + match &property.kind {
                PropertyKind::Complex { properties, .. } => property_count(properties),
                _ => 0,
            }
        })
        .sum()
}
