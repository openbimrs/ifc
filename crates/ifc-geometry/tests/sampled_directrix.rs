//! The directrix of a sweep as a polyline, sampled by the backend (#661).
//!
//! The reference backend chord-samples; every claim below is checked against
//! the file's own geometry, not against what the backend returned.

#![cfg(feature = "compile-reference-backend")]

use axiolid_contracts::{
    Backend, BackendDescriptor, BackendId, ExecutionOptions, ExecutionTarget, GeomError, GeomResult,
};
use axiolid_mesh_compile_contract::{PolylineCompiler, PolylineOutcome};
use axiolid_model::{GeometryGraph, NodeId};
use ifc_geometry::compile::{
    compile_sweep_directrix, compile_sweep_directrix_with, CompiledPolyline, Point3, Tolerance,
};
use ifc_geometry::{GeometryError, Transform};
use ifc_model::{Codec, EntityId, Model};
use ifc_step::StepCodec;

// Shared with the body suites; this one uses part of it.
#[allow(dead_code)]
#[path = "body_description/step.rs"]
mod step;

const TOLERANCE: Tolerance = Tolerance::MILLIMETRE;

fn compile(model: &Model, sweep: u64, frame: Transform) -> CompiledPolyline {
    compile_sweep_directrix(model, EntityId(sweep), frame, TOLERANCE).expect("samples")
}

fn refusal(model: &Model, sweep: u64) -> GeometryError {
    compile_sweep_directrix(model, EntityId(sweep), Transform::identity(), TOLERANCE)
        .expect_err("refused")
}

fn at(points: &[Point3], expected: [f64; 3], within: f64) -> bool {
    points.iter().any(|p| {
        (p.x - expected[0])
            .hypot(p.y - expected[1])
            .hypot(p.z - expected[2])
            <= within
    })
}

fn arr(p: &Point3) -> [f64; 3] {
    [p.x, p.y, p.z]
}

/// A stirrup-like rod in millimetres: a line, a half circle about
/// (100, 50, 0) of radius 50 and a line, as a 3D indexed curve.
const STIRRUP: &str = "#204=IFCCARTESIANPOINTLIST3D(((0.,0.,0.),(100.,0.,0.),(150.,50.,0.),(100.,100.,0.),(0.,100.,0.)));
     #205=IFCINDEXEDPOLYCURVE(#204,(IFCLINEINDEX((1,2)),IFCARCINDEX((2,3,4)),IFCLINEINDEX((4,5))),.F.);
     #206=IFCSWEPTDISKSOLID(#205,6.,$,$,$);";

#[test]
fn a_stirrup_directrix_keeps_its_authored_points_and_samples_the_arc_on_its_circle() {
    let model = step::model("IFC4", ".MILLI.", "1.,0.,0.", STIRRUP, "#206");
    let polyline = compile(&model, 206, Transform::identity());
    assert_eq!(polyline.curve, Some(EntityId(205)));
    assert!(!polyline.closed);
    let bound = polyline.deviation.bound;
    assert!(bound <= TOLERANCE.linear(), "{bound}");
    for authored in [
        [0.0, 0.0, 0.0],
        [0.1, 0.0, 0.0],
        [0.1, 0.1, 0.0],
        [0.0, 0.1, 0.0],
    ] {
        assert!(at(&polyline.points, authored, 1e-12), "{authored:?}");
    }
    assert!(polyline.points.len() > 4, "the arc was sampled");
    // Points right of x = 0.1 are the arc's samples: on the circle about
    // (0.1, 0.05) of radius 0.05, within the certified bound.
    let samples: Vec<_> = polyline
        .points
        .iter()
        .filter(|p| p.x > 0.1 + 1e-9)
        .collect();
    assert!(!samples.is_empty());
    for p in samples {
        let radial = (p.x - 0.1).hypot(p.y - 0.05);
        assert!((radial - 0.05).abs() <= bound + 1e-12, "{radial}");
        assert!(p.z.abs() <= 1e-12);
    }
}

/// A pipe along a unit circle in a degree model, `StartParam 0`, `EndParam
/// 90`: a quarter from (1, 0, 0) to (0, 1, 0).
#[test]
fn a_ranged_circle_directrix_is_trimmed_in_its_own_angle() {
    let model = step::degrees(
        "#101=IFCCARTESIANPOINT((0.,0.,0.));
         #102=IFCAXIS2PLACEMENT3D(#101,$,$);
         #103=IFCCIRCLE(#102,1.);
         #104=IFCSWEPTDISKSOLID(#103,0.05,$,0.,90.);",
        "#104",
    );
    let polyline = compile(&model, 104, Transform::identity());
    let bound = polyline.deviation.bound;
    assert!(bound <= TOLERANCE.linear(), "{bound}");
    let first = arr(polyline.points.first().expect("points"));
    let last = arr(polyline.points.last().expect("points"));
    assert!(
        (first[0] - 1.0).hypot(first[1]).hypot(first[2]) <= bound + 1e-9,
        "{first:?}"
    );
    assert!(
        last[0].hypot(last[1] - 1.0).hypot(last[2]) <= bound + 1e-9,
        "{last:?}"
    );
    for p in &polyline.points {
        assert!((p.x.hypot(p.y) - 1.0).abs() <= bound + 1e-9);
        assert!(p.z.abs() <= 1e-9);
        assert!(p.x >= -1e-9 && p.y >= -1e-9, "only the first quadrant");
    }
}

/// A composite curve of two unit polylines, range (0.5, 1.5): half of the
/// first edge and half of the second.
#[test]
fn a_ranged_composite_directrix_is_cut_structurally() {
    let model = step::metres(
        "#101=IFCCARTESIANPOINT((0.,0.,0.));
         #102=IFCCARTESIANPOINT((1.,0.,0.));
         #103=IFCCARTESIANPOINT((1.,1.,0.));
         #104=IFCPOLYLINE((#101,#102));
         #105=IFCPOLYLINE((#102,#103));
         #106=IFCCOMPOSITECURVESEGMENT(.CONTINUOUS.,.T.,#104);
         #107=IFCCOMPOSITECURVESEGMENT(.CONTINUOUS.,.T.,#105);
         #108=IFCCOMPOSITECURVE((#106,#107),.F.);
         #109=IFCSWEPTDISKSOLID(#108,0.05,$,0.5,1.5);",
        "#109",
    );
    let polyline = compile(&model, 109, Transform::identity());
    let points: Vec<_> = polyline.points.iter().map(arr).collect();
    assert_eq!(
        points,
        vec![[0.5, 0.0, 0.0], [1.0, 0.0, 0.0], [1.0, 0.5, 0.0]]
    );
    assert_eq!(polyline.deviation.bound, 0.0);
}

/// A frame with a rotation (local x onto world y), a translation and a scale
/// of 2.
fn rigid_scaled_frame() -> Transform {
    Transform::from_axes_or_identity(
        [1.0, 2.0, 3.0],
        Some([0.0, 0.0, 1.0]),
        Some([0.0, 1.0, 0.0]),
    )
    .scaled(2.0)
}

/// The frame places the points: a quarter circle about the origin, placed by
/// a rotation, translation and scale, lies on the placed circle, in the
/// placed plane, and ends at the placed end point.
#[test]
fn the_frame_places_the_sampled_points() {
    let model = step::degrees(
        "#101=IFCCARTESIANPOINT((0.,0.,0.));
         #102=IFCAXIS2PLACEMENT3D(#101,$,$);
         #103=IFCCIRCLE(#102,1.);
         #104=IFCSWEPTDISKSOLID(#103,0.05,$,0.,90.);",
        "#104",
    );
    let frame = rigid_scaled_frame();
    let placed = compile(&model, 104, frame);
    assert!(
        placed.deviation.bound <= TOLERANCE.linear(),
        "{}",
        placed.deviation.bound
    );
    let centre = frame.apply([0.0, 0.0, 0.0]);
    let normal = frame.apply_direction([0.0, 0.0, 1.0]);
    let normal_length = normal.iter().map(|c| c * c).sum::<f64>().sqrt();
    for p in &placed.points {
        let d = [p.x - centre[0], p.y - centre[1], p.z - centre[2]];
        let radial = d.iter().map(|c| c * c).sum::<f64>().sqrt();
        assert!(
            (radial - 2.0).abs() <= placed.deviation.bound + 1e-9,
            "{p:?}"
        );
        let off_plane = d.iter().zip(normal).map(|(c, n)| c * n).sum::<f64>() / normal_length;
        assert!(off_plane.abs() <= 1e-9, "{p:?}");
    }
    let start = frame.apply([1.0, 0.0, 0.0]);
    let end = frame.apply([0.0, 1.0, 0.0]);
    assert!(at(&placed.points[..1], start, 1e-9));
    assert!(at(&placed.points[placed.points.len() - 1..], end, 1e-9));
}

/// A polyline has no sampling error: every placed point is exactly the
/// frame applied to the authored one.
#[test]
fn the_frame_maps_authored_points_exactly() {
    let model = step::metres(
        "#101=IFCCARTESIANPOINT((0.,0.,0.));
         #102=IFCCARTESIANPOINT((1.,0.,0.));
         #103=IFCCARTESIANPOINT((1.,1.,0.5));
         #104=IFCPOLYLINE((#101,#102,#103));
         #105=IFCSWEPTDISKSOLID(#104,0.05,$,$,$);",
        "#105",
    );
    let frame = rigid_scaled_frame();
    let placed = compile(&model, 105, frame);
    assert_eq!(placed.points.len(), 3);
    for (p, local) in placed
        .points
        .iter()
        .zip([[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [1.0, 1.0, 0.5]])
    {
        let expected = frame.apply(local);
        let got = arr(p);
        for axis in 0..3 {
            assert!(
                (got[axis] - expected[axis]).abs() <= 1e-9,
                "{got:?} {expected:?}"
            );
        }
    }
}

/// A swept-area sweep's directrix is in its `Position` frame: the points are
/// the directrix mapped by `Position` (a translation by (5, 0, 0) after a
/// quarter turn about z), then by the caller's frame.
#[test]
fn the_position_of_a_swept_area_sweep_places_its_directrix() {
    let sweeps = [
        ("IFC4", "IFCFIXEDREFERENCESWEPTAREASOLID", "#104"),
        (
            "IFC4X3_ADD2",
            "IFCDIRECTRIXDERIVEDREFERENCESWEPTAREASOLID",
            "#104",
        ),
        ("IFC4", "IFCSURFACECURVESWEPTAREASOLID", "#107"),
    ];
    for (schema, kind, last) in sweeps {
        let model = step::model(
            schema,
            "$",
            "1.,0.,0.",
            &format!(
                "#100=IFCRECTANGLEPROFILEDEF(.AREA.,$,$,0.2,0.1);
                 #101=IFCCARTESIANPOINT((0.,0.,0.));
                 #102=IFCCARTESIANPOINT((2.,0.,0.));
                 #103=IFCPOLYLINE((#101,#102));
                 #104=IFCDIRECTION((0.,0.,1.));
                 #105=IFCCARTESIANPOINT((5.,0.,0.));
                 #106=IFCDIRECTION((0.,1.,0.));
                 #108=IFCAXIS2PLACEMENT3D(#105,#104,#106);
                 #109=IFCPLANE(#108);
                 #110={kind}(#100,#108,#103,$,$,{last_arg});",
                last_arg = if kind == "IFCSURFACECURVESWEPTAREASOLID" {
                    "#109"
                } else {
                    last
                }
            ),
            "#110",
        );
        let own = compile(&model, 110, Transform::identity());
        let own: Vec<_> = own.points.iter().map(arr).collect();
        assert_eq!(own.len(), 2, "{kind}");
        for (got, expected) in own.iter().zip([[5.0, 0.0, 0.0], [5.0, 2.0, 0.0]]) {
            for axis in 0..3 {
                assert!(
                    (got[axis] - expected[axis]).abs() <= 1e-9,
                    "{kind}: {got:?}"
                );
            }
        }
        let frame = Transform::translation([0.0, 0.0, 7.0]).scaled(1.0);
        let placed = compile(&model, 110, frame);
        for (p, expected) in placed.points.iter().zip([[5.0, 0.0, 7.0], [5.0, 2.0, 7.0]]) {
            let got = arr(p);
            for axis in 0..3 {
                assert!(
                    (got[axis] - expected[axis]).abs() <= 1e-9,
                    "{kind}: {got:?}"
                );
            }
        }
    }
}

/// A ranged directrix that is a curve relation is trimmed again, which the
/// kernel refuses (#693): `CompilationRefused` naming the sweep, no points.
#[test]
fn a_ranged_trimmed_curve_directrix_is_refused_by_the_kernel() {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../test/fixtures/ifclite-geometry/issue_1485_duct_elbow_surface_curve_swept.ifc"
    );
    let bytes = std::fs::read(path).expect("fixture is readable");
    let model = StepCodec.read_bytes(&bytes).expect("fixture parses");
    for sweep in [55, 71] {
        assert!(
            matches!(
                refusal(&model, sweep),
                GeometryError::CompilationRefused {
                    entity: EntityId(id),
                    ..
                } if id == sweep
            ),
            "#{sweep}"
        );
    }
}

#[test]
fn a_fixed_reference_sweep_gives_its_directrix() {
    let model = step::metres(
        "#100=IFCRECTANGLEPROFILEDEF(.AREA.,$,$,0.2,0.1);
         #101=IFCCARTESIANPOINT((0.,0.,0.));
         #102=IFCCARTESIANPOINT((2.,0.,0.));
         #103=IFCPOLYLINE((#101,#102));
         #104=IFCDIRECTION((0.,0.,1.));
         #105=IFCFIXEDREFERENCESWEPTAREASOLID(#100,$,#103,$,$,#104);",
        "#105",
    );
    let polyline = compile(&model, 105, Transform::identity());
    assert_eq!(polyline.curve, Some(EntityId(103)));
    let points: Vec<_> = polyline.points.iter().map(arr).collect();
    assert_eq!(points, vec![[0.0, 0.0, 0.0], [2.0, 0.0, 0.0]]);
}

#[test]
fn something_that_is_not_a_sweep_along_a_directrix_is_unsupported() {
    let model = step::metres(
        "#100=IFCRECTANGLEPROFILEDEF(.AREA.,$,$,0.2,0.1);
         #103=IFCDIRECTION((0.,0.,1.));
         #104=IFCEXTRUDEDAREASOLID(#100,$,#103,1.);",
        "#104",
    );
    assert!(matches!(
        refusal(&model, 104),
        GeometryError::Unsupported {
            entity: EntityId(104),
            ..
        }
    ));
}

#[test]
fn a_half_open_range_is_degenerate() {
    let model = step::metres(
        "#101=IFCCARTESIANPOINT((0.,0.,0.));
         #102=IFCCARTESIANPOINT((1.,0.,0.));
         #103=IFCPOLYLINE((#101,#102));
         #104=IFCSWEPTDISKSOLID(#103,0.05,$,0.,$);",
        "#104",
    );
    assert!(matches!(
        refusal(&model, 104),
        GeometryError::Degenerate {
            entity: EntityId(104),
            ..
        }
    ));
}

/// An IFC4X3 `IfcLengthMeasure` trim is a distance along the directrix, not
/// a parameter: refused, never read as one.
#[test]
fn a_length_measure_trim_of_a_directrix_sweep_is_unsupported() {
    let model = step::model(
        "IFC4X3_ADD2",
        "$",
        "1.,0.,0.",
        "#100=IFCRECTANGLEPROFILEDEF(.AREA.,$,$,0.2,0.1);
         #101=IFCCARTESIANPOINT((0.,0.,0.));
         #102=IFCCARTESIANPOINT((2.,0.,0.));
         #103=IFCPOLYLINE((#101,#102));
         #104=IFCDIRECTION((0.,0.,1.));
         #105=IFCFIXEDREFERENCESWEPTAREASOLID(#100,$,#103,IFCLENGTHMEASURE(0.),IFCLENGTHMEASURE(1.),#104);",
        "#105",
    );
    assert!(matches!(
        refusal(&model, 105),
        GeometryError::Unsupported {
            entity: EntityId(105),
            ..
        }
    ));
}

#[test]
fn a_dangling_directrix_is_missing() {
    let model = step::metres("#104=IFCSWEPTDISKSOLID(#999,0.05,$,$,$);", "#104");
    assert_eq!(
        refusal(&model, 104),
        GeometryError::MissingEntity {
            referrer: EntityId(104),
            missing: EntityId(999),
        }
    );
}

#[derive(Debug, Default)]
struct Refusing;

impl Backend for Refusing {
    fn descriptor(&self) -> BackendDescriptor {
        BackendDescriptor::new(BackendId::new("refusing"), ExecutionTarget::PortableCpu)
    }
}

impl PolylineCompiler for Refusing {
    fn compile_polyline(
        &self,
        _graph: &GeometryGraph,
        _root: NodeId,
        _options: &ExecutionOptions,
    ) -> GeomResult<PolylineOutcome> {
        Err(GeomError::InvalidInput("no curves here".to_owned()))
    }
}

#[test]
fn a_refusing_backend_refuses_naming_the_sweep() {
    let model = step::model("IFC4", ".MILLI.", "1.,0.,0.", STIRRUP, "#206");
    let error = compile_sweep_directrix_with(
        &Refusing,
        &model,
        EntityId(206),
        Transform::identity(),
        TOLERANCE,
    )
    .expect_err("the backend refuses");
    match error {
        GeometryError::CompilationRefused { entity, reason } => {
            assert_eq!(entity, EntityId(206));
            assert!(reason.contains("no curves here"), "{reason}");
        }
        other => panic!("expected CompilationRefused, got {other:?}"),
    }
}
