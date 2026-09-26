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
            global_id: "04OFr$t4X7Zf8NOew3FLOH",
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
    assign_tasks_to_control(&mut tx, "04OFr$t4X7Zf8NOew3FLOH", control, &[summary])
        .expect("task assignment");

    // ifc-systems nests ports under a distribution element.
    let system =
        create_system(&mut tx, "05OFr$t4X7Zf8NOew3FLOH", Some("Supply air")).expect("system");
    let duct = product(&mut tx, "IFCDUCTSEGMENT", "06OFr$t4X7Zf8NOew3FLOH");
    let inlet = create_port(&mut tx, "07OFr$t4X7Zf8NOew3FLOH", None, Some("SINK")).expect("inlet");
    let outlet =
        create_port(&mut tx, "08OFr$t4X7Zf8NOew3FLOH", None, Some("SOURCE")).expect("outlet");
    nest_ports(&mut tx, "09OFr$t4X7Zf8NOew3FLOH", duct, &[inlet, outlet]).expect("port nesting");
    assign_to_group(&mut tx, "05PFr$t4X7Zf8NOew3FLOH", system, &[duct]).expect("group assignment");
    connect_port_to_element(&mut tx, "06PFr$t4X7Zf8NOew3FLOH", inlet, duct)
        .expect("port attachment");
    connect_ports(&mut tx, "07PFr$t4X7Zf8NOew3FLOH", outlet, inlet, None).expect("port connection");
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

/// The four geometry families added after tessellation: CSG primitives,
/// curves, the remaining swept solids, and B-rep topology.
///
/// The B-rep case is the one this oracle earns its keep on: an
/// `IfcOrientedEdge` writes its inherited vertices as `*`, and the
/// validator's derived-slot check is what tells `*` from `$`. A
/// round-trip cannot: both decode to the same Rust value.
#[test]
fn geometry_authoring_is_conformant() {
    use ifc::geometry::authoring::{
        axis1_placement, axis2_placement_3d, block, cartesian_point, circle, cylinder, direction,
        edge_curve, face, face_outer_bound, line, manifold_solid_brep, oriented_edge, poly_loop,
        shell, sphere, swept_disk_solid, trimmed_curve, vertex_point, BrepKind, ShellKind,
        SweepTrim,
    };

    let model = model();
    let mut tx = Transaction::new(&model);

    let origin = cartesian_point(&mut tx, &[0.0, 0.0, 0.0]).expect("origin");
    let up = direction(&mut tx, &[0.0, 0.0, 1.0]).expect("up");
    let placement = axis2_placement_3d(&mut tx, origin, Some(up), None);

    // CSG primitives.
    block(&mut tx, placement, 1.0, 2.0, 3.0).expect("block");
    sphere(&mut tx, placement, 1.5).expect("sphere");
    cylinder(&mut tx, placement, 2.0, 0.5).expect("cylinder");

    // Curves, including a trim that must keep its measure wrapper.
    let arc = circle(&mut tx, placement, 1.0).expect("circle");
    trimmed_curve(
        &mut tx,
        arc,
        ifc::geometry::curve::Trim {
            cartesian: None,
            parameter: Some(0.0),
        },
        ifc::geometry::curve::Trim {
            cartesian: None,
            parameter: Some(1.5),
        },
        true,
        ifc::geometry::curve::TrimmingPreference::Parameter,
    )
    .expect("trimmed");

    // A swept disk over that curve.
    swept_disk_solid(&mut tx, arc, 0.2, Some(0.1), SweepTrim::default()).expect("disk");

    // B-rep topology: the oriented edge is the DERIVE case.
    let p1 = cartesian_point(&mut tx, &[1.0, 0.0, 0.0]).expect("p1");
    let p2 = cartesian_point(&mut tx, &[0.0, 1.0, 0.0]).expect("p2");
    let v0 = vertex_point(&mut tx, origin);
    let v1 = vertex_point(&mut tx, p1);
    let along = ifc::geometry::authoring::vector(&mut tx, up, 1.0).expect("vector");
    let geometry = line(&mut tx, origin, along);
    let straight = edge_curve(&mut tx, v0, v1, geometry, true);
    oriented_edge(&mut tx, straight, false);
    let ring = poly_loop(&mut tx, &[origin, p1, p2]).expect("loop");
    let bound = face_outer_bound(&mut tx, ring, true);
    let facet = face(&mut tx, &[bound]).expect("face");
    let hull = shell(&mut tx, ShellKind::Closed, &[facet]).expect("shell");
    manifold_solid_brep(&mut tx, BrepKind::Faceted, hull, &[]);

    let axis = axis1_placement(&mut tx, origin, Some(up));
    let _ = axis;

    let mut model = model;
    tx.commit(&mut model).expect("commit");
    assert_conformant(&model, "ifc-geometry authoring");
}

/// The later geometry families: surfaces, the remaining profiles,
/// transformation operators, curves on surfaces, and connection
/// geometry.
///
/// `IfcMirroredProfileDef` is the case this oracle is here for: its
/// `Operator` is DERIVE and must serialize as `*`. A round-trip cannot
/// tell that from `$`.
#[test]
#[cfg(feature = "geometry")]
fn later_geometry_authoring_is_conformant() {
    use ifc::geometry::authoring::{
        arbitrary_profile_with_voids, axis2_placement_3d, cartesian_point, circle,
        composite_curve_on_surface, composite_curve_segment, connection_geometry, direction,
        geometric_set, grid_axis, local_placement, mirrored_profile, pcurve, plane, point_on_curve,
        polyline, rectangle_profile, rounded_rectangle_profile, spherical_surface, surface_curve,
        toroidal_surface, transformation_operator_3d, virtual_grid_intersection, ConnectionKind,
        OnSurfaceKind, ProfileType, SurfaceCurveKind, SurfaceCurveRepresentation, Transform,
    };

    let mut model = model();
    let mut tx = Transaction::new(&model);

    let o = cartesian_point(&mut tx, &[0.0, 0.0, 0.0]).expect("origin");
    let up = direction(&mut tx, &[0.0, 0.0, 1.0]).expect("up");
    let at = axis2_placement_3d(&mut tx, o, Some(up), None);

    // Surfaces.
    let flat = plane(&mut tx, at);
    spherical_surface(&mut tx, at, 1.5).expect("sphere");
    toroidal_surface(&mut tx, at, 4.0, 1.0).expect("torus");

    // Profiles, including the DERIVE case.
    let parent = rectangle_profile(&mut tx, Some("P"), None, 0.3, 0.2).expect("parent");
    mirrored_profile(&mut tx, ProfileType::Area, Some("M"), parent, None);
    rounded_rectangle_profile(&mut tx, Some("R"), None, 4.0, 2.0, 0.5).expect("rounded");
    let a = cartesian_point(&mut tx, &[0.0, 0.0]).expect("a");
    let b = cartesian_point(&mut tx, &[1.0, 0.0]).expect("b");
    let c = cartesian_point(&mut tx, &[1.0, 1.0]).expect("c");
    let outer = polyline(&mut tx, &[a, b, c, a]).expect("outer");
    let inner = polyline(&mut tx, &[a, b, c, a]).expect("inner");
    arbitrary_profile_with_voids(&mut tx, Some("V"), outer, &[inner]).expect("voided");

    // Transformation operator.
    transformation_operator_3d(
        &mut tx,
        o,
        Transform {
            scale: Some(2.0),
            ..Transform::default()
        },
        Some(up),
    )
    .expect("operator");

    // Curves on surfaces.
    let arc = circle(&mut tx, at, 1.0).expect("arc");
    let on_plane = pcurve(&mut tx, flat, arc);
    surface_curve(
        &mut tx,
        SurfaceCurveKind::Plain,
        arc,
        &[on_plane],
        SurfaceCurveRepresentation::Curve3D,
    )
    .expect("surface curve");
    let segment = composite_curve_segment(
        &mut tx,
        ifc::geometry::curve::TransitionCode::Continuous,
        true,
        arc,
    );
    composite_curve_on_surface(&mut tx, OnSurfaceKind::Boundary, &[segment])
        .expect("boundary curve");

    // Placement, grids, connection geometry, sets.
    local_placement(&mut tx, None, at);
    let u = grid_axis(&mut tx, Some("A"), outer, true);
    let v = grid_axis(&mut tx, Some("1"), outer, false);
    virtual_grid_intersection(&mut tx, &[u, v], &[0.0, 0.0]).expect("intersection");
    connection_geometry(&mut tx, ConnectionKind::Curve, arc, None);
    point_on_curve(&mut tx, arc, 0.5).expect("point on curve");
    geometric_set(&mut tx, true, &[arc]).expect("curve set");

    tx.commit(&mut model).expect("commit");
    assert_conformant(&model, "ifc-geometry later authoring");
}
