//! The remaining profile forms, authored and read back.
//!
//! `IfcMirroredProfileDef` is the case worth guarding: its `Operator`
//! is DERIVE, so the slot must serialize as `*`.

use ifc_geometry::authoring::ProfileType;
use ifc_geometry::authoring::{
    arbitrary_open_profile, arbitrary_profile_with_voids, cartesian_point, center_line_profile,
    circle, composite_profile, derived_profile, mirrored_profile, polyline, rectangle_profile,
    rounded_rectangle_profile,
};
use ifc_model::{EntityId, Model, Transaction, Value};

/// A closed curve, for profiles that need one.
fn ring(tx: &mut Transaction) -> EntityId {
    let a = cartesian_point(tx, &[0.0, 0.0]).expect("a");
    let b = cartesian_point(tx, &[1.0, 0.0]).expect("b");
    let c = cartesian_point(tx, &[1.0, 1.0]).expect("c");
    polyline(tx, &[a, b, c, a]).expect("ring")
}

/// An open profile is a CURVE; a centre line profile is an AREA.
///
/// The schema forbids AREA on open profiles *except* the centre-line
/// subtype, where the thickness supplies the missing width. Both types
/// are fixed by the writer, so neither can be set wrongly.
#[test]
fn open_profiles_carry_the_profile_type_the_schema_fixes() {
    let model = Model::default();
    let mut tx = Transaction::new(&model);
    let curve = ring(&mut tx);

    let open = arbitrary_open_profile(&mut tx, Some("Open"), curve);
    let centred = center_line_profile(&mut tx, Some("Centred"), curve, 0.05).expect("centre");
    assert!(
        center_line_profile(&mut tx, None, curve, 0.0).is_err(),
        "a zero thickness centre line encloses nothing"
    );

    let mut model = model;
    tx.commit(&mut model).expect("commit");

    let entity = model.get(open).expect("open");
    assert_eq!(entity.type_name.as_ref(), "IFCARBITRARYOPENPROFILEDEF");
    assert_eq!(entity.attributes[0], Value::Enum("CURVE".into()));
    assert_eq!(entity.attributes[2], Value::Ref(curve));

    let entity = model.get(centred).expect("centred");
    assert_eq!(
        entity.attributes[0],
        Value::Enum("AREA".into()),
        "a centre line profile does enclose an area"
    );
    assert_eq!(entity.attributes[3], Value::Real(0.05));
}

/// `IfcMirroredProfileDef` writes its derived operator as `*`.
///
/// The schema computes the mirror transform, so the file must not
/// restate it. `$` would be a different claim -- that no transform
/// exists -- and both decode the same way in Rust, which is why this
/// asserts on the stored value rather than round-tripping.
#[test]
fn a_mirrored_profile_derives_its_operator() {
    let model = Model::default();
    let mut tx = Transaction::new(&model);
    let parent = rectangle_profile(&mut tx, Some("Parent"), None, 0.3, 0.2).expect("parent");

    let mirrored = mirrored_profile(&mut tx, ProfileType::Area, Some("Mirror"), parent, None);

    let mut model = model;
    tx.commit(&mut model).expect("commit");

    let entity = model.get(mirrored).expect("mirrored");
    assert_eq!(entity.type_name.as_ref(), "IFCMIRROREDPROFILEDEF");
    assert_eq!(entity.attributes[2], Value::Ref(parent));
    assert_eq!(
        entity.attributes[3],
        Value::Derived,
        "Operator is DERIVE on IfcMirroredProfileDef and must serialize as *"
    );
    assert_ne!(
        entity.attributes[3],
        Value::Null,
        "`$` would claim the transform is absent, not derived"
    );
}

/// A derived profile states the operator a mirrored one derives.
#[test]
fn a_derived_profile_states_its_operator() {
    let model = Model::default();
    let mut tx = Transaction::new(&model);
    let parent = rectangle_profile(&mut tx, Some("Parent"), None, 0.3, 0.2).expect("parent");
    // Any entity reference stands in: the writer does not resolve it.
    let operator = cartesian_point(&mut tx, &[0.0, 0.0]).expect("stand-in");

    let derived = derived_profile(
        &mut tx,
        ProfileType::Area,
        Some("Derived"),
        parent,
        operator,
        Some("label"),
    );

    let mut model = model;
    tx.commit(&mut model).expect("commit");

    let entity = model.get(derived).expect("derived");
    assert_eq!(entity.attributes[2], Value::Ref(parent));
    assert_eq!(
        entity.attributes[3],
        Value::Ref(operator),
        "here the operator is stated, not derived"
    );
    assert_eq!(entity.attributes[4], Value::Text("label".into()));
}

/// Composite and voided profiles enforce their set bounds.
#[test]
fn profile_sets_enforce_their_lower_bounds() {
    let model = Model::default();
    let mut tx = Transaction::new(&model);
    let one = rectangle_profile(&mut tx, Some("A"), None, 0.3, 0.2).expect("a");
    let two = rectangle_profile(&mut tx, Some("B"), None, 0.1, 0.4).expect("b");
    let outer = ring(&mut tx);
    let at = cartesian_point(&mut tx, &[0.0, 0.0]).expect("origin");
    let placement = ifc_geometry::authoring::axis2_placement_2d(&mut tx, at, None);
    let hole = circle(&mut tx, placement, 0.1).expect("hole");

    assert!(
        composite_profile(&mut tx, ProfileType::Area, None, &[one], None).is_err(),
        "SET [2:?]: a composite of one profile is just that profile"
    );
    let composite = composite_profile(
        &mut tx,
        ProfileType::Area,
        Some("Pair"),
        &[one, two],
        Some("L"),
    )
    .expect("composite");

    assert!(
        arbitrary_profile_with_voids(&mut tx, None, outer, &[]).is_err(),
        "SET [1:?]: without a void this is an IfcArbitraryClosedProfileDef"
    );
    let voided =
        arbitrary_profile_with_voids(&mut tx, Some("Voided"), outer, &[hole]).expect("voided");

    let mut model = model;
    tx.commit(&mut model).expect("commit");

    let entity = model.get(composite).expect("composite");
    assert_eq!(
        entity.attributes[2],
        Value::List(vec![Value::Ref(one), Value::Ref(two)])
    );
    assert_eq!(entity.attributes[3], Value::Text("L".into()));

    let entity = model.get(voided).expect("voided");
    // WR1 fixes the type to AREA regardless of what a caller wants.
    assert_eq!(entity.attributes[0], Value::Enum("AREA".into()));
    assert_eq!(entity.attributes[2], Value::Ref(outer), "outer curve");
    assert_eq!(
        entity.attributes[3],
        Value::List(vec![Value::Ref(hole)]),
        "the void belongs in InnerCurves, not folded into the outer curve"
    );
}

/// A rounding radius is bounded by half the rectangle it rounds.
#[test]
fn a_rounding_radius_cannot_exceed_the_rectangle() {
    let model = Model::default();
    let mut tx = Transaction::new(&model);

    // Exactly half the smaller dimension gives a stadium: still legal.
    let stadium = rounded_rectangle_profile(&mut tx, Some("Stadium"), None, 4.0, 2.0, 1.0)
        .expect("a radius of exactly half the smaller side is a stadium");
    assert!(
        rounded_rectangle_profile(&mut tx, None, None, 4.0, 2.0, 1.01).is_err(),
        "a radius past half the smaller side describes no shape"
    );
    assert!(
        rounded_rectangle_profile(&mut tx, None, None, 4.0, 2.0, 0.0).is_err(),
        "RoundingRadius is a positive length"
    );
    assert!(
        rounded_rectangle_profile(&mut tx, None, None, 0.0, 2.0, 0.5).is_err(),
        "XDim is a positive length"
    );

    let mut model = model;
    tx.commit(&mut model).expect("commit");
    let entity = model.get(stadium).expect("stadium");
    assert_eq!(entity.attributes.len(), 6, "RoundingRadius is slot 5");
    assert_eq!(entity.attributes[3], Value::Real(4.0), "XDim");
    assert_eq!(entity.attributes[4], Value::Real(2.0), "YDim");
    assert_eq!(entity.attributes[5], Value::Real(1.0), "RoundingRadius");
}
