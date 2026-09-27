//! Compile-and-run proof for the geometry code shown in `docs/`: contexts,
//! placements, representation selection and the unreachable-product lint.
//!
//! Split from `docs_examples.rs` by topic; the snippet contract is the same:
//! the code between `// docs:snippet <name>` and `// docs:end` is what
//! `cargo run -p xtask -- docs` publishes.
#![cfg(all(feature = "step", feature = "geometry-select"))]

use ifc::{Entity, EntityId, Model, Value};

type TestResult = Result<(), Box<dyn std::error::Error>>;

/// A plan sub-context written with `*` slots, as exporters do, and a wall
/// carrying a FootPrint and a Body.
fn wall_with_two_representations() -> (Model, EntityId, EntityId, EntityId) {
    let mut model = Model::new();
    let (placement, root, plan) = (EntityId(1), EntityId(2), EntityId(3));
    model.insert(placement, Entity::new("IFCAXIS2PLACEMENT3D", vec![]));
    model.insert(
        root,
        Entity::new(
            "IFCGEOMETRICREPRESENTATIONCONTEXT",
            vec![
                Value::Null,
                Value::Text("Model".into()),
                Value::Integer(3),
                Value::Real(1.0e-5),
                Value::Ref(placement),
                Value::Null,
            ],
        ),
    );
    let mut sub = vec![Value::Text("Plan".into()), Value::Text("Plan".into())];
    sub.extend([
        Value::Derived,
        Value::Derived,
        Value::Derived,
        Value::Derived,
    ]);
    sub.extend([
        Value::Ref(root),
        Value::Real(0.01),
        Value::Enum("PLAN_VIEW".into()),
        Value::Null,
    ]);
    model.insert(
        plan,
        Entity::new("IFCGEOMETRICREPRESENTATIONSUBCONTEXT", sub),
    );
    let (footprint, body) = (EntityId(10), EntityId(11));
    for (id, identifier, kind) in [
        (footprint, "FootPrint", "Curve2D"),
        (body, "Body", "SweptSolid"),
    ] {
        model.insert(
            id,
            Entity::new(
                "IFCSHAPEREPRESENTATION",
                vec![
                    Value::Ref(root),
                    Value::Text(identifier.into()),
                    Value::Text(kind.into()),
                    Value::List(vec![]),
                ],
            ),
        );
    }
    let shape = EntityId(12);
    model.insert(
        shape,
        Entity::new(
            "IFCPRODUCTDEFINITIONSHAPE",
            vec![
                Value::Null,
                Value::Null,
                Value::List(vec![Value::Ref(footprint), Value::Ref(body)]),
            ],
        ),
    );
    let wall = EntityId(13);
    let mut slots = vec![Value::Null; 7];
    slots[0] = Value::Text("3vB2YO$MX4xv5uCqZZG05x".into());
    slots[6] = Value::Ref(shape);
    model.insert(wall, Entity::new("IFCWALL", slots));
    (model, wall, footprint, body)
}

/// `docs/capabilities.md` -- the two selectors disagree on purpose.
#[test]
fn documented_selectors_pick_body_and_footprint() -> TestResult {
    let (model, wall, footprint, body_rep) = wall_with_two_representations();
    // docs:snippet select-representations
    use ifc::{select_plan_representation, select_shape_representation};

    let body = select_shape_representation(&model, wall)?; // what a 3D viewer draws
    let outline = select_plan_representation(&model, wall)?; // what a drawing draws
                                                             // docs:end
    assert_eq!((body, outline), (Some(body_rep), Some(footprint)));
    Ok(())
}

/// `docs/use-cases/2d-approval-plans.md` -- choosing what to draw.
#[test]
fn documented_plan_selection_runs() -> TestResult {
    let (model, wall, footprint, _) = wall_with_two_representations();
    // docs:snippet approval-plan-select
    use ifc::select_plan_representation; // feature = "geometry-select"

    let drawable = select_plan_representation(&model, wall)?;
    // docs:end
    assert_eq!(drawable, Some(footprint));
    Ok(())
}

/// `docs/capabilities.md` and the 2D guide -- plan contexts, with precision
/// inherited through `*`.
#[test]
fn documented_plan_contexts_inherit_precision() {
    let (model, ..) = wall_with_two_representations();
    // docs:snippet plan-contexts
    use ifc::plan_contexts; // feature = "geometry-select"

    for context in plan_contexts(&model) {
        let scale = context.target_scale(); // Some(0.01) for 1:100
        let precision = context.precision(&model); // inherited from the parent context
        println!("plan view at {scale:?}, precision {precision:?}");
    }
    // docs:end
    let seen: Vec<_> = plan_contexts(&model)
        .iter()
        .map(|context| (context.target_scale(), context.precision(&model)))
        .collect();
    assert_eq!(seen, [(Some(0.01), Some(1.0e-5))]);
}

/// A storey placed at +3 and a wall placed in it.
fn placed_wall() -> (Model, EntityId) {
    let mut model = Model::new();
    model.insert(
        EntityId(1),
        Entity::new(
            "IFCCARTESIANPOINT",
            vec![Value::List(vec![
                Value::Real(0.0),
                Value::Real(0.0),
                Value::Real(3.0),
            ])],
        ),
    );
    model.insert(
        EntityId(2),
        Entity::new(
            "IFCAXIS2PLACEMENT3D",
            vec![Value::Ref(EntityId(1)), Value::Null, Value::Null],
        ),
    );
    model.insert(
        EntityId(3),
        Entity::new(
            "IFCLOCALPLACEMENT",
            vec![Value::Null, Value::Ref(EntityId(2))],
        ),
    );
    let mut wall = vec![Value::Null; 7];
    wall[5] = Value::Ref(EntityId(3));
    model.insert(EntityId(4), Entity::new("IFCWALL", wall));
    (model, EntityId(4))
}

/// `docs/use-cases/2d-approval-plans.md` -- one product's world transform.
#[test]
fn documented_world_transform_resolves() -> TestResult {
    let (model, wall) = placed_wall();
    let units = ifc::geometry::units::resolve(&model);
    // docs:snippet world-transform
    use ifc::product_world_transform; // feature = "geometry-select"

    let world = product_world_transform(&model, &units, wall)?;
    // docs:end
    assert_eq!(world.origin, [0.0, 0.0, 3.0]);
    Ok(())
}

/// `docs/use-cases/2d-approval-plans.md` -- the batch form.
#[test]
fn documented_world_transforms_batch_resolves() {
    let (model, wall) = placed_wall();
    let units = ifc::geometry::units::resolve(&model);
    let ids = [wall];
    // docs:snippet world-transforms
    use ifc::products_world_transforms;

    for (id, world) in products_world_transforms(&model, &units, ids) {
        // Errors are per product: one broken chain does not hide the rest.
        match world {
            Ok(world) => println!("{id} sits at {:?}", world.origin),
            Err(error) => eprintln!("{id}: {error}"),
        }
    }
    // docs:end
    let batch = products_world_transforms(&model, &units, ids);
    assert_eq!(batch.len(), 1);
    assert_eq!(
        batch[0].1.as_ref().map(|w| w.origin).ok(),
        Some([0.0, 0.0, 3.0])
    );
}

/// `docs/use-cases/2d-approval-plans.md` -- "Before you export: will a viewer
/// actually draw it?" A product whose body lives only in a plan context is
/// reported, and the message says why.
#[test]
#[cfg(all(feature = "spatial", feature = "geometry-select"))]
fn documented_unreachable_example_reports_plan_only_geometry() -> TestResult {
    use std::sync::Arc;

    use ifc::Unreachable;

    let mut model = Model::new();
    let mut next = 0u64;
    let mut add = |model: &mut Model, name: &str, attributes: Vec<Value>| {
        next += 1;
        model.insert(EntityId(next), Entity::new(name, attributes));
        EntityId(next)
    };
    // A sub-context declaring PLAN_VIEW, which a model viewer skips.
    let mut ctx = vec![Value::Null; 10];
    ctx[0] = Value::Text(Arc::from("Annotation"));
    ctx[1] = Value::Text(Arc::from("Plan"));
    ctx[8] = Value::Enum(Arc::from("PLAN_VIEW"));
    let context = add(&mut model, "IFCGEOMETRICREPRESENTATIONSUBCONTEXT", ctx);
    let mut rep = vec![Value::Null; 4];
    rep[0] = Value::Ref(context);
    rep[1] = Value::Text(Arc::from("Annotation"));
    rep[2] = Value::Text(Arc::from("Curve2D"));
    let representation = add(&mut model, "IFCSHAPEREPRESENTATION", rep);
    let mut shp = vec![Value::Null; 3];
    shp[2] = Value::List(vec![Value::Ref(representation)]);
    let shape = add(&mut model, "IFCPRODUCTDEFINITIONSHAPE", shp);
    let mut product = vec![Value::Null; 7];
    product[0] = Value::Text(Arc::from("3vB2YO$MX4xv5uCqZZG05x"));
    product[6] = Value::Ref(shape);
    let sign = add(&mut model, "IFCANNOTATION", product);
    // Contained correctly, so the context is the only remaining defect.
    let storey = add(
        &mut model,
        "IFCBUILDINGSTOREY",
        vec![
            Value::Text(Arc::from("1storey0000000000000001")),
            Value::Null,
            Value::Null,
            Value::Null,
        ],
    );
    let mut rel = vec![Value::Null; 6];
    rel[4] = Value::List(vec![Value::Ref(sign)]);
    rel[5] = Value::Ref(storey);
    add(&mut model, "IFCRELCONTAINEDINSPATIALSTRUCTURE", rel);
    let bytes = StepCodec.write_bytes(&model)?;

    // docs:snippet unreachable-products
    use ifc::{unreachable_products, Codec, StepCodec}; // features = ["spatial", "geometry-select"]

    let model = StepCodec.read_bytes(&bytes)?;
    for (id, why) in unreachable_products(&model) {
        eprintln!("#{id}: {}", why.message());
    }
    // docs:end

    let findings = unreachable_products(&model);
    assert_eq!(findings.len(), 1, "the sign is the only finding");
    assert_eq!(findings[0].0, sign);
    assert!(matches!(
        findings[0].1,
        Unreachable::NoRepresentationInModelContext { .. }
    ));
    assert!(findings[0].1.message().contains("model viewer"));
    Ok(())
}
