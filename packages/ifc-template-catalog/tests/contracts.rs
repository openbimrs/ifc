use ifc_template_catalog::catalog::{Catalog, CatalogError, CatalogProfile};
use ifc_template_catalog::definition::{
    Applicability, CatalogEdition, PropertyDataType, PropertyKind, PropertySetType,
    PropertyTemplate, SetTemplate, SetTemplateKind, SourceManifest,
};

fn wall_common() -> SetTemplate {
    SetTemplate {
        name: "Pset_WallCommon".into(),
        guid: Some("fixture-guid".into()),
        definition: None,
        name_aliases: Vec::new(),
        definition_aliases: Vec::new(),
        source: None,
        raw_applicability: Some("IfcWall".into()),
        applicability: vec![Applicability::entity("IfcWall")],
        kind: SetTemplateKind::Property {
            set_type: PropertySetType::TypeDrivenOverride,
            properties: vec![PropertyTemplate {
                name: "IsExternal".into(),
                guid: None,
                definition: None,
                name_aliases: Vec::new(),
                definition_aliases: Vec::new(),
                kind: PropertyKind::SingleValue {
                    data_type: PropertyDataType::new("IfcBoolean"),
                },
            }],
        },
    }
}

fn manifest() -> SourceManifest {
    SourceManifest {
        edition: CatalogEdition::Ifc4Add2Tc1,
        source_label: "IFC4 ADD2 TC1".into(),
        source_url: "https://standards.buildingsmart.org/IFC/RELEASE/IFC4/ADD2_TC1/".into(),
        sha256: "0123456789abcdef".into(),
        property_set_count: 1,
        quantity_set_count: 0,
    }
}

#[test]
fn immutable_catalog_is_indexed_by_name() {
    let catalog =
        Catalog::try_new(manifest(), CatalogProfile::Official, vec![wall_common()]).unwrap();
    let set = catalog.get("Pset_WallCommon").unwrap();
    assert_eq!(set.name, "Pset_WallCommon");
    assert_eq!(catalog.len(), 1);
    assert_eq!(catalog.profile(), CatalogProfile::Official);
    assert_eq!(catalog.manifest().edition, CatalogEdition::Ifc4Add2Tc1);
}

#[test]
fn duplicate_set_names_are_rejected() {
    let error = Catalog::try_new(
        manifest(),
        CatalogProfile::Official,
        vec![wall_common(), wall_common()],
    )
    .unwrap_err();
    assert_eq!(
        error,
        CatalogError::DuplicateTemplate("Pset_WallCommon".into())
    );
}

#[test]
fn corrected_profile_requires_an_applied_patch_ledger() {
    let error =
        Catalog::try_new(manifest(), CatalogProfile::Corrected, vec![wall_common()]).unwrap_err();
    assert_eq!(error, CatalogError::CorrectedProfileRequiresPatches);
}

#[test]
fn applicability_keeps_raw_and_normalized_forms() {
    let applicability = Applicability::parse("IfcWall/USERDEFINED").unwrap();
    assert_eq!(applicability.raw, "IfcWall/USERDEFINED");
    assert_eq!(applicability.entity, "IfcWall");
    assert_eq!(
        applicability.predefined_type.as_deref(),
        Some("USERDEFINED")
    );
}
