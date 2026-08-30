//! Fixture loading shared by the property test binaries.

use ifc_model::{Codec, EntityId, Model};

/// The synthetic property fixture, parsed.
pub fn fixture() -> Model {
    let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../test/fixtures/synthetic-properties/synthetic_properties.ifc");
    ifc_step::StepCodec
        .read_path(&path)
        .expect("fixture parses")
}

/// The wall carrying this `Name`, which is how the fixture identifies them.
///
/// Each test binary compiles this module separately, so a helper used by only
/// one of them is dead code in the other. That is a compilation artifact, not
/// an unused function.
#[allow(dead_code)]
pub fn wall_named(model: &Model, name: &str) -> EntityId {
    *model
        .ids_of_type("IFCWALL")
        .iter()
        .find(|id| {
            model
                .get(**id)
                .and_then(|e| e.attributes.get(2))
                .and_then(|v| v.unwrap_typed().as_text())
                .map(|n| n == name)
                .unwrap_or(false)
        })
        .expect("wall in fixture")
}
