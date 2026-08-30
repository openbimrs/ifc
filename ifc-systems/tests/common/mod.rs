//! Shared fixtures for the ifc-systems test binaries.
//!
//! Included with `mod common;` rather than a helper crate: integration tests
//! are separate binaries, so this is the standard way to share setup without
//! publishing test-only helpers from the library.

use ifc_model::{Codec, Entity, EntityId, Model, Value};

/// Build a model stating one distribution system with two members.
/// The committed fixture: a real STEP file, not a synthetic model.
pub fn fixture() -> Model {
    let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../test/fixtures/synthetic-systems/synthetic_systems.ifc");
    ifc_step::StepCodec
        .read_path(&path)
        .expect("fixture parses")
}

#[allow(dead_code)] // used by systems.rs; each test binary compiles this separately
pub fn model_with_system() -> Model {
    let mut model = Model::new();
    let seg = EntityId(1);
    model.insert(seg, Entity::new("IfcFlowSegment", vec![]));
    let fitting = EntityId(2);
    model.insert(fitting, Entity::new("IfcFlowFitting", vec![]));
    let system = EntityId(3);
    model.insert(
        system,
        Entity::new(
            "IfcDistributionSystem",
            vec![
                Value::Text("guid".into()),
                Value::Null,
                Value::Text("Heating".into()),
                Value::Null,
                Value::Null,
                Value::Null,
                Value::Enum("HEATING".into()),
            ],
        ),
    );
    model.insert(
        EntityId(4),
        Entity::new(
            "IfcRelAssignsToGroup",
            vec![
                Value::Text("relguid".into()),
                Value::Null,
                Value::Null,
                Value::Null,
                Value::List(vec![Value::Ref(seg), Value::Ref(fitting)]),
                Value::Null,
                Value::Ref(system),
            ],
        ),
    );
    model
}
