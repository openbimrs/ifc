//! Material composition authoring: constituents, profiles, lists, usages.
//!
//! These cover the set/usage half of the material graph. The layer half is
//! already covered by `authoring.rs`; what is new here is that a composition
//! must not be empty and a usage must point at the right kind of set.

use ifc_material::{
    create_constituent, create_constituent_set, create_layer, create_layer_set,
    create_layer_set_usage, create_layer_with_offsets, create_material, create_material_list,
    create_profile, create_profile_set, create_profile_set_usage, ConstituentDraft, DirectionSense,
    LayerDraft, LayerSetDirection, LayerSetDraft, MaterialDraft, MaterialView, ProfileDraft,
};
use ifc_model::{Entity, Model, Transaction, Value};

fn material(tx: &mut Transaction, name: &str) -> ifc_model::EntityId {
    create_material(
        tx,
        MaterialDraft {
            name,
            description: None,
            category: None,
        },
    )
}

#[test]
fn a_constituent_set_commits_and_reads_back() {
    let mut model = Model::default();
    let mut tx = Transaction::new(&model);
    let steel = material(&mut tx, "Steel");
    let c = create_constituent(
        &mut tx,
        &model,
        ConstituentDraft {
            name: Some("Core"),
            description: None,
            material: steel,
            fraction: Some(0.6),
            category: None,
        },
    )
    .expect("constituent");
    let set =
        create_constituent_set(&mut tx, &model, &[c], Some("Wall"), None).expect("constituent set");
    tx.commit(&mut model).expect("commit");
    let view = MaterialView::new(&model);
    let found: Vec<_> = view.constituent_sets().map(|s| s.id()).collect();
    assert_eq!(found, vec![set], "the authored set must be readable");
}

#[test]
fn a_fraction_outside_the_normalised_range_is_refused() {
    let model = Model::default();
    let mut tx = Transaction::new(&model);
    let steel = material(&mut tx, "Steel");
    let err = create_constituent(
        &mut tx,
        &model,
        ConstituentDraft {
            name: None,
            description: None,
            material: steel,
            fraction: Some(1.5),
            category: None,
        },
    );
    assert!(err.is_err(), "a 150% constituent share must be refused");
}

#[test]
fn an_empty_composition_is_refused() {
    let model = Model::default();
    let mut tx = Transaction::new(&model);
    assert!(
        create_constituent_set(&mut tx, &model, &[], None, None).is_err(),
        "a set describing no constituents is not a composition"
    );
    assert!(
        create_material_list(&mut tx, &model, &[]).is_err(),
        "an empty material list names no materials"
    );
}

#[test]
fn a_layer_with_offsets_writes_inherited_slots_before_its_own() {
    // The subtype adds two attributes AFTER the seven it inherits. If the
    // constructor wrote only its own, OffsetDirection would land in slot 0
    // and be read as Material. Reading back through the subtype accessors
    // (slots 7 and 8) is what proves the layout.
    let mut model = Model::default();
    let mut tx = Transaction::new(&model);
    let steel = material(&mut tx, "Steel");
    let layer = create_layer_with_offsets(
        &mut tx,
        &model,
        LayerDraft {
            material: Some(steel),
            thickness: 0.2,
            is_ventilated: None,
            name: Some("Core"),
            description: None,
            category: None,
            priority: None,
        },
        LayerSetDirection::Axis2,
        [0.05, -0.05],
    )
    .expect("layer with offsets");
    tx.commit(&mut model).expect("commit");
    let view = MaterialView::new(&model);
    let found = view
        .layers_with_offsets()
        .find(|l| l.id() == layer)
        .expect("authored layer must be readable");
    assert_eq!(
        found.offset_direction().expect("direction"),
        LayerSetDirection::Axis2
    );
    assert_eq!(found.offset_values().expect("values"), [0.05, -0.05]);
    assert_eq!(found.thickness().expect("thickness"), 0.2);
}

#[test]
fn a_usage_must_point_at_the_matching_kind_of_set() {
    let model = Model::default();
    let mut tx = Transaction::new(&model);
    let steel = material(&mut tx, "Steel");
    let layer = create_layer(
        &mut tx,
        &model,
        LayerDraft {
            material: Some(steel),
            thickness: 0.1,
            is_ventilated: None,
            name: None,
            description: None,
            category: None,
            priority: None,
        },
    )
    .expect("layer");
    let layer_set = create_layer_set(
        &mut tx,
        &model,
        LayerSetDraft {
            layers: &[layer],
            name: None,
            description: None,
        },
    )
    .expect("layer set");
    // A layer set is not a profile set: the usage must refuse it.
    assert!(
        create_profile_set_usage(&mut tx, &model, layer_set, Some(1), None).is_err(),
        "a profile-set usage must not accept a layer set"
    );
    assert!(
        create_layer_set_usage(
            &mut tx,
            &model,
            layer_set,
            LayerSetDirection::Axis1,
            DirectionSense::Positive,
            0.0,
            None,
        )
        .is_ok(),
        "the matching usage must be accepted"
    );
}
#[test]
fn a_profile_set_commits_and_refuses_a_bad_priority() {
    let mut model = Model::default();
    let mut tx = Transaction::new(&model);
    let steel = material(&mut tx, "Steel");
    // A minimal profile record: the constructor checks existence, not shape,
    // because profile geometry is ifc-geometry's concern, not this crate's.
    let shape = tx.create(Entity::new(
        "IFCRECTANGLEPROFILEDEF",
        vec![
            Value::Enum("AREA".into()),
            Value::Null,
            Value::Null,
            Value::Real(0.3),
            Value::Real(0.5),
        ],
    ));
    let good = ProfileDraft {
        name: Some("Column"),
        description: None,
        material: Some(steel),
        profile: shape,
        priority: Some(50),
        category: None,
    };
    let profile = create_profile(&mut tx, &model, good).expect("profile");
    let set = create_profile_set(&mut tx, &model, &[profile], Some("S"), None, None)
        .expect("profile set");
    let usage = create_profile_set_usage(&mut tx, &model, set, Some(5), None);
    assert!(usage.is_ok(), "cardinal point 5 is inside 1..=9");
    assert!(
        create_profile_set_usage(&mut tx, &model, set, Some(12), None).is_err(),
        "cardinal point 12 names no reference point"
    );
    let mut bad = good;
    bad.priority = Some(101);
    assert!(
        create_profile(&mut tx, &model, bad).is_err(),
        "priority is a 0..=100 percentage"
    );
    tx.commit(&mut model).expect("commit");
}
