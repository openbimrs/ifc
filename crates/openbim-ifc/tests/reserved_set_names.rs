//! `Pset_`/`Qto_` names the release's catalog does not define (#511).
//!
//! The issue's reproduction: a custom property set named with the reserved
//! `Pset_` prefix passed `ifc::validate::validate` silently, however it was
//! attached. With the catalog built in, it is a warning in every release
//! the catalog covers; a set the catalog defines is not. The check reads
//! set names, not their relationships, so the sets here stand alone.
#![cfg(all(
    feature = "step",
    feature = "schema",
    feature = "validate",
    any(feature = "property-catalog", feature = "property-catalog-runtime")
))]

use ifc::validate::{validate, FindingDetail, Path, Severity};
use ifc::{Codec, EntityId, StepCodec};

/// Two property sets, `#3` and `#5`, in a file declaring `schema`.
fn model(schema: &str, type_set: &str, occurrence_set: &str) -> ifc::Model {
    let text = format!(
        "ISO-10303-21;\nHEADER;\nFILE_DESCRIPTION((''),'2;1');\n\
         FILE_NAME('sets.ifc','',(''),(''),'','','');\nFILE_SCHEMA(('{schema}'));\nENDSEC;\nDATA;\n\
         #2=IFCPROPERTYSINGLEVALUE('Reference',$,IFCIDENTIFIER('A'),$);\n\
         #3=IFCPROPERTYSET('0YvctVUKr0kugbFTf53O03',$,'{type_set}',$,(#2));\n\
         #5=IFCPROPERTYSET('0YvctVUKr0kugbFTf53O05',$,'{occurrence_set}',$,(#2));\n\
         ENDSEC;\nEND-ISO-10303-21;\n"
    );
    StepCodec
        .read_bytes(text.as_bytes())
        .expect("the model parses")
}

fn reserved(report: &ifc::validate::Report) -> Vec<(Severity, Path, String)> {
    report
        .findings()
        .iter()
        .filter(|finding| finding.rule == "property.reserved_set_name")
        .map(|finding| {
            (
                finding.severity,
                finding.path.clone(),
                finding.message.clone(),
            )
        })
        .collect()
}

#[test]
fn a_reserved_name_the_catalog_lacks_is_a_warning_in_every_catalogued_release() {
    for (token, schema) in [
        ("IFC2X3", ifc::schema::ifc2x3()),
        ("IFC4", ifc::schema::ifc4()),
        ("IFC4X3_ADD2", ifc::schema::ifc4x3()),
    ] {
        let report = validate(&model(token, "Pset_FRS_BASt", "Pset_WallCommon"), schema);
        let found = reserved(&report);
        assert_eq!(found.len(), 1, "{token}: {found:?}");
        let (severity, path, message) = &found[0];
        assert_eq!(*severity, Severity::Warning, "{token}");
        assert!(
            matches!(path, Path::Attribute { entity, name, .. }
                if *entity == EntityId(3) && name.as_deref() == Some("Name")),
            "{token}: {path}"
        );
        assert!(message.contains("Pset_FRS_BASt"), "{token}: {message}");
        let detail = report
            .findings()
            .iter()
            .find(|finding| finding.rule == "property.reserved_set_name")
            .and_then(|finding| finding.detail.clone());
        assert!(
            matches!(detail, Some(FindingDetail::ReservedSetName(ref set)) if set.name == "Pset_FRS_BASt" && set.prefix == "Pset_"),
            "{token}: {detail:?}"
        );

        let clean = validate(&model(token, "Pset_WallCommon", "Pset_WallCommon"), schema);
        assert!(reserved(&clean).is_empty(), "{token}");
    }
}

/// IFC4X1 and IFC4X2 have no catalog: their reserved names are reported
/// once as not checked.
#[test]
fn a_release_without_a_catalog_reports_its_names_unchecked() {
    let report = validate(
        &model("IFC4X1", "Pset_FRS_BASt", "Pset_WallCommon"),
        ifc::schema::ifc4x1(),
    );
    let found = reserved(&report);
    assert_eq!(found.len(), 1, "{found:?}");
    assert_eq!(found[0].0, Severity::Unsupported);
    assert_eq!(found[0].1, Path::File);
}
