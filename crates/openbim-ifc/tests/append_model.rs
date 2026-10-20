//! Copying entities from one model into another (#479): what comes along,
//! what is reused, `GlobalId` modes, and the refusals across releases and
//! units, over the cascade fixtures (see `copy_product.rs` for their map).

#![cfg(all(
    feature = "step",
    feature = "author",
    feature = "validate",
    feature = "ifc2x3",
    feature = "ifc4",
    feature = "ifc4x3"
))]

use ifc::{
    apply_append, apply_authoring, stage_append, AppendOptions, AuthorOp, AuthoringFailure, Codec,
    EntityId, GlobalIdMode, Model, RemoveProductOptions, ReuseOptions, Schema, StepCodec, Value,
};

fn read(name: &str) -> Model {
    let path = format!(
        "{}/../../test/fixtures/synthetic-relations/{name}",
        env!("CARGO_MANIFEST_DIR")
    );
    StepCodec
        .read_path(std::path::Path::new(&path))
        .unwrap_or_else(|error| panic!("{name}: {error}"))
}

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

fn ids(values: &[u64]) -> Vec<EntityId> {
    values.iter().copied().map(EntityId).collect()
}

/// The IFC4 fixture without Wall A and what it owned, so its `GlobalId`s
/// are free again.
fn without_wall_a(schema: &Schema) -> Model {
    let mut model = read("cascade_ifc4.ifc");
    let remove = AuthorOp::RemoveProduct {
        product: EntityId(20),
        options: RemoveProductOptions::new().rebase_placements(true),
    };
    apply_authoring(&mut model, schema, &[remove], 1).expect("removes");
    model
}

fn count(model: &Model, type_name: &str) -> usize {
    model.ids_of_type(type_name).len()
}

fn text(model: &Model, id: EntityId, slot: usize) -> String {
    model.get(id).expect("present").attributes[slot]
        .as_text()
        .expect("text")
        .to_owned()
}

#[test]
fn a_removed_wall_merges_back_with_what_belongs_to_it() {
    let schema = ifc::schema::ifc4();
    let source = read("cascade_ifc4.ifc");
    let mut target = without_wall_a(schema);
    let units = count(&target, "IFCSIUNIT");
    let merge = AppendOptions::new().global_ids(GlobalIdMode::Merge);
    let outcome = apply_append(
        &mut target,
        schema,
        &source,
        schema,
        &ids(&[20]),
        &merge,
        479,
    )
    .expect("appends");
    assert_valid("appended", &target, schema);
    let report = &outcome.report;
    let wall = report.copy_of(EntityId(20)).expect("the wall");
    assert_eq!(text(&target, wall, 0), "0OrFL0Eh_cEulcCubTghTp", "kept");
    assert!(report.regenerated.is_empty());
    // What the removal took comes back: its own property set, material
    // usage, classification, openings, body and style.
    for original in [64, 65, 73, 74, 79, 81, 91, 92, 96, 109, 134, 24, 25, 26, 33] {
        assert!(
            report
                .copied
                .iter()
                .any(|c| c.original == EntityId(original)),
            "#{original} was not copied"
        );
    }
    // What the target still holds is reused: by value the owner history,
    // context, layer set and style; by identity the shared property set,
    // the type, the door and window that filled the openings.
    for original in [5, 11, 72, 34, 61, 66, 100, 130] {
        assert!(
            report
                .reused
                .iter()
                .any(|r| r.original == EntityId(original)),
            "#{original} was not reused: {:#?}",
            report.reused
        );
    }
    assert_eq!(count(&target, "IFCSIUNIT"), units);
    assert_eq!(count(&target, "IFCOWNERHISTORY"), 1);
    assert_eq!(count(&target, "IFCMATERIALLAYERSET"), 1);
    assert_eq!(count(&target, "IFCDOOR"), 1);
    // The relationships the target kept are joined: the wall is typed,
    // shares its property set and is contained again.
    assert_eq!(report.joined, ids(&[19, 62, 67]));
    let contained = target.get(EntityId(19)).unwrap().attributes[4].clone();
    assert!(matches!(contained, Value::List(items) if items.contains(&Value::Ref(wall))));
    assert!(report.skipped.is_empty(), "{:?}", report.skipped);
}

#[test]
fn a_kept_wall_lands_beside_the_target_without_its_container() {
    let schema = ifc::schema::ifc4();
    let source = read("cascade_ifc4.ifc");
    let mut target = read("cascade_ifc4.ifc");
    let regenerate = AppendOptions::new().global_ids(GlobalIdMode::Regenerate);
    let outcome = apply_append(
        &mut target,
        schema,
        &source,
        schema,
        &ids(&[20]),
        &regenerate,
        9,
    )
    .expect("appends");
    assert_valid("beside", &target, schema);
    let report = &outcome.report;
    // The type, shared property set and fillings are copies now, related
    // to the copies only; the containment in the source's storey is
    // skipped for the caller to relate the copies.
    for original in [66, 67, 61, 62, 100, 130] {
        assert!(report.copy_of(EntityId(original)).is_some(), "#{original}");
    }
    let typing = report.copy_of(EntityId(67)).unwrap();
    let typed = target.get(typing).unwrap().attributes[4].clone();
    assert_eq!(
        typed,
        Value::List(vec![Value::Ref(report.copy_of(EntityId(20)).unwrap())])
    );
    assert!(
        report.skipped.contains(&EntityId(19)),
        "{:?}",
        report.skipped
    );
    assert_eq!(count(&target, "IFCOWNERHISTORY"), 1, "reused by value");
}

#[test]
fn global_ids_are_kept_refused_or_regenerated() {
    let schema = ifc::schema::ifc4();
    let source = read("cascade_ifc4.ifc");
    let mut target = read("cascade_ifc4.ifc");
    let before = target.revision();
    let error = apply_append(
        &mut target,
        schema,
        &source,
        schema,
        &ids(&[66]),
        &AppendOptions::new(),
        1,
    )
    .expect_err("collides");
    assert!(
        matches!(*error.failure, AuthoringFailure::DuplicateGlobalId { .. }),
        "{error}"
    );
    assert_eq!(target.revision(), before);

    let colliding = AppendOptions::new().global_ids(GlobalIdMode::RegenerateColliding);
    let outcome = apply_append(
        &mut target,
        schema,
        &source,
        schema,
        &ids(&[66]),
        &colliding,
        1,
    )
    .expect("appends");
    assert_valid("regenerated", &target, schema);
    let wall_type = outcome.report.copy_of(EntityId(66)).unwrap();
    let pset = outcome.report.copy_of(EntityId(68)).unwrap();
    assert!(outcome.report.regenerated.contains(&wall_type));
    assert!(outcome.report.regenerated.contains(&pset));
    assert_ne!(text(&target, wall_type, 0), "1QV6QaFiETx1ihyJgvQbkl");
    // The type's occurrences stay in the source: its typing is skipped.
    assert!(outcome.report.skipped.contains(&EntityId(67)));

    let merge = AppendOptions::new().global_ids(GlobalIdMode::Merge);
    let outcome =
        apply_append(&mut target, schema, &source, schema, &ids(&[66]), &merge, 1).expect("merges");
    assert!(outcome.report.copied.is_empty(), "{:#?}", outcome.report);
    assert_eq!(outcome.report.copy_of(EntityId(66)), Some(EntityId(66)));

    let all = AppendOptions::new().global_ids(GlobalIdMode::Regenerate);
    let outcome =
        apply_append(&mut target, schema, &source, schema, &ids(&[111]), &all, 2).expect("appends");
    assert_eq!(outcome.report.regenerated.len(), 1, "the door type alone");
    assert_valid("door type", &target, schema);
}

#[test]
fn a_type_comes_with_its_representation_maps() {
    let schema = ifc::schema::ifc4();
    let source = read("cascade_ifc4.ifc");
    let mut target = read("cascade_ifc4.ifc");
    let regenerate = AppendOptions::new().global_ids(GlobalIdMode::Regenerate);
    let staged = stage_append(
        &target,
        schema,
        &source,
        schema,
        &ids(&[111]),
        &regenerate,
        3,
    )
    .expect("plans");
    let before = target.revision();
    assert_eq!(target.revision(), before);
    let outcome = apply_append(
        &mut target,
        schema,
        &source,
        schema,
        &ids(&[111]),
        &regenerate,
        3,
    )
    .expect("appends");
    assert_eq!(staged.report, outcome.report);
    for original in [111, 112, 113, 114, 115, 116] {
        assert!(
            outcome.report.copy_of(EntityId(original)).is_some(),
            "#{original}"
        );
    }
    assert!(outcome
        .report
        .reused
        .iter()
        .any(|r| r.original == EntityId(11)));
    assert_valid("type", &target, schema);
}

#[test]
fn nothing_is_reused_when_asked() {
    let schema = ifc::schema::ifc4();
    let source = read("cascade_ifc4.ifc");
    let mut target = without_wall_a(schema);
    let options = AppendOptions::new()
        .global_ids(GlobalIdMode::Merge)
        .reuse(ReuseOptions::none());
    let outcome = apply_append(
        &mut target,
        schema,
        &source,
        schema,
        &ids(&[20]),
        &options,
        4,
    )
    .expect("appends");
    // Only what is the same object by identity is reused, no resource.
    for resource in [5, 6, 11, 34, 72] {
        assert!(outcome.report.copy_of(EntityId(resource)) != Some(EntityId(resource)));
    }
    assert_valid("copied resources", &target, schema);
    assert_eq!(count(&target, "IFCOWNERHISTORY"), 2);
    assert_eq!(count(&target, "IFCMATERIALLAYERSET"), 2);
}

#[test]
fn differing_length_units_are_refused() {
    let schema = ifc::schema::ifc4();
    let source = read("cascade_ifc4.ifc");
    let mut target = without_wall_a(schema);
    let millimetre = AuthorOp::Edit {
        entity: EntityId(6),
        attributes: vec![("Prefix".into(), Value::Enum("MILLI".into()))],
    };
    apply_authoring(&mut target, schema, &[millimetre], 1).expect("edits");
    let before = target.revision();
    let error = apply_append(
        &mut target,
        schema,
        &source,
        schema,
        &ids(&[20]),
        &AppendOptions::new(),
        1,
    )
    .expect_err("refused");
    match *error.failure {
        AuthoringFailure::UnitMismatch {
            ref unit_type,
            ref source,
            ref target,
        } => {
            assert_eq!(unit_type, "LENGTHUNIT");
            assert_eq!(source, "METRE");
            assert_eq!(target, "MILLI METRE");
        }
        ref other => panic!("{other:?}"),
    }
    assert!(error.to_string().contains("#469"), "{error}");
    assert_eq!(target.revision(), before);
}

#[test]
fn a_release_that_lacks_an_entity_or_attribute_is_refused() {
    let source = read("cascade_ifc4.ifc");
    let ifc4 = ifc::schema::ifc4();
    let ifc2x3 = ifc::schema::ifc2x3();
    let mut target = read("cascade_ifc2x3.ifc");
    let before = target.revision();
    let regenerate = AppendOptions::new().global_ids(GlobalIdMode::Regenerate);
    // IFC2X3 has no IfcDoorType.
    let error = apply_append(
        &mut target,
        ifc2x3,
        &source,
        ifc4,
        &ids(&[111]),
        &regenerate,
        1,
    )
    .expect_err("refused");
    assert!(
        matches!(
            &*error.failure,
            AuthoringFailure::IncompatibleRelease { entity, detail, .. }
                if *entity == EntityId(111) && detail.contains("does not declare")
        ),
        "{error}"
    );
    // IFC2X3's IfcWall has no PredefinedType, and the IFC4 wall sets it.
    let error = apply_append(
        &mut target,
        ifc2x3,
        &source,
        ifc4,
        &ids(&[20]),
        &regenerate,
        1,
    )
    .expect_err("refused");
    assert!(
        matches!(
            &*error.failure,
            AuthoringFailure::IncompatibleRelease { detail, .. } if detail.contains("PredefinedType")
        ),
        "{error}"
    );
    assert_eq!(target.revision(), before);
}

#[test]
fn ifc4_entities_move_into_ifc4x3_by_name() {
    let source = read("cascade_ifc4.ifc");
    let (ifc4, ifc4x3) = (ifc::schema::ifc4(), ifc::schema::ifc4x3());
    let mut target = read("cascade_ifc4x3.ifc");
    let regenerate = AppendOptions::new().global_ids(GlobalIdMode::Regenerate);
    let outcome = apply_append(
        &mut target,
        ifc4x3,
        &source,
        ifc4,
        &ids(&[20, 150]),
        &regenerate,
        5,
    )
    .expect("appends");
    assert_valid("IFC4 into IFC4X3", &target, ifc4x3);
    let wall = outcome.report.copy_of(EntityId(20)).unwrap();
    assert_eq!(&*target.get(wall).unwrap().type_name, "IFCWALL");
    // IFC4X3 moved PlacementRelTo up to IfcObjectPlacement: the copy
    // still names its parent.
    let placement = outcome.report.copy_of(EntityId(21)).unwrap();
    let slots = ifc::attribute_slots(ifc4x3, "IfcLocalPlacement").unwrap();
    let rel_to = slots
        .iter()
        .find(|s| s.name == "PlacementRelTo")
        .unwrap()
        .index;
    assert_eq!(
        target.get(placement).unwrap().attributes[rel_to],
        Value::Ref(outcome.report.copy_of(EntityId(18)).unwrap())
    );
}

#[test]
fn an_enumeration_constant_the_target_lacks_is_refused() {
    let (ifc4, ifc4x3) = (ifc::schema::ifc4(), ifc::schema::ifc4x3());
    let mut source = read("cascade_ifc4x3.ifc");
    let retaining = AuthorOp::Edit {
        entity: EntityId(66),
        attributes: vec![("PredefinedType".into(), Value::Enum("RETAININGWALL".into()))],
    };
    apply_authoring(&mut source, ifc4x3, &[retaining], 1).expect("edits");
    let mut target = read("cascade_ifc4.ifc");
    let regenerate = AppendOptions::new().global_ids(GlobalIdMode::Regenerate);
    let error = apply_append(
        &mut target,
        ifc4,
        &source,
        ifc4x3,
        &ids(&[66]),
        &regenerate,
        6,
    )
    .expect_err("refused");
    assert!(
        matches!(&*error.failure, AuthoringFailure::IncompatibleRelease { entity, detail, .. }
            if *entity == EntityId(66) && detail.contains("RETAININGWALL")),
        "{error}"
    );
}

#[test]
fn a_missing_root_is_refused() {
    let schema = ifc::schema::ifc4();
    let source = read("cascade_ifc4.ifc");
    let mut target = read("cascade_ifc4.ifc");
    let error = apply_append(
        &mut target,
        schema,
        &source,
        schema,
        &ids(&[9999]),
        &AppendOptions::new(),
        1,
    )
    .expect_err("refused");
    assert_eq!(
        *error.failure,
        AuthoringFailure::MissingEntity(EntityId(9999))
    );
}

#[test]
fn a_material_a_profile_and_a_template_come_with_their_closure() {
    let schema = ifc::schema::ifc4();
    let mut source = read("cascade_ifc4.ifc");
    let template = [
        AuthorOp::Create {
            type_name: "IfcSimplePropertyTemplate".into(),
            attributes: vec![
                ("Name".into(), Value::Text("Reference".into())),
                ("TemplateType".into(), Value::Enum("P_SINGLEVALUE".into())),
                (
                    "PrimaryMeasureType".into(),
                    Value::Text("IfcIdentifier".into()),
                ),
            ],
        },
        AuthorOp::Create {
            type_name: "IfcPropertySetTemplate".into(),
            attributes: vec![
                ("Name".into(), Value::Text("Cascade_Template".into())),
                (
                    "TemplateType".into(),
                    Value::Enum("PSET_OCCURRENCEDRIVEN".into()),
                ),
                ("ApplicableEntity".into(), Value::Text("IfcWall".into())),
                (
                    "HasPropertyTemplates".into(),
                    Value::List(vec![Value::Ref(ifc::authoring_handle(0))]),
                ),
            ],
        },
    ];
    let created = apply_authoring(&mut source, schema, &template, 1).expect("creates");
    let (property, set) = (created.ids[0].unwrap(), created.ids[1].unwrap());
    let mut target = read("cascade_ifc4.ifc");
    let options = AppendOptions::new().reuse(ReuseOptions::none());
    let outcome = apply_append(
        &mut target,
        schema,
        &source,
        schema,
        &[EntityId(70), EntityId(170), set],
        &options,
        7,
    )
    .expect("appends");
    assert_valid("assets", &target, schema);
    let report = &outcome.report;
    for original in [
        EntityId(70),
        EntityId(170),
        EntityId(171),
        EntityId(172),
        set,
        property,
    ] {
        assert!(report.copy_of(original).is_some(), "{original}");
    }
    let copy = report.copy_of(set).unwrap();
    assert_eq!(text(&target, copy, 0), text(&source, set, 0), "kept");
    assert_eq!(count(&target, "IFCMATERIAL"), 2);
}
