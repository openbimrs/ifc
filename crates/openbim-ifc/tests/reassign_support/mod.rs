//! Shared builders for the `reassign_class*` tests (#483, #634): an IFC4
//! scene made in memory, the synthetic relation fixtures and the refusal
//! check.
//!
//! Each test binary compiles this module separately, so a helper used by
//! only one of them is dead code in the other.
#![allow(dead_code)]

use ifc::{
    apply_authoring, authoring_handle, AuthorOp, AuthoringError, AuthoringFailure,
    AuthoringOutcome, Codec, EntityId, Model, ReassignClassOptions, Schema, StepCodec, Value,
};

pub fn ifc4() -> &'static Schema {
    ifc::schema::ifc4()
}

pub fn text(value: &str) -> Value {
    Value::Text(value.into())
}

pub fn enumeration(value: &str) -> Value {
    Value::Enum(value.into())
}

pub fn create(type_name: &str, attributes: Vec<(&str, Value)>) -> AuthorOp {
    AuthorOp::Create {
        type_name: type_name.to_owned(),
        attributes: attributes
            .into_iter()
            .map(|(name, value)| (name.to_owned(), value))
            .collect(),
    }
}

pub fn reassign(entity: EntityId, type_name: &str, attributes: Vec<(&str, Value)>) -> AuthorOp {
    AuthorOp::ReassignClass {
        entity,
        type_name: type_name.to_owned(),
        attributes: attributes
            .into_iter()
            .map(|(name, value)| (name.to_owned(), value))
            .collect(),
        options: ReassignClassOptions::new(),
    }
}

pub fn product(
    type_name: &str,
    container: Option<EntityId>,
    attributes: Vec<(&str, Value)>,
) -> AuthorOp {
    AuthorOp::Product {
        type_name: type_name.to_owned(),
        container,
        attributes: attributes
            .into_iter()
            .map(|(name, value)| (name.to_owned(), value))
            .collect(),
        placement: None,
        type_object: None,
        owner_history: None,
    }
}

/// The ids of the scene `scene` builds.
pub struct Scene {
    /// A column typed by `column_type`, contained in the storey.
    pub typed_column: EntityId,
    pub column_type: EntityId,
    /// A column with no type and no container.
    pub column: EntityId,
    /// A wall contained in the storey.
    pub contained_wall: EntityId,
    /// A wall voided by an opening.
    pub voided_wall: EntityId,
    /// A wall with no relationship.
    pub wall: EntityId,
    /// A door with `OperationType` set.
    pub door: EntityId,
}

pub fn scene() -> (Model, Scene) {
    let mut model = Model::new();
    let ops = [
        AuthorOp::Project {
            attributes: vec![("Name".into(), text("P"))],
            owner_history: None,
        },
        AuthorOp::Spatial {
            type_name: "IfcSite".into(),
            parent: authoring_handle(0),
            attributes: vec![],
            placement: None,
            owner_history: None,
        },
        AuthorOp::Spatial {
            type_name: "IfcBuilding".into(),
            parent: authoring_handle(1),
            attributes: vec![],
            placement: None,
            owner_history: None,
        },
        AuthorOp::Spatial {
            type_name: "IfcBuildingStorey".into(),
            parent: authoring_handle(2),
            attributes: vec![],
            placement: None,
            owner_history: None,
        },
        AuthorOp::TypeObject {
            type_name: "IfcColumnType".into(),
            attributes: vec![("PredefinedType".into(), enumeration("COLUMN"))],
            owner_history: None,
        },
        AuthorOp::Product {
            type_name: "IfcColumn".into(),
            container: Some(authoring_handle(3)),
            attributes: vec![
                ("Name".into(), text("C1")),
                ("PredefinedType".into(), enumeration("COLUMN")),
            ],
            placement: None,
            type_object: Some(authoring_handle(4)),
            owner_history: None,
        },
        product("IfcColumn", None, vec![("Name", text("C2"))]),
        product(
            "IfcWall",
            Some(authoring_handle(3)),
            vec![("Name", text("W1"))],
        ),
        product("IfcWall", None, vec![("Name", text("W2"))]),
        product("IfcOpeningElement", None, vec![]),
        create(
            "IfcRelVoidsElement",
            vec![
                ("RelatingBuildingElement", Value::Ref(authoring_handle(8))),
                ("RelatedOpeningElement", Value::Ref(authoring_handle(9))),
            ],
        ),
        product("IfcWall", None, vec![("Name", text("W3"))]),
        product(
            "IfcDoor",
            None,
            vec![("OperationType", enumeration("SINGLE_SWING_LEFT"))],
        ),
    ];
    let outcome = apply_authoring(&mut model, ifc4(), &ops, 7).expect("builds the scene");
    let id = |index: usize| outcome.ids[index].expect("produced");
    let scene = Scene {
        typed_column: id(5),
        column_type: id(4),
        column: id(6),
        contained_wall: id(7),
        voided_wall: id(8),
        wall: id(11),
        door: id(12),
    };
    (model, scene)
}

pub fn apply(
    model: &mut Model,
    schema: &Schema,
    ops: &[AuthorOp],
) -> Result<AuthoringOutcome, AuthoringError> {
    apply_authoring(model, schema, ops, 11)
}

pub fn refusal(model: &mut Model, schema: &Schema, op: AuthorOp) -> AuthoringFailure {
    let (bytes, revision) = (StepCodec.write_bytes(model).unwrap(), model.revision());
    let error = apply(model, schema, &[op]).expect_err("refused");
    assert_eq!(error.op, Some(0));
    assert_eq!(StepCodec.write_bytes(model).unwrap(), bytes, "bytes");
    assert_eq!(model.revision(), revision, "revision");
    *error.failure
}

pub fn class(model: &Model, id: EntityId) -> String {
    model.get(id).expect("exists").type_name.to_string()
}

pub fn read(name: &str) -> Model {
    let path = format!(
        "{}/../../test/fixtures/synthetic-relations/{name}",
        env!("CARGO_MANIFEST_DIR")
    );
    StepCodec
        .read_path(std::path::Path::new(&path))
        .unwrap_or_else(|error| panic!("{name}: {error}"))
}

pub fn bytes(model: &Model) -> Vec<u8> {
    StepCodec.write_bytes(model).expect("writes")
}
