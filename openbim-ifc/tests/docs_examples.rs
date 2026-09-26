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

/// `docs/guide/getting-started.md` -- reading a file and triaging it.
#[test]
fn getting_started_reads_and_triages_a_file() -> TestResult {
    let dir = std::env::temp_dir().join(format!("docs-getting-started-{}", std::process::id()));
    std::fs::create_dir_all(&dir)?;
    std::fs::write(dir.join("model.ifc"), SOURCE)?;
    let path = dir.join("model.ifc");
    // docs:snippet getting-started-read
    use ifc::{Codec, StepCodec};

    let bytes = std::fs::read(&path)?;
    let model = StepCodec.read_bytes(&bytes)?;

    println!("schema: {:?}", model.header().schema);
    println!("entities: {}", model.len());

    for (name, count) in model.type_histogram().iter().take(10) {
        println!("{count:>7}  {name}");
    }
    // docs:end

    // docs:snippet getting-started-find
    // Type names are the upper-case STEP form.
    for &id in model.ids_of_type("IFCANNOTATION") {
        let entity = model.get(id).expect("an indexed id resolves");
        // Attributes are positional. IfcAnnotation inherits IfcRoot:
        // 0 = GlobalId, 1 = OwnerHistory, 2 = Name, 3 = Description.
        if let Some(name) = entity.text(2) {
            println!("annotation {id}: {name}");
        }
    }
    // docs:end

    let out_path = dir.join("out.ifc");
    // docs:snippet getting-started-write
    let bytes = StepCodec.write_bytes(&model)?;
    std::fs::write(&out_path, bytes)?;
    // docs:end
    assert_eq!(
        StepCodec.read_bytes(&std::fs::read(&out_path)?)?.len(),
        model.len()
    );
    std::fs::remove_dir_all(&dir)?;
    Ok(())
}

/// `docs/guide/getting-started.md` -- converting between encodings.
#[cfg(feature = "ifcxml")]
#[test]
fn getting_started_converts_step_to_ifcxml() -> TestResult {
    let step_bytes = SOURCE;
    // docs:snippet getting-started-convert
    use ifc::{Codec, StepCodec, XmlCodec}; // XmlCodec needs the `ifcxml` feature

    let model = StepCodec.read_bytes(step_bytes)?;
    let xml = XmlCodec::default().write_bytes(&model)?;
    // docs:end
    let text = String::from_utf8_lossy(&xml).to_ascii_lowercase();
    assert!(
        text.contains("ifcannotation") && text.contains("brandwand"),
        "{text}"
    );
    Ok(())
}

/// `docs/use-cases/structural-analysis.md` -- inventorying analysis models.
#[cfg(feature = "structural")]
#[test]
fn documented_structural_inventory_runs() -> TestResult {
    let dir = std::env::temp_dir().join(format!("docs-structural-{}", std::process::id()));
    std::fs::create_dir_all(&dir)?;
    let path = dir.join("analysis.ifc");
    std::fs::write(
        &path,
        "ISO-10303-21;\nHEADER;\nFILE_DESCRIPTION((''),'2;1');\n\
         FILE_NAME('','',(''),(''),'','','');\nFILE_SCHEMA(('IFC4'));\nENDSEC;\nDATA;\n\
         #1=IFCSTRUCTURALANALYSISMODEL('2O2Fr$t4X7Zf8NOew3FLOH',$,'Frame',$,$,.LOADING_3D.,$,$,$,$);\n\
         ENDSEC;\nEND-ISO-10303-21;\n",
    )?;
    // docs:snippet structural-inventory
    use ifc::structural::StructuralView; // feature = "structural"
    use ifc::{Codec, StepCodec};

    let model = StepCodec.read_bytes(&std::fs::read(&path)?)?;
    let structural = StructuralView::for_model(&model)?;

    for &id in model.ids_of_type("IFCSTRUCTURALANALYSISMODEL") {
        let analysis = structural.analysis_model(id)?;
        println!("{}: {:?}", analysis.id(), analysis.name()?);

        for item in structural.analysis_items(id)? {
            println!("assigned analytical object: {item:?}");
        }
    }
    // docs:end
    let names = model
        .ids_of_type("IFCSTRUCTURALANALYSISMODEL")
        .iter()
        .map(|&id| Ok(structural.analysis_model(id)?.name()?.map(str::to_owned)))
        .collect::<Result<Vec<_>, ifc::structural::StructuralError>>()?;
    assert_eq!(names, [Some("Frame".to_owned())]);
    std::fs::remove_dir_all(&dir)?;
    Ok(())
}
