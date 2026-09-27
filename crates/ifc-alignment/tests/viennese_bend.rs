//! The Viennese bend lowered exactly from cant and gravity height.
//!
//! IFC 4.3 defines this family as a 7th order polynomial spiral whose
//! curvature depends on the superelevation swing, not only on the endpoint
//! radii. Expected curvatures here are computed from the spec formula
//! independently of the implementation.

use ifc_alignment::authoring::{
    alignment_segment, cant_layout, cant_segment, horizontal_layout, horizontal_segment,
    CantSegmentDraft, HorizontalSegmentDraft,
};
use ifc_alignment::{AlignmentUnits, CantLayout};
use ifc_model::{Entity, EntityId, Model, Transaction, Value};

fn metres() -> AlignmentUnits {
    AlignmentUnits {
        length_to_metres: 1.0,
        angle_to_radians: 1.0,
    }
}

/// A straight-to-R500 Viennese bend over 120 m, gravity height 1.8 m,
/// with cant ramping 0 -> 0.15 m on a 1.5 m rail head distance.
struct Fixture {
    model: Model,
    horizontal: EntityId,
    cant: EntityId,
}

fn fixture(gravity_height: Option<f64>, with_cant: bool) -> Fixture {
    let mut model = Model::default();
    *model.header_mut() = ifc_model::Header {
        schema: vec!["IFC4X3_ADD2".to_owned()],
        ..ifc_model::Header::default()
    };
    let mut tx = Transaction::new(&model);
    let start = tx.create(Entity::new(
        "IFCCARTESIANPOINT",
        vec![Value::List(vec![Value::Real(0.0), Value::Real(0.0)])],
    ));
    let parameters = horizontal_segment(
        &mut tx,
        &HorizontalSegmentDraft {
            start_point: start,
            start_direction: 0.0,
            start_radius: 0.0,
            end_radius: 500.0,
            segment_length: 120.0,
            gravity_center_line_height: gravity_height,
            predefined_type: "VIENNESEBEND",
        },
    )
    .expect("authored segment");
    let segment =
        alignment_segment(&mut tx, "0aBcDeFgHiJkLmNoPqRsTu", parameters).expect("wrapper");
    let horizontal =
        horizontal_layout(&mut tx, "1aBcDeFgHiJkLmNoPqRsTu", Some("H")).expect("layout");
    tx.create(Entity::new(
        "IFCRELNESTS",
        vec![
            Value::Text("nest-h".into()),
            Value::Null,
            Value::Null,
            Value::Null,
            Value::Ref(horizontal),
            Value::List(vec![Value::Ref(segment)]),
        ],
    ));

    let cant = cant_layout(&mut tx, "2aBcDeFgHiJkLmNoPqRsTu", Some("C"), 1.5)
        .expect("authored cant layout");
    if with_cant {
        let cseg = cant_segment(
            &mut tx,
            &CantSegmentDraft {
                start_dist_along: 0.0,
                horizontal_length: 120.0,
                start_cant_left: 0.0,
                end_cant_left: Some(0.15),
                start_cant_right: 0.0,
                end_cant_right: Some(-0.05),
                predefined_type: "LINEARTRANSITION",
            },
        )
        .expect("authored cant segment");
        let wrapper =
            alignment_segment(&mut tx, "3aBcDeFgHiJkLmNoPqRsTu", cseg).expect("cant wrapper");
        tx.create(Entity::new(
            "IFCRELNESTS",
            vec![
                Value::Text("nest-c".into()),
                Value::Null,
                Value::Null,
                Value::Null,
                Value::Ref(cant),
                Value::List(vec![Value::Ref(wrapper)]),
            ],
        ));
    }
    tx.commit(&mut model).expect("commit");
    Fixture {
        model,
        horizontal,
        cant,
    }
}

/// The lowered law reproduces the spec curvature at sampled stations.
///
/// Expected values come from evaluating the degree-7 polynomial derived
/// from the spec by hand, not from the lowering under test.
#[test]
fn the_lowered_law_matches_the_spec_curvature() {
    let f = fixture(Some(1.8), true);
    let layout = CantLayout::resolve(&f.model, f.cant, metres()).expect("cant resolves");
    let lowered =
        ifc_alignment::lower_horizontal_layout(&f.model, f.horizontal, metres(), Some(&layout))
            .expect("a Viennese bend with cant lowers");
    let law = sole_curvature_law(&lowered);

    // dpsi = asin(0.20 / 1.5) - asin(0) = 0.13373158940994154, where
    // 0.20 is the LEFT-RIGHT cant difference, not either rail alone.
    // Endpoint curvature is exact regardless of the cant term, because
    // f(1) = 1 and the second derivative at 1 = 0.
    assert!((evaluate(&law, 120.0) - 1.0 / 500.0).abs() < 1e-12);
    assert!((evaluate(&law, 0.0)).abs() < 1e-15);
    // Independently computed from the spec polynomial.
    assert!((evaluate(&law, 30.0) - 1.769_887_500_742_694_3e-5).abs() < 1e-15);
}

/// Pull the single intrinsic curvature law out of a lowered layout.
fn sole_curvature_law(
    lowered: &ifc_alignment::LoweredAlignmentCurve,
) -> axiolid_curve::CurvatureLaw {
    intrinsic_law(&lowered.graph).expect("no intrinsic curve in the lowered graph")
}

/// Evaluate a polynomial curvature law at an arc length.
fn evaluate(law: &axiolid_curve::CurvatureLaw, s: f64) -> f64 {
    match law {
        axiolid_curve::CurvatureLaw::Polynomial { coefficients } => coefficients
            .iter()
            .enumerate()
            .map(|(i, c)| c * s.powi(i as i32))
            .sum(),
        other => panic!("expected a polynomial law, got {other:?}"),
    }
}

/// Without a cant layout the law is undetermined, so it is refused.
///
/// This is the case the old code refused unconditionally. It still
/// refuses, but now because an input is genuinely absent rather than
/// because the family was unimplemented.
#[test]
fn a_viennese_bend_without_cant_is_refused() {
    let f = fixture(Some(1.8), false);
    let error = ifc_alignment::lower_horizontal_layout(&f.model, f.horizontal, metres(), None)
        .expect_err("no cant layout, so no law");
    let text = error.to_string();
    assert!(
        text.contains("transition-curve primitive") || text.contains("superelevation"),
        "refusal should name the missing capability, got: {text}"
    );
}

/// A cant layout without GravityCenterLineHeight is refused, not
/// silently treated as zero: zero is a different curve, not an unknown.
#[test]
fn a_viennese_bend_without_gravity_height_is_refused() {
    let f = fixture(None, true);
    let layout = CantLayout::resolve(&f.model, f.cant, metres()).expect("cant resolves");
    let error =
        ifc_alignment::lower_horizontal_layout(&f.model, f.horizontal, metres(), Some(&layout))
            .expect_err("no gravity height, so no law");
    assert!(
        error.to_string().contains("GravityCenterLineHeight"),
        "refusal should name the missing attribute, got: {error}"
    );
}

/// Cant changes the curve, except exactly at the midpoint.
///
/// The cant correction is scaled by the second derivative of the shape
/// function, which vanishes at xi = 1/2. So a Viennese bend with cant
/// and one without share their midpoint curvature exactly while
/// differing elsewhere. A lowering that ignored cant would pass the
/// midpoint check and fail the other two.
#[test]
fn cant_bends_the_curve_everywhere_except_the_midpoint() {
    let with_cant = fixture(Some(1.8), true);
    let layout =
        CantLayout::resolve(&with_cant.model, with_cant.cant, metres()).expect("cant resolves");
    let lowered = ifc_alignment::lower_horizontal_layout(
        &with_cant.model,
        with_cant.horizontal,
        metres(),
        Some(&layout),
    )
    .expect("lowers");
    let law = sole_curvature_law(&lowered);

    // Independently computed: cant present vs a zero-swing law.
    assert!((evaluate(&law, 30.0) - 1.769_887_500_742_694_3e-5).abs() < 1e-15);
    assert!((evaluate(&law, 30.0) - 0.000_141_113_281_249_999_96).abs() > 1e-9);
    // The midpoint is cant-independent, and equals half the end curvature.
    assert!((evaluate(&law, 60.0) - 0.001_000_000_000_000_000_5).abs() < 1e-15);
}

/// The bend reads the cant at its own station, not at zero.
///
/// A horizontal segment does not state its distance along, so the chain
/// walk accumulates it. Here a 50 m straight precedes the bend, and the
/// cant is flat until 50 m then ramps. Reading the swing from station 0
/// would land in the flat part and give a visibly different curvature --
/// it even flips sign at s = 30.
#[test]
fn the_bend_reads_cant_at_its_own_station() {
    let (model, horizontal, cant) = staggered_fixture();
    let layout = CantLayout::resolve(&model, cant, metres()).expect("cant resolves");
    let lowered =
        ifc_alignment::lower_horizontal_layout(&model, horizontal, metres(), Some(&layout))
            .expect("two-segment layout lowers");
    let law = sole_curvature_law(&lowered);

    // Swing across [50, 170] gives dpsi = 0.20135792079033077.
    // Reading from station 0 would give 0.11693296146237842 and a
    // positive curvature here instead of a negative one.
    assert!((evaluate(&law, 30.0) - -4.471_019_057_310_795e-5).abs() < 1e-12);
    assert!(
        evaluate(&law, 30.0) < 0.0,
        "station 0 would give a positive value"
    );
}

/// A 50 m straight then a 120 m Viennese bend, with cant flat until 50 m
/// and ramping after, so the bend station is observable in the curvature.
fn staggered_fixture() -> (Model, EntityId, EntityId) {
    let mut model = Model::default();
    *model.header_mut() = ifc_model::Header {
        schema: vec!["IFC4X3_ADD2".to_owned()],
        ..ifc_model::Header::default()
    };
    let mut tx = Transaction::new(&model);
    let origin = tx.create(Entity::new(
        "IFCCARTESIANPOINT",
        vec![Value::List(vec![Value::Real(0.0), Value::Real(0.0)])],
    ));
    let bend_start = tx.create(Entity::new(
        "IFCCARTESIANPOINT",
        vec![Value::List(vec![Value::Real(50.0), Value::Real(0.0)])],
    ));
    let straight = horizontal_segment(
        &mut tx,
        &HorizontalSegmentDraft {
            start_point: origin,
            start_direction: 0.0,
            start_radius: 0.0,
            end_radius: 0.0,
            segment_length: 50.0,
            gravity_center_line_height: None,
            predefined_type: "LINE",
        },
    )
    .expect("straight");
    let bend = horizontal_segment(
        &mut tx,
        &HorizontalSegmentDraft {
            start_point: bend_start,
            start_direction: 0.0,
            start_radius: 0.0,
            end_radius: 500.0,
            segment_length: 120.0,
            gravity_center_line_height: Some(1.8),
            predefined_type: "VIENNESEBEND",
        },
    )
    .expect("bend");
    let w1 = alignment_segment(&mut tx, "0aBcDeFgHiJkLmNoPqRsTu", straight).expect("w1");
    let w2 = alignment_segment(&mut tx, "1aBcDeFgHiJkLmNoPqRsTu", bend).expect("w2");
    let horizontal =
        horizontal_layout(&mut tx, "2aBcDeFgHiJkLmNoPqRsTu", Some("H")).expect("layout");
    tx.create(Entity::new(
        "IFCRELNESTS",
        vec![
            Value::Text("nest-h".into()),
            Value::Null,
            Value::Null,
            Value::Null,
            Value::Ref(horizontal),
            Value::List(vec![Value::Ref(w1), Value::Ref(w2)]),
        ],
    ));

    let cant = cant_layout(&mut tx, "3aBcDeFgHiJkLmNoPqRsTu", Some("C"), 1.5).expect("cant");
    let flat = cant_segment(
        &mut tx,
        &CantSegmentDraft {
            start_dist_along: 0.0,
            horizontal_length: 50.0,
            start_cant_left: 0.0,
            end_cant_left: Some(0.0),
            start_cant_right: 0.0,
            end_cant_right: Some(0.0),
            predefined_type: "LINEARTRANSITION",
        },
    )
    .expect("flat cant");
    let ramp = cant_segment(
        &mut tx,
        &CantSegmentDraft {
            start_dist_along: 50.0,
            horizontal_length: 120.0,
            start_cant_left: 0.0,
            end_cant_left: Some(0.30),
            start_cant_right: 0.0,
            end_cant_right: Some(0.0),
            predefined_type: "LINEARTRANSITION",
        },
    )
    .expect("ramp cant");
    let cw1 = alignment_segment(&mut tx, "04BcDeFgHiJkLmNoPqRsTu", flat).expect("cw1");
    let cw2 = alignment_segment(&mut tx, "05BcDeFgHiJkLmNoPqRsTu", ramp).expect("cw2");
    tx.create(Entity::new(
        "IFCRELNESTS",
        vec![
            Value::Text("nest-c".into()),
            Value::Null,
            Value::Null,
            Value::Null,
            Value::Ref(cant),
            Value::List(vec![Value::Ref(cw1), Value::Ref(cw2)]),
        ],
    ));
    tx.commit(&mut model).expect("commit");
    (model, horizontal, cant)
}

/// The partial lowering path stations the bend too.
///
/// `lower_horizontal_layout_partial` walks the chain separately from the
/// strict entry point, and its refusal branch `continue`s. Without its
/// own station accumulation a bend following any earlier segment would
/// read the cant at the wrong place, so this pins the second walk.
#[test]
fn the_partial_path_also_stations_the_bend() {
    let (model, horizontal, cant) = staggered_fixture();
    let layout = CantLayout::resolve(&model, cant, metres()).expect("cant resolves");
    let partial =
        ifc_alignment::lower_horizontal_layout_partial(&model, horizontal, metres(), Some(&layout))
            .expect("partial lowering succeeds");
    assert!(
        partial.refused.is_empty(),
        "the bend should lower, not be refused: {:?}",
        partial.refused
    );
    let law = partial
        .runs
        .iter()
        .find_map(|run| intrinsic_law(&run.graph))
        .expect("an intrinsic curve in some run");
    assert!((evaluate(&law, 30.0) - -4.471_019_057_310_795e-5).abs() < 1e-12);
}

/// The intrinsic curvature law in a graph, if it has one.
fn intrinsic_law(graph: &axiolid_model::GeometryGraph) -> Option<axiolid_curve::CurvatureLaw> {
    use axiolid_curve::Curve2;
    use axiolid_model::GeometryNode;
    graph.iter().find_map(|(_, node)| match node {
        GeometryNode::Curve2(Curve2::Intrinsic(curve)) => Some(curve.curvature.clone()),
        _ => None,
    })
}
