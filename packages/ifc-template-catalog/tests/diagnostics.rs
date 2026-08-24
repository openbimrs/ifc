use ifc_template_catalog::definition::CatalogEdition;
use ifc_template_catalog::diagnostic::DiagnosticCode;
use ifc_template_catalog::embedded::official_catalog;

#[test]
fn official_mistakes_are_preserved_and_reported_not_repaired() {
    let catalog = official_catalog(CatalogEdition::Ifc4Add2Tc1).unwrap();
    let diagnostics = catalog.diagnostics();
    let missing_types: Vec<_> = diagnostics
        .iter()
        .filter(|item| item.code == DiagnosticCode::MissingPropertyDataType)
        .collect();
    assert_eq!(missing_types.len(), 16);
    assert!(missing_types.iter().any(|item| {
        item.template == "Pset_CivilElementCommon" && item.member.as_deref() == Some("Reference")
    }));
}
