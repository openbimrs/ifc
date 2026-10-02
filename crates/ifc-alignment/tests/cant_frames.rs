//! Cant at chosen stations as rail heights, bank angle and section frame
//! (#93), and the typed refusals around the cant-carrying centreline.
//!
//! The fixture is an authored alignment: a 100 m straight along +x, a 2%
//! grade from height 10, and a cant layout rotating about the low (right)
//! rail -- a linear ramp of the left rail from 0 to 120 mm over 0..60 m, then
//! constant 120 mm to 100 m -- with `RailHeadDistance` 1.5 m.
//!
//! Expected values are hand-computed from the IFC4.3 definitions:
//! linear cant `D(s) = D1 + xi * dD`, bank angle `psi = arcsin(D / b)`.

use axiolid_core::Vec3;
use ifc_alignment::{
    alignment, alignment_segment, cant_layout, cant_segment, gradient_curve3, horizontal_layout,
    horizontal_segment, lower_segmented_reference_curve, vertical_layout, vertical_segment,
    AlignmentError, AlignmentUnits, CantLayout, CantSegmentDraft, HorizontalSegmentDraft,
    VerticalSegmentDraft,
};
use ifc_model::{Entity, EntityId, Model, Transaction, Value};

fn metres() -> AlignmentUnits {
    AlignmentUnits {
        length_to_metres: 1.0,
        angle_to_radians: 1.0,
    }
}

fn guid(n: u32) -> String {
    format!("{n:0>22}")
}

fn nest(tx: &mut Transaction, parent: EntityId, children: Vec<EntityId>) {
    tx.create(Entity::new(
        "IFCRELNESTS",
        vec![
            Value::Text("nest".into()),
            Value::Null,
            Value::Null,
            Value::Null,
            Value::Ref(parent),
            Value::List(children.into_iter().map(Value::Ref).collect()),
        ],
    ));
}

/// The fixture alignment with `cant_layouts` cant layouts nested (0, 1 or 2).
fn fixture(cant_layouts: usize) -> (Model, EntityId) {
    let mut model = Model::default();
    model.header_mut().schema = vec!["IFC4X3_ADD2".to_owned()];
    let mut tx = Transaction::new(&model);
    let mut n = 0;
    let mut next_guid = || {
        n += 1;
        guid(n)
    };

    let start = tx.create(Entity::new(
        "IFCCARTESIANPOINT",
        vec![Value::List(vec![Value::Real(0.0), Value::Real(0.0)])],
    ));
    let h_params = horizontal_segment(
        &mut tx,
        &HorizontalSegmentDraft::new(start, 0.0, 0.0, 0.0, 100.0, "LINE"),
    )
    .expect("line");
    let h_seg = alignment_segment(&mut tx, &next_guid(), h_params).expect("h segment");
    let h = horizontal_layout(&mut tx, &next_guid(), Some("H")).expect("h");
    nest(&mut tx, h, vec![h_seg]);

    let v_params = vertical_segment(
        &mut tx,
        &VerticalSegmentDraft::new(0.0, 100.0, 10.0, 0.02, 0.02, "CONSTANTGRADIENT"),
    )
    .expect("grade");
    let v_seg = alignment_segment(&mut tx, &next_guid(), v_params).expect("v segment");
    let v = vertical_layout(&mut tx, &next_guid(), Some("V")).expect("v");
    nest(&mut tx, v, vec![v_seg]);

    let mut layouts = vec![h, v];
    for _ in 0..cant_layouts {
        let ramp = cant_segment(
            &mut tx,
            &CantSegmentDraft::new(0.0, 60.0, 0.0, 0.0, "LINEARTRANSITION")
                .end_cant_left(0.12)
                .end_cant_right(0.0),
        )
        .expect("ramp");
        let hold = cant_segment(
            &mut tx,
            &CantSegmentDraft::new(60.0, 40.0, 0.12, 0.0, "CONSTANTCANT"),
        )
        .expect("hold");
        let ramp_seg = alignment_segment(&mut tx, &next_guid(), ramp).expect("ramp segment");
        let hold_seg = alignment_segment(&mut tx, &next_guid(), hold).expect("hold segment");
        let c = cant_layout(&mut tx, &next_guid(), Some("C"), 1.5).expect("cant");
        nest(&mut tx, c, vec![ramp_seg, hold_seg]);
        layouts.push(c);
    }

    let a = alignment(&mut tx, &next_guid(), Some("A"), None).expect("alignment");
    nest(&mut tx, a, layouts);
    tx.commit(&mut model).expect("commit");
    (model, a)
}

fn close(actual: f64, expected: f64, what: &str) {
    assert!(
        (actual - expected).abs() < 1e-12,
        "{what}: {actual} != {expected}"
    );
}

/// Rail heights, cant, bank angle and rotation-point elevation at three
/// stations: mid-ramp, the ramp's end, and inside the constant part.
#[test]
fn cant_values_at_chosen_stations_match_the_ifc_definitions() {
    let (model, a) = fixture(1);
    let cant = CantLayout::for_alignment(&model, a, metres()).expect("one cant layout");

    // (station, left, right): the ramp is linear in xi = s / 60.
    for (station, left) in [(30.0, 0.06), (60.0, 0.12), (80.0, 0.12)] {
        let frame = cant.frame_at_distance(station).expect("frame");
        close(frame.distance_along, station, "station");
        close(frame.left, left, "left rail");
        close(frame.right, 0.0, "right rail");
        close(frame.cant, left, "cant D = left - right");
        // Rotation about the low rail lifts the rotation point by D / 2.
        close(frame.axis_elevation, left / 2.0, "axis elevation");
        // psi = arcsin(D / b) with b = 1.5: arcsin(0.04) at 30 m,
        // arcsin(0.08) at 60 and 80 m.
        close(frame.bank_angle, (left / 1.5_f64).asin(), "bank angle");
        close(frame.lateral.y, left / 1.5, "sin psi");
        close(
            frame.lateral.x,
            (1.0 - (left / 1.5).powi(2)).sqrt(),
            "cos psi",
        );
        close(frame.up.x, -left / 1.5, "-sin psi");
        close(frame.up.y, frame.lateral.x, "cos psi");
    }
}

/// The frame at 80 m on the evaluated gradient curve.
///
/// By hand: the centreline point is (80, 0, 10 + 0.02 * 80) = (80, 0, 11.6),
/// the unit tangent `t = (1, 0, 0.02) / s` with `s = sqrt(1.0004)`, the left
/// normal `n = (0, 1, 0)` and `u = t x n = (-0.02, 0, 1) / s`. With
/// `sin psi = 0.08` and `cos psi = sqrt(0.9936)`:
///
/// - origin = point + 0.06 up = (80, 0, 11.66);
/// - y = cos psi n + sin psi u = (-0.0016 / s, sqrt(0.9936), 0.08 / s);
/// - z = -sin psi n + cos psi u = (-0.02 cos psi / s, -0.08, cos psi / s).
///
/// The left rail head, b/2 along y from the origin, rises
/// `0.75 * 0.08 / s` above the rotation point: the authored `D / 2 = 0.06`
/// times `cos(theta) = 1 / s`, the grade's cosine. That is the documented
/// consequence of rotating about the 3D tangent; IFC does not say which
/// reading it intends, and the upstream request asks Axiolid to fix it.
#[test]
fn the_section_frame_on_the_evaluated_centreline_matches_the_hand_values() {
    let (model, a) = fixture(1);
    let cant = CantLayout::for_alignment(&model, a, metres()).expect("cant");
    let curve = gradient_curve3(&model, a, metres()).expect("gradient curve");
    let axiolid_curve::Curve3::Elevated(elevated) = &curve else {
        panic!("expected an elevated curve, got {curve:?}");
    };
    let point = axiolid_evaluate::elevated_point(elevated, 80.0).expect("point");
    let tangent = axiolid_evaluate::elevated_tangent(elevated, 80.0).expect("tangent");
    let frame = cant
        .frame_at_distance(80.0)
        .expect("frame")
        .orient(point, tangent)
        .expect("oriented");

    let s = 1.0004_f64.sqrt();
    let cos = 0.9936_f64.sqrt();
    let near = |actual: Vec3, expected: Vec3, what: &str| {
        assert!(
            (actual - expected).length() < 1e-12,
            "{what}: {actual:?} != {expected:?}"
        );
    };
    near(frame.origin, Vec3::new(80.0, 0.0, 11.66), "origin");
    near(frame.x, Vec3::new(1.0 / s, 0.0, 0.02 / s), "tangent");
    near(frame.y, Vec3::new(-0.0016 / s, cos, 0.08 / s), "rail axis");
    near(
        frame.z,
        Vec3::new(-0.02 * cos / s, -0.08, cos / s),
        "section up",
    );
    close(
        (frame.origin + frame.y * 0.75).z - frame.origin.z,
        0.06 / s,
        "rise",
    );
}

/// Mid-ramp, the rail heads seen from the profile: the right rail stays on
/// the low-rail pivot when there is no grade term to account for.
#[test]
fn the_low_rail_stays_on_the_profile_in_a_level_section() {
    let (model, a) = fixture(1);
    let frame = CantLayout::for_alignment(&model, a, metres())
        .expect("cant")
        .frame_at_distance(30.0)
        .expect("frame");
    let oriented = frame
        .orient(Vec3::new(30.0, 0.0, 10.6), Vec3::X)
        .expect("level frame");
    let right_rail = oriented.origin - oriented.y * 0.75;
    let left_rail = oriented.origin + oriented.y * 0.75;
    close(right_rail.z, 10.6, "right rail on the profile");
    close(left_rail.z, 10.66, "left rail raised by D = 0.06");
}

#[test]
fn a_missing_cant_layout_is_a_typed_refusal() {
    let (model, a) = fixture(0);
    let expected = AlignmentError::SemanticViolation {
        entity: Some(a),
        rule: "the alignment nests no IfcAlignmentCant layout",
    };
    assert_eq!(
        CantLayout::for_alignment(&model, a, metres()),
        Err(expected.clone())
    );
    assert_eq!(
        lower_segmented_reference_curve(&model, a, metres()),
        Err(expected)
    );
}

#[test]
fn an_ambiguous_cant_layout_is_a_typed_refusal() {
    let (model, a) = fixture(2);
    let expected = AlignmentError::SemanticViolation {
        entity: Some(a),
        rule: "an alignment with several cant layouts is ambiguous to compose",
    };
    assert_eq!(
        CantLayout::for_alignment(&model, a, metres()),
        Err(expected.clone())
    );
    assert_eq!(
        lower_segmented_reference_curve(&model, a, metres()),
        Err(expected)
    );
}

/// With exactly one sound cant layout, the refusal names the structural gap:
/// no roll law in the pinned neutral vocabulary, at the cant layout entity.
#[test]
fn the_cant_carrying_centreline_is_refused_for_want_of_a_roll_law() {
    let (model, a) = fixture(1);
    let cant = CantLayout::for_alignment(&model, a, metres()).expect("cant");
    let error =
        lower_segmented_reference_curve(&model, a, metres()).expect_err("no roll law upstream");
    let AlignmentError::Unsupported {
        entity,
        type_name,
        detail,
    } = error
    else {
        panic!("expected Unsupported, got {error}");
    };
    assert_eq!(entity, cant.entity);
    assert_eq!(type_name, "IfcSegmentedReferenceCurve");
    assert!(detail.contains("roll"), "{detail}");
}

/// Cant beyond the rail head distance has no real bank angle.
#[test]
fn cant_exceeding_the_rail_head_distance_is_refused() {
    let mut model = Model::default();
    model.header_mut().schema = vec!["IFC4X3_ADD2".to_owned()];
    let mut tx = Transaction::new(&model);
    let params = cant_segment(
        &mut tx,
        &CantSegmentDraft::new(0.0, 10.0, 1.6, 0.0, "CONSTANTCANT"),
    )
    .expect("cant segment");
    let seg = alignment_segment(&mut tx, &guid(1), params).expect("segment");
    let c = cant_layout(&mut tx, &guid(2), Some("C"), 1.5).expect("layout");
    nest(&mut tx, c, vec![seg]);
    tx.commit(&mut model).expect("commit");

    let layout = CantLayout::resolve(&model, c, metres()).expect("layout");
    assert!(matches!(
        layout.frame_at_distance(5.0),
        Err(AlignmentError::SemanticViolation { entity: Some(e), .. }) if e == c
    ));
}
