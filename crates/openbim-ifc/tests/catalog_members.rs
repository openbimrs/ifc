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
use ifc::validate::{
    validate, CatalogMember, CatalogMemberForm, CatalogMismatch, FindingDetail, Path, Report,
    Severity,
};
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

/// IFC2X3's PSD writes `type="IfcBoolean "`; the declared type is the
/// trimmed name, so an `IfcLabel` there is a data-type warning (#537).
#[test]
fn a_declared_type_with_published_whitespace_still_matches() {
    let text = "ISO-10303-21;\nHEADER;\nFILE_DESCRIPTION((''),'2;1');\n\
         FILE_NAME('sets.ifc','',(''),(''),'','','');\nFILE_SCHEMA(('IFC2X3'));\nENDSEC;\nDATA;\n\
         #2=IFCPROPERTYSINGLEVALUE('IsCoated',$,IFCLABEL('yes'),$);\n\
         #3=IFCPROPERTYSET('0YvctVUKr0kugbFTf53O03',$,'Pset_DoorWindowGlazingType',$,(#2));\n\
         ENDSEC;\nEND-ISO-10303-21;\n";
    let model = StepCodec
        .read_bytes(text.as_bytes())
        .expect("the model parses");
    let report = validate(&model, ifc::schema::ifc2x3());
    let found = members(&report, "property.member_type");
    assert_eq!(found.len(), 1, "{:?}", report.findings());
    assert!(
        matches!(&found[0].detail, Some(FindingDetail::CatalogMember(detail))
        if detail.mismatch == CatalogMismatch::DataType {
            declared: "IfcBoolean".into(),
            found: "IfcLabel".into(),
        }),
        "{:?}",
        found[0].detail
    );
}

/// The neutral member the facade hands `ifc-validate` has the form and the
/// value types of the catalog's own template, for every set of every
/// edition. The expectation is spelled out here, independently of the
/// conversion, so a form mapped to a neighbour fails.
fn expect_form(
    context: &str,
    property: &ifc::property_catalog::definition::PropertyTemplate,
    member: &CatalogMember,
    seen: &mut std::collections::BTreeSet<&'static str>,
) {
    let declared = |data_type: &ifc::property_catalog::definition::PropertyDataType| {
        data_type.type_name.clone()
    };
    assert_eq!(member.name, property.name, "{context}");
    let entity = match (&property.kind, &member.form) {
        (
            PropertyKind::SingleValue { data_type },
            CatalogMemberForm::SingleValue { data_type: got },
        ) => {
            assert_eq!(got, &declared(data_type), "{context}");
            "IfcPropertySingleValue"
        }
        (
            PropertyKind::BoundedValue { data_type },
            CatalogMemberForm::BoundedValue { data_type: got },
        ) => {
            assert_eq!(got, &declared(data_type), "{context}");
            "IfcPropertyBoundedValue"
        }
        (
            PropertyKind::ListValue { data_type },
            CatalogMemberForm::ListValue { data_type: got },
        ) => {
            assert_eq!(got, &declared(data_type), "{context}");
            "IfcPropertyListValue"
        }
        (
            PropertyKind::EnumeratedValue { data_type, .. },
            CatalogMemberForm::EnumeratedValue { data_type: got },
        ) => {
            assert_eq!(got, &data_type.as_ref().and_then(declared), "{context}");
            "IfcPropertyEnumeratedValue"
        }
        (
            PropertyKind::TableValue {
                defining_type,
                defined_type,
                ..
            },
            CatalogMemberForm::TableValue {
                defining_type: got_defining,
                defined_type: got_defined,
            },
        ) => {
            assert_eq!(got_defining, &declared(defining_type), "{context}");
            assert_eq!(got_defined, &declared(defined_type), "{context}");
            "IfcPropertyTableValue"
        }
        (
            PropertyKind::ReferenceValue { reference_type },
            CatalogMemberForm::ReferenceValue {
                reference_type: got,
            },
        ) => {
            assert_eq!(got, reference_type, "{context}");
            "IfcPropertyReferenceValue"
        }
        (PropertyKind::Complex { properties, .. }, CatalogMemberForm::Complex { members }) => {
            assert_eq!(properties.len(), members.len(), "{context}");
            for (nested, got) in properties.iter().zip(members) {
                expect_form(context, nested, got, seen);
            }
            "IfcComplexProperty"
        }
        (kind, form) => panic!("{context}: {} became {form:?} ({kind:?})", property.name),
    };
    assert_eq!(member.form.entity_name(), entity, "{context}");
    seen.insert(entity);
}

#[test]
fn every_catalog_member_keeps_its_form_through_the_facade() {
    use ifc::property_catalog::definition::QuantityKind;
    use ifc::validate::{CatalogQuantityKind, CatalogSetKind, PropertySetCatalog, ReleaseCatalog};

    let mut seen = std::collections::BTreeSet::new();
    let mut quantities = std::collections::BTreeSet::new();
    for (token, edition, profile) in RELEASES {
        let version = schema(token).version().expect("a bundled release");
        let catalog = load_catalog(edition, profile).expect("embedded catalog");
        for template in catalog.iter() {
            let context = format!("{token} {}", template.name);
            let set = ReleaseCatalog
                .template(version, &template.name)
                .unwrap_or_else(|| panic!("{context}: no template"));
            match &template.kind {
                SetTemplateKind::Property { properties, .. } => {
                    assert_eq!(set.kind, CatalogSetKind::Property, "{context}");
                    assert_eq!(set.members.len(), properties.len(), "{context}");
                    for (property, member) in properties.iter().zip(&set.members) {
                        expect_form(&context, property, member, &mut seen);
                    }
                }
                SetTemplateKind::Quantity { quantities: q, .. } => {
                    assert_eq!(set.kind, CatalogSetKind::Quantity, "{context}");
                    assert_eq!(set.members.len(), q.len(), "{context}");
                    for (quantity, member) in q.iter().zip(&set.members) {
                        let expected = match quantity.kind {
                            QuantityKind::Length => CatalogQuantityKind::Length,
                            QuantityKind::Area => CatalogQuantityKind::Area,
                            QuantityKind::Volume => CatalogQuantityKind::Volume,
                            QuantityKind::Weight => CatalogQuantityKind::Weight,
                            QuantityKind::Time => CatalogQuantityKind::Time,
                            QuantityKind::Count => CatalogQuantityKind::Count,
                            QuantityKind::Number => CatalogQuantityKind::Number,
                            _ => panic!("{context}: an unlisted quantity kind"),
                        };
                        assert_eq!(member.name, quantity.name, "{context}");
                        assert_eq!(
                            member.form,
                            CatalogMemberForm::Quantity(expected),
                            "{context}"
                        );
                        quantities.insert(expected.entity_name());
                    }
                }
                _ => panic!("{context}: an unlisted set kind"),
            }
        }
    }
    // The comparison is not vacuous: every form occurs in the catalogs.
    for entity in [
        "IfcPropertySingleValue",
        "IfcPropertyBoundedValue",
        "IfcPropertyListValue",
        "IfcPropertyEnumeratedValue",
        "IfcPropertyTableValue",
        "IfcPropertyReferenceValue",
        "IfcComplexProperty",
    ] {
        assert!(
            seen.contains(entity),
            "no {entity} in the catalogs: {seen:?}"
        );
    }
    assert!(quantities.len() >= 6, "{quantities:?}");
}
