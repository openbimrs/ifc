//! IFC4X3 alignment curves, read back from the committed fixture (#243).
//!
//! `synthetic_ifc4x3_alignment_curves.ifc` (see its generator's docstring):
//! plan = 100 m line, clothoid 0 -> 1/200 over 60 m, 50 m arc of R = 200,
//! closing segment; profile = grade 0.02 from 10 m over 0..80, parabola
//! `11.6 + 0.02 t - 0.0001875 t^2` over 80..160, grade -0.01 over 160..210.
//!
//! Heights and straight-line plan points are closed form. Plan points on the
//! clothoid are Fresnel integrals, quoted from an independent mpmath
//! quadrature at 30 digits. The plan's end is checked against the closing
//! segment the exporter wrote, which proves the single intrinsic plan is
//! anchored and chained the way the file's own placements say.

#![cfg(feature = "lowering")]

use axiolid_curve::{CurvatureLaw, Curve3, Elevated3};
use axiolid_model::GeometryNode;
use ifc_geometry::lower::dispatch::{Support, PARTIAL, PLANNED};
use ifc_geometry::lower::{lower_representation_item, LoweredGeometry, LoweringSession};
use ifc_geometry::transform::Transform;
use ifc_geometry::{units, GeometryResult};
use ifc_model::{Codec, Entity, EntityId, Model, Value};
use ifc_step::StepCodec;

fn fixture() -> Model {
    let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../test/fixtures/synthetic-surfaces/synthetic_ifc4x3_alignment_curves.ifc");
    StepCodec.read_path(&path).expect("fixture parses")
}

fn lower(model: &Model, id: EntityId) -> GeometryResult<LoweredGeometry> {
    let scale = units::resolve(model);
    let mut session = LoweringSession::new(model, &scale);
    let root = lower_representation_item(&mut session, id, Transform::identity())?;
    session.finish(root)
}

fn only(model: &Model, type_name: &str) -> EntityId {
    match model.ids_of_type(type_name) {
        [id] => *id,
        other => panic!("expected one {type_name}, found {other:?}"),
    }
}

fn refs(entity: &Entity, slot: usize) -> Vec<EntityId> {
    match &entity.attributes[slot] {
        Value::List(items) => items.iter().filter_map(Value::as_ref_id).collect(),
        other => panic!("expected a list, got {other:?}"),
    }
}

fn reference(entity: &Entity, slot: usize) -> EntityId {
    entity.attributes[slot].as_ref_id().expect("a reference")
}

fn centreline(model: &Model) -> Elevated3 {
    let lowered = lower(model, only(model, "IFCGRADIENTCURVE")).expect("gradient curve");
    match lowered.graph.get(lowered.root) {
        Some(GeometryNode::Curve3(Curve3::Elevated(curve))) => curve.clone(),
        other => panic!("expected an elevated curve, got {other:?}"),
    }
}

fn at(curve: &Elevated3, d: f64) -> [f64; 3] {
    axiolid_evaluate::elevated_point(curve, d)
        .expect("evaluate")
        .to_array()
}

fn assert_near(actual: [f64; 3], expected: [f64; 3], tolerance: f64) {
    for (a, e) in actual.iter().zip(expected) {
        assert!(
            (a - e).abs() <= tolerance,
            "{actual:?} vs {expected:?} at {tolerance}"
        );
    }
}

/// Closed-form value of the polynomial laws the plan carries.
fn curvature(law: &CurvatureLaw, s: f64) -> f64 {
    let poly = |c: &[f64]| c.iter().rev().fold(0.0, |acc, c| acc * s + c);
    match law {
        CurvatureLaw::Constant { curvature } => *curvature,
        CurvatureLaw::Polynomial { coefficients } => poly(coefficients),
        CurvatureLaw::Composite {
            polynomial,
            harmonics,
        } => {
            poly(polynomial)
                + harmonics
                    .iter()
                    .map(|h| h.amplitude * (h.angular_frequency * s + h.phase).sin())
                    .sum::<f64>()
        }
        other => panic!("unexpected law {other:?}"),
    }
}

#[test]
fn the_gradient_curve_heights_are_the_authored_profile() {
    let curve = centreline(&fixture());
    // Grade: 10 + 0.02 * 40. Parabola, t = 40: 11.6 + 0.8 - 0.3.
    // Grade: 12 - 0.01 * 40.
    for (d, z) in [(40.0, 10.8), (120.0, 12.1), (200.0, 11.6), (210.0, 11.5)] {
        let height = at(&curve, d)[2];
        assert!((height - z).abs() <= 1e-12, "z({d}) = {height}, want {z}");
    }
}

#[test]
fn the_gradient_curve_plan_follows_line_clothoid_and_arc() {
    let model = fixture();
    let curve = centreline(&model);
    assert_near(at(&curve, 50.0), [50.0, 0.0, 11.0], 1e-12);
    // 30 m into the clothoid (A^2 = 12000), mpmath quadrature; the height is
    // the parabola at t = 50: 11.6 + 1.0 - 0.0001875 * 2500.
    assert_near(
        at(&curve, 130.0),
        [129.995_781_524_649, 0.374_962_334_275, 12.131_25],
        1e-9,
    );
    // The arc's heading is closed form: 0.15 + 25 / 200.
    let tangent = axiolid_evaluate::elevated_tangent(&curve, 185.0).expect("tangent");
    let heading = tangent.y.atan2(tangent.x);
    assert!((heading - 0.275).abs() <= 1e-12, "{heading}");
}

/// The single plan curve is anchored at the first segment and chained by arc
/// length; its end must be where the exporter put the closing segment.
#[test]
fn the_plan_ends_at_the_authored_closing_segment() {
    let model = fixture();
    let curve = centreline(&model);
    let gradient = model
        .get(only(&model, "IFCGRADIENTCURVE"))
        .expect("gradient");
    let base = model.get(reference(gradient, 2)).expect("base curve");
    let closing = *refs(base, 0).last().expect("segments");
    let placement = model
        .get(reference(model.get(closing).expect("closing"), 1))
        .expect("placement");
    let location = model.get(reference(placement, 0)).expect("location");
    let Value::List(xy) = &location.attributes[0] else {
        panic!("coordinates");
    };
    let xy: Vec<f64> = xy.iter().filter_map(Value::as_f64).collect();
    let end = at(&curve, 210.0);
    assert_near([end[0], end[1], 0.0], [xy[0], xy[1], 0.0], 1e-9);
}

/// Every transition in the gallery runs from straight to 1/200 over 60 m:
/// each spiral family's law, read from its own terms, says so in closed form.
#[test]
fn every_spiral_family_lowers_inside_its_curve_segment() {
    let model = fixture();
    let mut seen = Vec::new();
    for (_, composite) in model
        .iter()
        .filter(|(_, e)| e.type_name.eq_ignore_ascii_case("IFCCOMPOSITECURVE"))
    {
        let [segment] = refs(composite, 0)[..] else {
            continue;
        };
        let parent = reference(model.get(segment).expect("segment"), 4);
        let kind = model.get(parent).expect("parent").type_name.to_string();
        let lowered = lower(&model, segment).expect("gallery segment");
        let Some(GeometryNode::Curve3(Curve3::Intrinsic(spiral))) = lowered.graph.get(lowered.root)
        else {
            panic!("{kind} did not lower to an intrinsic curve");
        };
        assert_eq!(spiral.length, 60.0, "{kind}");
        assert!(spiral.is_planar(), "{kind}");
        assert!(
            curvature(&spiral.curvature, 0.0).abs() <= 1e-15,
            "{kind} k(0)"
        );
        let end = curvature(&spiral.curvature, 60.0);
        assert!((end - 1.0 / 200.0).abs() <= 1e-14, "{kind} k(L) = {end}");
        seen.push(kind.to_ascii_uppercase());
    }
    seen.sort();
    assert_eq!(
        seen,
        [
            "IFCCLOTHOID",
            "IFCCOSINESPIRAL",
            "IFCSECONDORDERPOLYNOMIALSPIRAL",
            "IFCSEVENTHORDERPOLYNOMIALSPIRAL",
            "IFCSINESPIRAL",
            "IFCTHIRDORDERPOLYNOMIALSPIRAL",
        ]
    );
}

/// The standalone parents and the cant curve report their ledger reason.
#[test]
fn standalone_parents_and_cant_report_their_planned_reason() {
    let model = fixture();
    for (kind, reason) in PLANNED {
        for id in model.ids_of_type(kind) {
            let error = lower(&model, *id).expect_err(kind);
            assert!(error.is_unsupported(), "{kind}: {error}");
            assert!(error.to_string().contains(reason), "{kind}: {error}");
        }
    }
}

/// Probe each `PARTIAL` row of the IFC4X3 families against the runtime by
/// editing the fixture one attribute at a time.
#[test]
fn ifc4x3_partial_rows_match_runtime_behaviour() {
    let model = fixture();
    let gradient_id = only(&model, "IFCGRADIENTCURVE");
    let gradient = model.get(gradient_id).expect("gradient").clone();
    let base = model.get(reference(&gradient, 2)).expect("base").clone();
    let plan = refs(&base, 0);
    let profile = refs(&gradient, 0);
    let segment = |id: EntityId| model.get(id).expect("segment").clone();

    let edited = |edit: &dyn Fn(&mut Model)| {
        let mut copy = model.clone();
        edit(&mut copy);
        copy
    };
    let with_slot = |id: EntityId, slot: usize, value: Value| {
        let mut entity = segment(id);
        entity.attributes[slot] = value;
        move |m: &mut Model| m.insert(id, entity.clone())
    };
    let next = model.next_id();

    let probes: Vec<(&str, &str, Model, EntityId)> = vec![
        (
            "IFCCURVESEGMENT",
            "ParentCurve is an IfcLine, IfcCircle or 2D IfcPolyline, \
             measured by IfcLengthMeasure",
            model.clone(),
            plan[2],
        ),
        (
            "IFCCURVESEGMENT",
            "ParentCurve is an IfcSpiral subtype, measured by IfcLengthMeasure",
            model.clone(),
            plan[1],
        ),
        (
            "IFCCURVESEGMENT",
            "SegmentLength is zero (the closing segment of a layout)",
            model.clone(),
            plan[3],
        ),
        (
            "IFCCURVESEGMENT",
            "ParentCurve is a 2D IfcPolynomialCurve with a degree-one \
             coordinate, cut forwards from SegmentStart 0",
            model.clone(),
            profile[1],
        ),
        (
            "IFCCURVESEGMENT",
            "ParentCurve is an IfcPolynomialCurve cut from a non-zero \
             SegmentStart, walked backwards, 3D, or with no degree-one \
             coordinate",
            edited(&with_slot(
                profile[1],
                2,
                Value::Typed {
                    type_name: "IFCLENGTHMEASURE".into(),
                    value: Box::new(Value::Real(5.0)),
                },
            )),
            profile[1],
        ),
        (
            "IFCCURVESEGMENT",
            "SegmentStart or SegmentLength is an IfcParameterValue",
            edited(&with_slot(
                plan[0],
                3,
                Value::Typed {
                    type_name: "IFCPARAMETERVALUE".into(),
                    value: Box::new(Value::Real(1.0)),
                },
            )),
            plan[0],
        ),
        (
            "IFCCURVESEGMENT",
            "Placement is an IfcAxis2PlacementLinear, the segment met on its \
             own or in a plain IfcCompositeCurve",
            edited(&|m: &mut Model| {
                m.insert(
                    next,
                    Entity::new(
                        "IFCPOINTBYDISTANCEEXPRESSION",
                        vec![
                            Value::Typed {
                                type_name: "IFCLENGTHMEASURE".into(),
                                value: Box::new(Value::Real(10.0)),
                            },
                            Value::Null,
                            Value::Null,
                            Value::Null,
                            Value::Ref(gradient_id),
                        ],
                    ),
                );
                m.insert(
                    EntityId(next.0 + 1),
                    Entity::new(
                        "IFCAXIS2PLACEMENTLINEAR",
                        vec![Value::Ref(next), Value::Null, Value::Null],
                    ),
                );
                let mut placed = segment(plan[0]);
                placed.attributes[1] = Value::Ref(EntityId(next.0 + 1));
                m.insert(EntityId(next.0 + 2), placed);
            }),
            EntityId(next.0 + 2),
        ),
        (
            "IFCCURVESEGMENT",
            "Placement is an IfcAxis2PlacementLinear in an IfcGradientCurve",
            edited(&|m: &mut Model| {
                m.insert(
                    next,
                    Entity::new("IFCAXIS2PLACEMENTLINEAR", vec![Value::Null; 3]),
                );
                with_slot(plan[0], 1, Value::Ref(next))(m);
            }),
            gradient_id,
        ),
        (
            "IFCGRADIENTCURVE",
            "horizontal IfcCurveSegments over lines, arcs, spirals and 2D \
             IfcPolynomialCurves; vertical IfcCurveSegments over IfcLine, \
             IfcCircle, an IfcSpiral subtype, or a degree-2 IfcPolynomialCurve \
             that keeps its start tangent",
            model.clone(),
            gradient_id,
        ),
        (
            "IFCGRADIENTCURVE",
            "a vertical parabola or spiral with no following segment, closing \
             segment or EndPoint",
            edited(&|m: &mut Model| {
                let mut g = gradient.clone();
                g.attributes[0] = Value::List(vec![Value::Ref(profile[0]), Value::Ref(profile[1])]);
                m.insert(gradient_id, g);
            }),
            gradient_id,
        ),
        (
            "IFCGRADIENTCURVE",
            "a heading kink, a closed-form position gap, or a profile that does \
             not span the base curve",
            edited(&with_slot(
                plan[2],
                1,
                Value::Ref(
                    segment(plan[1]).attributes[1]
                        .as_ref_id()
                        .expect("placement"),
                ),
            )),
            gradient_id,
        ),
    ];

    for (family, variant, probe, id) in probes {
        let row = PARTIAL
            .iter()
            .find(|v| v.family == family && v.variant == variant)
            .unwrap_or_else(|| panic!("no PARTIAL row for {family} / {variant:?}"));
        let outcome = lower(&probe, id);
        match row.support {
            Support::Admitted => assert!(
                outcome.is_ok(),
                "{family} / {variant}: declared Admitted, got {:?}",
                outcome.err()
            ),
            Support::Refused => {
                let error = outcome
                    .err()
                    .unwrap_or_else(|| panic!("{family} / {variant}: declared Refused"));
                assert!(error.is_unsupported(), "{family} / {variant}: {error}");
            }
            other => panic!("{other:?}"),
        }
    }
}

/// A profile boundary built from `IfcCurveSegment`s is refused by name.
///
/// Profile contours hold only line and circle pieces. The old path read the
/// member through `IfcCompositeCurveSegment` slots and reported a wrong value
/// kind for `SegmentLength`, which reads as a corrupt file.
#[test]
fn a_curve_segment_profile_boundary_is_refused_by_name() {
    let mut model = fixture();
    let gradient = model
        .get(only(&model, "IFCGRADIENTCURVE"))
        .expect("gradient");
    let horizontal = reference(gradient, 2);
    let profile = EntityId(900_000);
    model.insert(
        profile,
        Entity::new(
            "IFCARBITRARYCLOSEDPROFILEDEF",
            vec![
                Value::Enum("AREA".into()),
                Value::Null,
                Value::Ref(horizontal),
            ],
        ),
    );
    let scale = units::resolve(&model);
    let error = ifc_geometry::lower::profile::lower_profile(&model, profile, &scale)
        .expect_err("an IfcCurveSegment boundary is not a contour");
    assert!(
        matches!(
            &error,
            ifc_geometry::GeometryError::Unsupported { type_name, .. }
                if type_name == "IFCCURVESEGMENT"
        ),
        "{error}"
    );
}
