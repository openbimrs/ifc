//! Compile-and-run proof for the code shown in `docs/`.
//!
//! Documentation that ships uncompiled code is a liability: it drifts silently
//! and a coding agent will reproduce the drift. The code between a
//! `// docs:snippet <name>` and `// docs:end` marker here IS the published
//! snippet: `cargo run -p xtask -- docs` copies it verbatim into every page
//! holding a `<!-- SNIPPET:<name> -->` region, and fails when a page shows
//! Rust that no test runs. Setup and assertions stay outside the markers.
//!
//! Every example reads or writes STEP, so the file is gated on that feature:
//! the facade's own `--no-default-features` matrix build must still compile.
#![cfg(feature = "step")]

use std::sync::Arc;

use ifc::{Codec, Entity, EntityId, Model, StepCodec, Value};

type TestResult = Result<(), Box<dyn std::error::Error>>;

/// Minimal schema-valid STEP payload used by the read-side examples.
///
/// Carries one `IfcAnnotation` so the lossless-passthrough claim on the
/// 2D approval-plan page is exercised against a real parse, not asserted.
const SOURCE: &[u8] = b"ISO-10303-21;\n\
HEADER;\n\
FILE_DESCRIPTION((''),'2;1');\n\
FILE_NAME('plan.ifc','',(''),(''),'','','');\n\
FILE_SCHEMA(('IFC4'));\n\
ENDSEC;\n\
DATA;\n\
#1= IFCANNOTATION('3vB2YO$MX4xv5uCqZZG05x',$,'Brandwand',$,$,$,$);\n\
#2= IFCPOLYLINE((#3,#4));\n\
#3= IFCCARTESIANPOINT((0.,0.));\n\
#4= IFCCARTESIANPOINT((1000.,0.));\n\
ENDSEC;\n\
END-ISO-10303-21;\n";

/// `docs/index.md` -- reading a model.
#[test]
fn overview_snippet_reads_a_model() -> TestResult {
    let source = SOURCE;
    // docs:snippet index-read
    use ifc::{Codec, StepCodec};

    let model = StepCodec.read_bytes(source)?;
    println!("{} entities", model.len());
    // docs:end
    assert_eq!(model.len(), 4);
    Ok(())
}

/// `docs/api/rust.md` -- a lossless read/write cycle.
#[test]
fn round_trip_snippet_writes_what_it_read() -> TestResult {
    let bytes = SOURCE;
    // docs:snippet api-round-trip
    use ifc::{Codec, StepCodec};

    let model = StepCodec.read_bytes(bytes)?;
    let out = StepCodec.write_bytes(&model)?;
    // docs:end
    assert_eq!(StepCodec.read_bytes(&out)?.len(), model.len());
    Ok(())
}

/// `docs/use-cases/2d-approval-plans.md` -- lossless passthrough.
///
/// An entity this build does not interpret still survives a read/write cycle.
/// No domain crate is involved.
#[test]
fn unknown_entities_survive_a_round_trip() -> TestResult {
    let source = SOURCE;
    // docs:snippet approval-passthrough
    use ifc::{Codec, StepCodec};

    let model = StepCodec.read_bytes(source)?;

    // Entities of any type survive, interpreted or not.
    let annotations = model.ids_of_type("IFCANNOTATION");
    println!("{} annotations passed through untouched", annotations.len());

    let out = StepCodec.write_bytes(&model)?;
    // docs:end
    assert_eq!(annotations.len(), 1);
    let text = String::from_utf8_lossy(&out);
    assert!(text.contains("IFCANNOTATION") && text.contains("Brandwand"));
    Ok(())
}

/// The positional layout the authoring section describes: seven slots with
/// the inherited `GlobalId` first. `Model::push` stays public and unchecked.
#[test]
fn positional_construction_matches_the_documented_layout() {
    let mut model = Model::new();
    let id = model.push(Entity::new(
        "IFCANNOTATION",
        vec![
            Value::Text(Arc::from("3vB2YO$MX4xv5uCqZZG05x")), // GlobalId
            Value::Null,                                      // OwnerHistory
            Value::Text(Arc::from("Brandwand")),              // Name
            Value::Null,                                      // Description
            Value::Null,                                      // ObjectType
            Value::Null,                                      // ObjectPlacement
            Value::Null,                                      // Representation
        ],
    ));
    let entity = model.get(id).expect("entity present");
    assert!(entity.is_type("IFCANNOTATION"));
    assert_eq!(entity.text(2), Some("Brandwand"));
    let out = StepCodec.write_bytes(&model).expect("write");
    assert!(String::from_utf8_lossy(&out).contains("IFCANNOTATION"));
}

/// `docs/capabilities.md` and `docs/use-cases/2d-approval-plans.md` --
/// schema-checked construction resolves names to inherited-first slots and
/// refuses a typo.
#[cfg(feature = "author")]
#[test]
fn documented_authoring_example_resolves_slots_and_refuses_typos() -> TestResult {
    let schema = ifc::Schema::from_express(
        "SCHEMA IFC4;\n\
         TYPE IfcGloballyUniqueId = STRING; END_TYPE;\n\
         TYPE IfcLabel = STRING; END_TYPE;\n\
         ENTITY IfcRoot;\n\
           GlobalId : IfcGloballyUniqueId;\n\
           Name : OPTIONAL IfcLabel;\n\
         END_ENTITY;\n\
         ENTITY IfcAnnotation SUBTYPE OF (IfcRoot);\n\
           ObjectType : OPTIONAL IfcLabel;\n\
         END_ENTITY;\n\
         END_SCHEMA;",
    );
    let mut model = Model::new();

    // docs:snippet author-annotation
    use ifc::EntityBuilder; // feature = "author"

    let id = EntityBuilder::new(&schema, "IfcAnnotation")
        .text("GlobalId", "3vB2YO$MX4xv5uCqZZG05x")
        .text("Name", "Brandwand")
        .insert(&mut model)?;
    // docs:end

    let entity = model.get(id).expect("inserted");
    assert_eq!(entity.text(0), Some("3vB2YO$MX4xv5uCqZZG05x"));
    assert_eq!(entity.text(1), Some("Brandwand"));
    assert!(EntityBuilder::new(&schema, "IfcAnnotaton")
        .text("GlobalId", "3vB2YO$MX4xv5uCqZZG05x")
        .build()
        .is_err());
    Ok(())
}

/// `docs/api/rust.md` -- reading an entity's attributes.
#[test]
fn documented_entity_accessors_return_the_published_types() {
    let target = EntityId(9);
    let entity = Entity::new(
        "IFCWALL",
        vec![
            Value::Null,
            Value::Null,
            Value::Text("Wall".into()),
            Value::Real(2.5),
            Value::Ref(target),
        ],
    );
    // docs:snippet api-entity-accessors
    let first: Option<&Value> = entity.attribute(0);
    let name: Option<&str> = entity.text(2);
    let height: Option<f64> = entity.number(3);
    let placement: Option<EntityId> = entity.reference(4);
    let outgoing: Vec<EntityId> = entity.references(); // every outgoing reference
    let is_wall: bool = entity.is_type("IFCWALL");
    // docs:end
    assert_eq!(first, Some(&Value::Null));
    assert_eq!(
        (name, height, placement),
        (Some("Wall"), Some(2.5), Some(target))
    );
    assert_eq!(outgoing, [target]);
    assert!(is_wall);
}

/// `docs/api/rust.md` -- model-level queries.
#[test]
fn documented_model_queries_run() {
    let mut model = Model::new();
    model.insert(
        EntityId(1),
        Entity::new("IFCWALL", vec![Value::Ref(EntityId(99))]),
    );
    // docs:snippet api-model-queries
    let walls: &[EntityId] = model.ids_of_type("IFCWALL"); // indexed, not a scan
    let pairs: Vec<(EntityId, &Entity)> = model.of_type("IFCWALL").collect();
    let histogram: Vec<(&str, usize)> = model.type_histogram(); // good for triage
    let dangling: Vec<(EntityId, EntityId)> = model.dangling_references(); // (from, missing)
                                                                           // docs:end
    assert_eq!(walls, [EntityId(1)]);
    assert_eq!(pairs.len(), 1);
    assert_eq!(histogram, [("IFCWALL", 1)]);
    assert_eq!(dangling, [(EntityId(1), EntityId(99))]);
}

/// `docs/api/rust.md` -- on-demand reverse-reference lookup.
#[test]
fn documented_reverse_index_example_reports_referrer_and_slot() {
    // docs:snippet api-reverse-index
    use ifc_model::{EntityId, Model, ReverseIndex};

    fn print_referrers(model: &Model, target: EntityId) {
        let reverse = ReverseIndex::build(model);
        for hit in reverse.referrers(target) {
            println!(
                "referenced by {:?} in attribute slot {}",
                hit.from, hit.slot
            );
        }
    }
    // docs:end

    let target = EntityId(1);
    let from = EntityId(2);
    let mut model = Model::new();
    model.insert(target, Entity::new("IFCWALL", vec![]));
    model.insert(
        from,
        Entity::new("IFCRELAGGREGATES", vec![Value::Ref(target)]),
    );
    print_referrers(&model, target);
    let reverse = ReverseIndex::build(&model);
    assert_eq!(reverse.referrers(target).len(), 1);
    assert_eq!(reverse.referrers(target)[0].from, from);
    assert_eq!(reverse.referrers(target)[0].slot, 0);
}

/// A building, a storey aggregated into it, and a wall contained in the storey.
#[cfg(feature = "spatial")]
fn storey_with_wall() -> (Model, EntityId, EntityId, EntityId) {
    let mut model = Model::new();
    let (building, storey, wall) = (EntityId(1), EntityId(2), EntityId(3));
    model.insert(building, Entity::new("IFCBUILDING", vec![]));
    model.insert(storey, Entity::new("IFCBUILDINGSTOREY", vec![]));
    model.insert(wall, Entity::new("IFCWALL", vec![]));
    let relation = |relating_first: bool, relating: EntityId, related: EntityId| {
        let mut slots = vec![Value::Null; 4];
        let (relating, related) = (Value::Ref(relating), Value::List(vec![Value::Ref(related)]));
        if relating_first {
            slots.extend([relating, related]);
        } else {
            slots.extend([related, relating]);
        }
        slots
    };
    // IfcRelAggregates is relating-first; containment is related-first.
    model.insert(
        EntityId(10),
        Entity::new("IFCRELAGGREGATES", relation(true, building, storey)),
    );
    model.insert(
        EntityId(11),
        Entity::new(
            "IFCRELCONTAINEDINSPATIALSTRUCTURE",
            relation(false, storey, wall),
        ),
    );
    (model, building, storey, wall)
}

/// `docs/capabilities.md` -- spatial traversal, and the inversion it avoids.
#[cfg(feature = "spatial")]
#[test]
fn documented_spatial_example_groups_elements_by_storey() {
    let (model, building, storey, wall) = storey_with_wall();
    // docs:snippet spatial-tree
    use ifc::{SpatialKind, SpatialTree}; // feature = "spatial"

    let tree = SpatialTree::build(&model);

    for level in tree.of_kind(SpatialKind::Storey) {
        for element in tree.elements_of(level.id) {
            println!("{element} is placed directly on {}", level.id);
        }
    }

    let home = tree.container_of(wall); // which storey is this wall on?
    let above = tree.ancestors(storey); // storey -> building -> site -> project
    let beneath = tree.elements_recursive(building); // everything beneath a container
                                                     // docs:end
    assert_eq!(home, Some(storey));
    assert_eq!(above, [building]);
    assert_eq!(beneath, [wall]);
    assert!(tree.node(wall).is_none(), "a wall is not a container");
}

/// `docs/use-cases/2d-approval-plans.md` -- grouping by storey.
#[cfg(feature = "spatial")]
#[test]
fn documented_storey_grouping_runs() {
    let (model, _, storey, wall) = storey_with_wall();
    // docs:snippet approval-storeys
    use ifc::{SpatialKind, SpatialTree}; // feature = "spatial"

    let tree = SpatialTree::build(&model);
    for level in tree.of_kind(SpatialKind::Storey) {
        let on_this_level = tree.elements_of(level.id);
        println!("{} elements on {}", on_this_level.len(), level.id);
    }
    // docs:end
    assert_eq!(tree.elements_of(storey), [wall]);
}

#[cfg(feature = "geometry-select")]
mod geometry {
    //! Placement, context and representation-selection snippets.

    use super::*;

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
}

/// `docs/use-cases/2d-approval-plans.md` -- "Before you export: will a viewer
/// actually draw it?" A product whose body lives only in a plan context is
/// reported, and the message says why.
#[test]
#[cfg(all(feature = "spatial", feature = "geometry-select"))]
fn documented_unreachable_example_reports_plan_only_geometry() -> TestResult {
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

/// `docs/guide/resources.md` -- bounded IFC4 resource authoring and reads,
/// surviving a STEP round trip.
#[cfg(feature = "resource")]
#[test]
fn documented_resource_example_round_trips_authored_composition() -> TestResult {
    // docs:snippet resources-compose
    use ifc::resource::{
        AllocationDraft, NestingDraft, ResourceDraft, ResourceEditor, ResourceKind,
        ResourceTimeDraft, ResourceView,
    };
    use ifc::Model;

    let mut model = Model::new();
    model.header_mut().schema.push("IFC4".into());

    let (crew, carpenter, usage, allocation) = {
        let mut editor = ResourceEditor::for_model(&mut model)?;
        let usage = editor.create_time(
            ResourceTimeDraft::new()
                .name("Day shift")
                .schedule_work("PT8H")
                .schedule_usage(1.0),
        )?;
        let crew = editor.create_resource(
            ResourceDraft::new(ResourceKind::Crew, "2O2Fr$t4X7Zf8NOew3FLOH")
                .name("Envelope crew")
                .predefined_type("OFFICE")
                .usage(usage),
        )?;
        let carpenter = editor.create_resource(
            ResourceDraft::new(ResourceKind::Labor, "1O2Fr$t4X7Zf8NOew3FLOH")
                .name("Carpenter")
                .predefined_type("CARPENTRY"),
        )?;
        let allocation = editor.create_allocation(
            AllocationDraft::new("3O2Fr$t4X7Zf8NOew3FLOH", carpenter, vec![crew])
                .related_objects_type("RESOURCE"),
        )?;
        editor.create_nesting(NestingDraft::new(
            "0O2Fr$t4X7Zf8NOew3FLOH",
            crew,
            vec![carpenter],
        ))?;
        (crew, carpenter, usage, allocation)
    };

    let resources = ResourceView::for_model(&model)?;
    assert_eq!(
        resources.allocation(allocation)?.related_objects_type(),
        Some("RESOURCE")
    );
    assert_eq!(resources.resource(carpenter)?.name()?, Some("Carpenter"));
    assert_eq!(
        resources.resource_time(usage)?.schedule_work()?,
        Some("PT8H")
    );
    assert_eq!(
        resources.descendants(crew, Default::default())?,
        vec![carpenter]
    );
    // docs:end

    let reparsed = StepCodec.read_bytes(&StepCodec.write_bytes(&model)?)?;
    let resources = ResourceView::for_model(&reparsed)?;
    assert_eq!(
        resources.allocation(allocation)?.related_objects_type(),
        Some("RESOURCE")
    );
    assert_eq!(resources.resource(carpenter)?.name()?, Some("Carpenter"));
    assert_eq!(
        resources.descendants(crew, Default::default())?,
        vec![carpenter]
    );
    Ok(())
}

/// `docs/use-cases/2d-approval-plans.md` -- the downloadable round-trip fixture
/// (DOC-007, #9).
///
/// Reads the file the site publishes, not a copy, so the page and the download
/// cannot drift apart. Every entity the page names is read back through its
/// typed view, and the written file reparses to the same model apart from the
/// documented edit.
#[cfg(all(feature = "style", feature = "classification", feature = "author"))]
#[test]
fn documented_annotation_fixture_round_trips() -> TestResult {
    use ifc::classification::ClassificationView;
    use ifc::style::BoxAlignment;

    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../docs/public/fixtures/annotation-plan.ifc");
    let bytes = std::fs::read(&path)?;

    // docs:snippet annotation-fixture-edit
    use ifc::style::StyleView; // features = ["step", "style", "author"]
    use ifc::{Codec, EntityEditor, StepCodec, Transaction};

    let mut model = StepCodec.read_bytes(&bytes)?;
    let schema = ifc::schema::ifc4();

    let text = model.ids_of_type("IFCTEXTLITERALWITHEXTENT")[0];
    let view = StyleView::new(&model, schema);
    assert_eq!(view.text_literal(text)?.literal()?, "Brandwand F90");

    let mut tx = Transaction::new(&model);
    EntityEditor::new(schema, &model, text)?
        .text("Literal", "Brandwand F90 (geprüft)")
        .stage(&mut tx)?;
    tx.commit(&mut model)
        .map_err(|conflicts| format!("{conflicts:?}"))?;

    let out = StepCodec.write_bytes(&model)?;
    // docs:end

    let original = StepCodec.read_bytes(&bytes)?;
    let reparsed = StepCodec.read_bytes(&out)?;
    assert_eq!(original.len(), reparsed.len(), "no entity gained or lost");
    for (id, entity) in original.iter() {
        let after = reparsed.get(id).expect("every entity survives");
        assert_eq!(entity.type_name, after.type_name);
        if id == text {
            assert_eq!(after.text(0), Some("Brandwand F90 (geprüft)"));
            assert_eq!(entity.attributes[1..], after.attributes[1..]);
        } else {
            assert_eq!(entity.attributes, after.attributes, "#{} changed", id.0);
        }
    }

    // Every entity the page names reads back through its typed view.
    let view = StyleView::new(&reparsed, schema);
    let first = |type_name: &str| reparsed.ids_of_type(type_name)[0];
    let annotation = view.annotation(first("IFCANNOTATION"))?;
    assert_eq!(annotation.name()?, Some("Brandwand"));
    assert_eq!(
        view.text_literal_with_extent(text)?.box_alignment()?,
        BoxAlignment::BottomLeft
    );
    let (line, curve_style) = (first("IFCPOLYLINE"), first("IFCCURVESTYLE"));
    assert_eq!(
        view.resolve_item_style(line)?.effective_styles(),
        [curve_style]
    );
    let style = view.curve_style(curve_style)?;
    assert_eq!(style.name()?, Some("Brandwand"));
    assert_eq!(
        view.colour_rgb(style.curve_colour()?.expect("a colour"))?
            .red()?,
        1.0
    );
    let text_style = first("IFCTEXTSTYLE");
    assert_eq!(
        view.resolve_item_style(text)?.effective_styles(),
        [text_style]
    );
    assert_eq!(view.text_style(text_style)?.name()?, Some("Beschriftung"));
    let layer = view.presentation_layer(first("IFCPRESENTATIONLAYERASSIGNMENT"))?;
    assert_eq!(layer.name()?, "A-ANNO-FIRE");
    assert_eq!(layer.assigned_items()?, [first("IFCSHAPEREPRESENTATION")]);
    let library = ClassificationView::new(&reparsed);
    let symbol = library
        .library_references()
        .next()
        .expect("a library reference");
    assert_eq!(symbol.identification()?, Some("BW-F90"));
    assert_eq!(library.library_assignments_for(annotation.id())?.len(), 1);
    Ok(())
}
