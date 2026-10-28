//! The indexes behind a checked authoring batch change its cost, never its
//! answer (#587).
//!
//! A batch is staged against a model whose indexes the model keeps current,
//! and against a clone whose indexes were dropped and rebuild on demand.
//! Over a history of batches, undos and redos, and over the cascade
//! fixtures of three releases, both must give the same edits, ids,
//! reports and copies, or the same refusal; after every write the model's
//! reverse index must equal a fresh build; a refused batch leaves the
//! entities and the revision as they were. Where the model is ambiguous
//! (an object in two relationships), the refusal names the one the type
//! index lists first.

#![cfg(all(
    feature = "step",
    feature = "authoring",
    feature = "ifc2x3",
    feature = "ifc4",
    feature = "ifc4x3"
))]

use ifc::{
    apply_authoring, authoring_handle, name_based_guid, stage_authoring, AssignTypeOptions,
    AuthorOp, AuthoringFailure, Codec, CopyProductOptions, Entity, EntityId, History, Model,
    PurgeOptions, RelateOptions, RemoveProductOptions, ReverseIndex, Schema, SchemaVersion,
    StepCodec, Value,
};

fn text(s: &str) -> Value {
    Value::Text(s.into())
}

fn reals(v: &[f64]) -> Value {
    Value::List(v.iter().map(|x| Value::Real(*x)).collect())
}

fn create(type_name: &str, attributes: Vec<(&str, Value)>) -> AuthorOp {
    AuthorOp::Create {
        type_name: type_name.into(),
        attributes: attributes
            .into_iter()
            .map(|(name, value)| (name.to_owned(), value))
            .collect(),
    }
}

/// The ids a wall refers to.
#[derive(Debug, Clone, Copy)]
struct Scene {
    storey: EntityId,
    storey2: EntityId,
    placement: EntityId,
    body: EntityId,
}

/// A project down to two storeys, in IFC4X3.
fn scene_ops() -> Vec<AuthorOp> {
    let h = authoring_handle;
    let spatial = |type_name: &str, parent, placement| AuthorOp::Spatial {
        type_name: type_name.into(),
        parent,
        attributes: vec![],
        placement: Some(placement),
        owner_history: None,
    };
    let placement = |relative_to| AuthorOp::Placement {
        relative_to,
        location: [0.0; 3],
        axes: None,
    };
    vec![
        create(
            "IfcSIUnit",
            vec![
                ("UnitType", Value::Enum("LENGTHUNIT".into())),
                ("Name", Value::Enum("METRE".into())),
            ],
        ),
        create(
            "IfcUnitAssignment",
            vec![("Units", Value::List(vec![Value::Ref(h(0))]))],
        ),
        create("IfcCartesianPoint", vec![("Coordinates", reals(&[0.0; 3]))]),
        create("IfcAxis2Placement3D", vec![("Location", Value::Ref(h(2)))]),
        create(
            "IfcGeometricRepresentationContext",
            vec![
                ("ContextType", text("Model")),
                ("CoordinateSpaceDimension", Value::Integer(3)),
                ("Precision", Value::Real(1e-5)),
                ("WorldCoordinateSystem", Value::Ref(h(3))),
            ],
        ),
        create(
            "IfcGeometricRepresentationSubContext",
            vec![
                ("ContextIdentifier", text("Body")),
                ("ContextType", text("Model")),
                ("ParentContext", Value::Ref(h(4))),
                ("TargetView", Value::Enum("MODEL_VIEW".into())),
            ],
        ),
        AuthorOp::Project {
            attributes: vec![
                ("Name".into(), text("P")),
                ("UnitsInContext".into(), Value::Ref(h(1))),
                (
                    "RepresentationContexts".into(),
                    Value::List(vec![Value::Ref(h(4))]),
                ),
            ],
            owner_history: None,
        },
        placement(None),
        spatial("IfcSite", h(6), h(7)),
        placement(Some(h(7))),
        spatial("IfcBuilding", h(8), h(9)),
        placement(Some(h(9))),
        spatial("IfcBuildingStorey", h(10), h(11)),
        placement(Some(h(9))),
        spatial("IfcBuildingStorey", h(10), h(13)),
    ]
}

/// `walls` walls named `tag<n>` with a body each, contained in the storey
/// and typed by `type_object`: seven operations a wall, handles absolute in
/// a batch that has `lead` operations ahead of these.
fn wall_ops(
    s: &Scene,
    walls: std::ops::Range<usize>,
    tag: &str,
    type_object: Option<EntityId>,
    lead: usize,
) -> Vec<AuthorOp> {
    let h = authoring_handle;
    let mut ops = Vec::new();
    for n in walls {
        let b = lead + ops.len();
        let n_text = n.to_string();
        ops.push(AuthorOp::Placement {
            relative_to: Some(s.placement),
            location: [n as f64, 0.0, 0.0],
            axes: None,
        });
        ops.push(create(
            "IfcRectangleProfileDef",
            vec![
                ("ProfileType", Value::Enum("AREA".into())),
                ("XDim", Value::Real(3.0)),
                ("YDim", Value::Real(0.2)),
            ],
        ));
        ops.push(create(
            "IfcDirection",
            vec![("DirectionRatios", reals(&[0.0, 0.0, 1.0]))],
        ));
        ops.push(create(
            "IfcExtrudedAreaSolid",
            vec![
                ("SweptArea", Value::Ref(h(b + 1))),
                ("ExtrudedDirection", Value::Ref(h(b + 2))),
                ("Depth", Value::Real(3.0)),
            ],
        ));
        ops.push(AuthorOp::ShapeRepresentation {
            context: s.body,
            items: vec![h(b + 3)],
            identifier: Some("Body".into()),
            representation_type: None,
        });
        ops.push(AuthorOp::Product {
            type_name: "IfcWall".into(),
            container: Some(s.storey),
            attributes: vec![
                ("Name".to_owned(), text(&format!("{tag}{n}"))),
                (
                    "GlobalId".to_owned(),
                    Value::Text(name_based_guid(&[tag, &n_text]).as_str().into()),
                ),
            ],
            placement: Some(h(b)),
            type_object,
            owner_history: None,
        });
        ops.push(AuthorOp::AssignRepresentation {
            representation: h(b + 4),
            to: h(b + 5),
        });
    }
    ops
}

fn wall_type(name: &str) -> AuthorOp {
    AuthorOp::TypeObject {
        type_name: "IfcWallType".into(),
        attributes: vec![
            ("Name".into(), text(name)),
            ("PredefinedType".into(), Value::Enum("STANDARD".into())),
        ],
        owner_history: None,
    }
}

/// The entities and revision a refusal must leave alone.
fn fingerprint(model: &Model) -> (u64, String) {
    (
        model.revision(),
        model
            .iter()
            .map(|(id, entity)| format!("{id} {entity:?}"))
            .collect::<Vec<_>>()
            .join("\n"),
    )
}

/// The model's reverse index equals a fresh build.
fn consistent(model: &Model, at: &str) {
    assert_eq!(
        model.reverse_index(),
        &ReverseIndex::build(model),
        "{at}: the maintained reverse index differs from a fresh build"
    );
}

/// Stage `ops` against the model with its indexes and against a clone that
/// has to rebuild them; both must answer alike. Whether the batch stages.
fn stage_both(model: &Model, schema: &Schema, ops: &[AuthorOp], at: &str) -> bool {
    let render = |model: &Model| match stage_authoring(model, schema, ops, 7) {
        Ok(staged) => (
            true,
            format!(
                "{:?}\n{:?}\n{:?}\n{:?}",
                staged.transaction.edits(),
                staged.ids,
                staged.reports,
                staged.copies
            ),
        ),
        Err(error) => (false, format!("{error:?}")),
    };
    let kept = render(model);
    let mut fresh = model.clone();
    fresh.drop_derived_indexes();
    let rebuilt = render(&fresh);
    assert_eq!(kept, rebuilt, "{at}: staged differently");
    kept.0
}

/// A history of batches over IFC4X3, probed after every step.
struct Lab {
    model: Model,
    history: History,
    schema: &'static Schema,
    scene: Scene,
    seed: u128,
}

impl Lab {
    fn new() -> Self {
        let schema = ifc::schema::ifc4x3();
        let mut model = Model::new();
        // Build the indexes first: every write below keeps them current.
        model.reverse_index();
        let mut history = History::new(64);
        let out = history
            .track(&mut model, |m| apply_authoring(m, schema, &scene_ops(), 1))
            .expect("the scene authors");
        let id = |i: usize| out.ids[i].expect("an entity");
        let scene = Scene {
            storey: id(12),
            storey2: id(14),
            placement: id(11),
            body: id(5),
        };
        let lab = Self {
            model,
            history,
            schema,
            scene,
            seed: 1,
        };
        consistent(&lab.model, "scene");
        lab
    }

    /// Probe batches against the current model; how many staged.
    fn probe(&self, at: &str) -> usize {
        let model = &self.model;
        let s = &self.scene;
        let walls = model.ids_of_type("IFCWALL");
        let types = model.ids_of_type("IFCWALLTYPE");
        let mut probes: Vec<Vec<AuthorOp>> = Vec::new();
        probes.push(wall_ops(s, 900..902, "probe", types.first().copied(), 0));
        probes.push(vec![
            wall_type("probe type"),
            AuthorOp::Product {
                type_name: "IfcWall".into(),
                container: Some(s.storey2),
                attributes: vec![],
                placement: None,
                type_object: Some(authoring_handle(0)),
                owner_history: None,
            },
        ]);
        if let (Some(&first), Some(&last)) = (walls.first(), walls.last()) {
            for replace in [false, true] {
                probes.push(vec![AuthorOp::Contain {
                    structure: s.storey2,
                    elements: vec![first, last],
                    owner_history: None,
                    options: RelateOptions::new().replace(replace),
                }]);
            }
            probes.push(vec![AuthorOp::RemoveProduct {
                product: first,
                options: RemoveProductOptions::new().untype_occurrences(true),
            }]);
            probes.push(vec![AuthorOp::RemoveProduct {
                product: last,
                options: RemoveProductOptions::new(),
            }]);
            probes.push(vec![AuthorOp::CopyProduct {
                product: first,
                options: CopyProductOptions::new(),
            }]);
            // A GlobalId another wall holds.
            let held = model
                .get(first)
                .and_then(|wall| wall.attributes.first().cloned())
                .expect("a wall has a GlobalId");
            probes.push(vec![AuthorOp::Product {
                type_name: "IfcWall".into(),
                container: None,
                attributes: vec![("GlobalId".into(), held)],
                placement: None,
                type_object: None,
                owner_history: None,
            }]);
            if let Some(&ty) = types.first() {
                for replace in [false, true] {
                    probes.push(vec![AuthorOp::AssignType {
                        type_object: ty,
                        objects: vec![first, last],
                        owner_history: None,
                        options: AssignTypeOptions::new().replace(replace),
                    }]);
                }
            }
        }
        probes.push(vec![AuthorOp::Purge(PurgeOptions::new())]);
        probes
            .iter()
            .enumerate()
            .filter(|(n, ops)| stage_both(model, self.schema, ops, &format!("{at}: probe {n}")))
            .count()
    }

    /// Apply `ops` as one tracked batch, checking the indexes after.
    fn apply(&mut self, at: &str, ops: &[AuthorOp]) -> Vec<Option<EntityId>> {
        self.probe(&format!("{at} (before)"));
        self.seed += 1;
        let seed = self.seed;
        let schema = self.schema;
        let out = self
            .history
            .track(&mut self.model, |m| apply_authoring(m, schema, ops, seed))
            .unwrap_or_else(|error| panic!("{at}: {error}"));
        consistent(&self.model, at);
        self.probe(at);
        out.ids
    }

    /// A batch the model refuses, leaving everything as it was.
    fn refuse(&mut self, at: &str, ops: &[AuthorOp]) -> AuthoringFailure {
        let before = fingerprint(&self.model);
        let error = apply_authoring(&mut self.model, self.schema, ops, 99).expect_err(at);
        assert_eq!(fingerprint(&self.model), before, "{at}: a refusal wrote");
        consistent(&self.model, at);
        self.probe(at);
        *error.failure
    }

    fn undo(&mut self, at: &str) {
        self.history.undo(&mut self.model).expect(at);
        consistent(&self.model, at);
        self.probe(at);
    }

    fn redo(&mut self, at: &str) {
        self.history.redo(&mut self.model).expect(at);
        consistent(&self.model, at);
        self.probe(at);
    }
}

#[test]
fn a_history_of_batches_gives_the_same_answers_with_and_without_kept_indexes() {
    let mut lab = Lab::new();
    let s = lab.scene;
    lab.probe("scene");

    lab.apply("200 walls", &wall_ops(&s, 0..200, "w", None, 0));
    // The probes are not vacuous: some stage, some are refused.
    let staged = lab.probe("200 walls");
    assert!((3..=12).contains(&staged), "{staged} probes staged");
    lab.undo("undo the walls");
    assert_eq!(lab.model.ids_of_type("IFCWALL").len(), 0);
    lab.redo("redo the walls");
    assert_eq!(lab.model.ids_of_type("IFCWALL").len(), 200);
    lab.apply("50 more walls", &wall_ops(&s, 200..250, "w", None, 0));

    // A type with 30 typed walls, then a second type taking over.
    let mut typed = vec![wall_type("T1")];
    typed.extend(wall_ops(&s, 300..330, "t", Some(authoring_handle(0)), 1));
    let ids = lab.apply("typed walls", &typed);
    let t1 = ids[0].expect("the type");
    let walls = lab.model.ids_of_type("IFCWALL").to_vec();
    let t2 = lab.apply("a second type", &[wall_type("T2")])[0].expect("the type");
    lab.apply(
        "assign by replace",
        &[
            AuthorOp::AssignType {
                type_object: t2,
                objects: walls[..5].to_vec(),
                owner_history: None,
                options: AssignTypeOptions::new(),
            },
            AuthorOp::AssignType {
                type_object: t2,
                objects: walls[250..255].to_vec(),
                owner_history: None,
                options: AssignTypeOptions::new().replace(true),
            },
        ],
    );
    assert!(lab.model.ids_of_type("IFCRELDEFINESBYTYPE").len() >= 2);
    assert!(lab.model.contains(t1));

    let removals: Vec<AuthorOp> = walls[10..20]
        .iter()
        .map(|wall| AuthorOp::RemoveProduct {
            product: *wall,
            options: RemoveProductOptions::new().untype_occurrences(true),
        })
        .collect();
    lab.apply("remove ten walls", &removals);
    lab.apply("purge", &[AuthorOp::Purge(PurgeOptions::new())]);

    let first = lab.model.ids_of_type("IFCWALL")[0];
    lab.apply(
        "copy a wall",
        &[AuthorOp::CopyProduct {
            product: first,
            options: CopyProductOptions::new(),
        }],
    );
    let rep = lab.model.ids_of_type("IFCSHAPEREPRESENTATION")[0];
    lab.apply(
        "copy and assign a representation",
        &[
            AuthorOp::CopyRepresentation {
                representation: rep,
            },
            AuthorOp::Product {
                type_name: "IfcWall".into(),
                container: Some(s.storey),
                attributes: vec![],
                placement: None,
                type_object: None,
                owner_history: None,
            },
            AuthorOp::AssignRepresentation {
                representation: authoring_handle(0),
                to: authoring_handle(1),
            },
        ],
    );
    let moved: Vec<EntityId> = lab.model.ids_of_type("IFCWALL")[30..35].to_vec();
    lab.apply(
        "contain with replace",
        &[AuthorOp::Contain {
            structure: s.storey2,
            elements: moved,
            owner_history: None,
            options: RelateOptions::new().replace(true),
        }],
    );

    // A batch whose last operation repeats a GlobalId.
    let held = lab
        .model
        .get(lab.model.ids_of_type("IFCWALL")[0])
        .and_then(|wall| wall.attributes.first().cloned())
        .expect("a GlobalId");
    let mut refused = wall_ops(&s, 400..410, "r", None, 0);
    refused.push(AuthorOp::Product {
        type_name: "IfcWall".into(),
        container: None,
        attributes: vec![("GlobalId".into(), held)],
        placement: None,
        type_object: None,
        owner_history: None,
    });
    let failure = lab.refuse("a repeated GlobalId", &refused);
    assert!(
        matches!(failure, AuthoringFailure::DuplicateGlobalId { .. }),
        "{failure:?}"
    );

    // Walk the whole history back and forward again.
    let mut undone = 0;
    while lab.history.can_undo() {
        lab.undo("undo all");
        undone += 1;
    }
    assert!(undone >= 8, "{undone}");
    while lab.history.can_redo() {
        lab.redo("redo all");
    }
}

const FIXTURES: [(&str, SchemaVersion); 3] = [
    ("cascade_ifc2x3.ifc", SchemaVersion::Ifc2x3),
    ("cascade_ifc4.ifc", SchemaVersion::Ifc4),
    ("cascade_ifc4x3.ifc", SchemaVersion::Ifc4x3),
];

fn read(name: &str) -> Model {
    let path = format!(
        "{}/../../test/fixtures/synthetic-relations/{name}",
        env!("CARGO_MANIFEST_DIR")
    );
    StepCodec
        .read_path(std::path::Path::new(&path))
        .unwrap_or_else(|error| panic!("{name}: {error}"))
}

/// The cascade fixtures: storey #14, Wall A #20 and Wall B #40 typed by
/// #66, a door #100, the owner history #5 (see `copy_product.rs`).
#[test]
fn the_cascade_fixtures_stage_alike_in_every_release() {
    for (name, version) in FIXTURES {
        let schema = ifc::schema::for_version(version).expect("bundled");
        let mut model = read(name);
        model.reverse_index();
        let id = EntityId;
        let batches: Vec<(&str, Vec<AuthorOp>)> = vec![
            (
                "remove a wall",
                vec![AuthorOp::RemoveProduct {
                    product: id(20),
                    options: RemoveProductOptions::new()
                        .untype_occurrences(true)
                        .rebase_placements(true),
                }],
            ),
            (
                "copy a wall",
                vec![AuthorOp::CopyProduct {
                    product: id(20),
                    options: CopyProductOptions::new(),
                }],
            ),
            (
                "copy a door",
                vec![AuthorOp::CopyProduct {
                    product: id(100),
                    options: CopyProductOptions::new(),
                }],
            ),
            ("purge", vec![AuthorOp::Purge(PurgeOptions::new())]),
            (
                "contain a contained wall",
                vec![AuthorOp::Contain {
                    structure: id(14),
                    elements: vec![id(20)],
                    owner_history: Some(id(5)),
                    options: RelateOptions::new(),
                }],
            ),
            (
                "type a typed wall",
                vec![AuthorOp::AssignType {
                    type_object: id(66),
                    objects: vec![id(20)],
                    owner_history: Some(id(5)),
                    options: AssignTypeOptions::new(),
                }],
            ),
            (
                "type a typed wall by replace",
                vec![AuthorOp::AssignType {
                    type_object: id(66),
                    objects: vec![id(20), id(100)],
                    owner_history: Some(id(5)),
                    options: AssignTypeOptions::new().replace(true),
                }],
            ),
        ];
        let mut staged = 0;
        for (what, ops) in &batches {
            let at = format!("{name}: {what}");
            if stage_both(&model, schema, ops, &at) {
                staged += 1;
                let before = fingerprint(&model);
                let (applied, inverse) =
                    ifc::Inverse::record(&mut model, |m| apply_authoring(m, schema, ops, 5));
                applied.unwrap_or_else(|error| panic!("{at}: {error}"));
                consistent(&model, &at);
                inverse.apply(&mut model).expect("undo");
                consistent(&model, &at);
                assert_eq!(fingerprint(&model).1, before.1, "{at}: undo");
            }
        }
        assert!(staged >= 4, "{name}: {staged} batches staged");
    }
}

/// An unrelated wall of the scene, no container, no type.
fn loose_wall(lab: &mut Lab) -> EntityId {
    let ids = lab.apply(
        "a loose wall",
        &[AuthorOp::Product {
            type_name: "IfcWall".into(),
            container: None,
            attributes: vec![],
            placement: None,
            type_object: None,
            owner_history: None,
        }],
    );
    ids[0].expect("the wall")
}

/// An `entity` inserted under `id`, with a GlobalId unique to `id`.
fn insert_rel(model: &mut Model, id: u64, type_name: &str, related: EntityId, relating: EntityId) {
    let global_id = name_based_guid(&["tie", &id.to_string()]);
    model.insert(
        EntityId(id),
        Entity::new(
            type_name,
            vec![
                Value::Text(global_id.as_str().into()),
                Value::Null,
                Value::Null,
                Value::Null,
                Value::List(vec![Value::Ref(related)]),
                Value::Ref(relating),
            ],
        ),
    );
}

/// An object in two relationships of one kind: the refusal names the one
/// the type index lists first, which is the one inserted first and need not
/// have the lower id.
#[test]
fn an_object_in_two_relationships_is_refused_naming_the_one_listed_first() {
    let mut lab = Lab::new();
    let wall = loose_wall(&mut lab);
    let base = lab.model.next_id().0;
    let (high, low) = (EntityId(base + 10), EntityId(base + 5));
    insert_rel(
        &mut lab.model,
        high.0,
        "IFCRELCONTAINEDINSPATIALSTRUCTURE",
        wall,
        lab.scene.storey,
    );
    insert_rel(
        &mut lab.model,
        low.0,
        "IFCRELCONTAINEDINSPATIALSTRUCTURE",
        wall,
        lab.scene.storey,
    );
    assert_eq!(
        lab.model.ids_of_type("IFCRELCONTAINEDINSPATIALSTRUCTURE"),
        [high, low],
        "the bucket lists the higher id first"
    );
    let contain = [AuthorOp::Contain {
        structure: lab.scene.storey2,
        elements: vec![wall],
        owner_history: None,
        options: RelateOptions::new(),
    }];
    assert!(!stage_both(&lab.model, lab.schema, &contain, "contain"));
    let failure = lab.refuse("contain", &contain);
    assert_eq!(
        failure,
        AuthoringFailure::AlreadyRelated {
            id: wall,
            relationship: "IFCRELCONTAINEDINSPATIALSTRUCTURE",
            existing: high,
        }
    );

    // The same for types.
    let types = lab.apply(
        "three types",
        &[wall_type("A"), wall_type("B"), wall_type("C")],
    );
    let (a, b, c) = (types[0].unwrap(), types[1].unwrap(), types[2].unwrap());
    let base = lab.model.next_id().0;
    let (high, low) = (EntityId(base + 10), EntityId(base + 5));
    insert_rel(&mut lab.model, high.0, "IFCRELDEFINESBYTYPE", wall, a);
    insert_rel(&mut lab.model, low.0, "IFCRELDEFINESBYTYPE", wall, b);
    let assign = [AuthorOp::AssignType {
        type_object: c,
        objects: vec![wall],
        owner_history: None,
        options: AssignTypeOptions::new(),
    }];
    assert!(!stage_both(&lab.model, lab.schema, &assign, "assign"));
    let failure = lab.refuse("assign", &assign);
    assert_eq!(
        failure,
        AuthoringFailure::AlreadyRelated {
            id: wall,
            relationship: "IFCRELDEFINESBYTYPE",
            existing: high,
        }
    );
}
