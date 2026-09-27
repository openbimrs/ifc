//! Composite-curve profile boundaries lower AND compile (#43).
//!
//! `IfcArbitraryClosedProfileDef` whose boundary is an `IfcCompositeCurve`
//! of polylines and trimmed circles is how real exporters write slotted and
//! rounded sections. Each test extrudes a profile by exactly 1 m, so the
//! compiled volume in cubic metres IS the profile area, and compares it with
//! the closed-form area.
//!
//! # Tolerance
//!
//! The reference compiler flattens arcs under a chord budget derived from
//! `Tolerance::MILLIMETRE`, so a curved profile loses a sliver of area to
//! its inscribed polygon: at most about `2/3 * sagitta * arc length`, a few
//! thousandths of a square metre for the unit-radius arcs used here. Every
//! plausible lowering mistake (arc on the wrong side of its chord, the long
//! way round, degrees read as radians, a dropped unit factor) moves the area
//! by at least 0.5, or breaks the boundary so it is refused. `VOLUME_EPS`
//! sits between those.
//!
//! # The D shape
//!
//! Most tests use one profile: the square `[-1, 1]^2` whose right edge is
//! replaced by a half circle of radius 1 centred on `(1, 0)`. Area
//! `4 + pi/2`. Traversal is counter-clockwise:
//!
//! 1. polyline `(-1,-1) -> (1,-1)`
//! 2. arc `(1,-1) -> (2,0) -> (1,1)`, parameters 270 -> 90 degrees
//! 3. polyline `(1,1) -> (-1,1) -> (-1,-1)`
#![cfg(feature = "compile-reference-backend")]

use std::f64::consts::PI;

use axiolid_core::Tolerance;
use axiolid_mesh::TriMesh;
use ifc_geometry::compile::compile_product_mesh;
use ifc_model::{Codec, EntityId};
use ifc_step::StepCodec;

const VOLUME_EPS: f64 = 0.01;
const D_SHAPE_AREA: f64 = 4.0 + PI / 2.0;

/// A proxy extruding profile `#100` by 1 m, lengths written as `value * unit`.
///
/// The plane-angle unit is always DEGREE, as in the models that motivated
/// #43, so a trim parameter read as radians fails loudly.
fn file(unit: f64, profile: &str) -> String {
    let prefix = if unit == 1.0 { "$" } else { ".MILLI." };
    format!(
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
#5=IFCSIUNIT(*,.LENGTHUNIT.,{prefix},.METRE.);
#6=IFCCARTESIANPOINT((0.,0.,0.));
#7=IFCDIRECTION((0.,0.,1.));
#8=IFCCONVERSIONBASEDUNIT(#9,.PLANEANGLEUNIT.,'DEGREE',#10);
#9=IFCDIMENSIONALEXPONENTS(0,0,0,0,0,0,0);
#10=IFCMEASUREWITHUNIT(IFCPLANEANGLEMEASURE(0.017453292519943295),#11);
#11=IFCSIUNIT(*,.PLANEANGLEUNIT.,$,.RADIAN.);
#20=IFCBUILDINGELEMENTPROXY('1YvctVUKr0kugbFTf53O9L',$,'P',$,$,#21,#22,$,$);
#21=IFCLOCALPLACEMENT($,#4);
#22=IFCPRODUCTDEFINITIONSHAPE($,$,(#23));
#23=IFCSHAPEREPRESENTATION(#2,'Body','SweptSolid',(#24));
#24=IFCEXTRUDEDAREASOLID(#100,#4,#7,{depth});
{profile}
ENDSEC;
END-ISO-10303-21;
",
        depth = length(1.0, unit),
    )
}

fn length(value: f64, unit: f64) -> String {
    format!("{:?}", value * unit)
}

/// `#id=IFCCARTESIANPOINT((x,y))` with both coordinates scaled.
fn point(id: u32, x: f64, y: f64, unit: f64) -> String {
    format!(
        "#{id}=IFCCARTESIANPOINT(({},{}));",
        length(x, unit),
        length(y, unit)
    )
}

/// The D shape's corner points `#140..#143` and its arc circle `#122`.
fn d_shape_geometry(unit: f64) -> String {
    [
        point(140, -1.0, -1.0, unit),
        point(141, 1.0, -1.0, unit),
        point(142, 1.0, 1.0, unit),
        point(143, -1.0, 1.0, unit),
        format!("#122=IFCCIRCLE(#123,{});", length(1.0, unit)),
        "#123=IFCAXIS2PLACEMENT2D(#124,$);".to_string(),
        point(124, 1.0, 0.0, unit),
    ]
    .join("\n")
}

/// The D shape, arc trimmed by parameter (degrees), all segments `.T.`.
fn d_shape_by_parameter(unit: f64) -> String {
    format!(
        "#100=IFCARBITRARYCLOSEDPROFILEDEF(.AREA.,$,#101);
#101=IFCCOMPOSITECURVE((#110,#120,#130),.F.);
#110=IFCCOMPOSITECURVESEGMENT(.CONTINUOUS.,.T.,#111);
#111=IFCPOLYLINE((#140,#141));
#120=IFCCOMPOSITECURVESEGMENT(.CONTINUOUS.,.T.,#121);
#121=IFCTRIMMEDCURVE(#122,(IFCPARAMETERVALUE(270.)),(IFCPARAMETERVALUE(90.)),.T.,.PARAMETER.);
#130=IFCCOMPOSITECURVESEGMENT(.CONTINUOUS.,.T.,#131);
#131=IFCPOLYLINE((#142,#143,#140));
{}",
        d_shape_geometry(unit)
    )
}

/// Compile product `#20` and return its enclosed volume in cubic metres.
fn compiled_volume(text: &str) -> f64 {
    let model = StepCodec
        .read_bytes(text.as_bytes())
        .expect("fixture parses");
    let mesh = compile_product_mesh(&model, EntityId(20), Tolerance::MILLIMETRE)
        .unwrap_or_else(|error| panic!("the profile must compile, got {error}"))
        .expect("the product has a body");
    signed_volume(&mesh)
}

/// The refusal text when compilation (or lowering) must fail.
fn refusal(text: &str) -> String {
    let model = StepCodec
        .read_bytes(text.as_bytes())
        .expect("fixture parses");
    match compile_product_mesh(&model, EntityId(20), Tolerance::MILLIMETRE) {
        Ok(_) => panic!("the profile must be refused"),
        Err(error) => format!("{error} | {error:?}"),
    }
}

/// Divergence-theorem volume, summed about the first vertex for conditioning.
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

fn assert_area(actual: f64, expected: f64, case: &str) {
    assert!(
        (actual - expected).abs() < VOLUME_EPS,
        "{case}: volume {actual}, expected {expected}"
    );
}

/// Parameter trims in DEGREES, the arc wrapping through 0 (270 -> 450).
#[test]
fn a_parameter_trimmed_arc_compiles_to_the_analytic_area() {
    let volume = compiled_volume(&file(1.0, &d_shape_by_parameter(1.0)));
    assert_area(volume, D_SHAPE_AREA, "parameter trims");
}

/// The same D shape in millimetres: radius, centre, points and depth all
/// convert, and the angle unit is independent of the length unit.
#[test]
fn a_millimetre_profile_compiles_to_the_same_solid() {
    let volume = compiled_volume(&file(1000.0, &d_shape_by_parameter(1000.0)));
    assert_area(volume, D_SHAPE_AREA, "millimetres");
}

/// Cartesian trims, `SenseAgreement = .F.` and `SameSense = .F.` together.
///
/// The trimmed curve runs CLOCKWISE from `(1,1)` through `(2,0)` to
/// `(1,-1)`; the segment reverses it back into the contour's direction.
/// Ignoring `SenseAgreement` takes the arc through `(0,0)` instead; ignoring
/// `SameSense` leaves a gap at both ends and is refused.
#[test]
fn cartesian_trims_with_both_reversals_compile_to_the_analytic_area() {
    let profile = format!(
        "#100=IFCARBITRARYCLOSEDPROFILEDEF(.AREA.,$,#101);
#101=IFCCOMPOSITECURVE((#110,#120,#130),.F.);
#110=IFCCOMPOSITECURVESEGMENT(.CONTINUOUS.,.T.,#111);
#111=IFCPOLYLINE((#140,#141));
#120=IFCCOMPOSITECURVESEGMENT(.CONTINUOUS.,.F.,#121);
#121=IFCTRIMMEDCURVE(#122,(#142),(#141),.F.,.CARTESIAN.);
#130=IFCCOMPOSITECURVESEGMENT(.CONTINUOUS.,.T.,#131);
#131=IFCPOLYLINE((#142,#143,#140));
{}",
        d_shape_geometry(1.0)
    );
    let volume = compiled_volume(&file(1.0, &profile));
    assert_area(volume, D_SHAPE_AREA, "cartesian trims, both reversals");
}

/// `MasterRepresentation` decides which trim form wins.
///
/// Both forms are present and deliberately disagree: the parameters (0 and
/// 180) name the wrong half of the circle. Reading them instead of the
/// points leaves gaps, so only honouring `.CARTESIAN.` compiles.
#[test]
fn master_representation_selects_the_trim_form() {
    let profile = format!(
        "#100=IFCARBITRARYCLOSEDPROFILEDEF(.AREA.,$,#101);
#101=IFCCOMPOSITECURVE((#110,#120,#130),.F.);
#110=IFCCOMPOSITECURVESEGMENT(.CONTINUOUS.,.T.,#111);
#111=IFCPOLYLINE((#140,#141));
#120=IFCCOMPOSITECURVESEGMENT(.CONTINUOUS.,.T.,#121);
#121=IFCTRIMMEDCURVE(#122,(#141,IFCPARAMETERVALUE(0.)),(#142,IFCPARAMETERVALUE(180.)),.T.,.CARTESIAN.);
#130=IFCCOMPOSITECURVESEGMENT(.CONTINUOUS.,.T.,#131);
#131=IFCPOLYLINE((#142,#143,#140));
{}",
        d_shape_geometry(1.0)
    );
    let volume = compiled_volume(&file(1.0, &profile));
    assert_area(volume, D_SHAPE_AREA, "cartesian master");
}

/// A composite nested inside a composite segment lowers the same shape.
#[test]
fn a_nested_composite_compiles_to_the_analytic_area() {
    let inner = d_shape_by_parameter(1.0).replacen(
        "#100=IFCARBITRARYCLOSEDPROFILEDEF(.AREA.,$,#101);\n#101=",
        "#101=",
        1,
    );
    let profile = format!(
        "#100=IFCARBITRARYCLOSEDPROFILEDEF(.AREA.,$,#150);
#150=IFCCOMPOSITECURVE((#151),.F.);
#151=IFCCOMPOSITECURVESEGMENT(.CONTINUOUS.,.T.,#101);
{inner}"
    );
    let volume = compiled_volume(&file(1.0, &profile));
    assert_area(volume, D_SHAPE_AREA, "nested composite");
}

/// A nested composite walked backwards (`SameSense = .F.`) is the same region.
///
/// Reversing a chain must reverse BOTH its segment order and each segment's
/// direction. Reversing only the directions leaves the segments in the old
/// order, so consecutive ends no longer meet and the joints report gaps.
#[test]
fn a_reversed_nested_composite_compiles_to_the_analytic_area() {
    let inner = d_shape_by_parameter(1.0).replacen(
        "#100=IFCARBITRARYCLOSEDPROFILEDEF(.AREA.,$,#101);\n#101=",
        "#101=",
        1,
    );
    let profile = format!(
        "#100=IFCARBITRARYCLOSEDPROFILEDEF(.AREA.,$,#150);
#150=IFCCOMPOSITECURVE((#151),.F.);
#151=IFCCOMPOSITECURVESEGMENT(.CONTINUOUS.,.F.,#101);
{inner}"
    );
    let volume = compiled_volume(&file(1.0, &profile));
    assert_area(volume, D_SHAPE_AREA, "reversed nested composite");
}

/// A closed full-circle composite as a VOID: 4 x 4 square minus a unit disc.
///
/// One trimmed segment from 0 to 360 degrees is a full turn: the parameters
/// differ by exactly one period, which is what distinguishes it from a
/// degenerate trim whose ends coincide.
#[test]
fn a_full_circle_composite_hole_compiles_to_the_analytic_area() {
    let profile = [
        "#100=IFCARBITRARYPROFILEDEFWITHVOIDS(.AREA.,$,#101,(#150));".to_string(),
        "#101=IFCPOLYLINE((#140,#141,#142,#143,#140));".to_string(),
        point(140, -2.0, -2.0, 1.0),
        point(141, 2.0, -2.0, 1.0),
        point(142, 2.0, 2.0, 1.0),
        point(143, -2.0, 2.0, 1.0),
        "#150=IFCCOMPOSITECURVE((#151),.F.);".to_string(),
        "#151=IFCCOMPOSITECURVESEGMENT(.CONTINUOUS.,.T.,#152);".to_string(),
        "#152=IFCTRIMMEDCURVE(#122,(IFCPARAMETERVALUE(0.)),(IFCPARAMETERVALUE(360.)),.T.,.PARAMETER.);"
            .to_string(),
        "#122=IFCCIRCLE(#123,1.);".to_string(),
        "#123=IFCAXIS2PLACEMENT2D(#124,$);".to_string(),
        point(124, 0.0, 0.0, 1.0),
    ]
    .join("\n");
    let volume = compiled_volume(&file(1.0, &profile));
    assert_area(volume, 16.0 - PI, "full-circle hole");
}

/// A gap between consecutive segments is refused, never bridged.
///
/// The last polyline starts 10 mm above where the arc ends. Bridging it
/// would compile a slightly different solid with no signal.
#[test]
fn a_gap_between_segments_is_refused_not_closed() {
    let original = d_shape_by_parameter(1.0);
    let profile = original.replace(
        "#131=IFCPOLYLINE((#142,#143,#140));",
        "#131=IFCPOLYLINE((#145,#143,#140));\n#145=IFCCARTESIANPOINT((1.,1.01));",
    );
    assert_ne!(profile, original, "the gap substitution must take effect");
    let text = refusal(&file(1.0, &profile));
    assert!(text.contains("Degenerate"), "must be Degenerate: {text}");
    assert!(text.contains("gap"), "must name the gap: {text}");
    assert!(text.contains("#101"), "must blame the composite: {text}");
}

/// A basis curve outside the lowered families keeps a typed refusal naming it.
#[test]
fn an_unlowered_parent_family_is_refused_by_name() {
    let circle = d_shape_by_parameter(1.0);
    let line = circle
        .lines()
        .find(|line| line.starts_with("#122=IFCCIRCLE("))
        .expect("the fixture authors its arc basis as #122");
    let profile = circle.replace(line, "#122=IFCELLIPSE(#123,1.,0.5);");
    assert_ne!(profile, circle, "the basis substitution must take effect");
    let text = refusal(&file(1.0, &profile));
    assert!(text.contains("Unsupported"), "must be Unsupported: {text}");
    assert!(text.contains("IFCELLIPSE"), "must name the basis: {text}");
}
