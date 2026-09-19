//! Every authored record is checked against the schema itself.
//!
//! # Why an oracle rather than pairwise comparison
//!
//! Six entity types are constructed by more than one domain crate, and
//! sibling crates cannot depend on one another, so nothing makes two
//! writers of the same record agree. Comparing them in pairs scales as
//! the square of the writers and still proves only that they match --
//! two writers wrong in the same way pass.
//!
//! `ifc-validate` already answers the stronger question directly. It
//! checks record arity against the declared attribute list, rejects a
//! derived slot written as `$` and a plain slot written as `*`, and
//! reports required attributes left unset. Those are exactly the
//! failures found by hand in this crate's history: a quantity written
//! with four slots where five are declared, and `IfcSIUnit.Dimensions`
//! written as `$` where the schema derives it.
//!
//! So each crate authors through its own public API, and the result is
//! validated. A new writer is covered by adding it here; no existing
//! case has to change.
//!
//! # This does not replace `quantity_writer_agreement`
//!
//! The two checks are complementary, and neither subsumes the other.
//! The validator reasons about structure: arity, derived slots,
//! required attributes. It is blind to a value of the right shape in
//! the right slot that says the wrong thing -- `IfcCountMeasure`
//! written as a real still has five attributes and no derived-slot
//! violation, so this file passes while the comparison fails. Verified
//! by mutation, not assumed.

#![cfg(all(
    feature = "validate",
    feature = "properties",
    feature = "cost",
    feature = "schedule",
    feature = "systems",
    feature = "resource",
    feature = "alignment",
    feature = "georef",
))]

use ifc::{Entity, Model, Value};
use ifc_model::{EntityId, Transaction};
use ifc_schema::ifc4;

/// A model carrying the schema token the validator resolves against.
fn model() -> Model {
    let mut model = Model::new();
    model.header_mut().schema = vec!["IFC4".to_owned()];
    model
}

/// Name an entity by type and id, so a finding points somewhere.
fn describe(model: &Model, id: EntityId) -> String {
    model.get(id).map_or_else(
        || format!("#{}", id.0),
        |e| format!("{}#{}", e.type_name, id.0),
    )
}

/// Validate and fail with the findings, not just a boolean.
///
/// `Unsupported` findings are not failures: they mark rules the
/// validator declines to evaluate, such as those needing geometry.
fn assert_conformant(model: &Model, what: &str) {
    let report = ifc_validate::validate(model, ifc4());
    let errors: Vec<String> = report
        .sorted()
        .iter()
        .filter(|finding| finding.severity == ifc_validate::Severity::Error)
        .map(|finding| {
            let where_ = match &finding.path {
                ifc_validate::Path::Entity(id) => describe(model, *id),
                ifc_validate::Path::Attribute {
                    entity,
                    index,
                    name,
                } => format!(
                    "{} slot {index}{}",
                    describe(model, *entity),
                    name.as_ref().map(|n| format!(" ({n})")).unwrap_or_default(),
                ),
                other => format!("{other:?}"),
            };
            format!("{}: {} [{}]", finding.rule, finding.message, where_)
        })
        .collect();
    assert!(
        errors.is_empty(),
        "{what} is not schema-conformant:\n  {}",
        errors.join("\n  ")
    );
}

/// Stage a minimal product for relationships to point at.
///
/// The arity comes from the schema rather than a literal: these stubs
/// stand in for entities other crates author, and a hand-counted width
/// would fail the very check this file exists to make.
fn product(tx: &mut Transaction, type_name: &'static str, guid: &str) -> EntityId {
    let mut attrs = vec![Value::Null; ifc4().attributes(type_name).len()];
    attrs[0] = Value::Text(guid.into());
    tx.create(Entity::new(type_name, attrs))
}

/// `ifc-properties`: property sets, single values, units, quantities.
///
/// Covers `IFCPROPERTYSET`, `IFCPROPERTYSINGLEVALUE`, `IFCMONETARYUNIT`
/// and `IFCRELDEFINESBYPROPERTIES` -- four of the six shared types.
#[test]
fn properties_authoring_is_conformant() {
    use ifc::properties::{
        add_context_dependent_unit, add_dimensional_exponents, add_measure_with_unit,
        add_monetary_unit, add_property_bounded_value, add_property_enumerated_value,
        add_property_list_value, add_property_reference_value, add_property_set,
        add_property_single_value, add_si_unit, attach_property_set, create_quantity,
        create_quantity_with, MonetaryUnitDraft, QuantityExtras, QuantityKind, SiUnitDraft,
    };

    let mut model = model();
    let mut tx = Transaction::new(&model);
    let wall = product(&mut tx, "IFCWALL", "0aBcDeFgHiJkLmNoPqRsTu");

    let metre = add_si_unit(
        &mut tx,
        SiUnitDraft {
            unit_type: "LENGTHUNIT",
            prefix: None,
            name: "METRE",
        },
    )
    .expect("si unit");
    let exponents = add_dimensional_exponents(&mut tx, [1, 0, 0, 0, 0, 0, 0]);
    add_context_dependent_unit(&mut tx, exponents, "LENGTHUNIT", "Module")
        .expect("context dependent unit");
    add_measure_with_unit(&mut tx, Value::Real(25.4), metre).expect("measure with unit");
    add_monetary_unit(&mut tx, MonetaryUnitDraft { currency: "EUR" }).expect("monetary unit");

    let height =
        add_property_single_value(&mut tx, "Height", None, None, None).expect("single value");
    let range = add_property_bounded_value(&mut tx, "Range", None, None, None, None, None)
        .expect("bounded value");
    let listed =
        add_property_list_value(&mut tx, "Layers", None, Some(vec![Value::Real(1.0)]), None)
            .expect("list value");
    let referenced =
        add_property_reference_value(&mut tx, "Doc", None, None, None).expect("reference value");
    let enumerated =
        add_property_enumerated_value(&mut tx, "Grade", None, Some(vec![Value::Real(2.0)]), None)
            .expect("enumerated value");

    let count = create_quantity(&mut tx, QuantityKind::Count, "Doors", 4.0);
    let area = create_quantity_with(
        &mut tx,
        QuantityKind::Area,
        "GrossArea",
        12.5,
        QuantityExtras {
            description: Some("Painted"),
            unit: Some(metre),
            formula: Some("l * h"),
        },
    );

    let pset = add_property_set(
        &mut tx,
        "1aBcDeFgHiJkLmNoPqRsTu",
        "Pset_WallCommon",
        None,
        &[
            ("Height", height),
            ("Range", range),
            ("Layers", listed),
            ("Doc", referenced),
            ("Grade", enumerated),
        ],
    )
    .expect("property set");
    tx.commit(&mut model).expect("commit");

    let mut tx = Transaction::new(&model);
    attach_property_set(&mut tx, &model, "2aBcDeFgHiJkLmNoPqRsTu", &[wall], pset).expect("attach");
    tx.commit(&mut model).expect("commit");

    // Both quantity entry points reach the schema arity, not only the
    // one the rest of this test happens to exercise.
    let declared = ifc4().attributes("IFCQUANTITYCOUNT").len();
    for (id, path) in [(count, "create_quantity"), (area, "create_quantity_with")] {
        assert_eq!(
            model.get(id).expect("quantity").attributes.len(),
            declared,
            "{path} must write every declared attribute",
        );
    }

    assert_conformant(&model, "ifc-properties authoring");
}

/// `ifc-cost` and `ifc-schedule`: the two writers of
/// `IFCRELASSIGNSTOCONTROL`, plus their own `IFCRELNESTS`.
#[test]
fn cost_and_schedule_authoring_is_conformant() {
    use ifc::cost::mutation::{
        assign_schedule_items, create_cost_item, create_cost_schedule, create_monetary_unit,
        nest_cost_items, CostItemDraft, CostItemType, CostScheduleDraft, CostScheduleType,
        NestingDraft as CostNesting, ScheduleAssignmentDraft,
    };

    let mut model = model();
    let mut tx = Transaction::new(&model);
    create_monetary_unit(&mut tx, "EUR").expect("currency");
    let schedule = create_cost_schedule(
        &mut tx,
        &model,
        CostScheduleDraft {
            global_id: "0O2Fr$t4X7Zf8NOew3FLOH",
            name: Some("Tender"),
            predefined_type: Some(CostScheduleType::Estimate),
            ..Default::default()
        },
    )
    .expect("schedule");
    let root = create_cost_item(
        &mut tx,
        &model,
        CostItemDraft {
            global_id: "1O2Fr$t4X7Zf8NOew3FLOH",
            name: Some("Superstructure"),
            predefined_type: Some(CostItemType::NotDefined),
            ..Default::default()
        },
    )
    .expect("root");
    let child = create_cost_item(
        &mut tx,
        &model,
        CostItemDraft {
            global_id: "2O2Fr$t4X7Zf8NOew3FLOH",
            name: Some("Slab"),
            predefined_type: Some(CostItemType::NotDefined),
            ..Default::default()
        },
    )
    .expect("child");
    tx.commit(&mut model).expect("commit");

    let mut tx = Transaction::new(&model);
    nest_cost_items(
        &mut tx,
        &model,
        CostNesting {
            global_id: "3O2Fr$t4X7Zf8NOew3FLOH",
            parent: root,
            children: &[child],
        },
    )
    .expect("cost nesting");
    assign_schedule_items(
        &mut tx,
        &model,
        ScheduleAssignmentDraft {
            global_id: "4O2Fr$t4X7Zf8NOew3FLOH",
            schedule,
            items: &[root],
        },
    )
    .expect("assignment");
    tx.commit(&mut model).expect("commit");

    assert_conformant(&model, "ifc-cost authoring");
}

/// `ifc-schedule`, `ifc-systems` and `ifc-resource`: the other three
/// writers of `IFCRELNESTS`, each nesting a different thing.
#[test]
fn nesting_writers_are_conformant() {
    use ifc::resource::{NestingDraft, ResourceDraft, ResourceEditor, ResourceKind};
    use ifc::schedule::{assign_tasks_to_control, create_task, nest_tasks, TaskDraft};
    use ifc::systems::{
        assign_to_group, connect_port_to_element, connect_ports, create_port, create_system,
        nest_ports,
    };

    let mut model = model();
    let mut tx = Transaction::new(&model);

    // ifc-schedule nests tasks under a summary task.
    let mut task = |guid: &str, name: &str| {
        create_task(
            &mut tx,
            TaskDraft {
                global_id: guid,
                name: Some(name),
                is_milestone: false,
                ..TaskDraft::default()
            },
        )
        .expect("task")
    };
    let summary = task("0O2Fr$t4X7Zf8NOew3FLOH", "Structure");
    let leaf = task("1O2Fr$t4X7Zf8NOew3FLOH", "Pour slab");
    nest_tasks(&mut tx, "2O2Fr$t4X7Zf8NOew3FLOH", summary, &[leaf]).expect("task nesting");

    // IfcWorkSchedule requires CreationDate and StartTime; this stub
    // stands in for an ifc-schedule product, so it states them.
    let control = {
        let mut attrs = vec![Value::Null; ifc4().attributes("IFCWORKSCHEDULE").len()];
        attrs[0] = Value::Text("3O2Fr$t4X7Zf8NOew3FLOH".into());
        attrs[6] = Value::Text("2026-09-19T00:00:00".into());
        attrs[11] = Value::Text("2026-09-19T00:00:00".into());
        tx.create(Entity::new("IFCWORKSCHEDULE", attrs))
    };
    assign_tasks_to_control(&mut tx, "4O2Fr$t4X7Zf8NOew3FLOH", control, &[summary])
        .expect("task assignment");

    // ifc-systems nests ports under a distribution element.
    let system =
        create_system(&mut tx, "5O2Fr$t4X7Zf8NOew3FLOH", Some("Supply air")).expect("system");
    let duct = product(&mut tx, "IFCDUCTSEGMENT", "6O2Fr$t4X7Zf8NOew3FLOH");
    let inlet = create_port(&mut tx, "7O2Fr$t4X7Zf8NOew3FLOH", None, Some("SINK")).expect("inlet");
    let outlet =
        create_port(&mut tx, "8O2Fr$t4X7Zf8NOew3FLOH", None, Some("SOURCE")).expect("outlet");
    nest_ports(&mut tx, "9O2Fr$t4X7Zf8NOew3FLOH", duct, &[inlet, outlet]).expect("port nesting");
    assign_to_group(&mut tx, "5P2Fr$t4X7Zf8NOew3FLOH", system, &[duct]).expect("group assignment");
    connect_port_to_element(&mut tx, "6P2Fr$t4X7Zf8NOew3FLOH", inlet, duct)
        .expect("port attachment");
    connect_ports(&mut tx, "7P2Fr$t4X7Zf8NOew3FLOH", outlet, inlet, None).expect("port connection");
    tx.commit(&mut model).expect("commit");

    // ifc-resource resolves its slots by attribute name, not by index.
    let mut editor = ResourceEditor::for_model(&mut model).expect("editor");
    let crew = editor
        .create_resource(ResourceDraft::new(
            ResourceKind::Crew,
            "0P2Fr$t4X7Zf8NOew3FLOH",
        ))
        .expect("crew");
    let labour = editor
        .create_resource(ResourceDraft::new(
            ResourceKind::Labor,
            "1P2Fr$t4X7Zf8NOew3FLOH",
        ))
        .expect("labour");
    editor
        .create_nesting(NestingDraft::new(
            "2P2Fr$t4X7Zf8NOew3FLOH",
            crew,
            vec![labour],
        ))
        .expect("resource nesting");
    editor
        .create_resource_type(
            ResourceDraft::new(ResourceKind::Crew, "3P2Fr$t4X7Zf8NOew3FLOH")
                .predefined_type("OFFICE"),
        )
        .expect("resource type");

    assert_conformant(&model, "nesting authoring");
}

/// `ifc-georef` and `ifc-alignment`: the derived-attribute cases.
///
/// `IfcGeometricRepresentationSubContext` redeclares four inherited
/// attributes as DERIVE and `IfcSIUnit` derives `Dimensions`. The
/// validator distinguishes `*` from `$` on exactly those slots, which
/// is the check that would have caught both historical bugs.
#[test]
fn georeferencing_and_alignment_authoring_is_conformant() {
    use ifc::alignment::{
        alignment, axis2_placement_linear, cartesian_point, linear_placement, point_by_distance,
        referent, stationing,
    };
    use ifc::georef::{
        create_direction, create_projected_crs, create_representation_context,
        create_representation_subcontext, ProjectedCrsDraft,
    };
    use ifc::properties::{add_si_unit, SiUnitDraft};

    let mut model = model();
    let mut tx = Transaction::new(&model);

    let metre = add_si_unit(
        &mut tx,
        SiUnitDraft {
            unit_type: "LENGTHUNIT",
            prefix: None,
            name: "METRE",
        },
    )
    .expect("si unit");
    let origin = cartesian_point(&mut tx, &[0.0, 0.0, 0.0]).expect("origin");
    let placement = tx.create(Entity::new(
        "IFCAXIS2PLACEMENT3D",
        vec![Value::Ref(origin), Value::Null, Value::Null],
    ));
    let north = create_direction(&mut tx, &[0.0, 1.0]).expect("north");
    let context = create_representation_context(
        &mut tx,
        Some("Model"),
        3,
        Some(1e-5),
        placement,
        Some(north),
    )
    .expect("context");
    create_representation_subcontext(&mut tx, context, false, Some("Body"), "MODEL_VIEW", None)
        .expect("subcontext");
    create_projected_crs(
        &mut tx,
        ProjectedCrsDraft {
            name: "EPSG:25832",
            map_unit: Some(metre),
            ..ProjectedCrsDraft::default()
        },
    )
    .expect("crs");

    // ifc-alignment: a referent positioned along a curve, with its
    // stationing property set.
    let curve = tx.create(Entity::new("IFCPOLYLINE", vec![Value::List(vec![])]));
    alignment(&mut tx, "0Q2Fr$t4X7Zf8NOew3FLOH", Some("Main line"), None).expect("alignment");
    let point = point_by_distance(&mut tx, 125.0, (None, None, None), curve).expect("point");
    let axis = axis2_placement_linear(&mut tx, point, None, None).expect("axis");
    let linear = linear_placement(&mut tx, axis, None, None).expect("linear placement");
    let marker = referent(
        &mut tx,
        "1Q2Fr$t4X7Zf8NOew3FLOH",
        Some("KM 0+125"),
        Some("STATION"),
        Some(linear),
    )
    .expect("referent");
    stationing(
        &mut tx,
        "2Q2Fr$t4X7Zf8NOew3FLOH",
        "3Q2Fr$t4X7Zf8NOew3FLOH",
        marker,
        125.0,
        None,
        Some(true),
    )
    .expect("stationing");
    tx.commit(&mut model).expect("commit");

    assert_conformant(&model, "georef and alignment authoring");
}
/// `ifc-geometry` tessellation: meshes carried as indices, where the
/// validator checks the record shape the writer produced.
#[test]
#[cfg(feature = "geometry")]
fn tessellation_authoring_is_conformant() {
    use ifc::geometry::authoring::{
        cartesian_point_list_3d, indexed_polygonal_face, indexed_polygonal_face_with_voids,
        polygonal_face_set, triangulated_face_set, TriangulatedExtras,
    };

    let mut model = model();
    let mut tx = Transaction::new(&model);
    let pts = [
        [0.0, 0.0, 0.0],
        [4.0, 0.0, 0.0],
        [4.0, 4.0, 0.0],
        [0.0, 4.0, 0.0],
        [1.0, 1.0, 0.0],
        [2.0, 1.0, 0.0],
        [2.0, 2.0, 0.0],
        [1.0, 2.0, 0.0],
    ];
    let list = cartesian_point_list_3d(&mut tx, &pts, None).expect("points");
    let _mesh = triangulated_face_set(
        &mut tx,
        list,
        pts.len(),
        &[[0, 1, 2], [0, 2, 3]],
        TriangulatedExtras {
            closed: Some(true),
            ..TriangulatedExtras::default()
        },
    )
    .expect("mesh");
    let plain = indexed_polygonal_face(&mut tx, &[0, 1, 2, 3], pts.len()).expect("face");
    let holed =
        indexed_polygonal_face_with_voids(&mut tx, &[0, 1, 2, 3], &[&[4, 5, 6, 7]], pts.len())
            .expect("holed");
    polygonal_face_set(&mut tx, list, pts.len(), &[plain, holed], Some(true), None)
        .expect("face set");
    tx.commit(&mut model).expect("commit");

    assert_conformant(&model, "ifc-geometry tessellation authoring");
}
