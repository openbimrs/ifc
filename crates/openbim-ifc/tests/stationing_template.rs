//! Facade composition: a referent's `Pset_Stationing`, authored by
//! `ifc-alignment` and read back by `ifc-properties`, checked against the
//! bundled IFC4X3 ADD2 template catalogue (#216).
//!
//! `ifc-alignment` writes all three members the ADD2 documentation lists
//! (6.6.4.10, Table 6.6.4.10.A). The official catalogue profile is the PSD
//! XML as published, which omits `HasIncreasingStation`; the corrected
//! profile adds it, so only the corrected profile checks the authored set
//! without findings.

#![cfg(all(
    feature = "alignment",
    feature = "properties",
    feature = "property-catalog",
    feature = "step"
))]

use ifc::alignment::{
    axis2_placement_linear, linear_placement, point_by_distance, referent, stationing,
};
use ifc::properties::exact_properties;
use ifc::property_catalog::catalog::Catalog;
use ifc::property_catalog::compliance::{
    validate, MemberForm, ObservedMember, ObservedSet, UnexpectedMemberPolicy, ValidationCode,
    ValidationPolicy, ValidationReport,
};
use ifc::property_catalog::definition::CatalogEdition;
use ifc::property_catalog::embedded::{corrected_catalog, official_catalog};
use ifc::{Codec, Entity, Model, StepCodec, Transaction, Value};

/// A written and re-read IFC4X3 model with one stationed referent; the
/// referent's `Pset_Stationing` as an observed set.
fn authored_stationing() -> ObservedSet {
    let mut model = Model::default();
    model.header_mut().schema = vec!["IFC4X3_ADD2".to_owned()];
    let mut tx = Transaction::new(&model);
    let curve = tx.create(Entity::new("IFCPOLYLINE", vec![Value::List(vec![])]));
    let point = point_by_distance(&mut tx, 125.0, (None, None, None), curve).expect("point");
    let axis = axis2_placement_linear(&mut tx, point, None, None).expect("axis");
    let placement = linear_placement(&mut tx, axis, None, None).expect("placement");
    let marker = referent(
        &mut tx,
        "0aBcDeFgHiJkLmNoPqRsTu",
        None,
        Some("STATION"),
        Some(placement),
    )
    .expect("referent");
    stationing(
        &mut tx,
        "1aBcDeFgHiJkLmNoPqRsTu",
        "2aBcDeFgHiJkLmNoPqRsTu",
        marker,
        1125.0,
        Some(1100.0),
        Some(true),
    )
    .expect("stationing");
    tx.commit(&mut model).expect("commit");
    let bytes = StepCodec.write_bytes(&model).expect("written");
    let model = StepCodec.read_bytes(&bytes).expect("read back");

    let mut observed = ObservedSet::new("Pset_Stationing");
    let entries = exact_properties(&model, marker).expect("stationing resolves");
    assert_eq!(entries.len(), 3, "all three members are authored");
    for entry in entries {
        assert_eq!(&*entry.property.property_set, "Pset_Stationing");
        let value_type = entry.property.value_type.expect("single values are typed");
        observed = observed.with_member(
            ObservedMember::property(&*entry.name, MemberForm::SingleValue)
                .with_data_type(&*value_type),
        );
    }
    observed
}

fn check(catalog: &Catalog, observed: &ObservedSet) -> ValidationReport {
    let template = catalog.get("Pset_Stationing").expect("Pset_Stationing");
    let mut policy = ValidationPolicy::default();
    policy.require_all_members = true;
    policy.unexpected_members = UnexpectedMemberPolicy::Error;
    validate(template, observed, policy)
}

#[test]
fn an_authored_stationing_set_checks_clean_against_the_corrected_catalogue() {
    let observed = authored_stationing();
    let corrected = corrected_catalog(CatalogEdition::Ifc4x3Add2).expect("corrected");
    let report = check(&corrected, &observed);
    assert!(report.issues.is_empty(), "{:?}", report.issues);
}

#[test]
fn the_official_catalogue_keeps_the_psd_xml_and_reports_the_member() {
    let observed = authored_stationing();
    let official = official_catalog(CatalogEdition::Ifc4x3Add2).expect("official");
    let report = check(&official, &observed);
    let codes: Vec<_> = report.issues.iter().map(|issue| issue.code).collect();
    assert_eq!(codes, [ValidationCode::UnexpectedMember]);
}
