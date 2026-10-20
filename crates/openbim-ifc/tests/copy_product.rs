//! Copying a product with what it solely owns, and the low-level deep copy
//! (#479), over the cascade fixtures in IFC2X3, IFC4 and IFC4X3.
//!
//! `test/fixtures/synthetic-relations/cascade_*.ifc` (see
//! `cascade_delete.rs` for the full map): Wall A #20, placed by #21 with
//! body #24..#28 styled by #33, own pset #64 (#65), pset #61 shared with
//! Wall B (#62), typed by #66 (#67, with Wall B), material usage #73 (#74)
//! of layer set #72, classified by #78 (#79), on layer #80, contained by
//! #19; openings #81 (#91) and #92 (#96), filled by Door 1 #100 (#109) and
//! Window 1 #130 (#134); the door typed by #111 (#120), whose map #112 its
//! mapped item #106 maps; the pipe #150 with its port #154 (#158).

#![cfg(all(
    feature = "step",
    feature = "author",
    feature = "validate",
    feature = "ifc2x3",
    feature = "ifc4",
    feature = "ifc4x3"
))]

use ifc::{
    apply_authoring, authoring_handle, stage_authoring, AuthorOp, AuthoringFailure, Codec,
    CopyPlacement, CopyPolicy, CopyProductOptions, CopyReport, EntityId, Model, Schema,
    SchemaVersion, StepCodec, Value,
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
        check(name, model, schema);
    }
}

/// The model has no validation error and nothing dangles.
fn assert_valid(what: &str, model: &Model, schema: &Schema) {
    // With INVERSE cardinalities (#533): a copy joins its type's
    // IfcRelDefinesByType rather than adding a second one.
    let options = ifc::validate::ValidateOptions::new().inverse_cardinality(true);
    let report = ifc::validate::validate_with_options(model, schema, &options);
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

fn copy(product: u64, options: CopyProductOptions) -> AuthorOp {
    AuthorOp::CopyProduct {
        product: EntityId(product),
        options,
    }
}

/// Apply one copy; the copy and its report.
fn apply_copy(model: &mut Model, schema: &Schema, op: AuthorOp) -> (EntityId, CopyReport) {
    let outcome = apply_authoring(model, schema, &[op], 479).expect("applies");
    let report = outcome.copies[0].clone().expect("a copy report");
    (outcome.ids[0].expect("the copy"), report)
}

fn refusal(model: &mut Model, schema: &Schema, op: AuthorOp) -> AuthoringFailure {
    let before = model.revision();
    let error = apply_authoring(model, schema, &[op], 479).expect_err("refused");
    assert_eq!(model.revision(), before, "a refusal leaves the model alone");
    *error.failure
}

fn slot(schema: &Schema, model: &Model, id: EntityId, name: &str) -> usize {
    let entity = model.get(id).expect("present");
    schema
        .attributes(&entity.type_name)
        .iter()
        .position(|attribute| attribute.name.eq_ignore_ascii_case(name))
        .unwrap_or_else(|| panic!("{} has no {name}", entity.type_name))
}

fn reference(schema: &Schema, model: &Model, id: EntityId, name: &str) -> Option<EntityId> {
    let at = slot(schema, model, id, name);
    model.get(id).expect("present").attributes[at].as_ref_id()
}

fn references(schema: &Schema, model: &Model, id: EntityId, name: &str) -> Vec<EntityId> {
    let at = slot(schema, model, id, name);
    let mut found = Vec::new();
    model.get(id).expect("present").attributes[at].for_each_ref(&mut |target| found.push(target));
    found
}

fn global_id(model: &Model, id: EntityId) -> String {
    model.get(id).expect("present").attributes[0]
        .as_text()
        .expect("a GlobalId")
        .to_owned()
}

fn copy_of(report: &CopyReport, original: u64) -> EntityId {
    report
        .copy_of(EntityId(original))
        .unwrap_or_else(|| panic!("#{original} was not copied: {report:#?}"))
}

fn copied(report: &CopyReport, original: u64) -> bool {
    report.copy_of(EntityId(original)).is_some()
}

fn coordinates(model: &Model, point: EntityId) -> Vec<f64> {
    model.get(point).expect("present").attributes[0]
        .as_list()
        .expect("a list")
        .iter()
        .map(|value| value.as_f64().expect("a number"))
        .collect()
}

#[test]
fn a_wall_copies_what_it_owns_and_joins_what_it_shares() {
    each_release(|name, mut model, schema| {
        let (wall, report) = apply_copy(&mut model, schema, copy(20, CopyProductOptions::new()));
        assert_valid(name, &model, schema);
        assert_eq!(copy_of(&report, 20), wall, "{name}");
        assert_ne!(global_id(&model, wall), global_id(&model, EntityId(20)));
        assert_eq!(
            model.get(wall).unwrap().attributes[2],
            Value::Text("Wall A".into())
        );

        // Placement: a copy, relative to the same storey placement.
        let placement = reference(schema, &model, wall, "ObjectPlacement").unwrap();
        assert_eq!(placement, copy_of(&report, 21), "{name}");
        assert_eq!(
            reference(schema, &model, placement, "PlacementRelTo"),
            Some(EntityId(18))
        );
        // Representation: copied by #481's rules, the context shared.
        let shape = reference(schema, &model, wall, "Representation").unwrap();
        assert_eq!(shape, copy_of(&report, 24));
        for original in [25, 26, 27, 33] {
            assert!(copied(&report, original), "{name}: #{original}");
        }
        let body = references(schema, &model, shape, "Representations")[0];
        assert_eq!(
            reference(schema, &model, body, "ContextOfItems"),
            Some(EntityId(11))
        );
        // Own property set copied, with its relationship.
        for original in [63, 64, 65] {
            assert!(copied(&report, original), "{name}: #{original}");
        }
        assert_eq!(
            references(schema, &model, copy_of(&report, 65), "RelatedObjects"),
            [wall]
        );
        // Shared property set, type, classification, container, layer:
        // joined, not copied.
        for shared in [61, 62, 66, 67, 78, 79, 19, 80] {
            assert!(!copied(&report, shared), "{name}: #{shared} copied");
        }
        for joined in [62, 67, 79, 19, 80] {
            assert!(
                report.joined.contains(&EntityId(joined)),
                "{name}: #{joined}"
            );
        }
        assert!(references(schema, &model, EntityId(62), "RelatedObjects").contains(&wall));
        assert!(references(schema, &model, EntityId(67), "RelatedObjects").contains(&wall));
        assert!(references(schema, &model, EntityId(79), "RelatedObjects").contains(&wall));
        assert!(references(schema, &model, EntityId(19), "RelatedElements").contains(&wall));
        // Material usage copied, the layer set shared.
        let usage = copy_of(&report, 73);
        assert_eq!(
            reference(schema, &model, usage, "ForLayerSet"),
            Some(EntityId(72))
        );
        assert_eq!(
            reference(schema, &model, copy_of(&report, 74), "RelatingMaterial"),
            Some(usage)
        );
        // Openings copied, relative to the copied placement, voiding the
        // copy; their fillings copied into them.
        for (opening, voids) in [(81, 91), (92, 96)] {
            let copied_opening = copy_of(&report, opening);
            let opening_placement =
                reference(schema, &model, copied_opening, "ObjectPlacement").unwrap();
            assert_eq!(
                reference(schema, &model, opening_placement, "PlacementRelTo"),
                Some(placement)
            );
            let rel = copy_of(&report, voids);
            assert_eq!(
                reference(schema, &model, rel, "RelatingBuildingElement"),
                Some(wall)
            );
        }
        let door = copy_of(&report, 100);
        assert_eq!(
            reference(
                schema,
                &model,
                copy_of(&report, 109),
                "RelatedBuildingElement"
            ),
            Some(door)
        );
        assert_eq!(
            reference(
                schema,
                &model,
                copy_of(&report, 109),
                "RelatingOpeningElement"
            ),
            Some(copy_of(&report, 81))
        );
        // The door joins its type, and its mapped item still maps the
        // type's representation map.
        assert!(references(schema, &model, EntityId(120), "RelatedObjects").contains(&door));
        assert_eq!(
            reference(schema, &model, copy_of(&report, 106), "MappingSource"),
            Some(EntityId(112))
        );
        assert!(copied(&report, 130) && copied(&report, 134), "{name}");
        // The original keeps everything.
        assert_eq!(
            references(schema, &model, EntityId(19), "RelatedElements").len(),
            8,
            "{name}: wall, door and window copies joined the container"
        );
        assert!(report.skipped.is_empty(), "{name}: {:?}", report.skipped);
    });
}

#[test]
fn a_door_alone_shares_its_type_geometry_and_leaves_its_opening() {
    each_release(|name, mut model, schema| {
        let (door, report) = apply_copy(&mut model, schema, copy(100, CopyProductOptions::new()));
        assert_valid(name, &model, schema);
        let item = copy_of(&report, 106);
        assert_eq!(
            reference(schema, &model, item, "MappingSource"),
            Some(EntityId(112)),
            "{name}: the type's representation map is shared"
        );
        assert!(!copied(&report, 112) && !copied(&report, 114), "{name}");
        assert_ne!(
            reference(schema, &model, item, "MappingTarget"),
            Some(EntityId(107))
        );
        assert_eq!(
            report.skipped,
            [EntityId(109)],
            "{name}: the filled opening"
        );
        assert!(report.joined.contains(&EntityId(120)), "{name}");
        // The door was relative to the opening's placement: so is the copy.
        let placement = reference(schema, &model, door, "ObjectPlacement").unwrap();
        assert_eq!(
            reference(schema, &model, placement, "PlacementRelTo"),
            Some(EntityId(82))
        );
    });
}

#[test]
fn a_type_copies_its_representation_maps_but_not_its_occurrences() {
    each_release(|name, mut model, schema| {
        let (door_type, report) =
            apply_copy(&mut model, schema, copy(111, CopyProductOptions::new()));
        assert_valid(name, &model, schema);
        let maps = references(schema, &model, door_type, "RepresentationMaps");
        assert_eq!(maps, [copy_of(&report, 112)], "{name}");
        assert_eq!(
            reference(schema, &model, maps[0], "MappedRepresentation"),
            Some(copy_of(&report, 114))
        );
        assert_eq!(report.skipped, [EntityId(120)], "{name}: the occurrences");
        // The wall type's own property set is copied with it.
        let (wall_type, report) =
            apply_copy(&mut model, schema, copy(66, CopyProductOptions::new()));
        assert_eq!(
            references(schema, &model, wall_type, "HasPropertySets"),
            [copy_of(&report, 68)],
            "{name}"
        );
        assert_valid(name, &model, schema);
    });
}

#[test]
fn repeated_copies_stay_valid_and_distinct() {
    each_release(|name, mut model, schema| {
        let ops = [
            copy(20, CopyProductOptions::new()),
            copy(20, CopyProductOptions::new()),
            copy(150, CopyProductOptions::new()),
        ];
        let outcome = apply_authoring(&mut model, schema, &ops, 479).expect("applies");
        let (first, second) = (outcome.ids[0].unwrap(), outcome.ids[1].unwrap());
        assert_ne!(global_id(&model, first), global_id(&model, second));
        let (again, _) = apply_copy(&mut model, schema, copy(first.0, CopyProductOptions::new()));
        assert_valid(name, &model, schema);
        let typed = references(schema, &model, EntityId(67), "RelatedObjects");
        for wall in [first, second, again] {
            assert!(typed.contains(&wall), "{name}");
        }
        // The pipe's port is copied and nested in (or connected to) the copy.
        let pipe = outcome.copies[2].as_ref().unwrap();
        assert!(copied(pipe, 154) && copied(pipe, 158), "{name}");
        assert_eq!(
            model.get(copy_of(pipe, 158)).unwrap().type_name,
            model.get(EntityId(158)).unwrap().type_name
        );
    });
}

#[test]
fn an_offset_moves_the_copy_and_what_hangs_on_it() {
    each_release(|name, mut model, schema| {
        let options = CopyProductOptions::new().placement(CopyPlacement::Offset([0.0, 10.0, 0.5]));
        let (wall, report) = apply_copy(&mut model, schema, copy(20, options));
        assert_valid(name, &model, schema);
        let placement = reference(schema, &model, wall, "ObjectPlacement").unwrap();
        let axis = reference(schema, &model, placement, "RelativePlacement").unwrap();
        let point = reference(schema, &model, axis, "Location").unwrap();
        assert_eq!(coordinates(&model, point), [1.0, 10.0, 0.5], "{name}");
        assert_eq!(coordinates(&model, EntityId(23)), [1.0, 0.0, 0.0]);
        // The original's axes are kept.
        assert!(reference(schema, &model, axis, "Axis").is_some());
        let opening = copy_of(&report, 81);
        let opening_placement = reference(schema, &model, opening, "ObjectPlacement").unwrap();
        assert_eq!(
            reference(schema, &model, opening_placement, "PlacementRelTo"),
            Some(placement)
        );
    });
}

#[test]
fn a_given_placement_carries_the_openings() {
    each_release(|name, mut model, schema| {
        let ops = [
            AuthorOp::Placement {
                relative_to: Some(EntityId(18)),
                location: [20.0, 0.0, 0.0],
                axes: None,
            },
            copy(
                20,
                CopyProductOptions::new().placement(CopyPlacement::At(authoring_handle(0))),
            ),
        ];
        let outcome = apply_authoring(&mut model, schema, &ops, 479).expect("applies");
        assert_valid(name, &model, schema);
        let (given, wall) = (outcome.ids[0].unwrap(), outcome.ids[1].unwrap());
        assert_eq!(
            reference(schema, &model, wall, "ObjectPlacement"),
            Some(given)
        );
        let report = outcome.copies[1].as_ref().unwrap();
        assert!(
            !copied(report, 21),
            "{name}: the given placement is no copy"
        );
        let opening = copy_of(report, 92);
        let opening_placement = reference(schema, &model, opening, "ObjectPlacement").unwrap();
        assert_eq!(
            reference(schema, &model, opening_placement, "PlacementRelTo"),
            Some(given)
        );
    });
}

#[test]
fn options_leave_out_openings_fillings_and_the_container() {
    each_release(|name, mut model, schema| {
        let (_, bare) = apply_copy(
            &mut model,
            schema,
            copy(20, CopyProductOptions::new().openings(false)),
        );
        assert!(!copied(&bare, 81) && !copied(&bare, 100), "{name}");
        assert!(bare.skipped.contains(&EntityId(91)) && bare.skipped.contains(&EntityId(96)));

        let (_, empty) = apply_copy(
            &mut model,
            schema,
            copy(20, CopyProductOptions::new().fillings(false)),
        );
        assert!(copied(&empty, 81) && !copied(&empty, 100), "{name}");
        assert!(empty.skipped.contains(&EntityId(109)), "{name}");

        let (wall, loose) = apply_copy(
            &mut model,
            schema,
            copy(20, CopyProductOptions::new().container(false)),
        );
        assert!(loose.skipped.contains(&EntityId(19)), "{name}");
        assert!(!references(schema, &model, EntityId(19), "RelatedElements").contains(&wall));
        assert_valid(name, &model, schema);
    });
}

#[test]
fn a_dry_run_reports_without_writing() {
    let mut model = read("cascade_ifc4.ifc");
    let schema = ifc::schema::ifc4();
    let before = model.revision();
    let staged =
        stage_authoring(&model, schema, &[copy(20, CopyProductOptions::new())], 1).expect("plans");
    assert!(staged.copies[0].as_ref().unwrap().copied.len() > 30);
    assert_eq!(model.revision(), before);
    let outcome = apply_authoring(
        &mut model,
        schema,
        &[copy(20, CopyProductOptions::new())],
        1,
    )
    .expect("applies");
    assert_eq!(
        staged.copies, outcome.copies,
        "the same seed plans the same copy"
    );
}

#[test]
fn moving_a_copy_refuses_what_it_would_leave_behind() {
    each_release(|name, mut model, schema| {
        // The window's opening placed on the storey, not on the wall.
        let rel_to = slot(schema, &model, EntityId(93), "PlacementRelTo");
        let edit = AuthorOp::Edit {
            entity: EntityId(93),
            attributes: vec![(
                schema.attributes("IfcLocalPlacement")[rel_to].name.clone(),
                Value::Ref(EntityId(18)),
            )],
        };
        apply_authoring(&mut model, schema, &[edit], 1).expect("edits");
        let options = CopyProductOptions::new().placement(CopyPlacement::Offset([1.0, 0.0, 0.0]));
        let failure = refusal(&mut model, schema, copy(20, options));
        assert!(
            matches!(failure, AuthoringFailure::UnsupportedCopy { entity, .. } if entity == EntityId(92)),
            "{name}: {failure:?}"
        );
        // In place it is copied, on the storey like the original.
        apply_copy(&mut model, schema, copy(20, CopyProductOptions::new()));
        assert_valid(name, &model, schema);
    });
}

#[test]
fn neither_a_product_nor_a_type_is_refused() {
    each_release(|name, mut model, schema| {
        let failure = refusal(&mut model, schema, copy(61, CopyProductOptions::new()));
        assert!(
            matches!(failure, AuthoringFailure::NotA { .. }),
            "{name}: {failure:?}"
        );
        let failure = refusal(&mut model, schema, copy(9999, CopyProductOptions::new()));
        assert_eq!(failure, AuthoringFailure::MissingEntity(EntityId(9999)));
    });
}

#[test]
fn copy_entity_follows_the_policy() {
    each_release(|name, mut model, schema| {
        let deep = AuthorOp::CopyEntity {
            entity: EntityId(26),
            policy: CopyPolicy::new(),
        };
        let shared_profile = AuthorOp::CopyEntity {
            entity: EntityId(26),
            policy: CopyPolicy::new().share(["IfcProfileDef"]),
        };
        let outcome =
            apply_authoring(&mut model, schema, &[deep, shared_profile], 479).expect("applies");
        let (solid, report) = (outcome.ids[0].unwrap(), outcome.copies[0].as_ref().unwrap());
        assert_eq!(copy_of(report, 26), solid, "{name}");
        // The profile, its position and point, the placement and direction.
        for original in [27, 28, 32, 9, 8, 31] {
            assert!(copied(report, original), "{name}: #{original}");
        }
        let lean = outcome.ids[1].unwrap();
        assert_eq!(
            reference(schema, &model, lean, "SweptArea"),
            Some(EntityId(27)),
            "{name}"
        );
        // A property set copies its properties, sharing its owner history.
        let pset = AuthorOp::CopyEntity {
            entity: EntityId(64),
            policy: CopyPolicy::new(),
        };
        let outcome = apply_authoring(&mut model, schema, &[pset], 479).expect("applies");
        let copy = outcome.ids[0].unwrap();
        assert_ne!(global_id(&model, copy), global_id(&model, EntityId(64)));
        assert_eq!(
            reference(schema, &model, copy, "OwnerHistory"),
            Some(EntityId(5))
        );
        assert_ne!(
            references(schema, &model, copy, "HasProperties"),
            [EntityId(63)]
        );
        let unknown = AuthorOp::CopyEntity {
            entity: EntityId(26),
            policy: CopyPolicy::new().share(["IfcNoSuchThing"]),
        };
        let failure = refusal(&mut model, schema, unknown);
        assert!(
            matches!(failure, AuthoringFailure::UnknownEntity { .. }),
            "{name}: {failure:?}"
        );
    });
}

#[test]
fn an_entity_a_unique_clause_binds_is_shared() {
    each_release(|name, mut model, schema| {
        let label = |text: &str| Value::Typed {
            type_name: "IFCLABEL".into(),
            value: Box::new(Value::Text(text.into())),
        };
        let ops = [
            AuthorOp::Create {
                type_name: "IfcPropertyEnumeration".into(),
                attributes: vec![
                    ("Name".into(), Value::Text("Finish".into())),
                    (
                        "EnumerationValues".into(),
                        Value::List(vec![label("Matt"), label("Gloss")]),
                    ),
                ],
            },
            AuthorOp::Create {
                type_name: "IfcPropertyEnumeratedValue".into(),
                attributes: vec![
                    ("Name".into(), Value::Text("Finish".into())),
                    ("EnumerationValues".into(), Value::List(vec![label("Matt")])),
                    (
                        "EnumerationReference".into(),
                        Value::Ref(authoring_handle(0)),
                    ),
                ],
            },
            AuthorOp::Create {
                type_name: "IfcPropertySet".into(),
                attributes: vec![
                    ("OwnerHistory".into(), Value::Ref(EntityId(5))),
                    ("Name".into(), Value::Text("Cascade_Finish".into())),
                    (
                        "HasProperties".into(),
                        Value::List(vec![Value::Ref(authoring_handle(1))]),
                    ),
                ],
            },
            AuthorOp::Create {
                type_name: "IfcRelDefinesByProperties".into(),
                attributes: vec![
                    ("OwnerHistory".into(), Value::Ref(EntityId(5))),
                    (
                        "RelatedObjects".into(),
                        Value::List(vec![Value::Ref(EntityId(20))]),
                    ),
                    (
                        "RelatingPropertyDefinition".into(),
                        Value::Ref(authoring_handle(2)),
                    ),
                ],
            },
        ];
        let outcome = apply_authoring(&mut model, schema, &ops, 1).expect("creates");
        let enumeration = outcome.ids[0].unwrap();
        let (_, report) = apply_copy(&mut model, schema, copy(20, CopyProductOptions::new()));
        assert!(copied(&report, outcome.ids[1].unwrap().0), "{name}");
        assert!(!copied(&report, enumeration.0), "{name}: shared");
        assert_valid(name, &model, schema);
    });
}
