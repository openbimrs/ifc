use ifc_template_catalog::definition::CatalogEdition;
use ifc_template_catalog::embedded::official_catalog;
use ifc_template_catalog::export::{write_applicability_tsv, TSV_HEADER};

#[test]
fn committed_ifc4_applicability_export_is_current() {
    let catalog = official_catalog(CatalogEdition::Ifc4Add2Tc1).unwrap();
    let mut generated = Vec::new();
    let summary = write_applicability_tsv(&catalog, &mut generated).unwrap();
    let committed = include_bytes!("../data/ifc4-add2-tc1.tsv");
    assert_eq!(generated, committed, "regenerate with export_ifc4_tsv");
    assert_eq!(summary.set_count, 513);
    assert_eq!(summary.property_set_count, 420);
    assert_eq!(summary.quantity_set_count, 93);
    assert_eq!(summary.row_count, 3_525);
    assert!(generated.starts_with(TSV_HEADER.as_bytes()));
}

#[test]
fn export_contains_relevant_ifc4_property_and_quantity_paths() {
    let catalog = official_catalog(CatalogEdition::Ifc4Add2Tc1).unwrap();
    let mut generated = Vec::new();
    write_applicability_tsv(&catalog, &mut generated).unwrap();
    let text = String::from_utf8(generated).unwrap();
    assert!(text.contains("Pset_DoorCommon\t"));
    assert!(text.contains("Pset_OpeningElementCommon\t"));
    assert!(text.contains("Qto_OpeningElementBaseQuantities\t"));
    assert!(text.contains("\tIfcDoor\t"));
    assert!(text.contains("\tProtectedOpening\t"));
    assert!(text.contains("\tWidth\tlength\tQ_LENGTH\t"));
}
