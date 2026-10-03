//! A whole horizontal layout as one exact plan curve (#92), the seam rule
//! across transition spirals (#239), and the `CUBIC` read by arc length
//! (#90).
//!
//! Expected positions are independent of the code under test: the spiral
//! fixture's authored `StartPoint`s were computed outside this crate, and
//! the arc and profile values below are worked by hand from the IFC
//! definitions.

use std::sync::Arc;

use axiolid_curve::{ChainPiece2, CurvatureLaw, Curve2, Curve3, Elevated3};
use axiolid_model::{CurveRelation, GeometryNode, TrimSelector};
use ifc_alignment::{
    lower_gradient_curve, lower_horizontal_layout, lower_horizontal_layout_partial,
    lower_horizontal_plan, lower_horizontal_segment, profile_law, read_vertical_segment,
    AlignmentError, AlignmentUnits, SeamCheck,
};
use ifc_model::{Codec, Entity, EntityId, Model, Value};
use ifc_step::StepCodec;

fn metres() -> AlignmentUnits {
    AlignmentUnits {
        length_to_metres: 1.0,
        angle_to_radians: 1.0,
    }
}

/// The `IfcAlignment` and `IfcAlignmentHorizontal` of the spiral fixture.
const ALIGNMENT: EntityId = EntityId(201);
const HORIZONTAL: EntityId = EntityId(191);

/// line 100 -> CLOTHOID 60 -> arc R300 120 -> CLOTHOID 60 -> line 100.
fn spiral_fixture() -> Model {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../test/fixtures/synthetic-surfaces/synthetic_alignment_spiral.ifc");
    StepCodec.read_path(&path).expect("fixture parses")
}

/// One vertical segment: (start, length, height, entry, exit, kind).
type Vertical = (f64, f64, f64, f64, f64, &'static str);

/// The profile the spiral fixture is elevated over, 440 m long:
/// +2% from 10.0 over 150 m, a 200 m parabola from +2% to -1%, then -1%.
const PROFILE: [Vertical; 3] = [
    (0.0, 150.0, 10.0, 0.02, 0.02, "CONSTANTGRADIENT"),
    (150.0, 200.0, 13.0, 0.02, -0.01, "PARABOLICARC"),
    (350.0, 90.0, 14.0, -0.01, -0.01, "CONSTANTGRADIENT"),
];

/// Insert a vertical layout under `alignment`, next to its horizontal one.
/// Returns the vertical segment parameter entities in order.
fn add_profile(model: &mut Model, alignment: EntityId, profile: &[Vertical]) -> Vec<EntityId> {
    let mut id = 5000;
    let mut next = || {
        id += 1;
        EntityId(id)
    };
    let mut params = Vec::new();
    let mut wrappers = Vec::new();
    for (start, length, height, entry, exit, kind) in profile {
        let segment = next();
        model.insert(
            segment,
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
                    // A parabola states its radius, L / (g2 - g1); a grade has none.
                    if *kind == "PARABOLICARC" {
                        Value::Real(length / (exit - entry))
                    } else {
                        Value::Null
                    },
                    Value::Enum(Arc::from(*kind)),
                ],
            ),
        );
        let wrapper = next();
        model.insert(
            wrapper,
            Entity::new("IFCALIGNMENTSEGMENT", wrapper_attrs(segment)),
        );
        params.push(segment);
        wrappers.push(wrapper);
    }
    let layout = next();
    model.insert(layout, Entity::new("IFCALIGNMENTVERTICAL", product("V")));
    let nest = next();
    model.insert(nest, nests(layout, wrappers));

    // Add the vertical layout to the alignment's existing nest.
    let (alignment_nest, mut attrs) = model
        .iter()
        .filter(|(_, e)| e.type_name.eq_ignore_ascii_case("IFCRELNESTS"))
        .find(|(_, e)| e.attributes[4] == Value::Ref(alignment))
        .map(|(id, e)| (id, e.attributes.clone()))
        .expect("alignment nest");
    let Value::List(mut children) = attrs[5].clone() else {
        panic!("RelatedObjects must be a list");
    };
    children.push(Value::Ref(layout));
    attrs[5] = Value::List(children);
    model.insert(alignment_nest, Entity::new("IFCRELNESTS", attrs));
    params
}

fn product(name: &str) -> Vec<Value> {
    let mut attrs = vec![Value::Null; 7];
    attrs[0] = Value::Text(Arc::from(name));
    attrs
}

fn wrapper_attrs(design: EntityId) -> Vec<Value> {
    let mut attrs = product("segment");
    attrs.push(Value::Ref(design));
    attrs
}

fn nests(parent: EntityId, children: Vec<EntityId>) -> Entity {
    Entity::new(
        "IFCRELNESTS",
        vec![
            Value::Text(Arc::from("nest")),
            Value::Null,
            Value::Null,
            Value::Null,
            Value::Ref(parent),
            Value::List(children.into_iter().map(Value::Ref).collect()),
        ],
    )
}

/// One horizontal segment: (x, y, direction, start R, end R, length, kind).
type Horizontal = (f64, f64, f64, f64, f64, f64, &'static str);

/// An alignment with only a horizontal layout of the given segments.
/// Returns (model, alignment, horizontal layout, segment parameter ids).
fn layout_model(segments: &[Horizontal]) -> (Model, EntityId, EntityId, Vec<EntityId>) {
    let mut model = Model::new();
    model.header_mut().schema = vec!["IFC4X3_ADD2".to_owned()];
    let mut id = 0;
    let mut next = || {
        id += 1;
        EntityId(id)
    };
    let mut params = Vec::new();
    let mut wrappers = Vec::new();
    for (x, y, direction, start_radius, end_radius, length, kind) in segments {
        let point = next();
        model.insert(
            point,
            Entity::new(
                "IFCCARTESIANPOINT",
                vec![Value::List(vec![Value::Real(*x), Value::Real(*y)])],
            ),
        );
        let segment = next();
        model.insert(
            segment,
            Entity::new(
                "IFCALIGNMENTHORIZONTALSEGMENT",
                vec![
                    Value::Null,
                    Value::Null,
                    Value::Ref(point),
                    Value::Real(*direction),
                    Value::Real(*start_radius),
                    Value::Real(*end_radius),
                    Value::Real(*length),
                    Value::Null,
                    Value::Enum(Arc::from(*kind)),
                ],
            ),
        );
        let wrapper = next();
        model.insert(
            wrapper,
            Entity::new("IFCALIGNMENTSEGMENT", wrapper_attrs(segment)),
        );
        params.push(segment);
        wrappers.push(wrapper);
    }
    let horizontal = next();
    model.insert(
        horizontal,
        Entity::new("IFCALIGNMENTHORIZONTAL", product("H")),
    );
    let nest = next();
    model.insert(nest, nests(horizontal, wrappers));
    let alignment = next();
    let mut attrs = product("A");
    attrs.push(Value::Null);
    model.insert(alignment, Entity::new("IFCALIGNMENT", attrs));
    let nest = next();
    model.insert(nest, nests(alignment, vec![horizontal]));
    (model, alignment, horizontal, params)
}

fn elevated(lowered: &ifc_alignment::LoweredAlignmentCurve) -> &Elevated3 {
    let Some(GeometryNode::Curve3(Curve3::Elevated(elevated))) = lowered.graph.get(lowered.root)
    else {
        panic!("the root must be an elevated 3D curve");
    };
    elevated
}

/// The done-when of #92: a line -> clothoid -> arc plan elevates, and the
/// heights at the plan seams are the profile's own heights there.
#[test]
fn a_line_clothoid_arc_plan_elevates_with_profile_heights_at_its_seams() {
    let mut model = spiral_fixture();
    let vertical_ids = add_profile(&mut model, ALIGNMENT, &PROFILE);
    let lowered = lower_gradient_curve(&model, ALIGNMENT, metres()).expect("elevates");
    let elevated = elevated(&lowered);

    // One intrinsic curve, one piece per segment, seams at the segment
    // boundaries, and the law of each piece exactly what the segment states.
    let Curve2::Intrinsic(plan) = elevated.plan.as_ref() else {
        panic!("the plan must be one intrinsic curve");
    };
    assert_eq!(plan.length, 440.0);
    let CurvatureLaw::Piecewise { breaks, laws } = &plan.curvature else {
        panic!("a five-segment plan carries a piecewise law");
    };
    assert_eq!(breaks, &vec![100.0, 160.0, 280.0, 340.0]);
    assert_eq!(laws.len(), 5);
    assert!(laws[0].is_straight() && laws[4].is_straight());
    assert_eq!(laws[2].constant_value(), Some(1.0 / 300.0));
    assert_eq!(
        laws[1],
        CurvatureLaw::Polynomial {
            coefficients: vec![0.0, (1.0 / 300.0) / 60.0]
        }
    );
    // Heading is closed form: the clothoids each turn 0.1 rad and the arc
    // 120/300 = 0.4 rad, as the authored StartDirections state.
    let heading = plan.total_turning().expect("closed form");
    assert!((heading - 0.6).abs() < 1e-12, "turning was {heading}");

    // Seam positions: the authored StartPoints, which the plan does NOT
    // store past the first segment. Matching them means the single curve
    // is the authored road, not a curve that merely starts in the right place.
    let seams = [
        (100.0, 100.0, 0.0),
        (160.0, 159.94002777137186, 1.9985718830375365),
        (280.0, 273.81766435858435, 37.225052899333434),
        (340.0, 324.4167826082655, 69.42024588149852),
    ];
    // Profile heights at those distances, by hand from
    // z = H + g1 t + (g2 - g1) / (2 L) t^2 in each piece's own distance t:
    //   100: 10 + 0.02*100                            = 12.0
    //   160: 13 + 0.02*10  - 0.03/400 * 10^2           = 13.1925
    //   280: 13 + 0.02*130 - 0.03/400 * 130^2          = 14.3325
    //   340: 13 + 0.02*190 - 0.03/400 * 190^2          = 14.0925
    let by_hand = [12.0, 13.1925, 14.3325, 14.0925];
    let vertical: Vec<_> = vertical_ids
        .iter()
        .map(|id| read_vertical_segment(&model, *id, metres()).expect("vertical"))
        .collect();
    let profile = profile_law(&vertical).expect("profile");
    for ((distance, x, y), height) in seams.into_iter().zip(by_hand) {
        let point =
            axiolid_evaluate::arc_length::elevated_point(elevated, distance).expect("evaluates");
        assert!(
            (point.x - x).abs() < 1e-6 && (point.y - y).abs() < 1e-6,
            "plan at {distance}: ({}, {}) vs authored ({x}, {y})",
            point.x,
            point.y
        );
        let expected = profile.height_at(distance).expect("profile height");
        assert!(
            (point.z - expected).abs() < 1e-12,
            "z {} vs profile_law {expected} at {distance}",
            point.z
        );
        assert!(
            (point.z - height).abs() < 1e-9,
            "z {} vs hand {height}",
            point.z
        );
    }

    // The seams after the clothoids are reported as authored, not verified.
    let checks: Vec<(u64, SeamCheck)> = lowered
        .seams
        .iter()
        .map(|seam| (seam.next.0, seam.position))
        .collect();
    assert_eq!(
        checks,
        vec![
            (103, SeamCheck::Verified),
            (105, SeamCheck::Authored),
            (107, SeamCheck::Verified),
            (109, SeamCheck::Authored),
        ]
    );
}

/// `lower_horizontal_plan` and the gradient curve share one plan.
#[test]
fn the_plan_is_available_without_a_profile() {
    let model = spiral_fixture();
    let plan = lower_horizontal_plan(&model, HORIZONTAL, metres(), None).expect("plan");
    assert_eq!(
        plan.sources.iter().map(|id| id.0).collect::<Vec<_>>(),
        vec![101, 103, 105, 107, 109]
    );
    assert_eq!(plan.seams.len(), 4);
    let Curve2::Intrinsic(curve) = &plan.curve else {
        panic!("intrinsic plan");
    };
    // End of the road: the last line's authored start plus 100 m at 0.6 rad.
    let end = axiolid_evaluate::arc_length::intrinsic_point(curve, 440.0).expect("end");
    let (x, y) = (
        324.4167826082655 + 100.0 * 0.6_f64.cos(),
        69.42024588149852 + 100.0 * 0.6_f64.sin(),
    );
    assert!((end.x - x).abs() < 1e-6 && (end.y - y).abs() < 1e-6);
}

/// A heading kink cannot be carried by one intrinsic curve, so the plan
/// refuses it. The per-segment composite, which keeps each authored frame,
/// still accepts it as before.
#[test]
fn a_heading_kink_is_refused_by_the_plan_but_not_by_the_composite() {
    let (model, _, horizontal, ids) = layout_model(&[
        (0.0, 0.0, 0.0, 0.0, 0.0, 100.0, "LINE"),
        (100.0, 0.0, 0.3, 0.0, 0.0, 50.0, "LINE"),
    ]);
    let error = lower_horizontal_plan(&model, horizontal, metres(), None)
        .expect_err("a kink is not one smooth curve");
    assert!(
        matches!(
            &error,
            AlignmentError::SemanticViolation { entity: Some(entity), rule }
                if *entity == ids[1] && rule.contains("tangent direction")
        ),
        "got {error:?}"
    );
    lower_horizontal_layout(&model, horizontal, metres(), None)
        .expect("the composite keeps each segment's authored frame");
}

/// The heading check runs after a spiral too, because the turning integral
/// is closed form even where the end point is not.
#[test]
fn a_heading_kink_after_a_spiral_is_refused() {
    let (model, _, horizontal, _) = layout_model(&[
        (0.0, 0.0, 0.0, 0.0, 300.0, 60.0, "CLOTHOID"),
        // The clothoid turns 0.1 rad; 0.2 is a kink.
        (59.94, 2.0, 0.2, 300.0, 300.0, 50.0, "CIRCULARARC"),
    ]);
    let error = lower_horizontal_plan(&model, horizontal, metres(), None).expect_err("kink");
    assert!(
        matches!(&error, AlignmentError::SemanticViolation { rule, .. } if rule.contains("tangent")),
        "got {error:?}"
    );
}

/// A position gap after a line is closed form, so it is refused, not
/// recorded as authored.
#[test]
fn a_position_gap_after_a_line_is_refused() {
    let (model, _, horizontal, _) = layout_model(&[
        (0.0, 0.0, 0.0, 0.0, 0.0, 100.0, "LINE"),
        (100.5, 0.0, 0.0, 0.0, 0.0, 50.0, "LINE"),
    ]);
    let error = lower_horizontal_plan(&model, horizontal, metres(), None).expect_err("gap");
    assert!(
        matches!(&error, AlignmentError::SemanticViolation { rule, .. } if rule.contains("endpoint")),
        "got {error:?}"
    );
}

/// A clockwise arc elevates forward along the road.
///
/// By hand: start (0, 0) heading +x, R = -100 (clockwise), so the centre is
/// (0, -100). At 50 m the arc has turned 0.5 rad, giving
/// (100 sin 0.5, -100 + 100 cos 0.5) = (47.9425538604203, -12.2417438109627).
#[test]
fn a_clockwise_arc_elevates_forward() {
    let (mut model, alignment, _, _) =
        layout_model(&[(0.0, 0.0, 0.0, -100.0, -100.0, 50.0, "CIRCULARARC")]);
    add_profile(
        &mut model,
        alignment,
        &[(0.0, 50.0, 5.0, 0.0, 0.0, "CONSTANTGRADIENT")],
    );
    let lowered = lower_gradient_curve(&model, alignment, metres()).expect("elevates");
    let point =
        axiolid_evaluate::arc_length::elevated_point(elevated(&lowered), 50.0).expect("evaluates");
    assert!(
        (point.x - 47.942_553_860_420_3).abs() < 1e-9,
        "x {}",
        point.x
    );
    assert!(
        (point.y - -12.241_743_810_962_7).abs() < 1e-9,
        "y {}",
        point.y
    );
}

/// A malformed CUBIC is invalid data, named before the capability gap.
#[test]
fn a_cubic_with_equal_radii_is_an_invalid_segment() {
    let (model, _, _, ids) = layout_model(&[(0.0, 0.0, 0.0, 300.0, 300.0, 60.0, "CUBIC")]);
    let error = lower_horizontal_segment(&model, ids[0], metres()).expect_err("malformed");
    assert!(
        matches!(&error, AlignmentError::InvalidSegment { entity, detail }
            if *entity == ids[0] && detail.contains("radii must differ")),
        "got {error:?}"
    );
}

/// The end of a CUBIC `y = x^3 / (6 R L)` by arc length, computed here
/// independently of the evaluator: composite Simpson quadrature of
/// `sqrt(1 + (x^2 / (2 R L))^2)` and bisection for the abscissa where it
/// reaches `L`.
fn cubic_end(radius: f64, length: f64) -> (f64, f64) {
    let slope = |x: f64| x * x / (2.0 * radius * length);
    let arc = |x: f64| {
        let n = 2000;
        let h = x / n as f64;
        let f = |t: f64| (1.0 + slope(t).powi(2)).sqrt();
        let mut sum = f(0.0) + f(x);
        for i in 1..n {
            sum += f(i as f64 * h) * if i % 2 == 1 { 4.0 } else { 2.0 };
        }
        sum * h / 3.0
    };
    let (mut lo, mut hi) = (0.0, length);
    for _ in 0..200 {
        let mid = 0.5 * (lo + hi);
        if arc(mid) < length {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    let x = 0.5 * (lo + hi);
    (x, x.powi(3) / (6.0 * radius * length))
}

/// A CUBIC lowers exactly on every path (#90): its cubic parabola, trimmed
/// where its arc length reaches `SegmentLength`.
///
/// Line 100 m east, then a CUBIC leaving the straight to `R = 300` over
/// `L = 60` m. IFC4X3_ADD2 (`IfcAlignmentHorizontalSegmentTypeEnum`):
/// `y = x^3 / (6 R L)`; `SegmentLength` is measured along the curve. The
/// cubic is stored as the Bezier `(0, 0)`, `(20, 0)`, `(40, 0)`,
/// `(60, 60^2 / 1800 = 2)` placed at `(100, 0)`.
#[test]
fn a_cubic_lowers_exactly_on_every_path() {
    let (mut model, alignment, horizontal, ids) = layout_model(&[
        (0.0, 0.0, 0.0, 0.0, 0.0, 100.0, "LINE"),
        (100.0, 0.0, 0.0, 0.0, 300.0, 60.0, "CUBIC"),
    ]);
    let (x_end, y_end) = cubic_end(300.0, 60.0);

    // Per segment: the Bezier and the arc-length trim.
    let single = lower_horizontal_segment(&model, ids[1], metres()).expect("cubic");
    let Some(GeometryNode::CurveRelation(CurveRelation::Trimmed {
        basis, start, end, ..
    })) = single.graph.get(single.root)
    else {
        panic!("a CUBIC is a trimmed curve");
    };
    assert_eq!(start.as_slice(), [TrimSelector::Parameter(0.0)]);
    assert_eq!(end.as_slice(), [TrimSelector::ArcLength(60.0)]);
    let Some(GeometryNode::Curve2(Curve2::BSpline(bezier))) = single.graph.get(*basis) else {
        panic!("its basis is a B-spline");
    };
    assert_eq!(bezier.degree, 3);
    assert_eq!(bezier.knots, vec![0.0, 60.0]);
    assert_eq!(bezier.multiplicities, vec![4, 4]);
    assert!(bezier.weights.is_none());
    let expected = [(100.0, 0.0), (120.0, 0.0), (140.0, 0.0), (160.0, 2.0)];
    for (point, (x, y)) in bezier.control_points.iter().zip(expected) {
        assert!((point.x - x).abs() < 1e-12 && (point.y - y).abs() < 1e-12);
    }
    // The curve is y = x^3 / (6 R L) in its own frame, at any abscissa.
    let curve = Curve2::BSpline(bezier.clone());
    for x in [0.0, 15.0, 30.0, 45.0, 60.0] {
        let point = axiolid_evaluate::evaluate2(&curve, x).expect("point");
        assert!((point.x - (100.0 + x)).abs() < 1e-12, "x at {x}");
        assert!((point.y - x.powi(3) / 108_000.0).abs() < 1e-12, "y at {x}");
    }
    // The evaluator resolves the trim where the independent quadrature does.
    let resolved = axiolid_evaluate::parameter_at_arc_length2(&curve, 0.0, 60.0).expect("trim");
    assert!((resolved - x_end).abs() < 1e-9, "{resolved} != {x_end}");

    // The composite and the partial walk accept it; its seam after the line
    // is verified, the line's end being closed form.
    let strict = lower_horizontal_layout(&model, horizontal, metres(), None).expect("strict");
    assert_eq!(strict.seams[0].position, SeamCheck::Verified);
    let partial =
        lower_horizontal_layout_partial(&model, horizontal, metres(), None).expect("partial");
    assert!(partial.is_complete());
    assert_eq!(partial.runs.len(), 1);

    // The plan is an arc-length chain: the straight, then the cubic read by
    // arc length in its own frame.
    let plan = lower_horizontal_plan(&model, horizontal, metres(), None).expect("plan");
    let Curve2::Chain(chain) = &plan.curve else {
        panic!("a plan with a CUBIC is a chain, got {:?}", plan.curve);
    };
    assert_eq!(chain.length(), Some(160.0));
    assert!(matches!(
        &chain.pieces[0],
        ChainPiece2::Intrinsic { length, curvature } if *length == 100.0 && curvature.is_straight()
    ));
    let ChainPiece2::Parametric { start, length, .. } = &chain.pieces[1] else {
        panic!("the CUBIC is a parametric piece");
    };
    assert_eq!((*start, *length), (0.0, 60.0));
    let end = axiolid_evaluate::chain_point(chain, 160.0).expect("end");
    assert!(
        (end.x - (100.0 + x_end)).abs() < 1e-9,
        "{} vs {}",
        end.x,
        100.0 + x_end
    );
    assert!((end.y - y_end).abs() < 1e-9, "{} vs {y_end}", end.y);

    // Seam continuity at the join, through the evaluator: position and
    // heading approach the join from both sides.
    for side in [100.0 - 1e-9, 100.0 + 1e-9] {
        let point = axiolid_evaluate::chain_point(chain, side).expect("point");
        let tangent = axiolid_evaluate::chain_tangent(chain, side).expect("tangent");
        assert!((point.x - 100.0).abs() < 1e-8 && point.y.abs() < 1e-8);
        assert!((tangent.x - 1.0).abs() < 1e-12 && tangent.y.abs() < 1e-12);
    }

    // And it elevates: height at plan distance, on the chain's position.
    add_profile(
        &mut model,
        alignment,
        &[(0.0, 160.0, 5.0, 0.01, 0.01, "CONSTANTGRADIENT")],
    );
    let lowered = lower_gradient_curve(&model, alignment, metres()).expect("elevates");
    let point = axiolid_evaluate::elevated_point(elevated(&lowered), 160.0).expect("point");
    assert!((point.x - (100.0 + x_end)).abs() < 1e-9);
    assert!((point.z - 6.6).abs() < 1e-12, "z {}", point.z);
}

/// After a CUBIC neither the end point nor the end heading is closed form:
/// the next segment's start is reported as authored, not refused.
#[test]
fn the_seam_after_a_cubic_is_authored() {
    let (x_end, y_end) = cubic_end(300.0, 60.0);
    let heading = (x_end * x_end / (2.0 * 300.0 * 60.0)).atan();
    let (model, _, horizontal, _) = layout_model(&[
        (0.0, 0.0, 0.0, 0.0, 300.0, 60.0, "CUBIC"),
        (x_end, y_end, heading, 300.0, 300.0, 20.0, "CIRCULARARC"),
    ]);
    let plan = lower_horizontal_plan(&model, horizontal, metres(), None).expect("plan");
    assert_eq!(plan.seams[0].position, SeamCheck::Authored);
    let strict = lower_horizontal_layout(&model, horizontal, metres(), None).expect("strict");
    assert_eq!(strict.seams[0].position, SeamCheck::Authored);
}

/// IFC4.3 states the CUBIC only as leaving a straight; one that starts
/// curved is a typed refusal naming that, on every path.
#[test]
fn a_cubic_that_starts_curved_is_refused_by_name() {
    let (model, _, horizontal, ids) = layout_model(&[(0.0, 0.0, 0.0, 600.0, 300.0, 60.0, "CUBIC")]);
    let is_gap = |error: &AlignmentError| {
        matches!(error, AlignmentError::Unsupported { entity, type_name, detail }
            if *entity == ids[0] && type_name == "CUBIC" && detail.contains("starting curved"))
    };
    let single = lower_horizontal_segment(&model, ids[0], metres()).expect_err("curved start");
    assert!(is_gap(&single), "{single:?}");
    let plan = lower_horizontal_plan(&model, horizontal, metres(), None).expect_err("plan");
    assert!(is_gap(&plan), "{plan:?}");
    let partial =
        lower_horizontal_layout_partial(&model, horizontal, metres(), None).expect("readable");
    assert!(is_gap(&partial.refused[0].reason));
}
