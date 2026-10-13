//! Removing a product with what it solely owns, and purging what nothing
//! uses (#478), over one model in IFC2X3, IFC4 and IFC4X3.
//!
//! `test/fixtures/synthetic-relations/cascade_*.ifc`, the same ids in the
//! three releases (IFC2X3 adds `IfcPresentationStyleAssignment`s #300 and
//! #301 between the styled items and the surface style, and connects the
//! port with `IfcRelConnectsPortToElement` where the others nest it):
//!
//! ```text
//! #20 Wall A      placement #21 (rotated a quarter turn), body #24..#28,
//!                 styled by #33; own pset #64 (#65), shared pset #61 (#62,
//!                 with Wall B), typed by #66 (with Wall B), material
//!                 usage #73 (#74) of layer set #72, classified by #78 (#79)
//! #40 Wall B      shares #61, #66, the layer set #72, the style #34 and the
//!                 layer #80 with Wall A
//! #81, #92        openings voiding Wall A (#91, #96), placed relative to #21
//! #100 Door 1     fills #81 (#109), placed relative to #82, typed by #111,
//!                 whose representation map #112 its mapped item #106 maps
//! #130 Window 1   fills #92 (#134), placed relative to #93, typed by #135
//! #150 Pipe       with its port #154 (#158), placed relative to #151
//! #170..#172      a profile nothing uses; #173 a property set nothing uses
//! ```

#![cfg(all(
    feature = "step",
    feature = "author",
    feature = "validate",
    feature = "ifc2x3",
    feature = "ifc4",
    feature = "ifc4x3"
))]

use ifc::{
    apply_authoring, stage_authoring, AuthorOp, AuthoringFailure, Codec, EntityId, KeepReason,
    Model, PurgeOptions, RemovalReport, RemoveProductOptions, Schema, SchemaVersion, StepCodec,
    Value,
};

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

fn each_release(mut check: impl FnMut(&str, Model, &'static Schema)) {
    for (name, version) in FIXTURES {
        let model = read(name);
        let schema = ifc::schema::for_version(version).expect("bundled");
        assert_valid(name, &model, schema);
        check(name, model, schema);
    }
}

/// The model has no validation error.
fn assert_valid(what: &str, model: &Model, schema: &Schema) {
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
}

fn remove_product(product: u64, options: RemoveProductOptions) -> AuthorOp {
    AuthorOp::RemoveProduct {
        product: EntityId(product),
        options,
    }
}

/// Apply `ops` and return the report of each.
fn apply(model: &mut Model, schema: &Schema, ops: &[AuthorOp]) -> Vec<RemovalReport> {
    let outcome = apply_authoring(model, schema, ops, 478).expect("applies");
    outcome.reports.into_iter().map(Option::unwrap).collect()
}

fn refusal(model: &mut Model, schema: &Schema, op: AuthorOp) -> AuthoringFailure {
    let before = model.revision();
    let error = apply_authoring(model, schema, &[op], 478).expect_err("refused");
    assert_eq!(model.revision(), before, "a refusal leaves the model alone");
    *error.failure
}

fn removed(report: &RemovalReport) -> Vec<u64> {
    report.removed.iter().map(|entity| entity.id.0).collect()
}

fn kept(report: &RemovalReport, id: u64) -> (KeepReason, Vec<u64>) {
    let entity = report
        .kept
        .iter()
        .find(|entity| entity.id.0 == id)
        .unwrap_or_else(|| panic!("#{id} is not reported as kept: {:#?}", report.kept));
    (
        entity.reason,
        entity.users.iter().map(|user| user.0).collect(),
    )
}

fn refs(model: &Model, id: u64, slot: usize) -> Vec<u64> {
    let mut found = Vec::new();
    model.get(EntityId(id)).expect("present").attributes[slot]
        .for_each_ref(&mut |target| found.push(target.0));
    found
}

fn reals(model: &Model, id: u64) -> Vec<f64> {
    model.get(EntityId(id)).expect("present").attributes[0]
        .as_list()
        .expect("a list")
        .iter()
        .map(|value| value.as_f64().expect("a number"))
        .collect()
}

fn rebase() -> RemoveProductOptions {
    RemoveProductOptions::new().rebase_placements(true)
}

#[test]
fn a_wall_goes_with_its_openings_and_what_it_alone_owns() {
    each_release(|name, mut model, schema| {
        // The door and the window are placed relative to the openings,
        // which are relative to the wall: nothing is removed without
        // re-basing them.
        assert_eq!(
            refusal(
                &mut model,
                schema,
                remove_product(20, RemoveProductOptions::new())
            ),
            AuthoringFailure::PlacementHasDependents {
                placement: EntityId(82),
                dependents: vec![EntityId(101)],
            },
            "{name}"
        );

        let report = &apply(&mut model, schema, &[remove_product(20, rebase())])[0];
        let mut expected = vec![
            20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 32, 33, 63, 64, 65, 73, 74, 79, 81, 82, 83,
            84, 85, 86, 87, 88, 89, 90, 91, 92, 93, 94, 95, 96, 102, 103, 109, 132, 133, 134,
        ];
        if name.contains("2x3") {
            expected.push(300);
        }
        assert_eq!(removed(report), expected, "{name}");
        assert_eq!(report.rebased, vec![EntityId(101), EntityId(131)], "{name}");
        assert_eq!(
            report.detached,
            [19, 62, 67, 80].map(EntityId).to_vec(),
            "{name}"
        );

        // Shared data stays, and says who shares it.
        assert_eq!(kept(report, 61), (KeepReason::Shared, vec![62]), "{name}");
        assert_eq!(kept(report, 9).0, KeepReason::Shared, "{name}");
        assert_eq!(
            kept(report, 31),
            (KeepReason::Shared, vec![46, 115]),
            "{name}"
        );
        assert_eq!(kept(report, 18).0, KeepReason::Shared, "{name}");
        let style_user = if name.contains("2x3") { 301 } else { 50 };
        assert_eq!(
            kept(report, 34),
            (KeepReason::Shared, vec![style_user]),
            "{name}"
        );
        // Objects of their own and definitions are never cascaded into.
        for object in [66, 100, 130] {
            assert_eq!(
                kept(report, object).0,
                KeepReason::RelatedObject,
                "{name} #{object}"
            );
        }
        for definition in [5, 11, 72, 78] {
            assert_eq!(
                kept(report, definition).0,
                KeepReason::Definition,
                "{name} #{definition}"
            );
        }

        // The shared property set now defines Wall B alone; the type and
        // the layer keep Wall B; the storey keeps the door and the window.
        assert_eq!(refs(&model, 62, 4), vec![40], "{name}");
        assert_eq!(refs(&model, 67, 4), vec![40], "{name}");
        assert_eq!(refs(&model, 80, 2), vec![45], "{name}");
        assert_eq!(refs(&model, 19, 4), vec![40, 100, 130, 150], "{name}");

        // The door and the window did not move: re-based onto the storey
        // with the wall's quarter turn and the openings' offsets composed.
        assert_eq!(refs(&model, 101, 0), vec![18], "{name}");
        let door_axes = refs(&model, 101, 1)[0];
        let door = model.get(EntityId(door_axes)).unwrap();
        let point = door.attributes[0].as_ref_id().unwrap().0;
        assert_eq!(reals(&model, point), vec![0.5, 1.0, 0.0], "{name}");
        let axis = door.attributes[1].as_ref_id().unwrap().0;
        let ref_direction = door.attributes[2].as_ref_id().unwrap().0;
        assert_eq!(reals(&model, axis), vec![0.0, 0.0, 1.0], "{name}");
        assert_eq!(reals(&model, ref_direction), vec![0.0, 1.0, 0.0], "{name}");
        let window_axes = refs(&model, 131, 1)[0];
        let window_point = model.get(EntityId(window_axes)).unwrap().attributes[0]
            .as_ref_id()
            .unwrap()
            .0;
        assert_eq!(reals(&model, window_point), vec![1.0, 3.0, 1.0], "{name}");

        assert_valid(name, &model, schema);
    });
}

#[test]
fn a_door_goes_and_its_types_representation_maps_stay() {
    each_release(|name, mut model, schema| {
        let report = &apply(
            &mut model,
            schema,
            &[remove_product(100, RemoveProductOptions::new())],
        )[0];
        assert_eq!(
            removed(report),
            vec![100, 101, 102, 103, 104, 105, 106, 107, 108, 109, 120],
            "{name}"
        );
        assert_eq!(kept(report, 112), (KeepReason::Shared, vec![111]), "{name}");
        assert_eq!(kept(report, 111).0, KeepReason::RelatedObject, "{name}");
        assert_eq!(kept(report, 81).0, KeepReason::RelatedObject, "{name}");
        for map in [111, 112, 113, 114, 115] {
            assert!(model.get(EntityId(map)).is_some(), "{name} #{map}");
        }
        assert_valid(name, &model, schema);
    });
}

#[test]
fn a_type_with_occurrences_is_refused_unless_they_are_untyped() {
    each_release(|name, mut model, schema| {
        assert_eq!(
            refusal(
                &mut model,
                schema,
                remove_product(66, RemoveProductOptions::new())
            ),
            AuthoringFailure::TypeHasOccurrences {
                type_object: EntityId(66),
                occurrences: vec![EntityId(20), EntityId(40)],
            },
            "{name}"
        );
        let report = &apply(
            &mut model,
            schema,
            &[remove_product(
                66,
                RemoveProductOptions::new().untype_occurrences(true),
            )],
        )[0];
        // The type, its relationship and its own property set.
        assert_eq!(removed(report), vec![66, 67, 68, 69], "{name}");
        assert!(ifc::type_of(&model, schema, EntityId(20))
            .unwrap()
            .is_none());

        // A type with representation maps its occurrence still maps: the
        // maps stay, shared with the mapped item.
        let report = &apply(
            &mut model,
            schema,
            &[remove_product(
                111,
                RemoveProductOptions::new().untype_occurrences(true),
            )],
        )[0];
        assert_eq!(removed(report), vec![111, 120], "{name}");
        assert_eq!(kept(report, 112), (KeepReason::Shared, vec![106]), "{name}");
        assert_valid(name, &model, schema);
    });
}

#[test]
fn a_pipe_goes_with_its_port() {
    each_release(|name, mut model, schema| {
        let report = &apply(
            &mut model,
            schema,
            &[remove_product(150, RemoveProductOptions::new())],
        )[0];
        assert_eq!(
            removed(report),
            vec![150, 151, 152, 153, 154, 155, 156, 157, 158],
            "{name}"
        );
        assert_valid(name, &model, schema);
    });
}

#[test]
fn only_products_and_type_objects_are_removed_this_way() {
    each_release(|name, mut model, schema| {
        assert!(
            matches!(
                refusal(
                    &mut model,
                    schema,
                    remove_product(61, RemoveProductOptions::new())
                ),
                AuthoringFailure::NotA { .. }
            ),
            "{name}"
        );
        assert_eq!(
            refusal(
                &mut model,
                schema,
                remove_product(9999, RemoveProductOptions::new())
            ),
            AuthoringFailure::MissingEntity(EntityId(9999)),
            "{name}"
        );
    });
}

#[test]
fn a_purge_dry_run_reports_and_changes_nothing() {
    each_release(|name, model, schema| {
        let staged = stage_authoring(&model, schema, &[AuthorOp::Purge(PurgeOptions::new())], 478)
            .expect("stages");
        let report = staged.reports[0].as_ref().expect("a report");
        assert_eq!(removed(report), vec![170, 171, 172], "{name}");
        assert!(model.get(EntityId(170)).is_some());

        // By class, a property set is an IfcRoot: kept unless asked.
        let staged = stage_authoring(
            &model,
            schema,
            &[AuthorOp::Purge(
                PurgeOptions::new().classes(["IfcPropertySet"]),
            )],
            478,
        )
        .expect("stages");
        let report = staged.reports[0].as_ref().expect("a report");
        assert!(report.removed.is_empty(), "{name}");
        assert_eq!(kept(report, 173).0, KeepReason::Protected, "{name}");
    });
}

#[test]
fn a_purge_removes_what_nothing_uses() {
    each_release(|name, mut model, schema| {
        let reports = apply(
            &mut model,
            schema,
            &[
                AuthorOp::Purge(
                    PurgeOptions::new()
                        .classes(["ifcpropertyset"])
                        .include_roots(true),
                ),
                AuthorOp::Purge(PurgeOptions::new()),
            ],
        );
        assert_eq!(removed(&reports[0]), vec![173], "{name}");
        assert_eq!(removed(&reports[1]), vec![170, 171, 172, 174], "{name}");
        assert_valid(name, &model, schema);

        // Both walls gone: their material and classification are unused
        // definitions now, which the cascade keeps and the purge takes.
        let reports = apply(
            &mut model,
            schema,
            &[
                remove_product(20, rebase()),
                remove_product(40, rebase()),
                AuthorOp::Purge(PurgeOptions::new()),
            ],
        );
        assert_eq!(kept(&reports[1], 72).0, KeepReason::Definition, "{name}");
        let purged = removed(&reports[2]);
        for orphan in [70, 71, 72, 77, 78] {
            assert!(purged.contains(&orphan), "{name}: #{orphan} in {purged:?}");
        }
        assert!(!purged.contains(&5), "{name}: the owner history is used");
        assert_valid(name, &model, schema);
    });
}

#[test]
fn a_purge_refuses_an_unknown_class() {
    each_release(|name, mut model, schema| {
        assert!(
            matches!(
                refusal(
                    &mut model,
                    schema,
                    AuthorOp::Purge(PurgeOptions::new().classes(["IfcNoSuchThing"]))
                ),
                AuthoringFailure::UnknownEntity { .. }
            ),
            "{name}"
        );
    });
}

#[test]
fn a_cascade_sees_what_the_batch_did_before_it() {
    each_release(|name, mut model, schema| {
        // A product the batch creates, then removes: nothing is left.
        let before = model.len();
        let outcome = apply_authoring(
            &mut model,
            schema,
            &[
                AuthorOp::Placement {
                    relative_to: Some(EntityId(18)),
                    location: [1.0, 2.0, 3.0],
                    axes: None,
                },
                AuthorOp::Product {
                    type_name: "IfcWall".into(),
                    container: Some(EntityId(14)),
                    attributes: vec![("Name".into(), Value::Text("Temporary".into()))],
                    placement: Some(ifc::authoring_handle(0)),
                    type_object: Some(EntityId(66)),
                    owner_history: Some(EntityId(5)),
                },
                AuthorOp::RemoveProduct {
                    product: ifc::authoring_handle(1),
                    options: RemoveProductOptions::new(),
                },
            ],
            478,
        )
        .expect("applies");
        assert_eq!(model.len(), before, "{name}");
        assert_eq!(
            outcome.reports[2].as_ref().unwrap().removed.len(),
            6,
            "{name}"
        );
        assert_valid(name, &model, schema);
    });
}
