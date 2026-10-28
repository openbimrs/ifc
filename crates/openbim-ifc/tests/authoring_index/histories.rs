//! Batches over a history, the cascade fixtures, ambiguous models and the
//! batch's own records.

use crate::lab::*;

#[test]
fn a_history_of_batches_gives_the_same_answers_with_and_without_kept_indexes() {
    let mut lab = Lab::new();
    let s = lab.scene;
    lab.probe("scene");

    lab.apply("200 walls", &wall_ops(&s, 0..200, "w", None, 0));
    // The probes are not vacuous: some stage, some are refused.
    let staged = lab.probe("200 walls");
    assert!((3..=12).contains(&staged), "{staged} probes staged");
    lab.undo("undo the walls");
    assert_eq!(lab.model.ids_of_type("IFCWALL").len(), 0);
    lab.redo("redo the walls");
    assert_eq!(lab.model.ids_of_type("IFCWALL").len(), 200);
    lab.apply("50 more walls", &wall_ops(&s, 200..250, "w", None, 0));

    // A type with 30 typed walls, then a second type taking over.
    let mut typed = vec![wall_type("T1")];
    typed.extend(wall_ops(&s, 300..330, "t", Some(authoring_handle(0)), 1));
    let ids = lab.apply("typed walls", &typed);
    let t1 = ids[0].expect("the type");
    let walls = lab.model.ids_of_type("IFCWALL").to_vec();
    let t2 = lab.apply("a second type", &[wall_type("T2")])[0].expect("the type");
    lab.apply(
        "assign by replace",
        &[
            AuthorOp::AssignType {
                type_object: t2,
                objects: walls[..5].to_vec(),
                owner_history: None,
                options: AssignTypeOptions::new(),
            },
            AuthorOp::AssignType {
                type_object: t2,
                objects: walls[250..255].to_vec(),
                owner_history: None,
                options: AssignTypeOptions::new().replace(true),
            },
        ],
    );
    assert!(lab.model.ids_of_type("IFCRELDEFINESBYTYPE").len() >= 2);
    assert!(lab.model.contains(t1));

    let removals: Vec<AuthorOp> = walls[10..20]
        .iter()
        .map(|wall| AuthorOp::RemoveProduct {
            product: *wall,
            options: RemoveProductOptions::new().untype_occurrences(true),
        })
        .collect();
    lab.apply("remove ten walls", &removals);
    lab.apply("purge", &[AuthorOp::Purge(PurgeOptions::new())]);

    let first = lab.model.ids_of_type("IFCWALL")[0];
    lab.apply(
        "copy a wall",
        &[AuthorOp::CopyProduct {
            product: first,
            options: CopyProductOptions::new(),
        }],
    );
    let rep = lab.model.ids_of_type("IFCSHAPEREPRESENTATION")[0];
    lab.apply(
        "copy and assign a representation",
        &[
            AuthorOp::CopyRepresentation {
                representation: rep,
            },
            AuthorOp::Product {
                type_name: "IfcWall".into(),
                container: Some(s.storey),
                attributes: vec![],
                placement: None,
                type_object: None,
                owner_history: None,
            },
            AuthorOp::AssignRepresentation {
                representation: authoring_handle(0),
                to: authoring_handle(1),
            },
        ],
    );
    let moved: Vec<EntityId> = lab.model.ids_of_type("IFCWALL")[30..35].to_vec();
    lab.apply(
        "contain with replace",
        &[AuthorOp::Contain {
            structure: s.storey2,
            elements: moved,
            owner_history: None,
            options: RelateOptions::new().replace(true),
        }],
    );

    // A batch whose last operation repeats a GlobalId.
    let held = lab
        .model
        .get(lab.model.ids_of_type("IFCWALL")[0])
        .and_then(|wall| wall.attributes.first().cloned())
        .expect("a GlobalId");
    let mut refused = wall_ops(&s, 400..410, "r", None, 0);
    refused.push(AuthorOp::Product {
        type_name: "IfcWall".into(),
        container: None,
        attributes: vec![("GlobalId".into(), held)],
        placement: None,
        type_object: None,
        owner_history: None,
    });
    let failure = lab.refuse("a repeated GlobalId", &refused);
    assert!(
        matches!(failure, AuthoringFailure::DuplicateGlobalId { .. }),
        "{failure:?}"
    );

    // Walk the whole history back and forward again.
    let mut undone = 0;
    while lab.history.can_undo() {
        lab.undo("undo all");
        undone += 1;
    }
    assert!(undone >= 8, "{undone}");
    while lab.history.can_redo() {
        lab.redo("redo all");
    }
}

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

/// The cascade fixtures: storey #14, Wall A #20 and Wall B #40 typed by
/// #66, a door #100, the owner history #5 (see `copy_product.rs`).
#[test]
fn the_cascade_fixtures_stage_alike_in_every_release() {
    for (name, version) in FIXTURES {
        let schema = ifc::schema::for_version(version).expect("bundled");
        let mut model = read(name);
        model.reverse_index();
        let id = EntityId;
        let batches: Vec<(&str, Vec<AuthorOp>)> = vec![
            (
                "remove a wall",
                vec![AuthorOp::RemoveProduct {
                    product: id(20),
                    options: RemoveProductOptions::new()
                        .untype_occurrences(true)
                        .rebase_placements(true),
                }],
            ),
            (
                "copy a wall",
                vec![AuthorOp::CopyProduct {
                    product: id(20),
                    options: CopyProductOptions::new(),
                }],
            ),
            (
                "copy a door",
                vec![AuthorOp::CopyProduct {
                    product: id(100),
                    options: CopyProductOptions::new(),
                }],
            ),
            ("purge", vec![AuthorOp::Purge(PurgeOptions::new())]),
            (
                "contain a contained wall",
                vec![AuthorOp::Contain {
                    structure: id(14),
                    elements: vec![id(20)],
                    owner_history: Some(id(5)),
                    options: RelateOptions::new(),
                }],
            ),
            (
                "type a typed wall",
                vec![AuthorOp::AssignType {
                    type_object: id(66),
                    objects: vec![id(20)],
                    owner_history: Some(id(5)),
                    options: AssignTypeOptions::new(),
                }],
            ),
            (
                "type a typed wall by replace",
                vec![AuthorOp::AssignType {
                    type_object: id(66),
                    objects: vec![id(20), id(100)],
                    owner_history: Some(id(5)),
                    options: AssignTypeOptions::new().replace(true),
                }],
            ),
        ];
        let mut staged = 0;
        for (what, ops) in &batches {
            let at = format!("{name}: {what}");
            if stage_both(&model, schema, ops, &at) {
                staged += 1;
                let before = fingerprint(&model);
                let (applied, inverse) =
                    ifc::Inverse::record(&mut model, |m| apply_authoring(m, schema, ops, 5));
                applied.unwrap_or_else(|error| panic!("{at}: {error}"));
                consistent(&model, &at);
                inverse.apply(&mut model).expect("undo");
                consistent(&model, &at);
                assert_eq!(fingerprint(&model).1, before.1, "{at}: undo");
            }
        }
        assert!(staged >= 4, "{name}: {staged} batches staged");
    }
}

/// An unrelated wall of the scene, no container, no type.
fn loose_wall(lab: &mut Lab) -> EntityId {
    let ids = lab.apply(
        "a loose wall",
        &[AuthorOp::Product {
            type_name: "IfcWall".into(),
            container: None,
            attributes: vec![],
            placement: None,
            type_object: None,
            owner_history: None,
        }],
    );
    ids[0].expect("the wall")
}

/// An `entity` inserted under `id`, with a GlobalId unique to `id`.
fn insert_rel(model: &mut Model, id: u64, type_name: &str, related: EntityId, relating: EntityId) {
    let global_id = name_based_guid(&["tie", &id.to_string()]);
    model.insert(
        EntityId(id),
        Entity::new(
            type_name,
            vec![
                Value::Text(global_id.as_str().into()),
                Value::Null,
                Value::Null,
                Value::Null,
                Value::List(vec![Value::Ref(related)]),
                Value::Ref(relating),
            ],
        ),
    );
}

/// An object in two relationships of one kind: the refusal names the one
/// the type index lists first, which is the one inserted first and need not
/// have the lower id.
#[test]
fn an_object_in_two_relationships_is_refused_naming_the_one_listed_first() {
    let mut lab = Lab::new();
    let wall = loose_wall(&mut lab);
    let base = lab.model.next_id().0;
    let (high, low) = (EntityId(base + 10), EntityId(base + 5));
    insert_rel(
        &mut lab.model,
        high.0,
        "IFCRELCONTAINEDINSPATIALSTRUCTURE",
        wall,
        lab.scene.storey,
    );
    insert_rel(
        &mut lab.model,
        low.0,
        "IFCRELCONTAINEDINSPATIALSTRUCTURE",
        wall,
        lab.scene.storey,
    );
    assert_eq!(
        lab.model.ids_of_type("IFCRELCONTAINEDINSPATIALSTRUCTURE"),
        [high, low],
        "the bucket lists the higher id first"
    );
    let contain = [AuthorOp::Contain {
        structure: lab.scene.storey2,
        elements: vec![wall],
        owner_history: None,
        options: RelateOptions::new(),
    }];
    assert!(!stage_both(&lab.model, lab.schema, &contain, "contain"));
    let failure = lab.refuse("contain", &contain);
    assert_eq!(
        failure,
        AuthoringFailure::AlreadyRelated {
            id: wall,
            relationship: "IFCRELCONTAINEDINSPATIALSTRUCTURE",
            existing: high,
        }
    );

    // The same for types.
    let types = lab.apply(
        "three types",
        &[wall_type("A"), wall_type("B"), wall_type("C")],
    );
    let (a, b, c) = (types[0].unwrap(), types[1].unwrap(), types[2].unwrap());
    let base = lab.model.next_id().0;
    let (high, low) = (EntityId(base + 10), EntityId(base + 5));
    insert_rel(&mut lab.model, high.0, "IFCRELDEFINESBYTYPE", wall, a);
    insert_rel(&mut lab.model, low.0, "IFCRELDEFINESBYTYPE", wall, b);
    let assign = [AuthorOp::AssignType {
        type_object: c,
        objects: vec![wall],
        owner_history: None,
        options: AssignTypeOptions::new(),
    }];
    assert!(!stage_both(&lab.model, lab.schema, &assign, "assign"));
    let failure = lab.refuse("assign", &assign);
    assert_eq!(
        failure,
        AuthoringFailure::AlreadyRelated {
            id: wall,
            relationship: "IFCRELDEFINESBYTYPE",
            existing: high,
        }
    );
}

/// Two `IfcRoot`s of the base model share a GlobalId: claiming it is refused
/// for as long as either remains, naming the lowest id first.
#[test]
fn a_duplicate_base_global_id_is_refused_while_any_holder_remains() {
    let mut lab = Lab::new();
    let (a, b) = (loose_wall(&mut lab), loose_wall(&mut lab));
    assert!(a < b);
    let held = lab.model.get(a).unwrap().attributes[0].clone();
    lab.model.set_attribute(b, 0, held.clone());
    consistent(&lab.model, "duplicated");
    let text = held.as_text().expect("text").to_owned();
    let claim = |extra: Option<AuthorOp>| {
        let mut ops: Vec<AuthorOp> = extra.into_iter().collect();
        ops.push(AuthorOp::Product {
            type_name: "IfcWall".into(),
            container: None,
            attributes: vec![("GlobalId".into(), held.clone())],
            placement: None,
            type_object: None,
            owner_history: None,
        });
        ops
    };
    let duplicate = |holder| AuthoringFailure::DuplicateGlobalId {
        global_id: text.clone(),
        holder,
    };
    assert_eq!(lab.refuse("both remain", &claim(None)), duplicate(a));
    let remove = |wall| AuthorOp::RemoveProduct {
        product: wall,
        options: RemoveProductOptions::new(),
    };
    assert_eq!(
        lab.refuse("the lower is removed", &claim(Some(remove(a)))),
        duplicate(b)
    );
    assert_eq!(
        lab.refuse("the higher is removed", &claim(Some(remove(b)))),
        duplicate(a)
    );
    lab.apply("both removed", &[remove(a), remove(b)]);
    lab.apply("the id is free", &claim(None));
}

/// What a batch created and then rewrote or removed no longer references
/// anything: the batch's own indexes follow its records.
#[test]
fn a_batch_may_remove_and_rewrite_what_it_created() {
    let mut lab = Lab::new();
    let h = authoring_handle;
    let point = || create("IfcCartesianPoint", vec![("Coordinates", reals(&[1.0; 3]))]);
    let axes = |location| {
        create(
            "IfcAxis2Placement3D",
            vec![("Location", Value::Ref(location))],
        )
    };
    // The placement is removed, so the point it held is free to go.
    let removed = [
        point(),
        axes(h(0)),
        AuthorOp::Remove { entity: h(1) },
        AuthorOp::Remove { entity: h(0) },
    ];
    assert!(stage_both(&lab.model, lab.schema, &removed, "removed"));
    lab.apply("removed", &removed);
    // The placement moves to the second point, freeing the first.
    let rewritten = [
        point(),
        point(),
        axes(h(0)),
        AuthorOp::Edit {
            entity: h(2),
            attributes: vec![("Location".into(), Value::Ref(h(1)))],
        },
        AuthorOp::Remove { entity: h(0) },
    ];
    assert!(stage_both(&lab.model, lab.schema, &rewritten, "rewritten"));
    lab.apply("rewritten", &rewritten);
    // A wall of the model moves from one placement to another, then another:
    // only the last still holds a placement.
    let placement = || AuthorOp::Placement {
        relative_to: None,
        location: [0.0; 3],
        axes: None,
    };
    let wall = lab.apply(
        "a wall with three placements",
        &[
            placement(),
            placement(),
            placement(),
            AuthorOp::Product {
                type_name: "IfcWall".into(),
                container: None,
                attributes: vec![],
                placement: Some(h(0)),
                type_object: None,
                owner_history: None,
            },
        ],
    );
    let (second, third, wall) = (wall[1].unwrap(), wall[2].unwrap(), wall[3].unwrap());
    let move_to = |placement| AuthorOp::Edit {
        entity: wall,
        attributes: vec![("ObjectPlacement".into(), Value::Ref(placement))],
    };
    let moved = [
        move_to(second),
        move_to(third),
        AuthorOp::Remove { entity: second },
    ];
    assert!(stage_both(&lab.model, lab.schema, &moved, "moved"));
    lab.apply("moved", &moved);
    // A point still held is refused.
    let held = [point(), axes(h(0)), AuthorOp::Remove { entity: h(0) }];
    assert!(!stage_both(&lab.model, lab.schema, &held, "held"));
}
