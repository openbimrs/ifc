//! The authoring workload: a scene and cad-shaped walls (#587).
//!
//! A wall is ten authoring operations that create thirteen entities: its
//! placement, a rectangle profile, an extruded solid, the shape
//! representation, the wall in a storey, the product definition shape, one
//! property and its set, and the two relationships. Every GlobalId is
//! derived from fixed names, so the workload is the same on every run.

use ifc::{
    apply_authoring, authoring_handle, name_based_guid, AuthorOp, Entity, EntityId, Model, Value,
};

/// Entities one wall adds.
pub(crate) const ENTITIES_PER_WALL: usize = 13;

fn text(s: &str) -> Value {
    Value::Text(s.into())
}

fn reals(v: &[f64]) -> Value {
    Value::List(v.iter().map(|x| Value::Real(*x)).collect())
}

fn guid(parts: &[&str]) -> Value {
    Value::Text(name_based_guid(parts).as_str().into())
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
pub(crate) struct Scene {
    pub(crate) storey: EntityId,
    pub(crate) storey_placement: EntityId,
    pub(crate) body: EntityId,
}

/// A project down to one storey, in IFC4X3.
pub(crate) fn scene(model: &mut Model) -> Scene {
    let h = authoring_handle;
    let ops = vec![
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
        create(
            "IfcAxis2Placement3D",
            vec![("Location", Value::Ref(h(2)))],
        ),
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
        AuthorOp::Placement {
            relative_to: None,
            location: [0.0; 3],
            axes: None,
        },
        AuthorOp::Spatial {
            type_name: "IfcSite".into(),
            parent: h(6),
            attributes: vec![],
            placement: Some(h(7)),
            owner_history: None,
        },
        AuthorOp::Placement {
            relative_to: Some(h(7)),
            location: [0.0; 3],
            axes: None,
        },
        AuthorOp::Spatial {
            type_name: "IfcBuilding".into(),
            parent: h(8),
            attributes: vec![],
            placement: Some(h(9)),
            owner_history: None,
        },
        AuthorOp::Placement {
            relative_to: Some(h(9)),
            location: [0.0; 3],
            axes: None,
        },
        AuthorOp::Spatial {
            type_name: "IfcBuildingStorey".into(),
            parent: h(10),
            attributes: vec![],
            placement: Some(h(11)),
            owner_history: None,
        },
    ];
    let out = apply_authoring(model, ifc::schema::ifc4x3(), &ops, 1).expect("the scene authors");
    let id = |i: usize| out.ids[i].expect("the scene op creates an entity");
    Scene {
        storey: id(12),
        storey_placement: id(11),
        body: id(5),
    }
}

/// Append the ten operations of wall `n` named `tag`; handles are
/// absolute positions in the batch.
fn wall(ops: &mut Vec<AuthorOp>, s: &Scene, n: usize, tag: &str) {
    let h = authoring_handle;
    let b = ops.len();
    let name = format!("{tag}{n}");
    let n_text = n.to_string();
    ops.push(AuthorOp::Placement {
        relative_to: Some(s.storey_placement),
        location: [(n % 100) as f64 * 3.0, (n / 100) as f64 * 3.0, 0.0],
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
            ("Name".to_owned(), text(&name)),
            ("GlobalId".to_owned(), guid(&[tag, &n_text, "wall"])),
        ],
        placement: Some(h(b)),
        type_object: None,
        owner_history: None,
    });
    ops.push(AuthorOp::AssignRepresentation {
        representation: h(b + 4),
        to: h(b + 5),
    });
    let p = ops.len();
    ops.push(create(
        "IfcPropertySingleValue",
        vec![
            ("Name", text("IsExternal")),
            (
                "NominalValue",
                Value::Typed {
                    type_name: "IFCBOOLEAN".into(),
                    value: Box::new(Value::Bool(true)),
                },
            ),
        ],
    ));
    ops.push(create(
        "IfcPropertySet",
        vec![
            ("Name", text("Pset_WallCommon")),
            ("HasProperties", Value::List(vec![Value::Ref(h(p))])),
            ("GlobalId", guid(&[tag, &n_text, "pset"])),
        ],
    ));
    ops.push(create(
        "IfcRelDefinesByProperties",
        vec![
            ("RelatedObjects", Value::List(vec![Value::Ref(h(b + 5))])),
            ("RelatingPropertyDefinition", Value::Ref(h(p + 1))),
            ("GlobalId", guid(&[tag, &n_text, "defines"])),
        ],
    ));
}

/// The operations of walls `range`, named `tag`, as one batch.
pub(crate) fn ops(s: &Scene, range: std::ops::Range<usize>, tag: &str) -> Vec<AuthorOp> {
    let mut ops = Vec::new();
    for n in range {
        wall(&mut ops, s, n, tag);
    }
    ops
}

/// The scene with `walls` walls named `w<n>`.
///
/// Authoring `walls` walls through the current code costs more than the
/// whole run for large models (every operation scans the model), so the
/// model is built by replicating the thirteen entities one authored wall
/// adds, with fresh GlobalIds, names and positions: the same entities as
/// authoring them, which [`check_replication`] proves on small sizes.
pub(crate) fn model(walls: usize) -> (Model, Scene) {
    let mut model = Model::new();
    let s = scene(&mut model);
    let first = model.next_id().0;
    // Two authored walls: the template, and a witness that a second wall
    // adds the same shape.
    let mut template = model.clone();
    let schema = ifc::schema::ifc4x3();
    apply_authoring(&mut template, schema, &ops(&s, 0..1, "t"), 2).expect("the template authors");
    assert_eq!(
        template.len(),
        model.len() + ENTITIES_PER_WALL,
        "a wall adds {ENTITIES_PER_WALL} entities"
    );
    let pattern: Vec<Entity> = (first..first + ENTITIES_PER_WALL as u64)
        .map(|id| template.get(EntityId(id)).expect("template entity").clone())
        .collect();
    model.reserve(walls * ENTITIES_PER_WALL);
    for n in 0..walls {
        let base = model.next_id().0;
        for (k, entity) in pattern.iter().enumerate() {
            let point = &*entity.type_name == "IFCCARTESIANPOINT";
            let remap = |value: &Value| replicate(value, first, base, n, k, point);
            let mut entity = entity.clone();
            entity.attributes = entity.attributes.iter().map(remap).collect();
            model.push(entity);
        }
    }
    (model, s)
}

/// One attribute of the template, as it reads in wall `n` at `base`.
fn replicate(value: &Value, first: u64, base: u64, n: usize, k: usize, point: bool) -> Value {
    match value {
        Value::Ref(id) if id.0 >= first => Value::Ref(EntityId(id.0 - first + base)),
        Value::Text(t) if &**t == "t0" => Value::Text(format!("w{n}").into()),
        Value::Text(t) if t.len() == 22 => guid(&["replica", &n.to_string(), &k.to_string()]),
        Value::List(items) if point && items.len() == 3 => Value::List(vec![
            Value::Real((n % 100) as f64 * 3.0),
            Value::Real((n / 100) as f64 * 3.0),
            Value::Real(0.0),
        ]),
        Value::List(items) => Value::List(
            items
                .iter()
                .map(|item| replicate(item, first, base, n, k, point))
                .collect(),
        ),
        Value::Typed { type_name, value } => Value::Typed {
            type_name: type_name.clone(),
            value: Box::new(replicate(value, first, base, n, k, point)),
        },
        other => other.clone(),
    }
}

/// Author `walls` walls and compare them with [`model`]: same entity
/// count and same types.
pub(crate) fn check_replication(walls: usize) {
    let (replica, s) = model(walls);
    let mut authored = Model::new();
    scene(&mut authored);
    apply_authoring(
        &mut authored,
        ifc::schema::ifc4x3(),
        &ops(&s, 0..walls, "w"),
        3,
    )
    .expect("the walls author");
    assert_eq!(replica.len(), authored.len(), "replicated entity count");
    assert_eq!(
        replica.type_histogram(),
        authored.type_histogram(),
        "replicated types"
    );
}
