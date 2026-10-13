//! Entities by `GlobalId`, new `GlobalId`s and the type, opening and
//! filling queries every host binds (#463), over the IFC2X3, IFC4 and
//! IFC4X3 relation fixtures. The hosts' own suites only check their
//! conversion of these answers.

use openbim_ifc_binding_core::header::Header;
use openbim_ifc_binding_core::{name_based_guid, new_guid, DuplicateGlobalId, IfcModel};

fn fixture(name: &str) -> IfcModel {
    let path = format!(
        "{}/../../test/fixtures/synthetic-relations/{name}",
        env!("CARGO_MANIFEST_DIR")
    );
    IfcModel::open(std::path::Path::new(&path)).expect("fixture reads")
}

/// The fixtures this build bundles a release for.
fn fixtures() -> Vec<IfcModel> {
    let mut out = Vec::new();
    if cfg!(feature = "ifc2x3") {
        out.push(fixture("relations_ifc2x3.ifc"));
    }
    if cfg!(feature = "ifc4") {
        out.push(fixture("relations_ifc4.ifc"));
    }
    if cfg!(feature = "ifc4x3") {
        out.push(fixture("relations_ifc4x3.ifc"));
    }
    assert!(!out.is_empty(), "every build bundles a release");
    out
}

fn global_id(model: &IfcModel, id: u64) -> String {
    match model.attribute(id, 0).expect("entity") {
        openbim_ifc_binding_core::value::Tagged::Text(text) => text,
        other => panic!("#{id} GlobalId is {other:?}"),
    }
}

#[test]
fn every_entity_is_found_by_its_global_id() {
    for model in fixtures() {
        for id in [11, 17, 21, 28, 36] {
            let guid = global_id(&model, id);
            assert_eq!(model.by_guid(&guid), Ok(Some(id)), "{guid}");
        }
        assert_eq!(model.by_guid("3MtNd0Uaz8Ix2OPtRq_3Kk"), Ok(None));
        assert_eq!(model.duplicate_global_ids(), Ok(vec![]));
    }
}

#[test]
fn a_malformed_global_id_is_an_invalid_value() {
    let model = fixtures().remove(0);
    for text in [
        "",
        "too short",
        "4000000000000000000000",
        "0123456789ABCDEFGHIJ+/",
    ] {
        assert_eq!(
            model.by_guid(text).unwrap_err().code(),
            "invalid-value",
            "{text}"
        );
    }
}

/// The index follows the model: a copy made after the first lookup is
/// seen, refused by both holders' ids, and listed.
#[test]
fn a_duplicate_made_after_a_lookup_is_reported() {
    for mut model in fixtures() {
        let guid = global_id(&model, 17);
        assert_eq!(model.by_guid(&guid), Ok(Some(17)));
        let attributes = model.attributes(17).unwrap();
        let type_name = model.type_of(17).unwrap().to_owned();
        let copy = model.add(&type_name, attributes).unwrap();
        let error = model.by_guid(&guid).unwrap_err();
        assert_eq!(error.code(), "invalid-model");
        assert!(
            error.to_string().contains(&format!("#17, #{copy}")),
            "{error}"
        );
        assert_eq!(
            model.duplicate_global_ids(),
            Ok(vec![DuplicateGlobalId {
                global_id: guid.clone(),
                ids: vec![17, copy],
            }])
        );
        model.remove(copy).unwrap();
        assert_eq!(model.by_guid(&guid), Ok(Some(17)));
    }
}

#[test]
fn types_openings_and_fillings_follow_the_relationships() {
    for model in fixtures() {
        assert_eq!(model.type_object(17), Ok(Some(16)));
        assert_eq!(model.type_object(19), Ok(None));
        assert_eq!(model.type_object(28), Ok(Some(27)));
        assert_eq!(model.occurrences_of(16), Ok(vec![17, 18]));
        assert_eq!(model.openings_of(17), Ok(vec![21, 23]));
        assert_eq!(model.fillings_of(21), Ok(vec![28]));
        assert_eq!(model.fillings_of(23), Ok(vec![]));
        assert_eq!(model.voided_element_of(22), Ok(Some(18)));
    }
}

#[test]
fn a_relation_query_refuses_with_the_shared_codes() {
    let model = fixtures().remove(0);
    assert_eq!(model.type_object(999).unwrap_err().code(), "missing-entity");
    assert_eq!(
        model.type_object(8).unwrap_err().code(),
        "wrong-entity-type"
    );
    assert_eq!(
        model.occurrences_of(17).unwrap_err().code(),
        "wrong-entity-type"
    );
    assert_eq!(
        model.fillings_of(17).unwrap_err().code(),
        "wrong-entity-type"
    );
}

#[test]
fn a_header_naming_no_bundled_release_is_refused() {
    let mut model = IfcModel::empty();
    model.set_header(Header {
        schema: vec!["IFC9".to_owned()],
        ..Header::default()
    });
    assert_eq!(
        model.by_guid("3MtNd0Uaz8Ix2OPtRq_3Kk").unwrap_err().code(),
        "unsupported-schema"
    );
    assert_eq!(
        model.type_object(1).unwrap_err().code(),
        "unsupported-schema"
    );
}

#[test]
fn new_guids_are_valid_and_unique_and_name_based_ones_reproducible() {
    let mut seen = std::collections::HashSet::new();
    for _ in 0..1000 {
        let guid = new_guid().expect("native targets have a random source");
        assert_eq!(guid.len(), 22);
        assert!(seen.insert(guid));
    }
    let a = name_based_guid(&["my-tool", "wall", "1"]);
    assert_eq!(
        a,
        name_based_guid(&["my-tool".to_owned(), "wall".into(), "1".into()])
    );
    assert_ne!(a, name_based_guid(&["my-tool", "wall", "2"]));
    // The facade's pinned value: every host derives the same GlobalId.
    assert_eq!(
        name_based_guid(&["openbim-ifc", "example"]),
        "347eFU5Y69bPY9V9tumQuV"
    );
}
