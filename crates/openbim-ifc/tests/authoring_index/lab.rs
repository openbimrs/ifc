//! The scene, the walls and the `Lab` the cases run in.

pub(crate) use ifc::{
    apply_authoring, authoring_handle, name_based_guid, stage_authoring, AssignTypeOptions,
    AuthorOp, AuthoringFailure, Codec, CopyProductOptions, Entity, EntityId, History, Model,
    PurgeOptions, RelateOptions, RemoveProductOptions, ReverseIndex, Schema, SchemaVersion,
    StepCodec, Value,
};

pub(crate) fn text(s: &str) -> Value {
    Value::Text(s.into())
}

pub(crate) fn reals(v: &[f64]) -> Value {
    Value::List(v.iter().map(|x| Value::Real(*x)).collect())
}

pub(crate) fn create(type_name: &str, attributes: Vec<(&str, Value)>) -> AuthorOp {
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
pub(crate) struct Scene {
    pub(crate) storey: EntityId,
    pub(crate) storey2: EntityId,
    pub(crate) placement: EntityId,
    pub(crate) body: EntityId,
}

/// A project down to two storeys, in IFC4X3.
pub(crate) fn scene_ops() -> Vec<AuthorOp> {
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
pub(crate) fn wall_ops(
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

pub(crate) fn wall_type(name: &str) -> AuthorOp {
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
pub(crate) fn fingerprint(model: &Model) -> (u64, String) {
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
pub(crate) fn consistent(model: &Model, at: &str) {
    assert_eq!(
        model.reverse_index(),
        &ReverseIndex::build(model),
        "{at}: the maintained reverse index differs from a fresh build"
    );
}

/// Stage `ops` against the model with its indexes and against a clone that
/// has to rebuild them; both must answer alike. Whether the batch stages.
pub(crate) fn stage_both(model: &Model, schema: &Schema, ops: &[AuthorOp], at: &str) -> bool {
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
pub(crate) struct Lab {
    pub(crate) model: Model,
    pub(crate) history: History,
    pub(crate) schema: &'static Schema,
    pub(crate) scene: Scene,
    pub(crate) seed: u128,
}

impl Lab {
    pub(crate) fn new() -> Self {
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
    pub(crate) fn probe(&self, at: &str) -> usize {
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
    pub(crate) fn apply(&mut self, at: &str, ops: &[AuthorOp]) -> Vec<Option<EntityId>> {
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
    pub(crate) fn refuse(&mut self, at: &str, ops: &[AuthorOp]) -> AuthoringFailure {
        let before = fingerprint(&self.model);
        let error = apply_authoring(&mut self.model, self.schema, ops, 99).expect_err(at);
        assert_eq!(fingerprint(&self.model), before, "{at}: a refusal wrote");
        consistent(&self.model, at);
        self.probe(at);
        *error.failure
    }

    pub(crate) fn undo(&mut self, at: &str) {
        self.history.undo(&mut self.model).expect(at);
        consistent(&self.model, at);
        self.probe(at);
    }

    pub(crate) fn redo(&mut self, at: &str) {
        self.history.redo(&mut self.model).expect(at);
        consistent(&self.model, at);
        self.probe(at);
    }
}
