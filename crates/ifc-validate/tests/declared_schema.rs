//! A declared release known by name but published without an EXPRESS
//! schema (#467).

use ifc_model::{Codec, Model};
use ifc_step::StepCodec;
use ifc_validate::validate;

fn load(name: &str) -> Model {
    let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../test/fixtures/ifclite-geometry")
        .join(name);
    StepCodec
        .read_path(&path)
        .unwrap_or_else(|error| panic!("{name} should parse: {error}"))
}

/// IFC4X3_TC1 and IFC4X3_ADD1 are named releases with no published
/// EXPRESS schema (#467): `validate_declared` refuses them as such, never
/// as unknown and never by reading them as ADD2. A caller that chooses the
/// ADD2 tables itself gets a report carrying `header.schema.unpublished`.
#[test]
fn an_unpublished_ifc4x3_release_is_refused_by_name_not_aliased() {
    use ifc_validate::ValidateError;

    let mut model = load("bath_csg_solid.ifc");
    for token in ["IFC4X3_TC1", "IFC4X3_ADD1"] {
        model.header_mut().schema = vec![token.to_string()];
        assert_eq!(
            ifc_validate::validate_declared(&model),
            Err(ValidateError::UnpublishedSchema(token.to_string()))
        );
        let chosen = validate(
            &model,
            ifc_schema::for_version(ifc_schema::SchemaVersion::Ifc4x3).expect("bundled"),
        );
        let marked: Vec<_> = chosen
            .findings()
            .iter()
            .filter(|finding| finding.rule == "header.schema.unpublished")
            .collect();
        assert_eq!(marked.len(), 1, "{token}: {:#?}", chosen.findings());
        assert!(marked[0].message.contains(token), "{}", marked[0].message);
        assert_eq!(
            marked[0].detail,
            Some(ifc_validate::FindingDetail::UnpublishedSchema(
                ifc_schema::UnpublishedRelease::from_header_token(token).expect("named")
            ))
        );
    }
    model.header_mut().schema = vec!["IFC9_DRAFT".to_string()];
    assert_eq!(
        ifc_validate::validate_declared(&model),
        Err(ValidateError::UnknownSchema("IFC9_DRAFT".to_string()))
    );
}
