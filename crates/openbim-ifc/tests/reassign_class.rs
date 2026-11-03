//! Changing an object's class in place (#483): `AuthorOp::ReassignClass`.
//!
//! IFC4 scenes are built in memory by `apply_authoring`; the refusals that
//! need the release tables of all three also run over
//! `test/fixtures/synthetic-relations/cascade_*.ifc` (Wall A #20, Door 1 #100).

#![cfg(all(
    feature = "step",
    feature = "author",
    feature = "ifc2x3",
    feature = "ifc4",
    feature = "ifc4x3"
))]

use std::path::PathBuf;

use ifc::{
    apply_authoring, authoring_handle, stage_authoring, AuthorOp, AuthoringError, AuthoringFailure,
    AuthoringOutcome, Codec, EntityId, Model, ReassignClassOptions, Schema, SchemaVersion,
    StepCodec, Value,
};

fn ifc4() -> &'static Schema {
    ifc::schema::ifc4()
}

fn text(value: &str) -> Value {
    Value::Text(value.into())
}

fn enumeration(value: &str) -> Value {
    Value::Enum(value.into())
}

fn create(type_name: &str, attributes: Vec<(&str, Value)>) -> AuthorOp {
    AuthorOp::Create {
        type_name: type_name.to_owned(),
        attributes: attributes
            .into_iter()
            .map(|(name, value)| (name.to_owned(), value))
            .collect(),
    }
}

fn reassign(entity: EntityId, type_name: &str, attributes: Vec<(&str, Value)>) -> AuthorOp {
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

fn product(
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
struct Scene {
    /// A column typed by `column_type`, contained in the storey.
    typed_column: EntityId,
    column_type: EntityId,
    /// A column with no type and no container.
    column: EntityId,
    /// A wall contained in the storey.
    contained_wall: EntityId,
    /// A wall voided by an opening.
    voided_wall: EntityId,
    /// A wall with no relationship.
    wall: EntityId,
    /// A door with `OperationType` set.
    door: EntityId,
}

fn scene() -> (Model, Scene) {
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

fn apply(
    model: &mut Model,
    schema: &Schema,
    ops: &[AuthorOp],
) -> Result<AuthoringOutcome, AuthoringError> {
    apply_authoring(model, schema, ops, 11)
}

fn refusal(model: &mut Model, schema: &Schema, op: AuthorOp) -> AuthoringFailure {
    let (bytes, revision) = (StepCodec.write_bytes(model).unwrap(), model.revision());
    let error = apply(model, schema, &[op]).expect_err("refused");
    assert_eq!(error.op, Some(0));
    assert_eq!(StepCodec.write_bytes(model).unwrap(), bytes, "bytes");
    assert_eq!(model.revision(), revision, "revision");
    *error.failure
}

fn class(model: &Model, id: EntityId) -> String {
    model.get(id).expect("exists").type_name.to_string()
}

#[test]
fn a_column_becomes_a_standard_case_and_back_in_ifc4() {
    let (mut model, scene) = scene();
    let before = model.get(scene.typed_column).unwrap().attributes.clone();
    let revision = model.revision();

    let outcome = apply(
        &mut model,
        ifc4(),
        &[reassign(
            scene.typed_column,
            "IfcColumnStandardCase",
            vec![],
        )],
    )
    .expect("reassigns");
    assert_eq!(outcome.ids, vec![Some(scene.typed_column)]);
    let report = outcome.reassignments[0].as_ref().expect("a report");
    assert_eq!(report.entity, scene.typed_column);
    assert_eq!(
        (report.from.as_str(), report.to.as_str()),
        ("IfcColumn", "IfcColumnStandardCase")
    );
    assert!(report.dropped.is_empty());
    assert!(outcome.revision > revision);
    assert_eq!(class(&model, scene.typed_column), "IFCCOLUMNSTANDARDCASE");
    assert_eq!(model.get(scene.typed_column).unwrap().attributes, before);
    assert_eq!(
        model.ids_of_type("IFCCOLUMNSTANDARDCASE"),
        [scene.typed_column]
    );
    assert_eq!(model.ids_of_type("IFCCOLUMN"), [scene.column]);
    let typed_by = model
        .ids_of_type("IFCRELDEFINESBYTYPE")
        .iter()
        .filter_map(|id| model.get(*id))
        .any(|rel| rel.attributes.contains(&Value::Ref(scene.column_type)));
    assert!(typed_by, "the type relationship is intact");

    apply(
        &mut model,
        ifc4(),
        &[reassign(scene.typed_column, "IfcColumn", vec![])],
    )
    .expect("back");
    assert_eq!(class(&model, scene.typed_column), "IFCCOLUMN");
    assert_eq!(model.get(scene.typed_column).unwrap().attributes, before);
    assert!(model.ids_of_type("IFCCOLUMNSTANDARDCASE").is_empty());
}

#[test]
fn a_created_entity_is_reassigned_in_its_batch() {
    let (model, _) = scene();
    let staged = stage_authoring(
        &model,
        ifc4(),
        &[
            product("IfcColumn", None, vec![]),
            reassign(authoring_handle(0), "IfcColumnStandardCase", vec![]),
        ],
        3,
    )
    .expect("stages");
    assert_eq!(staged.ids[0], staged.ids[1]);
    let edits = staged.transaction.edits();
    let creates: Vec<_> = edits
        .iter()
        .filter_map(|edit| match edit {
            ifc_model::Edit::Create { entity, .. } => Some(entity.type_name.to_string()),
            _ => None,
        })
        .collect();
    assert_eq!(creates, ["IFCCOLUMNSTANDARDCASE"]);
    assert!(edits
        .iter()
        .all(|edit| !matches!(edit, ifc_model::Edit::Retype { .. })));
}

#[test]
fn a_reassignment_composes_with_edits() {
    let (model, scene) = scene();
    let rename = |name: &str| AuthorOp::Edit {
        entity: scene.typed_column,
        attributes: vec![("Name".into(), text(name))],
    };
    let swap = || reassign(scene.typed_column, "IfcColumnStandardCase", vec![]);
    for ops in [[swap(), rename("A")], [rename("A"), swap()]] {
        let mut copy = model.clone();
        apply(&mut copy, ifc4(), &ops).expect("applies");
        let column = copy.get(scene.typed_column).unwrap();
        assert_eq!(&*column.type_name, "IFCCOLUMNSTANDARDCASE");
        assert_eq!(column.attributes[2], text("A"));
    }
}

#[test]
fn the_predefined_type_must_fit_the_new_class() {
    let (mut model, scene) = scene();
    // A column with PredefinedType COLUMN is not a beam (and is untyped, so
    // no type rule is in play).
    apply(
        &mut model,
        ifc4(),
        &[AuthorOp::Edit {
            entity: scene.column,
            attributes: vec![("PredefinedType".into(), enumeration("COLUMN"))],
        }],
    )
    .unwrap();
    let failure = refusal(
        &mut model,
        ifc4(),
        reassign(scene.column, "IfcBeam", vec![]),
    );
    assert!(
        matches!(
            failure,
            AuthoringFailure::Author(ifc::author::AuthorError::NotEnumerationMember { .. })
        ),
        "{failure:?}"
    );
    apply(
        &mut model,
        ifc4(),
        &[reassign(
            scene.column,
            "IfcBeam",
            vec![("PredefinedType", enumeration("BEAM"))],
        )],
    )
    .expect("accepted with the override");
    assert_eq!(class(&model, scene.column), "IFCBEAM");
}

#[test]
fn unmapped_attributes_are_refused_unless_dropped() {
    let (mut model, scene) = scene();
    let failure = refusal(
        &mut model,
        ifc4(),
        reassign(scene.door, "IfcWindow", vec![]),
    );
    assert_eq!(
        failure,
        AuthoringFailure::AttributesDropped {
            entity: scene.door,
            from: "IfcDoor".into(),
            to: "IfcWindow".into(),
            attributes: vec!["OperationType".into()],
        }
    );
    let outcome = apply(
        &mut model,
        ifc4(),
        &[AuthorOp::ReassignClass {
            entity: scene.door,
            type_name: "IfcWindow".into(),
            attributes: vec![("PredefinedType".into(), enumeration("WINDOW"))],
            options: ReassignClassOptions::new().drop_unmapped(true),
        }],
    )
    .expect("accepted");
    let report = outcome.reassignments[0].as_ref().unwrap();
    assert_eq!(report.dropped, ["OperationType"]);
    assert_eq!(class(&model, scene.door), "IFCWINDOW");
}

#[test]
fn a_referrer_must_accept_the_new_class() {
    let (mut model, scene) = scene();
    let failure = refusal(
        &mut model,
        ifc4(),
        reassign(scene.voided_wall, "IfcSpace", vec![]),
    );
    assert!(
        matches!(
            &failure,
            AuthoringFailure::WrongReferenceType { entity, attribute, .. }
                if entity == "IFCRELVOIDSELEMENT" && attribute == "RelatingBuildingElement"
        ),
        "{failure:?}"
    );
}

#[test]
fn a_relationship_the_new_class_cannot_hold_is_refused() {
    let (mut model, scene) = scene();
    let failure = refusal(
        &mut model,
        ifc4(),
        reassign(scene.contained_wall, "IfcSpace", vec![]),
    );
    assert!(
        matches!(
            &failure,
            AuthoringFailure::RelationshipLost { entity, inverse, to, .. }
                if *entity == scene.contained_wall && inverse == "ContainedInStructure" && to == "IfcSpace"
        ),
        "{failure:?}"
    );
}

#[test]
fn an_inverse_the_new_class_requires_must_be_filled() {
    let (mut model, scene) = scene();
    let failure = refusal(
        &mut model,
        ifc4(),
        reassign(scene.wall, "IfcOpeningElement", vec![]),
    );
    assert_eq!(
        failure,
        AuthoringFailure::InverseCardinality {
            entity: scene.wall,
            to: "IfcOpeningElement".into(),
            inverse: "VoidsElements".into(),
            found: 0,
            lower: 1,
            upper: Some(1),
        }
    );
}

fn read(name: &str) -> Model {
    let path = format!(
        "{}/../../test/fixtures/synthetic-relations/{name}",
        env!("CARGO_MANIFEST_DIR")
    );
    StepCodec
        .read_path(std::path::Path::new(&path))
        .unwrap_or_else(|error| panic!("{name}: {error}"))
}

#[test]
fn refusals_leave_the_model_unchanged() {
    let (mut model, scene) = scene();
    let failure = refusal(
        &mut model,
        ifc4(),
        reassign(scene.wall, "IfcBuildingElement", vec![]),
    );
    assert_eq!(
        failure,
        AuthoringFailure::AbstractEntity {
            type_name: "IfcBuildingElement".into()
        }
    );
    let failure = refusal(
        &mut model,
        ifc4(),
        reassign(scene.wall, "IfcWallType", vec![]),
    );
    assert!(
        matches!(failure, AuthoringFailure::NotA { .. }),
        "{failure:?}"
    );
    let failure = refusal(
        &mut model,
        ifc4(),
        reassign(EntityId(9999), "IfcWall", vec![]),
    );
    assert_eq!(failure, AuthoringFailure::MissingEntity(EntityId(9999)));

    let mut model = read("cascade_ifc4x3.ifc");
    let schema = ifc::schema::for_version(SchemaVersion::Ifc4x3).unwrap();
    let failure = refusal(
        &mut model,
        schema,
        reassign(EntityId(20), "IfcColumnStandardCase", vec![]),
    );
    assert!(
        matches!(failure, AuthoringFailure::UnknownEntity { .. }),
        "{failure:?}"
    );

    for (name, version) in [
        ("cascade_ifc2x3.ifc", SchemaVersion::Ifc2x3),
        ("cascade_ifc4.ifc", SchemaVersion::Ifc4),
    ] {
        let mut model = read(name);
        let schema = ifc::schema::for_version(version).unwrap();
        let failure = refusal(
            &mut model,
            schema,
            reassign(EntityId(100), "IfcWall", vec![]),
        );
        assert!(
            matches!(
                &failure,
                AuthoringFailure::UnsupportedReassignment { detail, .. }
                    if detail.contains("a transaction cannot shorten a record")
            ),
            "{name}: {failure:?}"
        );
    }
}

fn spec_root() -> Option<PathBuf> {
    let crate_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    [
        "../../../../references/ifc-spec",
        "../../references/ifc-spec",
    ]
    .into_iter()
    .map(|rel| crate_dir.join(rel))
    .find(|path| path.is_dir())
}

/// The batch reads the type rules by their labels; a release that labels a
/// rule over `IsTypedBy` otherwise would pass unchecked.
#[test]
fn every_type_rule_is_labelled_as_the_batch_reads_it() {
    let Some(root) = spec_root() else {
        assert!(
            std::env::var_os("IFC_SPEC_REQUIRED").is_none(),
            "IFC_SPEC_REQUIRED is set but references/ifc-spec was not found; \
             run scripts/fetch-ifc-schemas.sh"
        );
        eprintln!("skipped: references/ifc-spec not present");
        return;
    };
    for (version, file, rules) in [
        (SchemaVersion::Ifc2x3, "ifc2x3-tc1/IFC2X3_TC1.exp", false),
        (SchemaVersion::Ifc4, "ifc4-add2-tc1/IFC4.exp", true),
        (SchemaVersion::Ifc4x3, "ifc4x3-add2/IFC4X3_ADD2.exp", true),
    ] {
        let bytes = std::fs::read(root.join(file)).expect("read the reference schema");
        let normative = Schema::from_express_bytes(&bytes);
        let mut found = 0;
        for entity in normative.entities() {
            for rule in &entity.where_rules {
                if rule.expression.contains("IsTypedBy") {
                    found += 1;
                    assert!(
                        ["CorrectTypeAssigned", "CorrectStyleAssigned"]
                            .contains(&rule.label.as_str()),
                        "{version:?} {}.{}",
                        entity.name,
                        rule.label
                    );
                }
            }
        }
        assert_eq!(
            found > 0,
            rules,
            "{version:?}: {found} rules over IsTypedBy"
        );
    }
}
