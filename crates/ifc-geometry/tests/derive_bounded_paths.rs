//! Derived linear placements on spans that start inside a curve whose
//! measure is a quadrature, and on paths with a piece the evaluator cannot
//! certify away from the placement (#436).
//!
//! A trim of an ellipse or a B-spline at an interior parameter, a point or
//! an arc length has no stated distance in the curve's measure, so this
//! crate cannot state it (ADR 0004). It reaches the evaluator as a bounded
//! `PathPiece` (`between_parameters`, `bounded`, axiolid/kernel#304), each
//! end resolved by the evaluator as the compiler resolves the same trim.
//! Joints are certified only near the placement
//! (`path_joint_distances_within`, axiolid/kernel#305), so a piece whose
//! length cannot be certified refuses a placement only when it lies before
//! the placement's window or reaches it.
//!
//! Each case is checked against the lowered station of the same
//! `IfcPointByDistanceExpression`, resolved through the reference kernel
//! with its seam-snapping window, and the path against the one
//! `axiolid-mesh-compile`'s `station::curve_path` flattens the lowered
//! basis into.
//!
//! Fixture: `synthetic-lowering/station_offset_bases_ifc4x3.ifc` (model
//! precision 1e-5 m), for its context, the line `#17`, the clothoid `#64`
//! and the tilted `IfcOffsetCurve3D` `#97` beside the gradient curve `#60`
//! (two pieces, split at the plan joint 30 m along, neither of whose
//! lengths the kernel certifies). Every other record is appended here.

#![cfg(feature = "compile-reference-backend")]

use axiolid_contracts::{Backend, BackendDescriptor, GeomError, GeomResult, Operation};
use axiolid_core::{Frame3, Point3, Vec3};
use axiolid_curve::{Curve3, CurvePath, PathBound};
use axiolid_curve_evaluate_contract::{
    CurveEvaluator, CurveMeasure, DistanceConvention, SeamSide, PATH_PIECE_BOUND_UNSUPPORTED,
    PATH_POINT_AMBIGUOUS, PATH_POINT_OFF_CURVE,
};
use axiolid_evaluate::ReferenceCurveEvaluator;
use axiolid_mesh_compile::station::{curve_path, resolve};
use axiolid_model::{GeometryGraph, GeometryNode, NodeId};
use ifc_alignment::AlignmentUnits;
use ifc_geometry::compile::SeamWindowRefusal;
use ifc_geometry::constraint::placement::derive::{basis_curve_path, derive_placement_transform};
use ifc_geometry::lower::{lower_point_by_distance_node, LoweringSession};
use ifc_geometry::{units, GeometryError, Transform};
use ifc_model::{Codec, EntityId, Model};
use std::path::PathBuf;

const EPS: f64 = 1e-9;

/// The bases appended to the fixture.
///
/// - `#2010`: a composite of the ellipse `#2002` (semi-axes 4 and 2)
///   trimmed between its parameters 0.3 and 1.2 (`#2003`), then a polyline
///   5 m west from the trim's end: a corner joint after a bounded piece.
/// - `#2011`: the ellipse trimmed from parameter 5.5 to 0.7 in its sense,
///   across its parameter seam; `#2012` the same arc trimmed from 0.7 to
///   5.5 against its sense.
/// - `#2013`: the CUBIC transition, an `IfcCurveSegment` of an
///   `IfcPolynomialCurve` `y = x^3 / 18000` (radius 100 m after 30 m),
///   which lowers to its Bezier trimmed from parameter 0 to an arc length
///   of 30 m.
/// - `#2024`: the quadratic B-spline `#2023` through `(0, 0)`, `(5, 5)` and
///   `(10, 0)`, `(10 t, 10 t (1 - t))`, trimmed by its points at `t = 0.25`
///   and `0.75`; `#2033` trimmed by those parameters; `#2025` trimmed at a
///   point 0.025 m above it.
/// - `#2032`: the closed quadratic B-spline `#2031` round a square from
///   `(0, 0)` back to it, trimmed from that point, which both ends of its
///   domain name, to parameter 1.5.
/// - `#2048`: `IfcOffsetCurve3D` 1 m along `(0, 1, 0) x T` from the
///   composite `#2047` of a 10 m line, the 20 m clothoid `#64` and a 20 m
///   line: the offset of a line is a line, whose length the kernel states,
///   and beside the clothoid, where the offset leaves its plane, the kernel
///   certifies no length (`UNCERTIFIED_LENGTH`).
/// - `#2056`: a composite of a 10 m polyline east to `(4, 0)` and the whole
///   ellipse from there, trimmed between its distances 2 and 15; `#2061` a
///   composite of the whole ellipse and a polyline after it, trimmed alike.
const BASES: &str = "#2000=IFCCARTESIANPOINT((0.,0.));
#2001=IFCAXIS2PLACEMENT2D(#2000,$);
#2002=IFCELLIPSE(#2001,4.,2.);
#2003=IFCTRIMMEDCURVE(#2002,(IFCPARAMETERVALUE(0.3)),(IFCPARAMETERVALUE(1.2)),.T.,.PARAMETER.);
#2004=IFCCARTESIANPOINT((1.4494310179066945,1.8640781719344526));
#2005=IFCCARTESIANPOINT((-3.5505689820933055,1.8640781719344526));
#2006=IFCPOLYLINE((#2004,#2005));
#2007=IFCCOMPOSITECURVESEGMENT(.CONTINUOUS.,.T.,#2003);
#2008=IFCCOMPOSITECURVESEGMENT(.DISCONTINUOUS.,.T.,#2006);
#2010=IFCCOMPOSITECURVE((#2007,#2008),.F.);
#2011=IFCTRIMMEDCURVE(#2002,(IFCPARAMETERVALUE(5.5)),(IFCPARAMETERVALUE(0.7)),.T.,.PARAMETER.);
#2012=IFCTRIMMEDCURVE(#2002,(IFCPARAMETERVALUE(0.7)),(IFCPARAMETERVALUE(5.5)),.F.,.PARAMETER.);
#2014=IFCCURVESEGMENT(.DISCONTINUOUS.,#24,IFCLENGTHMEASURE(0.),IFCLENGTHMEASURE(30.),#2015);
#2015=IFCPOLYNOMIALCURVE(#24,(0.,1.),(0.,0.,0.,5.5555555555555556E-05),$);
#2013=IFCCOMPOSITECURVE((#2014),.F.);
#2020=IFCCARTESIANPOINT((0.,0.));
#2021=IFCCARTESIANPOINT((5.,5.));
#2022=IFCCARTESIANPOINT((10.,0.));
#2023=IFCBSPLINECURVEWITHKNOTS(2,(#2020,#2021,#2022),.UNSPECIFIED.,.F.,.F.,(3,3),(0.,1.),.UNSPECIFIED.);
#2026=IFCCARTESIANPOINT((2.5,1.875));
#2027=IFCCARTESIANPOINT((7.5,1.875));
#2028=IFCCARTESIANPOINT((2.5,1.9));
#2024=IFCTRIMMEDCURVE(#2023,(#2026),(#2027),.T.,.CARTESIAN.);
#2025=IFCTRIMMEDCURVE(#2023,(#2028),(#2027),.T.,.CARTESIAN.);
#2029=IFCCARTESIANPOINT((10.,10.));
#2030=IFCCARTESIANPOINT((0.,10.));
#2031=IFCBSPLINECURVEWITHKNOTS(2,(#2020,#2022,#2029,#2030,#2020),.UNSPECIFIED.,.F.,.F.,(3,1,1,3),(0.,1.,2.,3.),.UNSPECIFIED.);
#2032=IFCTRIMMEDCURVE(#2031,(#2020),(IFCPARAMETERVALUE(1.5)),.T.,.CARTESIAN.);
#2033=IFCTRIMMEDCURVE(#2023,(IFCPARAMETERVALUE(0.25)),(IFCPARAMETERVALUE(0.75)),.T.,.PARAMETER.);
#2040=IFCCARTESIANPOINT((10.,0.));
#2041=IFCAXIS2PLACEMENT2D(#2040,#26);
#2042=IFCCARTESIANPOINT((28.090484758005331,6.2053660344676329));
#2043=IFCAXIS2PLACEMENT2D(#2042,#69);
#2044=IFCCURVESEGMENT(.CONTSAMEGRADIENT.,#24,IFCLENGTHMEASURE(0.),IFCLENGTHMEASURE(10.),#17);
#2045=IFCCURVESEGMENT(.CONTSAMEGRADIENT.,#2041,IFCLENGTHMEASURE(0.),IFCLENGTHMEASURE(20.),#64);
#2046=IFCCURVESEGMENT(.DISCONTINUOUS.,#2043,IFCLENGTHMEASURE(0.),IFCLENGTHMEASURE(20.),#17);
#2047=IFCCOMPOSITECURVE((#2044,#2045,#2046),.F.);
#2049=IFCDIRECTION((0.,1.,0.));
#2048=IFCOFFSETCURVE3D(#2047,1.,.F.,#2049);
#2050=IFCCARTESIANPOINT((-6.,0.));
#2051=IFCCARTESIANPOINT((4.,0.));
#2052=IFCPOLYLINE((#2050,#2051));
#2053=IFCCOMPOSITECURVESEGMENT(.DISCONTINUOUS.,.T.,#2052);
#2054=IFCCOMPOSITECURVESEGMENT(.DISCONTINUOUS.,.T.,#2002);
#2055=IFCCOMPOSITECURVE((#2053,#2054),.F.);
#2056=IFCTRIMMEDCURVE(#2055,(IFCPARAMETERVALUE(2.)),(IFCPARAMETERVALUE(15.)),.T.,.PARAMETER.);
#2057=IFCCARTESIANPOINT((4.,-10.));
#2058=IFCPOLYLINE((#2051,#2057));
#2059=IFCCOMPOSITECURVESEGMENT(.DISCONTINUOUS.,.T.,#2058);
#2060=IFCCOMPOSITECURVE((#2054,#2059),.F.);
#2061=IFCTRIMMEDCURVE(#2060,(IFCPARAMETERVALUE(2.)),(IFCPARAMETERVALUE(15.)),.T.,.PARAMETER.);";

/// The fixture with [`BASES`] and `points` appended.
fn model(points: &str) -> Model {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../test/fixtures/synthetic-lowering/station_offset_bases_ifc4x3.ifc");
    let text = std::fs::read_to_string(&path).expect("fixture reads");
    let extra = format!("{BASES}\n{points}");
    let text = text.replacen(
        "ENDSEC;\nEND-ISO-10303-21;",
        &format!("{extra}\nENDSEC;\nEND-ISO-10303-21;"),
        1,
    );
    assert!(text.contains(&extra), "records appended");
    ifc_step::StepCodec
        .read_bytes(text.as_bytes())
        .unwrap_or_else(|e| panic!("{e:?}"))
}

/// `IfcPointByDistanceExpression`s `#3000 + i`, 0.5 m left of and 0.25 m
/// up from `basis` at each of `distances`, and the model holding them.
fn points(basis: u64, distances: &[f64]) -> (Model, Vec<u64>) {
    let records: Vec<String> = distances
        .iter()
        .enumerate()
        .map(|(i, d)| {
            format!(
                "#{}=IFCPOINTBYDISTANCEEXPRESSION(IFCLENGTHMEASURE({d:?}),0.5,0.25,$,#{basis});",
                3000 + i
            )
        })
        .collect();
    let ids = (0..distances.len()).map(|i| 3000 + i as u64).collect();
    (model(&records.join("\n")), ids)
}

/// The placement `expression` derives through `evaluator`, in metres.
fn derived(
    model: &Model,
    expression: u64,
    evaluator: &dyn CurveEvaluator,
) -> Result<Transform, GeometryError> {
    let scale = units::resolve(model).expect("project units");
    let file_units = AlignmentUnits {
        length_to_metres: 1.0,
        angle_to_radians: scale.angle_to_radians,
    };
    let point = ifc_alignment::resolve_point_by_distance(model, EntityId(expression), file_units)
        .expect("the expression reads");
    derive_placement_transform(model, &scale, EntityId(expression), &point, evaluator)
}

/// The lowered station of `expression`: the graph, its root and its basis.
/// Each carries the precision as its window and reads the incoming side
/// (#423): its basis's joints are quadratures.
fn lowered(model: &Model, expression: u64) -> (GeometryGraph, NodeId, NodeId) {
    let scale = units::resolve(model).expect("project units");
    let mut session = LoweringSession::new(model, &scale);
    let root =
        lower_point_by_distance_node(&mut session, EntityId(expression), Transform::identity())
            .expect("the station lowers");
    let lowered = session.finish(root).expect("a valid graph");
    let Some(GeometryNode::OrientedCurveStation(station)) = lowered.graph.get(lowered.root) else {
        panic!("#{expression}: a windowed station");
    };
    assert_eq!(station.seam, SeamSide::Incoming, "#{expression}");
    assert_eq!(station.seam_window, 1e-5, "#{expression}: the precision");
    let basis = station.station.basis;
    (lowered.graph, lowered.root, basis)
}

fn close(actual: [f64; 3], expected: [f64; 3], what: &str) {
    let off = (0..3)
        .map(|i| (actual[i] - expected[i]).abs())
        .fold(0.0, f64::max);
    assert!(off < EPS, "{what}: got {actual:?}, expected {expected:?}");
}

fn array(v: Vec3) -> [f64; 3] {
    [v.x, v.y, v.z]
}

/// The derived placement of `expression` equals its lowered, windowed
/// station's frame, resolved by the reference compiler.
fn assert_matches_station(model: &Model, expression: u64) -> Transform {
    assert_matches_station_of(model, expression, model, expression)
}

/// The derived placement of `expression` equals the lowered station of
/// `station` in `other`, resolved by the reference compiler.
fn assert_matches_station_of(
    model: &Model,
    expression: u64,
    other: &Model,
    station: u64,
) -> Transform {
    let what = format!("#{expression}");
    let transform = derived(model, expression, &ReferenceCurveEvaluator::default())
        .unwrap_or_else(|e| panic!("{what}: {e}"));
    let (graph, root, _) = lowered(other, station);
    let station = resolve(&graph, root).unwrap_or_else(|e| panic!("{what} resolves: {e:?}"));
    close(
        transform.origin,
        array(station.point),
        &format!("{what}: origin"),
    );
    close(
        transform.basis[0],
        array(station.section.tangent),
        &format!("{what}: x, the tangent"),
    );
    close(
        transform.basis[1],
        array(station.section.lateral),
        &format!("{what}: y, the left"),
    );
    close(
        transform.basis[2],
        array(station.section.up),
        &format!("{what}: z, up"),
    );
    transform
}

/// The derived placement of `expression` is its lowered, windowed station's
/// frame, resolved by the reference compiler, bit for bit.
fn assert_is_station_bitwise(model: &Model, expression: u64) {
    let transform = derived(model, expression, &ReferenceCurveEvaluator::default())
        .unwrap_or_else(|e| panic!("#{expression}: {e}"));
    let (graph, root, _) = lowered(model, expression);
    let station = resolve(&graph, root).unwrap_or_else(|e| panic!("#{expression} resolves: {e:?}"));
    assert_eq!(
        [
            transform.origin,
            transform.basis[0],
            transform.basis[1],
            transform.basis[2],
        ],
        [
            array(station.point),
            array(station.section.tangent),
            array(station.section.lateral),
            array(station.section.up),
        ],
        "#{expression}: origin, x, y, z"
    );
}

/// The kernel refuses the lowered station of `expression` naming `needle`.
fn assert_kernel_refuses(model: &Model, expression: u64, needle: &str) {
    let (graph, root, _) = lowered(model, expression);
    match resolve(&graph, root) {
        Err(GeomError::UnsupportedInput { input, .. }) => {
            assert_eq!(input, needle, "#{expression}");
        }
        other => panic!("#{expression}: expected {needle:?}, got {other:?}"),
    }
}

/// The path derived for `expression`'s basis reads, through the reference
/// evaluator, as the stated path the compiler flattens the lowered basis
/// into, bit for bit at every distance inside it and from both sides
/// (within rounding at its two ends); its pieces are
/// bounded where `bounded` says. Returns the compiler's path.
fn assert_reads_as_compilers_path(
    model: &Model,
    expression: u64,
    basis: u64,
    bounded: &[bool],
) -> CurvePath {
    let scale = units::resolve(model).expect("project units");
    let evaluator = ReferenceCurveEvaluator::default();
    let ours = basis_curve_path(
        model,
        &scale,
        EntityId(expression),
        EntityId(basis),
        &evaluator,
    )
    .unwrap_or_else(|e| panic!("#{basis}: {e}"));
    let (graph, _, node) = lowered(model, expression);
    let theirs = curve_path(&graph, node).expect("the compiler flattens it");
    assert_eq!(
        ours.pieces().len(),
        theirs.pieces().len(),
        "#{basis}: {ours:?}\nvs\n{theirs:?}"
    );
    for ((a, b), bounded) in ours.pieces().iter().zip(theirs.pieces()).zip(bounded) {
        assert_eq!(
            a.start_bound.is_some() || a.end_bound.is_some(),
            *bounded,
            "#{basis}: {a:?}"
        );
        assert!(b.is_stated(), "#{basis}: theirs stated");
        assert_eq!(a.curve, b.curve, "#{basis}: the same curve");
        assert_eq!(a.reversed, b.reversed, "#{basis}");
    }
    assert_eq!(
        evaluator.path_distance_convention(&ours),
        DistanceConvention::ArcLength3d,
        "#{basis}"
    );
    let length = theirs.length();
    for step in 0..=40 {
        let at = CurveMeasure::Distance(length * f64::from(step) / 40.0);
        for side in [SeamSide::Incoming, SeamSide::Outgoing] {
            let a = evaluator.path_frame_at_on(&ours, at, side).expect("ours");
            let b = evaluator
                .path_frame_at_on(&theirs, at, side)
                .expect("theirs");
            if (1..40).contains(&step) {
                assert_eq!(a, b, "#{basis} at {at:?} {side:?}");
                continue;
            }
            // At the path's ends its stated and resolved lengths may part
            // by rounding.
            for (x, y, what) in [
                (a.origin, b.origin, "origin"),
                (a.x, b.x, "x"),
                (a.y, b.y, "y"),
            ] {
                close(
                    array(x),
                    array(y),
                    &format!("#{basis} at {at:?} {side:?}: {what}"),
                );
            }
        }
    }
    theirs
}

// ---- trims of an ellipse at interior parameters -----------------------------

/// The trimmed ellipse reaches the evaluator between its parameters 0.3 and
/// 1.2, the polyline after it stated; inside the ellipse, on the corner
/// joint after it (located by the evaluator's certified distance), 4 um
/// past the joint and on the polyline, the derived frame is the lowered
/// station's. On and near the joint the incoming piece, the ellipse, governs.
#[test]
fn a_placement_on_an_ellipse_trimmed_at_interior_parameters_is_the_stations() {
    let evaluator = ReferenceCurveEvaluator::default();
    let (probe, _) = points(2010, &[1.0]);
    let theirs = assert_reads_as_compilers_path(&probe, 3000, 2010, &[true, false]);
    let joint = theirs.pieces()[0].length();
    let (model, ids) = points(2010, &[1.0, joint, joint + 4e-6, joint + 2.0]);
    for id in &ids {
        assert_matches_station(&model, *id);
    }
    // The ellipse's tangent at 1.2, not the polyline's west.
    let ellipse = [-0.9816254412168572, 0.190817958158582, 0.0];
    for id in [ids[1], ids[2]] {
        let transform = derived(&model, id, &evaluator).expect("derived");
        close(transform.basis[0], ellipse, "the incoming tangent");
    }
    let west = derived(&model, ids[3], &evaluator).expect("derived");
    close(west.basis[0], [-1.0, 0.0, 0.0], "on the polyline");
    // The piece is the trim between its parameters, as given.
    let path = basis_curve_path(
        &model,
        &units::resolve(&model).expect("project units"),
        EntityId(ids[0]),
        EntityId(2010),
        &evaluator,
    )
    .expect("a path");
    let piece = &path.pieces()[0];
    assert_eq!(
        (piece.start_bound, piece.end_bound),
        (
            Some(PathBound::Parameter(0.3)),
            Some(PathBound::Parameter(1.2))
        )
    );
}

/// The CUBIC transition's arc-length trim end is a distance in its
/// Bezier's measure, stated; its start stays parameter 0 (refused as
/// `PathPieceLengthUnstated` before #436).
#[test]
fn a_cubic_transition_trimmed_by_an_arc_length_is_the_stations() {
    let (model, ids) = points(2013, &[10.0, 29.9]);
    let theirs = assert_reads_as_compilers_path(&model, ids[0], 2013, &[true]);
    assert!((theirs.pieces()[0].end - 30.0).abs() < 1e-10, "{theirs:?}");
    let path = basis_curve_path(
        &model,
        &units::resolve(&model).expect("project units"),
        EntityId(ids[0]),
        EntityId(2013),
        &ReferenceCurveEvaluator::default(),
    )
    .expect("a path");
    let piece = &path.pieces()[0];
    assert_eq!(
        (piece.start_bound, piece.end, piece.end_bound),
        (Some(PathBound::Parameter(0.0)), 30.0, None)
    );
    for id in ids {
        assert_matches_station(&model, id);
    }
}

// ---- a closed conic trimmed across its seam ---------------------------------

/// From parameter 5.5 to 0.7 in the ellipse's sense, and the same arc
/// against it, the span runs across the parameter seam at angle 0, as the
/// compiler's periodic trim does: before, on and after the seam the derived
/// frame is the lowered station's, and the two trims are the same arc
/// traversed both ways.
#[test]
fn an_ellipse_trimmed_across_its_seam_is_the_stations() {
    for basis in [2011, 2012] {
        let (probe, _) = points(basis, &[0.5]);
        let theirs = assert_reads_as_compilers_path(&probe, 3000, basis, &[true]);
        let length = theirs.length();
        let piece = &theirs.pieces()[0];
        // The span runs past the end of the ellipse's measure: across it.
        assert!(piece.end > piece.start, "{piece:?}");
        let (model, ids) = points(basis, &[0.5, length / 2.0, length - 0.5]);
        for id in ids {
            assert_matches_station(&model, id);
        }
    }
    let evaluator = ReferenceCurveEvaluator::default();
    let (forwards, a) = points(2011, &[0.75]);
    let length = assert_reads_as_compilers_path(&forwards, a[0], 2011, &[true]).length();
    let (backwards, b) = points(2012, &[length - 0.75]);
    let a = derived(&forwards, a[0], &evaluator).expect("forwards");
    let b = derived(&backwards, b[0], &evaluator).expect("backwards");
    // 0.5 m left and 0.25 m up of each: mirrored about the curve.
    let along = Vec3::from_array(a.basis[0]);
    let mid = |t: &Transform| {
        Vec3::from_array(t.origin)
            - 0.5 * Vec3::from_array(t.basis[1])
            - 0.25 * Vec3::from_array(t.basis[2])
    };
    close(array(mid(&a)), array(mid(&b)), "the same point of the arc");
    close(array(-along), b.basis[0], "the tangent turned back");
}

// ---- a B-spline trimmed by points -------------------------------------------

/// The B-spline trimmed by its points at `t = 0.25` and `0.75` reaches the
/// evaluator bounded by those points, which it inverts. The reference
/// compiler resolves the same trim through the same inversion
/// (axiolid/kernel#306, #439): the path it flattens the lowered basis into
/// reads as the derived one, and the derived frame is its station's bit for
/// bit. Both equal the trim by the parameters the points name, within
/// rounding.
#[test]
fn a_b_spline_trimmed_by_points_is_the_stations() {
    let distances = [0.5, 2.5, 4.5];
    let (model, ids) = points(2024, &distances);
    let theirs = assert_reads_as_compilers_path(&model, ids[0], 2024, &[true]);
    let path = basis_curve_path(
        &model,
        &units::resolve(&model).expect("project units"),
        EntityId(ids[0]),
        EntityId(2024),
        &ReferenceCurveEvaluator::default(),
    )
    .expect("a path");
    let piece = &path.pieces()[0];
    assert_eq!(
        (piece.start_bound, piece.end_bound),
        (
            Some(PathBound::Point(Point3::new(2.5, 1.875, 0.0))),
            Some(PathBound::Point(Point3::new(7.5, 1.875, 0.0)))
        )
    );
    for id in &ids {
        assert_is_station_bitwise(&model, *id);
    }
    let (by_parameters, stations) = points(2033, &distances);
    for (id, station) in ids.iter().zip(&stations) {
        assert_matches_station_of(&model, *id, &by_parameters, *station);
    }
    // Half way along the symmetric arc, at its top (5, 2.5), heading east.
    let evaluator = ReferenceCurveEvaluator::default();
    let (top, id) = points(2024, &[theirs.length() / 2.0]);
    let transform = derived(&top, id[0], &evaluator).expect("derived");
    close(transform.basis[0], [1.0, 0.0, 0.0], "east at the top");
    close(transform.origin, [5.0, 3.0, 0.25], "0.5 m left, 0.25 m up");
}

// ---- spans between distances along a relation -----------------------------

/// The trim of the composite between its distances 2 and 15 keeps 8 m of
/// the polyline and the ellipse's first 5 m: the ellipse is the last piece
/// and everything before it is stated, so its span is stated too (refused
/// as `PathPieceLengthUnstated` before #436). Its joint is stored data, so
/// the station lowering now snaps onto it exactly, with no window, and the
/// derived frame on it, near it and on the ellipse is the station's.
#[test]
fn a_trim_of_a_relation_ending_in_an_ellipse_is_stated() {
    let (model, ids) = points(2056, &[3.0, 8.0, 8.000004, 10.0]);
    let scale = units::resolve(&model).expect("project units");
    let evaluator = ReferenceCurveEvaluator::default();
    let ours = basis_curve_path(&model, &scale, EntityId(ids[0]), EntityId(2056), &evaluator)
        .expect("a path");
    assert!(ours.pieces().iter().all(|p| p.is_stated()), "{ours:?}");
    assert_eq!(
        ours.pieces()
            .iter()
            .map(|p| (p.start, p.end))
            .collect::<Vec<_>>(),
        [(2.0, 10.0), (0.0, 5.0)]
    );
    for (index, id) in ids.iter().enumerate() {
        let mut session = LoweringSession::new(&model, &scale);
        let root = lower_point_by_distance_node(&mut session, EntityId(*id), Transform::identity())
            .expect("lowers");
        let lowered = session.finish(root).expect("a valid graph");
        let station = resolve(&lowered.graph, lowered.root).expect("resolves");
        match lowered.graph.get(lowered.root) {
            Some(GeometryNode::OrientedCurveStation(on)) if index == 1 || index == 2 => {
                assert_eq!(on.station.station.distance, 8.0, "#{id}: on the joint");
                assert_eq!((on.seam, on.seam_window), (SeamSide::Incoming, 0.0));
            }
            Some(GeometryNode::CurveStation(_)) if index == 0 || index == 3 => {}
            other => panic!("#{id}: {other:?}"),
        }
        let transform = derived(&model, *id, &evaluator).unwrap_or_else(|e| panic!("#{id}: {e}"));
        close(transform.origin, array(station.point), "origin");
        close(
            transform.basis[0],
            array(station.section.tangent),
            "tangent",
        );
    }
    let on = derived(&model, ids[1], &evaluator).expect("derived");
    close(on.basis[0], [1.0, 0.0, 0.0], "the incoming polyline");
    // Reaching an ellipse that is not the last piece, which piece the
    // span ends in is a quadrature: still refused.
    let (model, ids) = points(2061, &[1.0]);
    assert_eq!(
        derived(&model, ids[0], &evaluator),
        Err(GeometryError::PathPieceLengthUnstated {
            placement: EntityId(ids[0]),
            basis: EntityId(2061),
        })
    );
}

// ---- refused points ---------------------------------------------------------

#[test]
fn a_trim_point_off_its_curve_is_refused_naming_the_placement() {
    let (model, ids) = points(2025, &[1.0]);
    let error =
        derived(&model, ids[0], &ReferenceCurveEvaluator::default()).expect_err("off the curve");
    assert_eq!(
        error,
        GeometryError::PathPointOffCurve {
            placement: EntityId(ids[0]),
            basis: EntityId(2025),
        }
    );
    assert_eq!(error.entity(), Some(EntityId(ids[0])));
    assert!(
        !error.is_unsupported(),
        "the file names no point of the curve"
    );
    assert!(error.to_string().contains("#2025"), "{error}");
    // The kernel refuses the station on the same trim by the same name
    // (axiolid/kernel#306; #439).
    assert_kernel_refuses(&model, ids[0], PATH_POINT_OFF_CURVE);
}

#[test]
fn a_trim_point_the_curve_passes_twice_is_refused_naming_the_placement() {
    let (model, ids) = points(2032, &[1.0]);
    let error =
        derived(&model, ids[0], &ReferenceCurveEvaluator::default()).expect_err("ambiguous");
    assert_eq!(
        error,
        GeometryError::PathPointAmbiguous {
            placement: EntityId(ids[0]),
            basis: EntityId(2032),
        }
    );
    assert!(!error.is_unsupported());
    assert!(error.to_string().contains("more than once"), "{error}");
    assert_kernel_refuses(&model, ids[0], PATH_POINT_AMBIGUOUS);
}

/// Delegates every query to the reference evaluator, but refuses a path
/// with a bounded piece as `PATH_PIECE_BOUND_UNSUPPORTED`, as a provider
/// that reads spans stated as distances only does.
#[derive(Debug, Default)]
struct Unbounded {
    reference: ReferenceCurveEvaluator,
}

impl Unbounded {
    fn check(&self, path: &CurvePath) -> GeomResult<()> {
        let bounded = path
            .pieces()
            .iter()
            .any(|piece| piece.start_bound.is_some() || piece.end_bound.is_some());
        if bounded {
            return Err(GeomError::UnsupportedInput {
                backend: self.descriptor().id,
                operation: Operation::CurveEvaluation,
                input: PATH_PIECE_BOUND_UNSUPPORTED,
            });
        }
        Ok(())
    }
}

impl Backend for Unbounded {
    fn descriptor(&self) -> BackendDescriptor {
        self.reference.descriptor()
    }
}

impl CurveEvaluator for Unbounded {
    fn distance_convention(&self, curve: &Curve3) -> DistanceConvention {
        self.reference.distance_convention(curve)
    }
    fn point_at(&self, curve: &Curve3, at: CurveMeasure) -> GeomResult<Point3> {
        self.reference.point_at(curve, at)
    }
    fn tangent_at(&self, curve: &Curve3, at: CurveMeasure) -> GeomResult<Vec3> {
        self.reference.tangent_at(curve, at)
    }
    fn frame_at(&self, curve: &Curve3, at: CurveMeasure) -> GeomResult<Frame3> {
        self.reference.frame_at(curve, at)
    }
    fn frame_at_on(&self, curve: &Curve3, at: CurveMeasure, side: SeamSide) -> GeomResult<Frame3> {
        self.reference.frame_at_on(curve, at, side)
    }
    fn path_distance_convention(&self, path: &CurvePath) -> DistanceConvention {
        if self.check(path).is_err() {
            return DistanceConvention::Unsupported;
        }
        self.reference.path_distance_convention(path)
    }
    fn path_frame_at(&self, path: &CurvePath, at: CurveMeasure) -> GeomResult<Frame3> {
        self.check(path)?;
        self.reference.path_frame_at(path, at)
    }
    fn path_frame_at_on(
        &self,
        path: &CurvePath,
        at: CurveMeasure,
        side: SeamSide,
    ) -> GeomResult<Frame3> {
        self.check(path)?;
        self.reference.path_frame_at_on(path, at, side)
    }
}

#[test]
fn an_evaluator_without_piece_bounds_is_refused_naming_the_basis() {
    for basis in [2010, 2024] {
        let (model, ids) = points(basis, &[1.0]);
        let error = derived(&model, ids[0], &Unbounded::default()).expect_err("no bounds");
        assert_eq!(
            error,
            GeometryError::CurvePathBoundUnsupported {
                placement: EntityId(ids[0]),
                basis: EntityId(basis),
            }
        );
        assert!(error.is_unsupported());
    }
}

// ---- an uncertifiable piece away from the placement -------------------------

/// Beside the line before the clothoid the offset is a line, its joint at
/// 10 m stated; beside the clothoid the kernel certifies no length. A
/// placement 5 m along needs no joint after the line, so it derives, and
/// equals the lowered station, which the kernel resolves without reading
/// the clothoid's offset either. On the joint and 4 um past it, the joint
/// is certified (the line before it is) and the incoming line governs: the
/// same frame as the evaluator reads on the incoming side of the
/// compiler's path there. On the last line, after the clothoid, the joint
/// before the placement cannot be certified: refused, as the kernel refuses
/// the station.
#[test]
fn an_uncertifiable_piece_after_the_placement_does_not_refuse_it() {
    let evaluator = ReferenceCurveEvaluator::default();
    let (model, ids) = points(2048, &[5.0, 10.0, 10.000004, 35.0]);
    let five = assert_matches_station(&model, ids[0]);
    close(
        five.origin,
        [5.0, 0.5, -0.75],
        "1 m below the line, 0.5 m left",
    );
    let (graph, _, node) = lowered(&model, ids[1]);
    let theirs = curve_path(&graph, node).expect("the compiler flattens it");
    // The full query refuses: the clothoid's offset lies before a joint.
    assert!(evaluator.path_joint_distances(&theirs, 1e-8).is_err());
    let incoming = evaluator
        .path_frame_at_on(&theirs, CurveMeasure::Distance(10.0), SeamSide::Incoming)
        .expect("the incoming line");
    for id in [ids[1], ids[2]] {
        let transform = derived(&model, id, &evaluator).unwrap_or_else(|e| panic!("#{id}: {e}"));
        close(
            transform.basis[0],
            array(incoming.x),
            "the incoming tangent",
        );
        close(
            transform.origin,
            array(incoming.origin + Vec3::new(0.0, 0.5, 0.25)),
            "on the joint, 0.5 m left and 0.25 m up",
        );
    }
    let error = derived(&model, ids[3], &evaluator).expect_err("the clothoid's offset before it");
    assert!(
        matches!(
            error,
            GeometryError::PlacementSeamWindowRefused {
                refusal: SeamWindowRefusal::UncertifiedLength,
                ..
            }
        ),
        "{error:?}"
    );
    let (graph, root, _) = lowered(&model, ids[3]);
    assert!(resolve(&graph, root).is_err(), "the kernel refuses it too");
}

/// The tilted offset `#97`: neither piece's length is certified. 5 m along
/// the first, far from the joint after it, the placement derives (refused
/// before #436, every joint certified) and reads the evaluator's own frame
/// of the compiler's path there; 40 m along, past the joint, it is refused.
/// The kernel's station resolution still certifies the piece holding the
/// station, so it refuses the station 5 m along, which the derivation
/// reads.
#[test]
fn an_uncertifiable_piece_before_the_placement_refuses_it() {
    let evaluator = ReferenceCurveEvaluator::default();
    let (model, ids) = points(97, &[5.0, 40.0]);
    let transform = derived(&model, ids[0], &evaluator).expect("derived");
    let (graph, root, node) = lowered(&model, ids[0]);
    let theirs = curve_path(&graph, node).expect("the compiler flattens it");
    let frame = evaluator
        .path_frame_at(&theirs, CurveMeasure::Distance(5.0))
        .expect("the evaluator's frame");
    close(transform.basis[0], array(frame.x), "the tangent");
    assert!(
        resolve(&graph, root).is_err(),
        "the kernel certifies piece 0"
    );
    let error = derived(&model, ids[1], &evaluator).expect_err("piece 0 before it");
    let GeometryError::PlacementSeamWindowRefused {
        placement,
        basis,
        refusal,
        ..
    } = error
    else {
        panic!("{error:?}");
    };
    assert_eq!(
        (placement, basis, refusal),
        (
            EntityId(ids[1]),
            EntityId(97),
            SeamWindowRefusal::UncertifiedLength
        )
    );
}
