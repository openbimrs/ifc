//! Plan and vertical profile composed as an exact 3D centreline.
//!
//! Heights are checked against values computed by hand from the IFC
//! definition, not by re-running the same code path that produced them.

use axiolid_curve::{Curve3, ElevationLaw};
use axiolid_model::GeometryNode;
use ifc_alignment::{
    lower_gradient_curve, profile_law, AlignmentUnits, VerticalSegment, VerticalSegmentType,
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

/// A circular vertical curve has no exact polynomial form in plan distance.
/// Substituting a parabola would move the road surface, so it is refused.
#[test]
fn a_circular_vertical_curve_is_refused_not_approximated() {
    let segment = vertical(
        0.0,
        200.0,
        100.0,
        0.03,
        -0.02,
        VerticalSegmentType::CircularArc,
    );
    let error = profile_law(std::slice::from_ref(&segment)).expect_err("no exact law");
    assert!(
        format!("{error}").contains("exact"),
        "the refusal must name the gap: {error}"
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
    let error = profile_law(std::slice::from_ref(&segment))
        .expect_err("a zero-length parabola must not produce an infinite coefficient");
    // The length is named as the fault, rather than the non-finite
    // coefficient it would otherwise produce downstream.
    let text = format!("{error}");
    assert!(
        text.contains("positive horizontal length"),
        "refusal was: {text}"
    );
}

/// `Elevated3.plan` is a single `Curve2` and the neutral vocabulary has no
/// composite `Curve2`, so a two-segment plan cannot be elevated. Flattening
/// it to a B-spline would discard the exact spiral this composition exists
/// to preserve, and elevating only the first segment would silently drop
/// the rest of the road. Refused by name instead.
#[test]
fn a_multi_segment_plan_is_refused_rather_than_partially_elevated() {
    // Two collinear lines: both lowerable, and continuous at the join, so
    // the refusal under test is the composite one and not a continuity rule.
    let (mut model, alignment) = alignment_model_with("LINE", 0.0);
    add_second_plan_segment(&mut model);
    let error = lower_gradient_curve(&model, alignment, metres())
        .expect_err("a composite plan has no single Curve2");
    let text = format!("{error}");
    assert!(
        text.contains("multi-segment"),
        "refusal must name the cause: {text}"
    );
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
