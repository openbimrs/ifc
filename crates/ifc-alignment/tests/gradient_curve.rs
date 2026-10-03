//! Plan and vertical profile composed as an exact 3D centreline.
//!
//! Heights are checked against values computed by hand from the IFC
//! definition, not by re-running the same code path that produced them.

use axiolid_curve::{Curve3, ElevationLaw};
use axiolid_model::GeometryNode;
use ifc_alignment::{
    lower_gradient_curve, profile_law, AlignmentError, AlignmentUnits, VerticalSegment,
    VerticalSegmentType,
};
use ifc_model::{Entity, EntityId, Model, Value};
use std::sync::Arc;

fn metres() -> AlignmentUnits {
    AlignmentUnits {
        length_to_metres: 1.0,
        angle_to_radians: 1.0,
    }
}

fn vertical(
    start_dist: f64,
    length: f64,
    height: f64,
    entry: f64,
    exit: f64,
    kind: VerticalSegmentType,
) -> VerticalSegment {
    VerticalSegment {
        entity: EntityId(1),
        start_dist_along: start_dist,
        horizontal_length: length,
        start_height: height,
        start_gradient: entry,
        end_gradient: exit,
        radius_of_curvature: None,
        predefined_type: kind,
    }
}

/// A crest curve joining +3%% to -2%% over 200 m.
///
/// Expected heights come from the IFC definition
/// `z(d) = H + g1*d + (g2-g1)/(2L)*d^2`, evaluated by hand:
/// d=0 -> 100.0, d=50 -> 101.1875, d=100 -> 101.75, d=200 -> 101.0.
#[test]
fn a_parabolic_arc_matches_the_ifc_definition() {
    let segment = vertical(
        0.0,
        200.0,
        100.0,
        0.03,
        -0.02,
        VerticalSegmentType::ParabolicArc,
    );
    let law = profile_law(std::slice::from_ref(&segment)).expect("exact law");

    assert_eq!(law.height_at(0.0), Some(100.0));
    assert_eq!(law.height_at(50.0), Some(101.1875));
    assert_eq!(law.height_at(100.0), Some(101.75));
    assert_eq!(law.height_at(200.0), Some(101.0));

    // Grade runs from the entry value to the exit value.
    assert_eq!(law.grade_at(0.0), Some(0.03));
    // Exit grade is reconstructed as g1 + 2*((g2-g1)/2L)*L, so it carries one
    // rounding step rather than being stored verbatim.
    let exit = law.grade_at(200.0).expect("grade");
    assert!((exit - -0.02).abs() < 1e-12, "exit grade was {exit}");
}

/// A grade then a crest, as a road is actually authored.
///
/// Piece 1 rises at 2%% from 50.0, reaching 52.0 at 100 m. Piece 2 is a
/// 200 m crest from +2%% to -3%%, written in its OWN distance restarting at
/// zero: 52.0 -> 52.75 at its midpoint -> 51.0 at its end.
#[test]
fn a_two_piece_profile_is_continuous_across_the_seam() {
    // Starting at a real station, not zero: the law must be written in
    // distance from the profile start, so a seam left at its absolute
    // station would put every height in the wrong piece.
    let segments = [
        vertical(
            1000.0,
            100.0,
            50.0,
            0.02,
            0.02,
            VerticalSegmentType::ConstantGradient,
        ),
        vertical(
            1100.0,
            200.0,
            52.0,
            0.02,
            -0.03,
            VerticalSegmentType::ParabolicArc,
        ),
    ];
    let law = profile_law(&segments).expect("exact profile");
    assert!(matches!(law, ElevationLaw::Piecewise { .. }));

    assert_eq!(law.height_at(0.0), Some(50.0));
    assert_eq!(law.height_at(100.0), Some(52.0), "seam height must agree");
    assert_eq!(law.height_at(200.0), Some(52.75), "crest midpoint");
    assert_eq!(law.height_at(300.0), Some(51.0), "profile end");

    // The seam is continuous in height: approaching it from the left piece
    // and landing on the right piece must not step.
    let just_before = law.height_at(100.0 - 1e-6).expect("height");
    assert!(
        (just_before - 52.0).abs() < 1e-7,
        "stepped at the seam: {just_before}"
    );
}

/// Build an alignment: a clothoid plan, and a vertical profile over it.
fn alignment_model() -> (Model, EntityId) {
    alignment_model_with("CLOTHOID", 300.0)
}

/// The same fixture with a chosen first plan segment, so a test can build a
/// plan whose segments actually compose continuously.
fn alignment_model_with(plan_kind: &str, end_radius: f64) -> (Model, EntityId) {
    let mut model = Model::new();
    model.header_mut().schema = vec!["IFC4X3_ADD2".to_owned()];
    let mut id = 0;
    let mut next = || {
        id += 1;
        EntityId(id)
    };

    let origin = next();
    model.insert(
        origin,
        Entity::new(
            "IFCCARTESIANPOINT",
            vec![Value::List(vec![Value::Real(0.0), Value::Real(0.0)])],
        ),
    );

    // A clothoid: straight into a 300 m radius over 100 m.
    let h_params = next();
    model.insert(
        h_params,
        Entity::new(
            "IFCALIGNMENTHORIZONTALSEGMENT",
            vec![
                Value::Null,
                Value::Null,
                Value::Ref(origin),
                Value::Real(0.0),
                Value::Real(0.0),
                Value::Real(end_radius),
                Value::Real(100.0),
                Value::Null,
                Value::Enum(Arc::from(plan_kind)),
            ],
        ),
    );

    // A 100 m constant 2%% grade from height 10.
    let v_params = next();
    model.insert(
        v_params,
        Entity::new(
            "IFCALIGNMENTVERTICALSEGMENT",
            vec![
                Value::Null,
                Value::Null,
                Value::Real(0.0),
                Value::Real(100.0),
                Value::Real(10.0),
                Value::Real(0.02),
                Value::Real(0.02),
                Value::Null,
                Value::Enum(Arc::from("CONSTANTGRADIENT")),
            ],
        ),
    );

    let product = |name: &str| {
        let mut attrs = vec![Value::Null; 7];
        attrs[0] = Value::Text(Arc::from(name));
        attrs
    };
    let wrapper = |design: EntityId, name: &str| {
        let mut attrs = product(name);
        attrs.push(Value::Ref(design));
        attrs
    };

    let h_seg = next();
    model.insert(
        h_seg,
        Entity::new("IFCALIGNMENTSEGMENT", wrapper(h_params, "hs")),
    );
    let v_seg = next();
    model.insert(
        v_seg,
        Entity::new("IFCALIGNMENTSEGMENT", wrapper(v_params, "vs")),
    );

    let horizontal = next();
    model.insert(
        horizontal,
        Entity::new("IFCALIGNMENTHORIZONTAL", product("H")),
    );
    let vertical_layout = next();
    model.insert(
        vertical_layout,
        Entity::new("IFCALIGNMENTVERTICAL", product("V")),
    );

    let alignment = next();
    let mut align_attrs = product("A1");
    align_attrs.push(Value::Null);
    model.insert(alignment, Entity::new("IFCALIGNMENT", align_attrs));

    let mut nest = |parent: EntityId, children: Vec<EntityId>, tag: &str| {
        let id = next();
        model.insert(
            id,
            Entity::new(
                "IFCRELNESTS",
                vec![
                    Value::Text(Arc::from(tag)),
                    Value::Null,
                    Value::Null,
                    Value::Null,
                    Value::Ref(parent),
                    Value::List(children.into_iter().map(Value::Ref).collect()),
                ],
            ),
        );
    };
    nest(horizontal, vec![h_seg], "nh");
    nest(vertical_layout, vec![v_seg], "nv");
    nest(alignment, vec![horizontal, vertical_layout], "na");

    (model, alignment)
}

/// The whole point: a clothoid plan and a vertical profile compose into an
/// exact 3D centreline, with neither half approximated.
#[test]
fn an_alignment_lowers_to_an_exact_elevated_curve() {
    let (model, alignment) = alignment_model();
    let lowered = lower_gradient_curve(&model, alignment, metres()).expect("composes");

    let Some(GeometryNode::Curve3(Curve3::Elevated(elevated))) = lowered.graph.get(lowered.root)
    else {
        panic!("the root must be an elevated 3D curve");
    };

    // The plan half comes back UNCHANGED: still an exact clothoid, not a
    // B-spline fitted through it. This is what composition buys.
    assert!(
        matches!(elevated.plan.as_ref(), axiolid_curve::Curve2::Intrinsic(_)),
        "the spiral must survive composition as an intrinsic curve"
    );

    // The vertical half comes back unchanged too: 2%% from height 10.
    assert_eq!(elevated.elevation.height_at(0.0), Some(10.0));
    assert_eq!(elevated.elevation.height_at(100.0), Some(12.0));
}

/// The composed curve evaluates to the right place in 3D.
///
/// Reference values are an independent Fresnel-style quadrature of the
/// clothoid (400k steps, computed outside this crate), not another run of
/// the kernel code under test:
///   plan end  = (99.72257921782685, 5.544542365620256)
///   height    = 10 + 0.02 * 100 = 12.0
#[test]
fn the_composed_centreline_evaluates_where_it_should() {
    let (model, alignment) = alignment_model();
    let lowered = lower_gradient_curve(&model, alignment, metres()).expect("composes");
    let Some(GeometryNode::Curve3(Curve3::Elevated(elevated))) = lowered.graph.get(lowered.root)
    else {
        panic!("expected an elevated curve");
    };

    let point = axiolid_evaluate::arc_length::elevated_point(elevated, 100.0)
        .expect("evaluates at the end of the spiral");
    assert!(
        (point.x - 99.72257921782685).abs() < 1e-7,
        "x was {}",
        point.x
    );
    assert!(
        (point.y - 5.544542365620256).abs() < 1e-7,
        "y was {}",
        point.y
    );
    // Height is a function of PLAN distance, so 100 m of plan gives 12.0
    // even though the 3D curve is slightly longer than 100 m.
    assert!((point.z - 12.0).abs() < 1e-9, "z was {}", point.z);
}

/// A circular vertical curve is the circle itself in the composed profile,
/// not the parabola it resembles near its vertex (#258): a crest of
/// `R = -4000` from +3% over 200 m, checked against the circle's own
/// equation about its centre.
#[test]
fn a_circular_vertical_curve_is_the_circle_not_a_parabola() {
    let mut segment = vertical(
        0.0,
        200.0,
        100.0,
        0.03,
        -0.02,
        VerticalSegmentType::CircularArc,
    );
    segment.radius_of_curvature = Some(-4000.0);
    let law = profile_law(std::slice::from_ref(&segment)).expect("exact circle");
    // Centre R along the left normal of the start tangent: below, for a crest.
    let norm = 0.03_f64.hypot(1.0);
    let (centre_d, centre_z) = (-4000.0 * -0.03 / norm, 100.0 + -4000.0 / norm);
    for d in [0.0, 50.0, 100.0, 150.0, 200.0] {
        let circle = centre_z + (4000.0_f64.powi(2) - (d - centre_d).powi(2)).sqrt();
        let height = law.height_at(d).expect("height");
        assert!(
            (height - circle).abs() < 1e-9,
            "at {d}: {height} != {circle}"
        );
    }
    // The EN 13803 parabola z0 + g0 d + d^2 / (2R) is an approximation of
    // it; the two part by millimetres over 200 m.
    let parabola = 100.0 + 0.03 * 200.0 - 200.0_f64.powi(2) / 8000.0;
    assert!((law.height_at(200.0).expect("height") - parabola).abs() > 1e-3);
}

/// A grade, a crest arc and a grade, joined tangentially, through the
/// reference evaluator: height and grade meet themselves at both seams.
/// The arc's end height and grade come from the circle about its centre,
/// not from the law under test.
#[test]
fn a_profile_through_a_circular_arc_is_continuous_at_its_seams() {
    let norm = 0.02_f64.hypot(1.0);
    let sin_end = 0.02 / norm + 200.0 / -4000.0;
    let end_grade = sin_end / (1.0 - sin_end * sin_end).sqrt();
    let centre = (100.0 + 4000.0 * 0.02 / norm, 52.0 - 4000.0 / norm);
    let end_height = centre.1 + (4000.0_f64.powi(2) - (300.0 - centre.0).powi(2)).sqrt();
    let mut arc = vertical(
        100.0,
        200.0,
        52.0,
        0.02,
        end_grade,
        VerticalSegmentType::CircularArc,
    );
    arc.radius_of_curvature = Some(-4000.0);
    let segments = [
        vertical(
            0.0,
            100.0,
            50.0,
            0.02,
            0.02,
            VerticalSegmentType::ConstantGradient,
        ),
        arc,
        vertical(
            300.0,
            100.0,
            end_height,
            end_grade,
            end_grade,
            VerticalSegmentType::ConstantGradient,
        ),
    ];
    let law = profile_law(&segments).expect("tangent profile");
    for seam in [100.0, 300.0] {
        let height = axiolid_evaluate::elevation_height(&law, seam).expect("height");
        let grade = axiolid_evaluate::elevation_grade(&law, seam).expect("grade");
        for side in [seam - 1e-9, seam + 1e-9] {
            let near = axiolid_evaluate::elevation_height(&law, side).expect("height");
            let slope = axiolid_evaluate::elevation_grade(&law, side).expect("grade");
            assert!((near - height).abs() < 1e-9, "height steps at {seam}");
            assert!((slope - grade).abs() < 1e-9, "grade breaks at {seam}");
        }
    }
    let at_end = axiolid_evaluate::elevation_height(&law, 300.0).expect("height");
    assert!(
        (at_end - end_height).abs() < 1e-9,
        "{at_end} != {end_height}"
    );
}

/// A profile with a gap does not describe one continuous road.
#[test]
fn a_discontinuous_profile_is_refused() {
    let segments = [
        vertical(
            0.0,
            100.0,
            50.0,
            0.02,
            0.02,
            VerticalSegmentType::ConstantGradient,
        ),
        vertical(
            150.0,
            100.0,
            52.0,
            0.02,
            0.02,
            VerticalSegmentType::ConstantGradient,
        ),
    ];
    assert!(
        profile_law(&segments).is_err(),
        "a 50 m gap must not be joined silently"
    );
}

/// A zero-length parabola divides by zero in the rate-of-grade-change term.
/// The kernel documents that naming this is a validator job; this is it.
#[test]
fn a_zero_length_parabola_is_refused_before_it_becomes_infinite() {
    let segment = vertical(
        0.0,
        0.0,
        100.0,
        0.03,
        -0.02,
        VerticalSegmentType::ParabolicArc,
    );
    let error = ifc_alignment::elevation_law(&segment)
        .expect_err("a zero-length parabola must not produce an infinite coefficient");
    // The length is named as the fault, rather than the non-finite
    // coefficient it would otherwise produce downstream.
    let text = format!("{error}");
    assert!(
        text.contains("positive horizontal length"),
        "refusal was: {text}"
    );
    // As a whole profile it is a closing segment with nothing before it.
    assert!(matches!(
        profile_law(std::slice::from_ref(&segment)),
        Err(AlignmentError::SemanticViolation { .. })
    ));
}

/// A two-segment plan elevates as ONE intrinsic curve (#92): the layout's
/// laws sit in a piecewise curvature law anchored at the first segment, so
/// the whole road is elevated rather than refused or truncated to its first
/// segment. The richer line -> clothoid -> arc case is in
/// `horizontal_plan.rs`.
#[test]
fn a_multi_segment_plan_elevates_as_one_intrinsic_curve() {
    // Two collinear lines: continuous in position and heading at the join.
    let (mut model, alignment) = alignment_model_with("LINE", 0.0);
    add_second_plan_segment(&mut model);
    // The profile must cover the whole 150 m plan (HorizontalLength, slot 3).
    let profile = find_by_type(&model, "IFCALIGNMENTVERTICALSEGMENT");
    let mut attrs = model.get(profile).expect("profile").attributes.clone();
    attrs[3] = Value::Real(150.0);
    model.insert(profile, Entity::new("IFCALIGNMENTVERTICALSEGMENT", attrs));
    let lowered = lower_gradient_curve(&model, alignment, metres()).expect("composes");
    let Some(GeometryNode::Curve3(Curve3::Elevated(elevated))) = lowered.graph.get(lowered.root)
    else {
        panic!("the root must be an elevated 3D curve");
    };
    let axiolid_curve::Curve2::Intrinsic(plan) = elevated.plan.as_ref() else {
        panic!("a multi-segment plan is one intrinsic curve");
    };
    assert_eq!(plan.length, 150.0, "both segments, not only the first");
    assert!(matches!(
        &plan.curvature,
        axiolid_curve::CurvatureLaw::Piecewise { breaks, .. } if breaks == &vec![100.0]
    ));
}

/// Append a second horizontal segment, so the plan lowers to a composite
/// with two children rather than one.
fn add_second_plan_segment(model: &mut Model) {
    let point = EntityId(100);
    model.insert(
        point,
        Entity::new(
            "IFCCARTESIANPOINT",
            vec![Value::List(vec![Value::Real(100.0), Value::Real(0.0)])],
        ),
    );
    let params = EntityId(101);
    model.insert(
        params,
        Entity::new(
            "IFCALIGNMENTHORIZONTALSEGMENT",
            vec![
                Value::Null,
                Value::Null,
                Value::Ref(point),
                Value::Real(0.0),
                Value::Real(0.0),
                Value::Real(0.0),
                Value::Real(50.0),
                Value::Null,
                Value::Enum(Arc::from("LINE")),
            ],
        ),
    );
    let wrapper = EntityId(102);
    let mut attrs = vec![Value::Null; 7];
    attrs[0] = Value::Text(Arc::from("hs2"));
    attrs.push(Value::Ref(params));
    model.insert(wrapper, Entity::new("IFCALIGNMENTSEGMENT", attrs));

    // Find the nest whose parent is the horizontal layout, rather than
    // assuming an id: the fixture numbers entities in creation order.
    let horizontal = find_by_type(model, "IFCALIGNMENTHORIZONTAL");
    let (nest, mut attrs) = model
        .iter()
        .filter(|(_, e)| e.type_name.eq_ignore_ascii_case("IFCRELNESTS"))
        .find(|(_, e)| e.attributes[4] == Value::Ref(horizontal))
        .map(|(id, e)| (id, e.attributes.clone()))
        .expect("horizontal nest");
    let Value::List(existing) = attrs[5].clone() else {
        panic!("RelatedObjects must be a list");
    };
    let mut children = existing;
    children.push(Value::Ref(wrapper));
    attrs[5] = Value::List(children);
    model.insert(nest, Entity::new("IFCRELNESTS", attrs));
}

/// The sole entity of a given IFC type in the fixture.
fn find_by_type(model: &Model, type_name: &str) -> EntityId {
    model
        .iter()
        .find(|(_, e)| e.type_name.eq_ignore_ascii_case(type_name))
        .map(|(id, _)| id)
        .expect("entity present")
}

/// `(StartDistAlong, HorizontalLength, StartHeight, StartGradient,
/// EndGradient, RadiusOfCurvature, PredefinedType)`.
type ProfileRow<'a> = (f64, f64, f64, f64, f64, Option<f64>, &'a str);

/// The fixture with its vertical profile replaced by `rows`, over a 200 m
/// straight.
fn alignment_with_profile(rows: &[ProfileRow]) -> (Model, EntityId) {
    let (mut model, alignment) = alignment_model_with("LINE", 0.0);
    // Lengthen the straight to 200 m (slot 6, SegmentLength).
    let plan = find_by_type(&model, "IFCALIGNMENTHORIZONTALSEGMENT");
    let mut attrs = model.get(plan).expect("plan").attributes.clone();
    attrs[6] = Value::Real(200.0);
    model.insert(plan, Entity::new("IFCALIGNMENTHORIZONTALSEGMENT", attrs));

    let vertical_layout = find_by_type(&model, "IFCALIGNMENTVERTICAL");
    let mut wrappers = Vec::new();
    for (index, (start, length, height, entry, exit, radius, kind)) in rows.iter().enumerate() {
        let params = EntityId(200 + 2 * index as u64);
        model.insert(
            params,
            Entity::new(
                "IFCALIGNMENTVERTICALSEGMENT",
                vec![
                    Value::Null,
                    Value::Null,
                    Value::Real(*start),
                    Value::Real(*length),
                    Value::Real(*height),
                    Value::Real(*entry),
                    Value::Real(*exit),
                    radius.map_or(Value::Null, Value::Real),
                    Value::Enum(Arc::from(*kind)),
                ],
            ),
        );
        let wrapper = EntityId(201 + 2 * index as u64);
        let mut attrs = vec![Value::Null; 7];
        attrs[0] = Value::Text(Arc::from("vs"));
        attrs.push(Value::Ref(params));
        model.insert(wrapper, Entity::new("IFCALIGNMENTSEGMENT", attrs));
        wrappers.push(Value::Ref(wrapper));
    }
    let (nest, mut attrs) = model
        .iter()
        .filter(|(_, e)| e.type_name.eq_ignore_ascii_case("IFCRELNESTS"))
        .find(|(_, e)| e.attributes[4] == Value::Ref(vertical_layout))
        .map(|(id, e)| (id, e.attributes.clone()))
        .expect("vertical nest");
    attrs[5] = Value::List(wrappers);
    model.insert(nest, Entity::new("IFCRELNESTS", attrs));
    (model, alignment)
}

/// `StartDistAlong` is measured from the start of the horizontal layout, so
/// a profile that begins 50 m before the plan is read at its own stations.
///
/// By hand: a 2% grade from height 50 at station -50 reaches 51.0 at
/// station 0 (plan start) and 52.0 at station 50, where a 200 m crest from
/// +2% to -3% begins: `52 + 0.02 x - 0.05/400 x^2` gives 52.75 at station
/// 150 (x = 100) and 52.1875 at station 200 (x = 150). Read from plan
/// distance 0 instead, station 0 would show
/// 50.0 and station 150 would fall 50 m further along the crest.
#[test]
fn a_profile_is_read_at_its_own_stations_along_the_plan() {
    let (model, alignment) = alignment_with_profile(&[
        (-50.0, 100.0, 50.0, 0.02, 0.02, None, "CONSTANTGRADIENT"),
        (
            50.0,
            200.0,
            52.0,
            0.02,
            -0.03,
            Some(-4000.0),
            "PARABOLICARC",
        ),
    ]);
    let Curve3::Elevated(elevated) =
        ifc_alignment::gradient_curve3(&model, alignment, metres()).expect("composes")
    else {
        panic!("expected an elevated curve");
    };
    for (plan_distance, height, grade) in [
        (0.0, 51.0, 0.02),
        (50.0, 52.0, 0.02),
        (150.0, 52.75, -0.005),
        (200.0, 52.1875, -0.0175),
    ] {
        let z = elevated.elevation.height_at(plan_distance).expect("height");
        let g = elevated.elevation.grade_at(plan_distance).expect("grade");
        assert!((z - height).abs() < 1e-12, "z({plan_distance}) = {z}");
        assert!((g - grade).abs() < 1e-14, "g({plan_distance}) = {g}");
    }
}

/// A profile beginning after the plan start leaves the first stations
/// without heights; the composed curve has no domain to say so, so it is
/// refused rather than extrapolated.
#[test]
fn a_profile_starting_after_the_plan_start_is_refused() {
    let (model, alignment) =
        alignment_with_profile(&[(100.0, 100.0, 50.0, 0.02, 0.02, None, "CONSTANTGRADIENT")]);
    let vertical = find_by_type(&model, "IFCALIGNMENTVERTICAL");
    let error = lower_gradient_curve(&model, alignment, metres()).expect_err("starts at 100 m");
    assert!(
        matches!(
            &error,
            AlignmentError::Unsupported { entity, type_name, detail }
                if *entity == vertical
                    && type_name == "IfcAlignmentVertical"
                    && detail.contains("whole plan")
        ),
        "{error}"
    );
}

/// A profile ending before the plan ends leaves the last stations without
/// heights; refused for the same reason. One ending exactly at the plan end
/// (here 0..200 m on the 200 m straight) composes.
#[test]
fn a_profile_ending_before_the_plan_end_is_refused() {
    let (short, alignment) =
        alignment_with_profile(&[(0.0, 150.0, 50.0, 0.02, 0.02, None, "CONSTANTGRADIENT")]);
    let error = lower_gradient_curve(&short, alignment, metres()).expect_err("ends at 150 m");
    assert!(
        matches!(&error, AlignmentError::Unsupported { detail, .. } if detail.contains("whole plan")),
        "{error}"
    );
    let (full, alignment) =
        alignment_with_profile(&[(0.0, 200.0, 50.0, 0.02, 0.02, None, "CONSTANTGRADIENT")]);
    lower_gradient_curve(&full, alignment, metres()).expect("covers the plan");
}
