#[path = "support/mod.rs"]
mod support;

use ifc_template_catalog::catalog::{Catalog, CatalogProfile};
use ifc_template_catalog::definition::{
    Applicability, PropertySetType, QuantitySetType, SetTemplateKind,
};
use ifc_template_catalog::query::{ApplicabilityContext, ApplicabilityTarget, EntityHierarchy};

use support::{manifest, property_set, quantity_set};

struct Hierarchy;

impl EntityHierarchy for Hierarchy {
    fn is_same_or_subtype(&self, candidate: &str, expected_supertype: &str) -> bool {
        candidate.eq_ignore_ascii_case(expected_supertype)
            || (candidate.eq_ignore_ascii_case("IfcWallStandardCase")
                && expected_supertype.eq_ignore_ascii_case("IfcWall"))
    }
}

#[test]
fn query_uses_injected_schema_hierarchy() {
    let mut set = property_set("Pset_WallCommon");
    set.applicability = vec![Applicability::entity("IfcWall")];
    let catalog = Catalog::try_new(manifest(1, 0), CatalogProfile::Official, vec![set]).unwrap();

    let matches = catalog.applicable_to(
        &ApplicabilityTarget::new("IfcWallStandardCase", None::<String>),
        &Hierarchy,
    );
    assert_eq!(matches.len(), 1);
    assert_eq!(matches[0].name, "Pset_WallCommon");
}

#[test]
fn predefined_type_restriction_is_enforced_case_insensitively() {
    let mut set = property_set("Pset_Predefined");
    set.applicability = vec![Applicability::parse("IfcWall/USERDEFINED").unwrap()];
    let catalog = Catalog::try_new(manifest(1, 0), CatalogProfile::Official, vec![set]).unwrap();

    assert_eq!(
        catalog
            .applicable_to(
                &ApplicabilityTarget::new("IfcWall", Some("userdefined")),
                &Hierarchy,
            )
            .len(),
        1
    );
    assert!(catalog
        .applicable_to(
            &ApplicabilityTarget::new("IfcWall", Some("STANDARD")),
            &Hierarchy,
        )
        .is_empty());
}

struct ReportingHierarchy;

impl EntityHierarchy for ReportingHierarchy {
    fn is_same_or_subtype(&self, _candidate: &str, _expected_supertype: &str) -> bool {
        false
    }

    fn entity_known(&self, entity: &str) -> Option<bool> {
        Some(!entity.eq_ignore_ascii_case("IfcFutureWall"))
    }
}

#[test]
fn detailed_query_reports_unknown_schema_entities() {
    let mut set = property_set("Pset_WallCommon");
    set.applicability = vec![Applicability::entity("IfcWall")];
    let catalog = Catalog::try_new(manifest(1, 0), CatalogProfile::Official, vec![set]).unwrap();

    let result = catalog.query_applicability(
        &ApplicabilityTarget::new("IfcFutureWall", None::<String>),
        &ReportingHierarchy,
    );
    assert!(result.matches.is_empty());
    assert_eq!(result.unresolved.len(), 1);
    assert_eq!(result.unresolved[0].selector.entity, "IfcWall");
}

#[test]
fn property_set_mode_filters_explicit_target_context() {
    let mut set = property_set("Pset_WallOccurrence");
    set.applicability = vec![Applicability::entity("IfcWall")];
    let SetTemplateKind::Property { set_type, .. } = &mut set.kind else {
        panic!()
    };
    *set_type = PropertySetType::OccurrenceDriven;
    let catalog = Catalog::try_new(manifest(1, 0), CatalogProfile::Official, vec![set]).unwrap();

    let type_target = ApplicabilityTarget::new("IfcWall", None::<String>)
        .with_context(ApplicabilityContext::Type);
    assert!(catalog.applicable_to(&type_target, &Hierarchy).is_empty());
    let occurrence_target = ApplicabilityTarget::new("IfcWall", None::<String>)
        .with_context(ApplicabilityContext::Occurrence);
    assert_eq!(
        catalog.applicable_to(&occurrence_target, &Hierarchy).len(),
        1
    );
}

#[test]
fn quantity_set_mode_filters_explicit_target_context() {
    let mut set = quantity_set("Qto_WallBaseQuantities", QuantitySetType::TypeDrivenOnly);
    set.applicability = vec![Applicability::entity("IfcWall")];
    let catalog = Catalog::try_new(manifest(0, 1), CatalogProfile::Official, vec![set]).unwrap();

    let occurrence = ApplicabilityTarget::new("IfcWall", None::<String>)
        .with_context(ApplicabilityContext::Occurrence);
    assert!(catalog.applicable_to(&occurrence, &Hierarchy).is_empty());
    let type_target = ApplicabilityTarget::new("IfcWall", None::<String>)
        .with_context(ApplicabilityContext::Type);
    assert_eq!(catalog.applicable_to(&type_target, &Hierarchy).len(), 1);
}
