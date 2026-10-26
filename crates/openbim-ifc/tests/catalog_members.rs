//! The members of catalog sets against the release's catalog (#537).
//!
//! A `Pset_WallCommon` holding a property the catalog does not declare, or
//! a value of another type than the catalog declares, is a warning in every
//! release the catalog covers; the declared member with the declared type
//! is not. The templates are asserted from the embedded catalog first, so a
//! catalog change shows up here rather than as a silent pass.
#![cfg(all(
    feature = "step",
    feature = "schema",
    feature = "validate",
    feature = "property-catalog"
))]

use ifc::property_catalog::catalog::CatalogProfile;
use ifc::property_catalog::definition::{
    CatalogEdition, PropertyKind, QuantityKind, SetTemplateKind,
};
use ifc::property_catalog::embedded::load_catalog;
use ifc::validate::{validate, CatalogMismatch, FindingDetail, Path, Report, Severity};
use ifc::{Codec, EntityId, StepCodec};

const RELEASES: [(&str, CatalogEdition, CatalogProfile); 3] = [
    (
        "IFC2X3",
        CatalogEdition::Ifc2x3Tc1,
        CatalogProfile::Official,
    ),
    (
        "IFC4",
        CatalogEdition::Ifc4Add2Tc1,
        CatalogProfile::Corrected,
    ),
    (
        "IFC4X3_ADD2",
        CatalogEdition::Ifc4x3Add2,
        CatalogProfile::Corrected,
    ),
];

fn schema(token: &str) -> &'static ifc::schema::Schema {
    match token {
        "IFC2X3" => ifc::schema::ifc2x3(),
        "IFC4" => ifc::schema::ifc4(),
        "IFC4X1" => ifc::schema::ifc4x1(),
        _ => ifc::schema::ifc4x3(),
    }
}

/// `#2` is the property, `#3` the set `Pset_WallCommon` holding it.
fn wall_common(token: &str, name: &str, value: &str) -> ifc::Model {
    let text = format!(
        "ISO-10303-21;\nHEADER;\nFILE_DESCRIPTION((''),'2;1');\n\
         FILE_NAME('sets.ifc','',(''),(''),'','','');\nFILE_SCHEMA(('{token}'));\nENDSEC;\nDATA;\n\
         #2=IFCPROPERTYSINGLEVALUE('{name}',$,{value},$);\n\
         #3=IFCPROPERTYSET('0YvctVUKr0kugbFTf53O03',$,'Pset_WallCommon',$,(#2));\n\
         ENDSEC;\nEND-ISO-10303-21;\n"
    );
    StepCodec
        .read_bytes(text.as_bytes())
        .expect("the model parses")
}

fn members<'a>(report: &'a Report, rule: &str) -> Vec<&'a ifc::validate::Finding> {
    report
        .findings()
        .iter()
        .filter(|finding| finding.rule == rule)
        .collect()
}

#[test]
fn the_catalog_declares_is_external_as_a_boolean_in_every_release() {
    for (token, edition, profile) in RELEASES {
        let catalog = load_catalog(edition, profile).expect("embedded catalog");
        let template = catalog.get("Pset_WallCommon").expect("Pset_WallCommon");
        let SetTemplateKind::Property { properties, .. } = &template.kind else {
            panic!("{token}: Pset_WallCommon is not a property set");
        };
        let is_external = properties
            .iter()
            .find(|property| property.name == "IsExternal")
            .unwrap_or_else(|| panic!("{token}: no IsExternal"));
        assert!(
            matches!(&is_external.kind, PropertyKind::SingleValue { data_type }
                if data_type.type_name.as_deref() == Some("IfcBoolean")),
            "{token}: {:?}",
            is_external.kind
        );
    }
}

#[test]
fn a_member_of_the_declared_type_is_clean_in_every_catalogued_release() {
    for (token, ..) in RELEASES {
        let report = validate(
            &wall_common(token, "IsExternal", "IFCBOOLEAN(.T.)"),
            schema(token),
        );
        for rule in ["property.undeclared_member", "property.member_type"] {
            assert!(
                members(&report, rule).is_empty(),
                "{token}: {:?}",
                report.findings()
            );
        }
    }
}

#[test]
fn another_value_type_is_a_warning_in_every_catalogued_release() {
    for (token, ..) in RELEASES {
        let report = validate(
            &wall_common(token, "IsExternal", "IFCLABEL('yes')"),
            schema(token),
        );
        let found = members(&report, "property.member_type");
        assert_eq!(found.len(), 1, "{token}: {:?}", report.findings());
        assert_eq!(found[0].severity, Severity::Warning, "{token}");
        assert!(
            matches!(&found[0].path, Path::Attribute { entity, name, .. }
                if *entity == EntityId(2) && name.as_deref() == Some("NominalValue")),
            "{token}: {}",
            found[0].path
        );
        assert!(
            matches!(&found[0].detail, Some(FindingDetail::CatalogMember(detail))
            if detail.set == "Pset_WallCommon"
                && detail.member.as_deref() == Some("IsExternal")
                && detail.mismatch == CatalogMismatch::DataType {
                    declared: "IfcBoolean".into(),
                    found: "IfcLabel".into(),
                }),
            "{token}: {:?}",
            found[0].detail
        );
    }
}

#[test]
fn a_property_the_catalog_does_not_declare_is_a_warning_in_every_catalogued_release() {
    for (token, ..) in RELEASES {
        let report = validate(
            &wall_common(token, "MyOwnProp", "IFCBOOLEAN(.T.)"),
            schema(token),
        );
        let found = members(&report, "property.undeclared_member");
        assert_eq!(found.len(), 1, "{token}: {:?}", report.findings());
        assert_eq!(found[0].severity, Severity::Warning, "{token}");
        assert!(found[0].message.contains("'MyOwnProp'"), "{token}");
        assert!(
            members(&report, "property.member_type").is_empty(),
            "{token}"
        );
    }
}

/// `#2` is a quantity named `Length`, in the set `Qto_WallBaseQuantities`.
fn wall_quantities(token: &str, quantity: &str) -> ifc::Model {
    let text = format!(
        "ISO-10303-21;\nHEADER;\nFILE_DESCRIPTION((''),'2;1');\n\
         FILE_NAME('sets.ifc','',(''),(''),'','','');\nFILE_SCHEMA(('{token}'));\nENDSEC;\nDATA;\n\
         #2={quantity}('Length',$,$,1.,$);\n\
         #3=IFCELEMENTQUANTITY('0YvctVUKr0kugbFTf53O03',$,'Qto_WallBaseQuantities',$,$,(#2));\n\
         ENDSEC;\nEND-ISO-10303-21;\n"
    );
    StepCodec
        .read_bytes(text.as_bytes())
        .expect("the model parses")
}

#[test]
fn a_quantity_of_another_kind_is_a_warning() {
    for (token, edition, profile) in &RELEASES[1..] {
        let catalog = load_catalog(*edition, *profile).expect("embedded catalog");
        let template = catalog
            .get("Qto_WallBaseQuantities")
            .expect("Qto_WallBaseQuantities");
        let SetTemplateKind::Quantity { quantities, .. } = &template.kind else {
            panic!("{token}: not a quantity set");
        };
        let length = quantities
            .iter()
            .find(|quantity| quantity.name == "Length")
            .unwrap_or_else(|| panic!("{token}: no Length"));
        assert_eq!(length.kind, QuantityKind::Length, "{token}");

        let clean = validate(&wall_quantities(token, "IFCQUANTITYLENGTH"), schema(token));
        assert!(
            members(&clean, "property.member_type").is_empty(),
            "{token}: {:?}",
            clean.findings()
        );
        let report = validate(&wall_quantities(token, "IFCQUANTITYAREA"), schema(token));
        let found = members(&report, "property.member_type");
        assert_eq!(found.len(), 1, "{token}: {:?}", report.findings());
        assert!(
            matches!(&found[0].detail, Some(FindingDetail::CatalogMember(detail))
                if matches!(&detail.mismatch, CatalogMismatch::Form { declared, found }
                    if declared == "IfcQuantityLength" && found == "IfcQuantityArea")),
            "{token}: {:?}",
            found[0].detail
        );
    }
}

/// IFC4X1 has no catalog: nothing about its members is judged.
#[test]
fn a_release_without_a_catalog_judges_no_members() {
    let report = validate(
        &wall_common("IFC4X1", "MyOwnProp", "IFCLABEL('yes')"),
        ifc::schema::ifc4x1(),
    );
    assert!(members(&report, "property.undeclared_member").is_empty());
    assert!(members(&report, "property.member_type").is_empty());
}
