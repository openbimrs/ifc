//! Sweep `StartParam`/`EndParam` on an `IfcCompositeCurve` directrix.
//!
//! ISO 10303-42 (and IFC4 `IfcCompositeCurve`, Figure 389) parameterises a
//! composite by accumulating each segment's PARAMETRIC length: an
//! `IfcPolyline` counts 1 per edge, a trimmed conic counts its angle span in
//! the file's plane-angle unit, a trimmed line counts its parameter span. The
//! sweep range is in that parameter, not in metres.
//!
//! Reading it as a length breaks both ways. Revit writes the full range, e.g.
//! `365 = 5 polyline pieces x 1 + 4 arcs x 90 degrees`, which as metres runs far
//! past a pipe a few metres long and is refused. The committed crankbar's
//! `4840.616 = mm lines + radian arcs` reads as 4.8406 m against a 4.862 m path
//! and silently cuts the pipe short.
//!
//! # The geometry
//!
//! A 0.05 m disk swept along: a 1 m line on +X, a quarter arc of radius 0.5
//! turning up to +Z (trimmed 270 -> 360 degrees in the XZ plane), and a 1 m
//! line on +Z. The path is tangent-continuous, so tube volume is proportional
//! to path length. It turns into Z rather than Y because the reference
//! swept-disk compiler seeds one fixed reference axis (+Y for an +X start)
//! for the whole path; a path that turns onto that axis is refused whatever
//! its parameters (see the kernel issue linked from CHANGELOG).
//! Full parametric length in degrees: `1 + 90 + 1 = 92`.
//!
//! Partial ranges are checked as volume RATIOS against the full sweep: the
//! disk's tessellation error cancels, leaving only path length.
#![cfg(feature = "compile-reference-backend")]

use std::f64::consts::{FRAC_PI_2, PI};

use axiolid_core::Tolerance;
use axiolid_mesh::TriMesh;
use axiolid_model::{CurveRelation, GeometryNode, SolidOperation};
use ifc_geometry::compile::compile_product_mesh;
use ifc_geometry::lower::{lower_representation_item, LoweringSession};
use ifc_geometry::transform::Transform;
use ifc_geometry::{units, GeometryError};
use ifc_model::{Codec, EntityId, Model};
use ifc_step::StepCodec;

const SWEEP: EntityId = EntityId(30);
const PRODUCT: EntityId = EntityId(20);
const ARC_RADIUS: f64 = 0.5;
/// Path length of the full sweep: two 1 m lines and a quarter arc.
const FULL_LENGTH: f64 = 2.0 + ARC_RADIUS * FRAC_PI_2;

/// How the arc segment is authored. Every form is the same quarter arc.
#[derive(Clone, Copy, PartialEq)]
enum Arc {
    /// Trimmed 270 -> 360 degrees, with the circle and the composite.
    Forward,
    /// The circle's frame rotated so the arc runs 315 -> 45 degrees across
    /// the basis seam. A cut must not wrap the long way round.
    AcrossSeam,
    /// Trimmed 360 -> 270 with `SenseAgreement = .F.`, and the segment
    /// `SameSense = .F.`: the arc is walked backwards twice.
    Reversed,
    /// Across the seam AND reversed: trimmed 45 -> 315 with
    /// `SenseAgreement = .F.` in a `SameSense = .F.` segment. A cut splits it
    /// at the seam into two pieces that must be walked in reverse order.
    ReversedAcrossSeam,
    /// Trimmed by Cartesian points only, with no parameters.
    Cartesian,
    /// An `IfcReparametrisedCompositeCurveSegment` with this `ParamLength`.
    Reparametrised(f64),
}

/// How the first 1 m of the path is authored.
#[derive(Clone, Copy, PartialEq)]
enum First {
    /// A two-point `IfcPolyline`: parametric length 1.
    Polyline,
    /// The same polyline written backwards with `SameSense = .F.`.
    ReversedPolyline,
    /// An `IfcLine` with a 0.5 m vector trimmed 0 -> 2: parametric length 2
    /// for 1 m of geometry, so a length reading is off by a factor of 2.
    TrimmedLine,
    /// The same line trimmed 2 -> 0 with `SenseAgreement = .F.`, in a
    /// `SameSense = .F.` segment: walked backwards twice.
    ReversedTrimmedLine,
    /// The same line with its trims written 2 -> 0 but `SenseAgreement =
    /// .T.`: the trimmed curve still runs with the line, from 0 to 2.
    SwappedTrimsLine,
    /// A three-point polyline, `(0,0,0) -> (0.5,0,0) -> (1,0,0)`: parametric
    /// length 2, and a cut past 1 must keep the interior vertex.
    ThreePointPolyline,
}

/// How the pipe is authored. Geometry is identical across every variant.
#[derive(Clone, Copy)]
struct Pipe {
    /// Plane-angle unit: degrees (Revit) or radians.
    degrees: bool,
    first: First,
    arc: Arc,
    /// `(StartParam, EndParam)` in the composite's parameterisation.
    range: Option<(f64, f64)>,
}

const DEGREES: Pipe = Pipe {
    degrees: true,
    first: First::Polyline,
    arc: Arc::Forward,
    range: Some((0.0, 92.0)),
};

fn step_real(value: f64) -> String {
    format!("{value:?}")
}

fn pipe(p: Pipe) -> Model {
    let (angle_unit, units) = if p.degrees {
        (
            "#8=IFCCONVERSIONBASEDUNIT(#9,.PLANEANGLEUNIT.,'DEGREE',#10);\n\
             #9=IFCDIMENSIONALEXPONENTS(0,0,0,0,0,0,0);\n\
             #10=IFCMEASUREWITHUNIT(IFCPLANEANGLEMEASURE(0.017453292519943295),#11);\n\
             #11=IFCSIUNIT(*,.PLANEANGLEUNIT.,$,.RADIAN.);",
            "(#5,#8)",
        )
    } else {
        ("#8=IFCSIUNIT(*,.PLANEANGLEUNIT.,$,.RADIAN.);", "(#5,#8)")
    };
    let angle = |degrees: f64| {
        step_real(if p.degrees {
            degrees
        } else {
            degrees.to_radians()
        })
    };
    let parameter_trims = |a: f64, b: f64, sense: &str| {
        format!(
            "(IFCPARAMETERVALUE({})),(IFCPARAMETERVALUE({})),{sense},.PARAMETER.",
            angle(a),
            angle(b)
        )
    };
    // The circle's RefDirection: +X puts the arc at 270..360; rotating it
    // -45 degrees about the circle axis moves the same arc to 315..45.
    let reference = if matches!(p.arc, Arc::AcrossSeam | Arc::ReversedAcrossSeam) {
        "(0.7071067811865476,0.,-0.7071067811865476)"
    } else {
        "(1.,0.,0.)"
    };
    let (arc_segment, arc_trims) = match p.arc {
        Arc::Forward => (
            "IFCCOMPOSITECURVESEGMENT(.CONTINUOUS.,.T.,#44)".to_string(),
            parameter_trims(270.0, 360.0, ".T."),
        ),
        Arc::AcrossSeam => (
            "IFCCOMPOSITECURVESEGMENT(.CONTINUOUS.,.T.,#44)".to_string(),
            parameter_trims(315.0, 45.0, ".T."),
        ),
        Arc::Reversed => (
            "IFCCOMPOSITECURVESEGMENT(.CONTINUOUS.,.F.,#44)".to_string(),
            parameter_trims(360.0, 270.0, ".F."),
        ),
        Arc::ReversedAcrossSeam => (
            "IFCCOMPOSITECURVESEGMENT(.CONTINUOUS.,.F.,#44)".to_string(),
            parameter_trims(45.0, 315.0, ".F."),
        ),
        Arc::Cartesian => (
            "IFCCOMPOSITECURVESEGMENT(.CONTINUOUS.,.T.,#44)".to_string(),
            "(#51),(#53),.T.,.CARTESIAN.".to_string(),
        ),
        Arc::Reparametrised(length) => (
            format!(
                "IFCREPARAMETRISEDCOMPOSITECURVESEGMENT(.CONTINUOUS.,.T.,#44,{})",
                step_real(length)
            ),
            parameter_trims(270.0, 360.0, ".T."),
        ),
    };
    let (first_segment, first_curve) = match p.first {
        First::Polyline => (
            "IFCCOMPOSITECURVESEGMENT(.CONTINUOUS.,.T.,#42)",
            "IFCPOLYLINE((#50,#51))",
        ),
        First::ReversedPolyline => (
            "IFCCOMPOSITECURVESEGMENT(.CONTINUOUS.,.F.,#42)",
            "IFCPOLYLINE((#51,#50))",
        ),
        First::TrimmedLine => (
            "IFCCOMPOSITECURVESEGMENT(.CONTINUOUS.,.T.,#42)",
            "IFCTRIMMEDCURVE(#57,(IFCPARAMETERVALUE(0.)),(IFCPARAMETERVALUE(2.)),.T.,.PARAMETER.)",
        ),
        First::ReversedTrimmedLine => (
            "IFCCOMPOSITECURVESEGMENT(.CONTINUOUS.,.F.,#42)",
            "IFCTRIMMEDCURVE(#57,(IFCPARAMETERVALUE(2.)),(IFCPARAMETERVALUE(0.)),.F.,.PARAMETER.)",
        ),
        First::SwappedTrimsLine => (
            "IFCCOMPOSITECURVESEGMENT(.CONTINUOUS.,.T.,#42)",
            "IFCTRIMMEDCURVE(#57,(IFCPARAMETERVALUE(2.)),(IFCPARAMETERVALUE(0.)),.T.,.PARAMETER.)",
        ),
        First::ThreePointPolyline => (
            "IFCCOMPOSITECURVESEGMENT(.CONTINUOUS.,.T.,#42)",
            "IFCPOLYLINE((#50,#60,#51))",
        ),
    };
    let (start, end) = match p.range {
        Some((a, b)) => (step_real(a), step_real(b)),
        None => ("$".to_string(), "$".to_string()),
    };
    let text = format!(
        "ISO-10303-21;
HEADER;
FILE_DESCRIPTION((''),'2;1');
FILE_NAME('','',(''),(''),'','','');
FILE_SCHEMA(('IFC4'));
ENDSEC;
DATA;
#1=IFCPROJECT('0YvctVUKr0kugbFTf53O9L',$,'P',$,$,$,$,(#2),#3);
#2=IFCGEOMETRICREPRESENTATIONCONTEXT($,'Model',3,1.E-05,#4,$);
#3=IFCUNITASSIGNMENT({units});
#4=IFCAXIS2PLACEMENT3D(#6,$,$);
#5=IFCSIUNIT(*,.LENGTHUNIT.,$,.METRE.);
#6=IFCCARTESIANPOINT((0.,0.,0.));
{angle_unit}
#20=IFCBUILDINGELEMENTPROXY('1YvctVUKr0kugbFTf53O9L',$,'Pipe',$,$,#21,#22,$,$);
#21=IFCLOCALPLACEMENT($,#4);
#22=IFCPRODUCTDEFINITIONSHAPE($,$,(#23));
#23=IFCSHAPEREPRESENTATION(#2,'Body','AdvancedSweptSolid',(#30));
#30=IFCSWEPTDISKSOLID(#40,0.05,$,{start},{end});
#40=IFCCOMPOSITECURVE((#41,#43,#47),.F.);
#41={first_segment};
#42={first_curve};
#43={arc_segment};
#44=IFCTRIMMEDCURVE(#45,{arc_trims});
#45=IFCCIRCLE(#46,0.5);
#46=IFCAXIS2PLACEMENT3D(#52,#55,#56);
#47=IFCCOMPOSITECURVESEGMENT(.DISCONTINUOUS.,.T.,#48);
#48=IFCPOLYLINE((#53,#54));
#50=IFCCARTESIANPOINT((0.,0.,0.));
#51=IFCCARTESIANPOINT((1.,0.,0.));
#52=IFCCARTESIANPOINT((1.,0.,0.5));
#53=IFCCARTESIANPOINT((1.5,0.,0.5));
#54=IFCCARTESIANPOINT((1.5,0.,1.5));
#55=IFCDIRECTION((0.,-1.,0.));
#56=IFCDIRECTION({reference});
#57=IFCLINE(#50,#58);
#58=IFCVECTOR(#59,0.5);
#59=IFCDIRECTION((1.,0.,0.));
#60=IFCCARTESIANPOINT((0.5,0.,0.));
ENDSEC;
END-ISO-10303-21;
"
    );
    StepCodec
        .read_bytes(text.as_bytes())
        .unwrap_or_else(|e| panic!("synthetic pipe must parse: {e:?}"))
}

fn signed_volume(mesh: &TriMesh) -> f64 {
    assert!(!mesh.indices.is_empty(), "compiled mesh is empty");
    let base = mesh.positions[0];
    mesh.indices
        .chunks_exact(3)
        .map(|t| {
            let [a, b, c] = [t[0], t[1], t[2]].map(|i| mesh.positions[i as usize] - base);
            a.dot(b.cross(c)) / 6.0
        })
        .sum()
}

fn volume(p: Pipe) -> f64 {
    let model = pipe(p);
    let mesh = compile_product_mesh(&model, PRODUCT, Tolerance::MILLIMETRE)
        .unwrap_or_else(|e| panic!("the pipe must compile: {e}"))
        .expect("the pipe has a body");
    signed_volume(&mesh)
}

/// The lowered sweep's `(directrix, parameter_range)`.
fn lowered_range(model: &Model) -> Result<Option<(f64, f64)>, GeometryError> {
    let scale = units::resolve(model);
    let mut session = LoweringSession::new(model, &scale);
    let node = lower_representation_item(&mut session, SWEEP, Transform::identity())?;
    let lowered = session.finish(node)?;
    match lowered.graph.get(lowered.root).expect("root") {
        GeometryNode::SolidOperation(SolidOperation::SweptDisk {
            parameter_range, ..
        }) => Ok(*parameter_range),
        other => panic!("expected a SweptDisk, got {other:?}"),
    }
}

fn assert_ratio(actual: f64, expected: f64, what: &str) {
    assert!(
        (actual / expected - 1.0).abs() < 5e-3,
        "{what}: volume ratio {actual}, expected {expected}"
    );
}

/// A full range is the whole path: nothing for the kernel to trim.
#[test]
fn the_full_parametric_range_sweeps_the_whole_path() {
    assert_eq!(lowered_range(&pipe(DEGREES)).expect("lowers"), None);
    // The same pipe with the range omitted (IFC4 allows it) is identical.
    let omitted = volume(Pipe {
        range: None,
        ..DEGREES
    });
    assert_ratio(volume(DEGREES), omitted, "full range vs omitted");
}

/// The plane-angle unit decides what an arc contributes, not the result.
#[test]
fn radians_and_degrees_describe_the_same_sweep() {
    let radians = Pipe {
        degrees: false,
        range: Some((0.0, 2.0 + FRAC_PI_2)),
        ..DEGREES
    };
    assert_eq!(lowered_range(&pipe(radians)).expect("lowers"), None);
    assert_ratio(volume(radians), volume(DEGREES), "radians vs degrees");
}

/// A partial range selects by parameter, then sweeps the true geometry.
///
/// `46 = 1 + 45`: the first line and half the arc. `0.5` starts halfway
/// along the first line. Lengths are exact geometry, not parameters.
#[test]
fn a_partial_range_sweeps_exactly_the_selected_pieces() {
    let full = volume(DEGREES);
    let half_arc = ARC_RADIUS * PI / 4.0;
    for (range, length) in [
        ((0.0, 46.0), 1.0 + half_arc),
        ((0.5, 92.0), FULL_LENGTH - 0.5),
        ((0.5, 46.0), 0.5 + half_arc),
        ((1.0, 91.0), ARC_RADIUS * FRAC_PI_2),
        ((91.5, 92.0), 0.5),
    ] {
        let part = volume(Pipe {
            range: Some(range),
            ..DEGREES
        });
        assert_ratio(
            part / full,
            length / FULL_LENGTH,
            &format!("range {range:?}"),
        );
    }
}

/// Every authoring of the same path sweeps the same piece for the same cut.
///
/// Each variant states the SAME geometry with a different parameterisation,
/// so its composite parameter for "the first 1 m, then half the arc" differs.
/// The cut is given in that variant's own parameter and must select the same
/// geometry. Catches: `SameSense` ignored (reversed polyline, reversed arc),
/// a line read by length instead of by vector multiple (trimmed line, 2 per
/// metre), a seam wrap taken the long way (across-seam arc), and
/// `ParamLength` ignored (reparametrised arc). A partial cut across the seam
/// is split there by lowering, so it compiles today; see the full-range
/// exception below for the uncut arc.
#[test]
fn every_authoring_of_the_path_cuts_the_same_geometry() {
    let full = volume(DEGREES);
    let half_arc = ARC_RADIUS * PI / 4.0;
    // (first, arc, parametric length of the first part, of the arc)
    let variants = [
        (First::Polyline, Arc::Forward, 1.0, 90.0),
        (First::ReversedPolyline, Arc::Forward, 1.0, 90.0),
        (First::TrimmedLine, Arc::Forward, 2.0, 90.0),
        (First::ReversedTrimmedLine, Arc::Forward, 2.0, 90.0),
        (First::Polyline, Arc::AcrossSeam, 1.0, 90.0),
        (First::Polyline, Arc::Reversed, 1.0, 90.0),
        (First::Polyline, Arc::ReversedAcrossSeam, 1.0, 90.0),
        (First::Polyline, Arc::Reparametrised(10.0), 1.0, 10.0),
        (First::SwappedTrimsLine, Arc::Forward, 2.0, 90.0),
        (First::ThreePointPolyline, Arc::Forward, 2.0, 90.0),
    ];
    for (first, arc, lead, sweep) in variants {
        let what = |cut: &str| format!("{} / {} / {cut}", name_first(first), name_arc(arc));
        let authored = |range| Pipe {
            first,
            arc,
            range: Some(range),
            ..DEGREES
        };
        let whole = lead + sweep + 1.0;
        // The full range is the authored path, untrimmed.
        assert_eq!(
            lowered_range(&pipe(authored((0.0, whole)))).expect("lowers"),
            None,
            "{}",
            what("full")
        );
        if matches!(arc, Arc::AcrossSeam | Arc::ReversedAcrossSeam) {
            // The untrimmed authored arc reaches the kernel as `315 -> 45`,
            // which its directrix sampler reads as the complementary arc
            // (axiolid/kernel#168). Lowering is right; pin the kernel
            // refusal so this row is revisited when #168 ships.
            let error = compile_product_mesh(
                &pipe(authored((0.0, whole))),
                PRODUCT,
                Tolerance::MILLIMETRE,
            )
            .expect_err("axiolid/kernel#168 is fixed: assert the volume here instead");
            assert!(error.to_string().contains("unit gap"), "{error}");
        } else {
            assert_ratio(volume(authored((0.0, whole))), full, &what("full"));
        }
        // Halfway along the first part, to halfway along the arc.
        let cut = (lead / 2.0, lead + sweep / 2.0);
        assert_ratio(
            volume(authored(cut)) / full,
            (0.5 + half_arc) / FULL_LENGTH,
            &what("mid first -> mid arc"),
        );
        // Only the middle of the arc: a cut inside one segment at both ends.
        let inner = (lead + sweep / 4.0, lead + 3.0 * sweep / 4.0);
        assert_ratio(
            volume(authored(inner)) / full,
            half_arc / FULL_LENGTH,
            &what("inner arc"),
        );
    }
}

fn name_first(first: First) -> &'static str {
    match first {
        First::Polyline => "polyline",
        First::ReversedPolyline => "reversed polyline",
        First::TrimmedLine => "trimmed line",
        First::ReversedTrimmedLine => "reversed trimmed line",
        First::SwappedTrimsLine => "trimmed line, swapped trims",
        First::ThreePointPolyline => "three-point polyline",
    }
}

fn name_arc(arc: Arc) -> String {
    match arc {
        Arc::Forward => "arc".into(),
        Arc::AcrossSeam => "arc across seam".into(),
        Arc::Reversed => "reversed arc".into(),
        Arc::ReversedAcrossSeam => "reversed arc across seam".into(),
        Arc::Cartesian => "cartesian arc".into(),
        Arc::Reparametrised(length) => format!("reparametrised arc ({length})"),
    }
}

/// A one-segment directrix built from `curve` and the extra records it names.
///
/// `#40` is the directrix itself; the sweep ranges over it directly. Metres,
/// degrees.
fn single(directrix: &str, records: &str, range: (f64, f64)) -> Model {
    let text = format!(
        "ISO-10303-21;
HEADER;
FILE_DESCRIPTION((''),'2;1');
FILE_NAME('','',(''),(''),'','','');
FILE_SCHEMA(('IFC4'));
ENDSEC;
DATA;
#1=IFCPROJECT('0YvctVUKr0kugbFTf53O9L',$,'P',$,$,$,$,(#2),#3);
#2=IFCGEOMETRICREPRESENTATIONCONTEXT($,'Model',3,1.E-05,#4,$);
#3=IFCUNITASSIGNMENT((#5,#8));
#4=IFCAXIS2PLACEMENT3D(#6,$,$);
#5=IFCSIUNIT(*,.LENGTHUNIT.,$,.METRE.);
#6=IFCCARTESIANPOINT((0.,0.,0.));
#8=IFCCONVERSIONBASEDUNIT(#9,.PLANEANGLEUNIT.,'DEGREE',#10);
#9=IFCDIMENSIONALEXPONENTS(0,0,0,0,0,0,0);
#10=IFCMEASUREWITHUNIT(IFCPLANEANGLEMEASURE(0.017453292519943295),#11);
#11=IFCSIUNIT(*,.PLANEANGLEUNIT.,$,.RADIAN.);
#20=IFCBUILDINGELEMENTPROXY('1YvctVUKr0kugbFTf53O9L',$,'Pipe',$,$,#21,#22,$,$);
#21=IFCLOCALPLACEMENT($,#4);
#22=IFCPRODUCTDEFINITIONSHAPE($,$,(#23));
#23=IFCSHAPEREPRESENTATION(#2,'Body','AdvancedSweptSolid',(#30));
#30=IFCSWEPTDISKSOLID(#40,0.05,$,{},{});
#40={directrix};
{records}
ENDSEC;
END-ISO-10303-21;
",
        step_real(range.0),
        step_real(range.1)
    );
    StepCodec
        .read_bytes(text.as_bytes())
        .unwrap_or_else(|e| panic!("synthetic sweep must parse: {e:?}"))
}

fn single_volume(model: &Model) -> f64 {
    let mesh = compile_product_mesh(model, PRODUCT, Tolerance::MILLIMETRE)
        .unwrap_or_else(|e| panic!("the sweep must compile: {e}"))
        .expect("the sweep has a body");
    signed_volume(&mesh)
}

/// A trimmed-circle directrix takes its range in the BASIS's parameter.
///
/// ISO 10303-42 parameterises a trimmed curve by its basis, so a range on a
/// trimmed circle is an angle in the plane-angle unit, not a length.
#[test]
fn a_trimmed_circle_directrix_ranges_in_degrees() {
    let arc = |range| {
        single(
            "IFCTRIMMEDCURVE(#41,(IFCPARAMETERVALUE(0.)),(IFCPARAMETERVALUE(90.)),.T.,.PARAMETER.)",
            "#41=IFCCIRCLE(#42,1.);\n#42=IFCAXIS2PLACEMENT3D(#6,$,$);",
            range,
        )
    };
    let whole = single_volume(&arc((0.0, 90.0)));
    let half = single_volume(&arc((0.0, 45.0)));
    assert_ratio(half / whole, 0.5, "half the quarter arc");
}

/// A full turn is a legitimate composite arc, not an empty one.
///
/// `0 -> 360` travels one period. Reducing it modulo the period gives 0 and
/// would refuse the segment as empty.
#[test]
fn a_full_turn_arc_in_a_composite_is_one_period() {
    let model = single(
        "IFCCOMPOSITECURVE((#43),.F.)",
        "#43=IFCCOMPOSITECURVESEGMENT(.DISCONTINUOUS.,.T.,#44);\n\
         #44=IFCTRIMMEDCURVE(#41,(IFCPARAMETERVALUE(0.)),(IFCPARAMETERVALUE(360.)),.T.,.PARAMETER.);\n\
         #41=IFCCIRCLE(#42,1.);\n#42=IFCAXIS2PLACEMENT3D(#6,$,$);",
        (0.0, 360.0),
    );
    assert_eq!(
        lowered_range(&model).expect("a full turn lowers"),
        None,
        "0 -> 360 is the whole turn"
    );
    let quarter = single(
        "IFCCOMPOSITECURVE((#43),.F.)",
        "#43=IFCCOMPOSITECURVESEGMENT(.DISCONTINUOUS.,.T.,#44);\n\
         #44=IFCTRIMMEDCURVE(#41,(IFCPARAMETERVALUE(0.)),(IFCPARAMETERVALUE(360.)),.T.,.PARAMETER.);\n\
         #41=IFCCIRCLE(#42,1.);\n#42=IFCAXIS2PLACEMENT3D(#6,$,$);",
        (0.0, 90.0),
    );
    lowered_range(&quarter).expect("a quarter of the turn lowers");
}

/// A cut across a polyline's interior vertex keeps the corner.
///
/// The polyline bends at `(1,0,0)` from +X up to +Z. Cutting `0.5 -> 1.5`
/// keeps 0.5 m either side of the corner, 1 m in all; dropping the corner
/// vertex would sweep the 0.71 m chord instead.
#[test]
fn a_cut_through_a_polyline_corner_keeps_the_corner() {
    let bent = |range| {
        single(
            "IFCCOMPOSITECURVE((#43),.F.)",
            "#43=IFCCOMPOSITECURVESEGMENT(.DISCONTINUOUS.,.T.,#44);\n\
             #44=IFCPOLYLINE((#45,#46,#47));\n\
             #45=IFCCARTESIANPOINT((0.,0.,0.));\n\
             #46=IFCCARTESIANPOINT((1.,0.,0.));\n\
             #47=IFCCARTESIANPOINT((1.,0.,1.));",
            range,
        )
    };
    let whole = single_volume(&bent((0.0, 2.0)));
    let middle = single_volume(&bent((0.5, 1.5)));
    // Both sweeps contain the same mitred corner, so its small error is
    // shared; 2 % separates 1 m (0.5) from the 0.71 m chord (0.35).
    assert!(
        (middle / whole - 0.5).abs() < 0.02,
        "corner kept: ratio {} vs 0.5",
        middle / whole
    );
}

/// A range past the end of the composite is an authoring error, named.
#[test]
fn a_range_beyond_the_composite_is_refused_naming_the_sweep() {
    let error = lowered_range(&pipe(Pipe {
        range: Some((0.0, 93.0)),
        ..DEGREES
    }))
    .expect_err("93 exceeds the parametric length 92");
    assert_eq!(error.entity(), Some(SWEEP), "{error}");
    assert!(
        error.to_string().contains("92"),
        "names the length: {error}"
    );
}

/// A segment trimmed only by points has no stated parametric length.
///
/// Computing one would mean inverting points against the arc, which the
/// schema never asks for: the composite's parameter is defined by the
/// parameter trims. Refused by name rather than guessed.
#[test]
fn a_range_over_point_trimmed_segments_is_refused_by_name() {
    let error = lowered_range(&pipe(Pipe {
        arc: Arc::Cartesian,
        ..DEGREES
    }))
    .expect_err("a point-trimmed arc has no parametric length");
    assert_eq!(error.entity(), Some(SWEEP), "{error}");
    assert!(error.is_unsupported(), "{error}");
    // Without a range nothing needs the parametric length: it still lowers.
    let lowered = lowered_range(&pipe(Pipe {
        arc: Arc::Cartesian,
        range: None,
        ..DEGREES
    }))
    .expect("no range, no parametric length needed");
    assert_eq!(lowered, None);
}

/// The committed crankbar states the full range as mm lines + radian arcs.
///
/// `4840.61604159709 = 2960.12 + 0.0823 + 561.19 + 0.0823 + 1319.14`. Read
/// as a length it was 4.8406 m, which cut 21 mm off the 4.862 m path with
/// no error.
#[test]
fn the_crankbar_full_range_is_its_whole_directrix() {
    let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../test/fixtures/ifclite-geometry/swept_disk_composite_arc_crankbar.ifc");
    let model = StepCodec.read_path(&path).expect("fixture parses");
    let sweep = model.ids_of_type("IFCSWEPTDISKSOLID")[0];
    let scale = units::resolve(&model);
    let mut session = LoweringSession::new(&model, &scale);
    let node = lower_representation_item(&mut session, sweep, Transform::identity())
        .expect("the crankbar lowers");
    // A full range must REUSE the authored directrix node, not rebuild an
    // equal composite: the graph is shared, and a duplicate breaks that.
    let composite = model
        .get(sweep)
        .and_then(|entity| entity.attributes.first())
        .and_then(|value| value.as_ref_id())
        .expect("the sweep names its directrix");
    let authored = ifc_geometry::lower::curve::lower_curve_node(
        &mut session,
        composite,
        Transform::identity(),
    )
    .expect("the authored directrix lowers");
    let lowered = session.finish(node).expect("finishes");
    let (range, directrix) = match lowered.graph.get(lowered.root).expect("root") {
        GeometryNode::SolidOperation(SolidOperation::SweptDisk {
            parameter_range,
            directrix,
            ..
        }) => (*parameter_range, *directrix),
        other => panic!("expected a SweptDisk, got {other:?}"),
    };
    assert_eq!(range, None, "the full range needs no trim");
    assert_eq!(
        directrix, authored,
        "the full range reuses the authored curve"
    );
    match lowered.graph.get(directrix).expect("directrix") {
        GeometryNode::CurveRelation(CurveRelation::Composite { segments }) => {
            assert_eq!(segments.len(), 5, "all five authored segments survive")
        }
        other => panic!("expected the authored composite, got {other:?}"),
    }
}
