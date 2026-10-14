//! Shared by the shape representation tests (#481): the cascade fixtures in
//! each release, validation (WR11 included) after every batch, and op
//! builders. Each test binary compiles this module separately, so a helper
//! one of them does not use is not dead code.
#![allow(dead_code)]

use ifc::{
    apply_authoring, authoring_handle, AuthorOp, AuthoringFailure, AuthoringOutcome, Codec,
    EntityId, Model, Schema, SchemaVersion, StepCodec, Value,
};

pub const FIXTURES: [(&str, SchemaVersion); 3] = [
    ("cascade_ifc2x3.ifc", SchemaVersion::Ifc2x3),
    ("cascade_ifc4.ifc", SchemaVersion::Ifc4),
    ("cascade_ifc4x3.ifc", SchemaVersion::Ifc4x3),
];

pub fn read(name: &str) -> Model {
    let path = format!(
        "{}/../../test/fixtures/synthetic-relations/{name}",
        env!("CARGO_MANIFEST_DIR")
    );
    StepCodec
        .read_path(std::path::Path::new(&path))
        .unwrap_or_else(|error| panic!("{name}: {error}"))
}

pub fn each_release(mut check: impl FnMut(&str, Model, &'static Schema)) {
    for (name, version) in FIXTURES {
        let model = read(name);
        let schema = ifc::schema::for_version(version).expect("bundled");
        assert_valid(name, &model, schema);
        check(name, model, schema);
    }
}

/// No validation error, nothing dangling, and every shape model used
/// exactly once (`IfcShapeModel.WR11`).
pub fn assert_valid(what: &str, model: &Model, schema: &Schema) {
    let report = ifc::validate::validate(model, schema);
    let errors: Vec<String> = report
        .sorted()
        .iter()
        .filter(|finding| finding.severity == ifc::validate::Severity::Error)
        .map(|finding| {
            format!(
                "{}: {} at {:?}",
                finding.rule, finding.message, finding.path
            )
        })
        .collect();
    assert!(errors.is_empty(), "{what}:\n  {}", errors.join("\n  "));
    assert!(model.dangling_references().is_empty(), "{what}: dangling");
    for (id, entity) in model.iter() {
        if !schema.is_a(&entity.type_name, "IfcShapeModel") {
            continue;
        }
        let uses = model
            .iter()
            .filter(|(_, user)| {
                let t = &user.type_name;
                (schema.is_a(t, "IfcProductRepresentation")
                    && refs_of(schema, user, "Representations").contains(&id.0))
                    || (schema.is_a(t, "IfcRepresentationMap")
                        && refs_of(schema, user, "MappedRepresentation").contains(&id.0))
                    || (schema.is_a(t, "IfcShapeAspect")
                        && refs_of(schema, user, "ShapeRepresentations").contains(&id.0))
            })
            .count();
        assert_eq!(uses, 1, "{what}: {id} has {uses} uses (WR11)");
    }
}

pub fn slot(schema: &Schema, type_name: &str, name: &str) -> usize {
    schema
        .attributes(type_name)
        .iter()
        .position(|attribute| attribute.name.eq_ignore_ascii_case(name))
        .unwrap_or_else(|| panic!("{type_name} has no {name}"))
}

pub fn attr(model: &Model, schema: &Schema, id: u64, name: &str) -> Value {
    let entity = model
        .get(EntityId(id))
        .unwrap_or_else(|| panic!("#{id} is gone"));
    entity.attributes[slot(schema, &entity.type_name, name)].clone()
}

pub fn refs_of(schema: &Schema, entity: &ifc::Entity, name: &str) -> Vec<u64> {
    let mut found = Vec::new();
    entity.attributes[slot(schema, &entity.type_name, name)]
        .for_each_ref(&mut |id| found.push(id.0));
    found
}

pub fn refs(model: &Model, schema: &Schema, id: u64, name: &str) -> Vec<u64> {
    let entity = model
        .get(EntityId(id))
        .unwrap_or_else(|| panic!("#{id} is gone"));
    refs_of(schema, entity, name)
}

pub fn text(value: &Value) -> Option<&str> {
    value.unwrap_typed().as_text()
}

pub fn apply(what: &str, model: &mut Model, schema: &Schema, ops: &[AuthorOp]) -> AuthoringOutcome {
    let outcome =
        apply_authoring(model, schema, ops, 481).unwrap_or_else(|error| panic!("{what}: {error}"));
    assert_valid(what, model, schema);
    outcome
}

pub fn refusal(model: &mut Model, schema: &Schema, ops: &[AuthorOp]) -> AuthoringFailure {
    let before = model.revision();
    let error = apply_authoring(model, schema, ops, 481).expect_err("refused");
    assert_eq!(model.revision(), before, "a refusal leaves the model alone");
    *error.failure
}

pub fn id(value: u64) -> EntityId {
    EntityId(value)
}

pub fn create(type_name: &str, attributes: Vec<(&str, Value)>) -> AuthorOp {
    AuthorOp::Create {
        type_name: type_name.into(),
        attributes: attributes
            .into_iter()
            .map(|(name, value)| (name.to_owned(), value))
            .collect(),
    }
}

pub fn reals(values: &[f64]) -> Value {
    Value::List(values.iter().copied().map(Value::Real).collect())
}

/// Ops 0..=3: a 1.2 x 0.1 x 1.0 extrusion; op 3 produces the solid.
pub fn extrusion() -> Vec<AuthorOp> {
    vec![
        create(
            "IfcCartesianPoint",
            vec![("Coordinates", reals(&[0.6, 0.05]))],
        ),
        create(
            "IfcAxis2Placement2D",
            vec![("Location", Value::Ref(authoring_handle(0)))],
        ),
        create(
            "IfcRectangleProfileDef",
            vec![
                ("ProfileType", Value::Enum("AREA".into())),
                ("Position", Value::Ref(authoring_handle(1))),
                ("XDim", Value::Real(1.2)),
                ("YDim", Value::Real(0.1)),
            ],
        ),
        create(
            "IfcExtrudedAreaSolid",
            vec![
                ("SweptArea", Value::Ref(authoring_handle(2))),
                ("Position", Value::Ref(id(9))),
                ("ExtrudedDirection", Value::Ref(id(31))),
                ("Depth", Value::Real(1.0)),
            ],
        ),
    ]
}

pub fn shape(
    context: u64,
    items: Vec<EntityId>,
    identifier: Option<&str>,
    kind: Option<&str>,
) -> AuthorOp {
    AuthorOp::ShapeRepresentation {
        context: id(context),
        items,
        identifier: identifier.map(str::to_owned),
        representation_type: kind.map(str::to_owned),
    }
}

pub fn assign(representation: EntityId, to: u64) -> AuthorOp {
    AuthorOp::AssignRepresentation {
        representation,
        to: id(to),
    }
}

pub fn unassign(representation: u64, from: u64) -> AuthorOp {
    AuthorOp::UnassignRepresentation {
        representation: id(representation),
        from: id(from),
    }
}

pub fn remove(representation: EntityId) -> AuthorOp {
    AuthorOp::RemoveRepresentation { representation }
}

/// `op` with every handle moved `by` operations later.
pub fn shift(op: AuthorOp, by: usize) -> AuthorOp {
    fn moved(value: Value, by: usize) -> Value {
        match value {
            Value::Ref(id) if id.0 >= ifc::HANDLE_BASE => Value::Ref(EntityId(id.0 + by as u64)),
            Value::List(items) => Value::List(items.into_iter().map(|v| moved(v, by)).collect()),
            other => other,
        }
    }
    match op {
        AuthorOp::Create {
            type_name,
            attributes,
        } => AuthorOp::Create {
            type_name,
            attributes: attributes
                .into_iter()
                .map(|(name, v)| (name, moved(v, by)))
                .collect(),
        },
        other => other,
    }
}

/// Build `items` (ops whose handles count from 0), then a representation
/// of the entities at `of` with the given type, assigned to Window 1;
/// return the type written, or the refusal.
pub fn represent(
    model: &Model,
    schema: &Schema,
    items: Vec<AuthorOp>,
    of: &[usize],
    kind: Option<&str>,
) -> Result<String, AuthoringFailure> {
    let mut model = model.clone();
    let mut ops = items;
    let at = ops.len();
    ops.push(shape(
        11,
        of.iter().map(|i| authoring_handle(*i)).collect(),
        None,
        kind,
    ));
    ops.push(assign(authoring_handle(at), 130));
    let outcome = apply_authoring(&mut model, schema, &ops, 481).map_err(|error| *error.failure)?;
    assert_valid("represent", &model, schema);
    let representation = outcome.ids[at].unwrap().0;
    Ok(
        text(&attr(&model, schema, representation, "RepresentationType"))
            .unwrap()
            .to_owned(),
    )
}

pub fn point(coordinates: &[f64]) -> AuthorOp {
    create(
        "IfcCartesianPoint",
        vec![("Coordinates", reals(coordinates))],
    )
}

pub fn polyline(from: usize, to: usize) -> AuthorOp {
    create(
        "IfcPolyline",
        vec![(
            "Points",
            Value::List(vec![
                Value::Ref(authoring_handle(from)),
                Value::Ref(authoring_handle(to)),
            ]),
        )],
    )
}

pub fn bounding_box(corner: usize) -> AuthorOp {
    create(
        "IfcBoundingBox",
        vec![
            ("Corner", Value::Ref(authoring_handle(corner))),
            ("XDim", Value::Real(1.0)),
            ("YDim", Value::Real(1.0)),
            ("ZDim", Value::Real(1.0)),
        ],
    )
}

/// Ops 4..=7 after [`extrusion`]: a half space, and the extrusion clipped
/// by it (op 7).
pub fn clipping() -> Vec<AuthorOp> {
    let mut ops = extrusion();
    ops.extend([
        create("IfcPlane", vec![("Position", Value::Ref(id(9)))]),
        create(
            "IfcHalfSpaceSolid",
            vec![
                ("BaseSurface", Value::Ref(authoring_handle(4))),
                ("AgreementFlag", Value::Bool(false)),
            ],
        ),
        create(
            "IfcBooleanClippingResult",
            vec![
                ("Operator", Value::Enum("DIFFERENCE".into())),
                ("FirstOperand", Value::Ref(authoring_handle(3))),
                ("SecondOperand", Value::Ref(authoring_handle(5))),
            ],
        ),
        create(
            "IfcBooleanResult",
            vec![
                ("Operator", Value::Enum("UNION".into())),
                ("FirstOperand", Value::Ref(authoring_handle(3))),
                ("SecondOperand", Value::Ref(authoring_handle(5))),
            ],
        ),
    ]);
    ops
}
