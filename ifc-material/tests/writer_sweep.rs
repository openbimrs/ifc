//! Sweep `create_material_list`, which nothing had called.
//!
//! `IfcMaterialList.Materials` is `LIST [1:?] OF IfcMaterial`: the list
//! is ordered, non-empty, and every member must be a material rather
//! than some other material-select type.

use ifc_material::{
    create_material, create_material_list, create_profile_set_usage_tapering, MaterialDraft,
};
use ifc_model::{Entity, Model, Transaction, Value};

/// The list stages, keeps file order, and is refused when empty.
#[test]
fn the_material_list_writer_stages() {
    let mut model = Model::default();
    let mut tx = Transaction::new(&model);

    let first = create_material(
        &mut tx,
        MaterialDraft {
            name: "Concrete",
            description: None,
            category: None,
        },
    );
    let second = create_material(
        &mut tx,
        MaterialDraft {
            name: "Steel",
            description: None,
            category: None,
        },
    );
    tx.commit(&mut model).expect("commit");

    let mut tx = Transaction::new(&model);
    let id = create_material_list(&mut tx, &model, &[first, second]).expect("material list");
    tx.commit(&mut model).expect("commit");

    let staged = model.get(id).expect("staged");
    assert_eq!(staged.type_name.as_ref(), "IFCMATERIALLIST");
    assert_eq!(
        staged.attributes[0],
        Value::List(vec![Value::Ref(first), Value::Ref(second)]),
        "Materials is an ordered list, first then second",
    );
}

/// `LIST [1:?]` excludes the empty list.
#[test]
fn an_empty_material_list_is_refused() {
    let model = Model::default();
    let mut tx = Transaction::new(&model);
    assert!(
        create_material_list(&mut tx, &model, &[]).is_err(),
        "an empty Materials list satisfies no LIST [1:?] bound",
    );
}

/// Members must be materials, not any material-select type.
#[test]
fn a_non_material_member_is_refused() {
    let mut model = Model::default();
    let mut tx = Transaction::new(&model);
    let imposter = tx.create(Entity::new("IFCMATERIALLAYER", vec![Value::Null; 7]));
    tx.commit(&mut model).expect("commit");

    let mut tx = Transaction::new(&model);
    assert!(
        create_material_list(&mut tx, &model, &[imposter]).is_err(),
        "a layer is a material select but not an IfcMaterial",
    );
}

/// The tapering form keeps its inherited slots.
///
/// `ForProfileEndSet` and `CardinalEndPoint` come *after* the
/// three inherited slots, so writing only the subtype pair
/// would shift every inherited value.
#[test]
fn the_tapering_usage_keeps_the_inherited_slots() {
    let mut model = Model::default();
    let mut tx = Transaction::new(&model);
    let start = tx.create(Entity::new("IFCMATERIALPROFILESET", vec![Value::Null; 4]));
    let end = tx.create(Entity::new("IFCMATERIALPROFILESET", vec![Value::Null; 4]));
    tx.commit(&mut model).expect("commit");

    let mut tx = Transaction::new(&model);
    let id =
        create_profile_set_usage_tapering(&mut tx, &model, start, end, Some(3), Some(7), Some(2.5))
            .expect("tapering usage");
    tx.commit(&mut model).expect("commit");

    let staged = model.get(id).expect("staged");
    assert_eq!(
        staged.type_name.as_ref(),
        "IFCMATERIALPROFILESETUSAGETAPERING",
    );
    assert_eq!(staged.attributes[0], Value::Ref(start), "ForProfileSet");
    assert_eq!(staged.attributes[1], Value::Integer(3), "CardinalPoint");
    assert_eq!(staged.attributes[2], Value::Real(2.5), "ReferenceExtent");
    assert_eq!(staged.attributes[3], Value::Ref(end), "ForProfileEndSet");
    assert_eq!(staged.attributes[4], Value::Integer(7), "CardinalEndPoint");
}

/// Both cardinal points are bounded to 1..=9.
#[test]
fn a_cardinal_point_outside_the_range_is_refused() {
    let mut model = Model::default();
    let mut tx = Transaction::new(&model);
    let set = tx.create(Entity::new("IFCMATERIALPROFILESET", vec![Value::Null; 4]));
    tx.commit(&mut model).expect("commit");

    for (start_point, end_point) in [(Some(0), None), (None, Some(10))] {
        let mut tx = Transaction::new(&model);
        assert!(
            create_profile_set_usage_tapering(
                &mut tx,
                &model,
                set,
                set,
                start_point,
                end_point,
                None,
            )
            .is_err(),
            "accepted a cardinal point outside 1..=9",
        );
    }
}

/// Both profile sets must be profile sets.
///
/// `ForProfileEndSet` is required and typed, so a non-profile-set
/// end makes the taper address nothing.
#[test]
fn a_non_profile_set_end_is_refused() {
    let mut model = Model::default();
    let mut tx = Transaction::new(&model);
    let start = tx.create(Entity::new("IFCMATERIALPROFILESET", vec![Value::Null; 4]));
    let wrong = tx.create(Entity::new("IFCMATERIAL", vec![Value::Null; 3]));
    tx.commit(&mut model).expect("commit");

    let mut tx = Transaction::new(&model);
    assert!(
        create_profile_set_usage_tapering(&mut tx, &model, start, wrong, None, None, None).is_err(),
        "accepted a material as the end profile set",
    );
    assert!(
        create_profile_set_usage_tapering(&mut tx, &model, wrong, start, None, None, None).is_err(),
        "accepted a material as the start profile set",
    );
}
