//! Curves and surfaces as output primitives (#458).
//!
//! Each product below is authored in IFC2X3, IFC4 and IFC4X3 (only the
//! product arities differ) and compiled through the reference backend.
//! Expected values are worked out by hand beside each test:
//!
//! - a wall placed at (10, 5, 0), turned 90 degrees about Z, whose `Axis`
//!   is an `IfcPolyline` from (0, 0) to (4, 0): world (10, 5, 0) to
//!   (10, 9, 0), its own curve, bound 0;
//! - a column far from the origin, whose `FootPrint` is an `IfcCircle` of
//!   radius 0.5 placed at (1, 0) in 2D: a closed polyline within its bound
//!   of that circle;
//! - a member whose `Axis` is an `IfcMappedItem` of a polyline lifted by 3:
//!   an instance, bounded by the rounding of placing it;
//! - an `IfcAnnotation` whose `Annotation` is an `IfcGeometricCurveSet` of
//!   two polylines: two polylines, in order;
//! - a slab whose `Surface` is an `IfcRectangularTrimmedSurface` of a plane
//!   over [0, 2] x [0, 3]: 6 m2, a surface; with two more items, a second
//!   trimmed plane and a polyline, the mesh holds both sheets and the
//!   polyline comes back alone;
//! - a slab whose `Surface` is a lone `IfcFaceSurface` on a plane, a face
//!   no closed shell gathers: 2 m2, a surface;
//! - walls whose `Axis` is an `IfcTrimmedCurve` of the unit circle about
//!   (2, 0), trimmed at the points (3, 0) and (2, 1): the quarter arc with
//!   `SenseAgreement = .T.`, the other three quarters with `.F.`; an
//!   `IfcOffsetCurve2D` 0.1 to the left of the quarter arc, an arc of radius
//!   0.9; and an offset of that offset, which the reference compiler
//!   refuses by name (#506);
//! - a wall whose `FootPrint` is a closed `IfcCompositeCurve`: two
//!   polylines (0, 0)-(1, 0)-(1, 1) and an `IfcTrimmedCurve` arc back to
//!   (0, 0) about (0.5, 0.5) through (0, 1), enclosing `1/2 + pi/4` m2;
//! - a dimension `IfcAnnotation` whose `Annotation2D` holds a dimension
//!   line and an extension line (`IfcPolyline`s), its value as an
//!   `IfcTextLiteralWithExtent` and an `IfcAnnotationFillArea` (#512): the
//!   two polylines, the text and the fill area skipped by kind; an
//!   annotation of text alone, and one whose text sits in a mapped
//!   representation, which is lowered and refused.
#![cfg(feature = "compile-reference-backend")]

use axiolid_core::Tolerance;
use ifc_geometry::compile::{
    compile_product_curves, compile_product_mesh_for, compile_product_mesh_reported,
    compile_product_polylines, CompiledPolyline, MeshClosure, NonCurveItem, SkippedItem,
};
use ifc_geometry::error::GeometryError;
use ifc_geometry::lower::RepresentationPurpose;
use ifc_model::{Codec, EntityId, Model};
use ifc_step::StepCodec;

const WALL: EntityId = EntityId(10);
const COLUMN: EntityId = EntityId(30);
const MEMBER: EntityId = EntityId(50);
const ANNOTATION: EntityId = EntityId(70);
const SLAB: EntityId = EntityId(90);
const FACE_SLAB: EntityId = EntityId(120);
const TRIMMED_WALL: EntityId = EntityId(140);
const TRIMMED_BACK_WALL: EntityId = EntityId(150);
const COMPOSITE_WALL: EntityId = EntityId(160);
const OFFSET_WALL: EntityId = EntityId(180);
const OFFSET_OFFSET_WALL: EntityId = EntityId(190);
const DIMENSION: EntityId = EntityId(200);
const TEXT_ONLY: EntityId = EntityId(220);
const MAPPED_TEXT: EntityId = EntityId(230);

const AXIS_POLYLINE: EntityId = EntityId(16);
const TRIMMED: EntityId = EntityId(145);
const TRIMMED_BACK: EntityId = EntityId(155);
const COMPOSITE: EntityId = EntityId(165);
const OFFSET: EntityId = EntityId(185);

const SCHEMAS: [&str; 3] = ["IFC2X3", "IFC4", "IFC4X3"];

/// A building element with `placement` and `shape`, in `schema`'s arity.
fn element(schema: &str, id: EntityId, class: &str, placement: u64, shape: u64) -> String {
    let tail = if schema == "IFC2X3" { "$" } else { "$,$" };
    format!(
        "#{}={class}('{:0>22}',$,'E',$,$,#{placement},#{shape},{tail});\n",
        id.0, id.0
    )
}

/// An `IfcAnnotation` with `shape`, in `schema`'s arity.
fn annotation(schema: &str, id: EntityId, shape: u64) -> String {
    let tail = if schema == "IFC4X3" { ",$" } else { "" };
    format!(
        "#{}=IFCANNOTATION('{:0>22}',$,'A',$,$,$,#{shape}{tail});\n",
        id.0, id.0
    )
}

fn fixture(schema: &str) -> Model {
    let mut data = String::from(
        "#1=IFCPROJECT('0YvctVUKr0kugbFTf53O9L',$,'P',$,$,$,$,(#2),#3);
#2=IFCGEOMETRICREPRESENTATIONCONTEXT($,'Model',3,1.E-05,#4,$);
#3=IFCUNITASSIGNMENT((#5));
#4=IFCAXIS2PLACEMENT3D(#6,$,$);
#5=IFCSIUNIT(*,.LENGTHUNIT.,$,.METRE.);
#6=IFCCARTESIANPOINT((0.,0.,0.));
#7=IFCDIRECTION((0.,0.,1.));
#8=IFCDIRECTION((0.,1.,0.));
#9=IFCCARTESIANPOINT((0.,0.));
#11=IFCLOCALPLACEMENT($,#13);
#12=IFCPRODUCTDEFINITIONSHAPE($,$,(#20,#15));
#13=IFCAXIS2PLACEMENT3D(#14,#7,#8);
#14=IFCCARTESIANPOINT((10.,5.,0.));
#15=IFCSHAPEREPRESENTATION(#2,'Axis','Curve2D',(#16));
#16=IFCPOLYLINE((#9,#17));
#17=IFCCARTESIANPOINT((4.,0.));
#20=IFCSHAPEREPRESENTATION(#2,'Body','SweptSolid',(#21));
#21=IFCEXTRUDEDAREASOLID(#22,#4,#7,3.);
#22=IFCRECTANGLEPROFILEDEF(.AREA.,$,#23,4.,0.3);
#23=IFCAXIS2PLACEMENT2D(#24,$);
#24=IFCCARTESIANPOINT((2.,0.));
#31=IFCLOCALPLACEMENT($,#33);
#32=IFCPRODUCTDEFINITIONSHAPE($,$,(#35));
#33=IFCAXIS2PLACEMENT3D(#34,$,$);
#34=IFCCARTESIANPOINT((512000.,5403000.,30.));
#35=IFCSHAPEREPRESENTATION(#2,'FootPrint','Curve2D',(#36));
#36=IFCCIRCLE(#37,0.5);
#37=IFCAXIS2PLACEMENT2D(#38,$);
#38=IFCCARTESIANPOINT((1.,0.));
#51=IFCLOCALPLACEMENT($,#4);
#52=IFCPRODUCTDEFINITIONSHAPE($,$,(#53));
#53=IFCSHAPEREPRESENTATION(#2,'Axis','MappedRepresentation',(#54));
#54=IFCMAPPEDITEM(#55,#58);
#55=IFCREPRESENTATIONMAP(#4,#56);
#56=IFCSHAPEREPRESENTATION(#2,'Axis','Curve3D',(#57));
#57=IFCPOLYLINE((#6,#59));
#58=IFCCARTESIANTRANSFORMATIONOPERATOR3D($,$,#60,$,$);
#59=IFCCARTESIANPOINT((0.,0.,2.));
#60=IFCCARTESIANPOINT((0.,0.,3.));
#72=IFCPRODUCTDEFINITIONSHAPE($,$,(#73));
#73=IFCSHAPEREPRESENTATION(#2,'Annotation','GeometricCurveSet',(#74));
#74=IFCGEOMETRICCURVESET((#75,#76));
#75=IFCPOLYLINE((#9,#77));
#76=IFCPOLYLINE((#9,#77,#78,#9));
#77=IFCCARTESIANPOINT((1.,0.));
#78=IFCCARTESIANPOINT((1.,1.));
#91=IFCLOCALPLACEMENT($,#4);
#92=IFCPRODUCTDEFINITIONSHAPE($,$,(#93,#100));
#93=IFCSHAPEREPRESENTATION(#2,'Surface','Surface3D',(#94));
#94=IFCRECTANGULARTRIMMEDSURFACE(#95,0.,0.,2.,3.,.T.,.T.);
#95=IFCPLANE(#4);
#100=IFCSHAPEREPRESENTATION(#2,'Body','SurfaceModel',(#94,#101,#57));
#101=IFCRECTANGULARTRIMMEDSURFACE(#102,0.,0.,1.,1.,.T.,.T.);
#102=IFCPLANE(#103);
#103=IFCAXIS2PLACEMENT3D(#104,$,$);
#104=IFCCARTESIANPOINT((0.,0.,5.));
#121=IFCLOCALPLACEMENT($,#4);
#122=IFCPRODUCTDEFINITIONSHAPE($,$,(#123));
#123=IFCSHAPEREPRESENTATION(#2,'Surface','Surface3D',(#124));
#124=IFCFACESURFACE((#125),#95,.T.);
#125=IFCFACEOUTERBOUND(#126,.T.);
#126=IFCPOLYLOOP((#6,#127,#128,#129));
#127=IFCCARTESIANPOINT((2.,0.,0.));
#128=IFCCARTESIANPOINT((2.,1.,0.));
#129=IFCCARTESIANPOINT((0.,1.,0.));
#141=IFCLOCALPLACEMENT($,#4);
#142=IFCPRODUCTDEFINITIONSHAPE($,$,(#143));
#143=IFCSHAPEREPRESENTATION(#2,'Axis','Curve2D',(#145));
#145=IFCTRIMMEDCURVE(#146,(#147),(#148),.T.,.CARTESIAN.);
#146=IFCCIRCLE(#23,1.);
#147=IFCCARTESIANPOINT((3.,0.));
#148=IFCCARTESIANPOINT((2.,1.));
#151=IFCLOCALPLACEMENT($,#4);
#152=IFCPRODUCTDEFINITIONSHAPE($,$,(#153));
#153=IFCSHAPEREPRESENTATION(#2,'Axis','Curve2D',(#155));
#155=IFCTRIMMEDCURVE(#146,(#147),(#148),.F.,.CARTESIAN.);
#161=IFCLOCALPLACEMENT($,#4);
#162=IFCPRODUCTDEFINITIONSHAPE($,$,(#163));
#163=IFCSHAPEREPRESENTATION(#2,'FootPrint','Curve2D',(#165));
#165=IFCCOMPOSITECURVE((#166,#167,#169),.F.);
#166=IFCCOMPOSITECURVESEGMENT(.CONTINUOUS.,.T.,#75);
#167=IFCCOMPOSITECURVESEGMENT(.CONTINUOUS.,.T.,#168);
#168=IFCPOLYLINE((#77,#78));
#169=IFCCOMPOSITECURVESEGMENT(.CONTINUOUS.,.T.,#170);
#170=IFCTRIMMEDCURVE(#171,(#78),(#9),.T.,.CARTESIAN.);
#171=IFCCIRCLE(#172,0.7071067811865476);
#172=IFCAXIS2PLACEMENT2D(#173,$);
#173=IFCCARTESIANPOINT((0.5,0.5));
#181=IFCLOCALPLACEMENT($,#4);
#182=IFCPRODUCTDEFINITIONSHAPE($,$,(#183));
#183=IFCSHAPEREPRESENTATION(#2,'Axis','Curve2D',(#185));
#185=IFCOFFSETCURVE2D(#145,0.1,.F.);
#191=IFCLOCALPLACEMENT($,#4);
#192=IFCPRODUCTDEFINITIONSHAPE($,$,(#193));
#193=IFCSHAPEREPRESENTATION(#2,'Axis','Curve2D',(#195));
#195=IFCOFFSETCURVE2D(#185,0.1,.F.);
#202=IFCPRODUCTDEFINITIONSHAPE($,$,(#203));
#203=IFCSHAPEREPRESENTATION(#2,'Annotation','Annotation2D',(#204,#205,#206,#209));
#204=IFCPOLYLINE((#9,#77));
#205=IFCPOLYLINE((#77,#78));
#206=IFCTEXTLITERALWITHEXTENT('1.00',#207,.LEFT.,#208,'bottom-middle');
#207=IFCAXIS2PLACEMENT2D(#9,$);
#208=IFCPLANAREXTENT(0.5,0.1);
#209=IFCANNOTATIONFILLAREA(#76,$);
#222=IFCPRODUCTDEFINITIONSHAPE($,$,(#223));
#223=IFCSHAPEREPRESENTATION(#2,'Annotation','Annotation2D',(#224));
#224=IFCTEXTLITERAL('label',#207,.LEFT.);
#232=IFCPRODUCTDEFINITIONSHAPE($,$,(#233));
#233=IFCSHAPEREPRESENTATION(#2,'Annotation','MappedRepresentation',(#204,#234));
#234=IFCMAPPEDITEM(#235,#237);
#235=IFCREPRESENTATIONMAP(#4,#236);
#236=IFCSHAPEREPRESENTATION(#2,'Annotation','Annotation2D',(#224));
#237=IFCCARTESIANTRANSFORMATIONOPERATOR2D($,$,#9,$);
",
    );
    data.push_str(&element(schema, WALL, "IFCWALL", 11, 12));
    data.push_str(&element(schema, COLUMN, "IFCCOLUMN", 31, 32));
    data.push_str(&element(schema, MEMBER, "IFCMEMBER", 51, 52));
    data.push_str(&annotation(schema, ANNOTATION, 72));
    data.push_str(&element(schema, SLAB, "IFCSLAB", 91, 92));
    data.push_str(&element(schema, FACE_SLAB, "IFCSLAB", 121, 122));
    data.push_str(&element(schema, TRIMMED_WALL, "IFCWALL", 141, 142));
    data.push_str(&element(schema, TRIMMED_BACK_WALL, "IFCWALL", 151, 152));
    data.push_str(&element(schema, COMPOSITE_WALL, "IFCWALL", 161, 162));
    data.push_str(&element(schema, OFFSET_WALL, "IFCWALL", 181, 182));
    data.push_str(&element(schema, OFFSET_OFFSET_WALL, "IFCWALL", 191, 192));
    data.push_str(&annotation(schema, DIMENSION, 202));
    data.push_str(&annotation(schema, TEXT_ONLY, 222));
    data.push_str(&annotation(schema, MAPPED_TEXT, 232));
    let text = format!(
        "ISO-10303-21;\nHEADER;\nFILE_DESCRIPTION((''),'2;1');\n\
         FILE_NAME('','',(''),(''),'','','');\nFILE_SCHEMA(('{schema}'));\n\
         ENDSEC;\nDATA;\n{data}ENDSEC;\nEND-ISO-10303-21;\n"
    );
    StepCodec
        .read_bytes(text.as_bytes())
        .expect("fixture parses")
}

fn polylines(
    model: &Model,
    product: EntityId,
    purpose: RepresentationPurpose,
) -> Vec<CompiledPolyline> {
    compile_product_polylines(model, product, purpose, Tolerance::MILLIMETRE)
        .expect("compiles")
        .expect("has the representation")
}

fn points(polyline: &CompiledPolyline) -> Vec<[f64; 3]> {
    polyline.points.iter().map(|p| [p.x, p.y, p.z]).collect()
}

fn assert_near(actual: [f64; 3], expected: [f64; 3], what: &str) {
    for (a, e) in actual.iter().zip(expected) {
        assert!((a - e).abs() < 1e-9, "{what}: {actual:?} != {expected:?}");
    }
}

/// Twice the area of each triangle, summed and halved.
fn area(mesh: &ifc_geometry::compile::TriMesh) -> f64 {
    mesh.indices
        .chunks_exact(3)
        .map(|t| {
            let [a, b, c] = [t[0], t[1], t[2]].map(|i| mesh.positions[i as usize]);
            let (u, v) = (b - a, c - a);
            let n = [
                u.y * v.z - u.z * v.y,
                u.z * v.x - u.x * v.z,
                u.x * v.y - u.y * v.x,
            ];
            0.5 * (n[0] * n[0] + n[1] * n[1] + n[2] * n[2]).sqrt()
        })
        .sum()
}

#[test]
fn an_ifc_polyline_axis_is_its_own_polyline_in_world_coordinates() {
    for schema in SCHEMAS {
        let model = fixture(schema);
        let axis = polylines(&model, WALL, RepresentationPurpose::Axis);
        assert_eq!(axis.len(), 1, "{schema}");
        let line = &axis[0];
        assert_eq!(line.curve, Some(AXIS_POLYLINE), "{schema}");
        assert!(!line.closed, "{schema}");
        assert_eq!(line.deviation.bound, 0.0, "{schema}: an authored polyline");
        assert_eq!(line.deviation.requested, 1e-3, "{schema}");
        let got = points(line);
        assert_eq!(got.len(), 2, "{schema}");
        assert_near(got[0], [10.0, 5.0, 0.0], schema);
        assert_near(got[1], [10.0, 9.0, 0.0], schema);

        // An Axis has no surface or solid to mesh.
        let mesh = compile_product_mesh_for(
            &model,
            WALL,
            RepresentationPurpose::Axis,
            Tolerance::MILLIMETRE,
        )
        .expect("compiles");
        assert!(mesh.is_none(), "{schema}");
    }
}

#[test]
fn the_body_stays_the_default_and_its_mesh_is_unchanged() {
    for schema in SCHEMAS {
        let model = fixture(schema);
        let body = compile_product_mesh_for(
            &model,
            WALL,
            RepresentationPurpose::Body,
            Tolerance::MILLIMETRE,
        )
        .expect("compiles")
        .expect("has a Body");
        let reported = compile_product_mesh_reported(&model, WALL, Tolerance::MILLIMETRE)
            .expect("compiles")
            .expect("has a Body");
        assert_eq!(body.closure, MeshClosure::Solid, "{schema}");
        assert_eq!(body.mesh.positions, reported.mesh.positions, "{schema}");
        assert_eq!(body.mesh.indices, reported.mesh.indices, "{schema}");
        // A solid Body has no curves.
        assert!(polylines(&model, WALL, RepresentationPurpose::Body).is_empty());
    }
}

#[test]
fn a_placed_2d_circle_footprint_is_closed_and_within_its_certified_bound() {
    let centre = [512_001.0, 5_403_000.0, 30.0];
    for schema in SCHEMAS {
        let model = fixture(schema);
        let footprint = polylines(&model, COLUMN, RepresentationPurpose::FootPrint);
        assert_eq!(footprint.len(), 1, "{schema}");
        let circle = &footprint[0];
        assert!(circle.closed, "{schema}");
        assert!(circle.deviation.meets_requested(), "{schema}: {circle:?}");
        assert!(circle.deviation.bound > 0.0, "{schema}: a chorded circle");
        let got = points(circle);
        assert!(got.len() > 8, "{schema}: {} vertices", got.len());
        assert_eq!(got.first(), got.last(), "{schema}: ends where it starts");
        for p in &got {
            assert!((p[2] - centre[2]).abs() < 1e-9, "{schema}: {p:?}");
            let r = ((p[0] - centre[0]).powi(2) + (p[1] - centre[1]).powi(2)).sqrt();
            // Vertices lie on the circle; the f64 placement far from the
            // origin rounds them by far less than the bound.
            assert!((r - 0.5).abs() < 1e-6, "{schema}: {p:?} at radius {r}");
        }
        // Plan picks the FootPrint when nothing better is authored.
        let plan = polylines(&model, COLUMN, RepresentationPurpose::Plan);
        assert_eq!(plan, footprint, "{schema}");
    }
}

#[test]
fn a_mapped_axis_is_its_source_polyline_moved_by_the_instance() {
    for schema in SCHEMAS {
        let model = fixture(schema);
        let axis = polylines(&model, MEMBER, RepresentationPurpose::Axis);
        assert_eq!(axis.len(), 1, "{schema}");
        assert_eq!(axis[0].curve, Some(EntityId(57)), "{schema}");
        // Its own curve, plus the rounding of placing it in f64
        // (axiolid/kernel#295): a few ulps, never zero claimed.
        let bound = axis[0].deviation.bound;
        assert!((0.0..1e-12).contains(&bound), "{schema}: {bound}");
        let got = points(&axis[0]);
        assert_eq!(got.len(), 2, "{schema}");
        assert_near(got[0], [0.0, 0.0, 3.0], schema);
        assert_near(got[1], [0.0, 0.0, 5.0], schema);
    }
}

#[test]
fn an_annotation_curve_set_is_one_polyline_per_curve_in_order() {
    for schema in SCHEMAS {
        let model = fixture(schema);
        let curves = polylines(&model, ANNOTATION, RepresentationPurpose::Annotation);
        assert_eq!(curves.len(), 2, "{schema}");
        assert_eq!(curves[0].curve, Some(EntityId(75)), "{schema}");
        assert_eq!(curves[1].curve, Some(EntityId(76)), "{schema}");
        assert_eq!(points(&curves[0]).len(), 2, "{schema}");
        // #76 returns to its first point: closed.
        assert!(curves[1].closed, "{schema}");
        assert_eq!(points(&curves[1]).len(), 4, "{schema}");
        assert_eq!(
            polylines(&model, ANNOTATION, RepresentationPurpose::Plan),
            curves,
            "{schema}: Plan prefers Annotation"
        );
        // An annotation has no Body.
        assert!(compile_product_polylines(
            &model,
            ANNOTATION,
            RepresentationPurpose::Body,
            Tolerance::MILLIMETRE
        )
        .expect("selects")
        .is_none());
    }
}

#[test]
fn a_rectangular_trimmed_surface_is_a_surface_mesh() {
    for schema in SCHEMAS {
        let model = fixture(schema);
        let surface = compile_product_mesh_for(
            &model,
            SLAB,
            RepresentationPurpose::Surface,
            Tolerance::MILLIMETRE,
        )
        .expect("compiles")
        .expect("has a Surface");
        assert_eq!(surface.closure, MeshClosure::Surface, "{schema}");
        assert!((area(&surface.mesh) - 6.0).abs() < 1e-9, "{schema}");
        assert!(matches!(
            surface.solid_mesh(SLAB),
            Err(GeometryError::NotASolid { .. })
        ));
        assert!(polylines(&model, SLAB, RepresentationPurpose::Surface).is_empty());
    }
}

#[test]
fn a_representation_mixing_surfaces_and_a_curve_splits_by_dimension() {
    for schema in SCHEMAS {
        let model = fixture(schema);
        let mesh = compile_product_mesh_for(
            &model,
            SLAB,
            RepresentationPurpose::Body,
            Tolerance::MILLIMETRE,
        )
        .expect("compiles")
        .expect("has a Body");
        assert_eq!(mesh.closure, MeshClosure::Surface, "{schema}");
        // Both sheets, 6 m2 and 1 m2; the polyline has no area.
        assert!((area(&mesh.mesh) - 7.0).abs() < 1e-9, "{schema}");
        let curves = polylines(&model, SLAB, RepresentationPurpose::Body);
        assert_eq!(curves.len(), 1, "{schema}");
        assert_eq!(curves[0].curve, Some(EntityId(57)), "{schema}");
    }
}

#[test]
fn a_lone_face_surface_is_a_surface_mesh() {
    for schema in SCHEMAS {
        let model = fixture(schema);
        let face = compile_product_mesh_for(
            &model,
            FACE_SLAB,
            RepresentationPurpose::Surface,
            Tolerance::MILLIMETRE,
        )
        .expect("compiles")
        .expect("has a Surface");
        assert_eq!(face.closure, MeshClosure::Surface, "{schema}");
        assert!((area(&face.mesh) - 2.0).abs() < 1e-9, "{schema}");
    }
}

/// The polyline of `product`'s single `Axis` or `FootPrint` curve.
fn single(model: &Model, product: EntityId, purpose: RepresentationPurpose) -> CompiledPolyline {
    let mut curves = polylines(model, product, purpose);
    assert_eq!(curves.len(), 1, "{product:?}");
    curves.remove(0)
}

/// Every vertex lies on the circle of `radius` about `centre` (z = 0), and
/// every chord's sagitta, the exact arc's furthest point from it, is within
/// the certified bound.
fn assert_on_arc(what: &str, got: &[[f64; 3]], bound: f64, centre: [f64; 2], radius: f64) {
    for p in got {
        let r = (p[0] - centre[0]).hypot(p[1] - centre[1]);
        assert!(
            (r - radius).abs() < 1e-9 && p[2] == 0.0,
            "{what}: {p:?} at {r}"
        );
    }
    for pair in got.windows(2) {
        let half = 0.5 * (pair[1][0] - pair[0][0]).hypot(pair[1][1] - pair[0][1]);
        let sagitta = radius - (radius * radius - half * half).sqrt();
        assert!(
            sagitta <= bound + 1e-12,
            "{what}: sagitta {sagitta} > {bound}"
        );
    }
}

/// The angle of `p` about `centre`, in `[0, 2 pi)`.
fn angle(p: [f64; 3], centre: [f64; 2]) -> f64 {
    (p[1] - centre[1])
        .atan2(p[0] - centre[0])
        .rem_euclid(2.0 * std::f64::consts::PI)
}

/// A trimmed-circle `Axis` is the arc between its trim points in its sense,
/// certified (#506, axiolid/kernel#317): the quarter from (3, 0) to (2, 1)
/// with `SenseAgreement = .T.`, and the other three quarters, clockwise
/// through (2, -1), with `.F.`.
#[test]
fn a_trimmed_circle_axis_is_its_arc_in_its_sense() {
    use std::f64::consts::FRAC_PI_2;
    let centre = [2.0, 0.0];
    for schema in SCHEMAS {
        let model = fixture(schema);
        for (product, curve, sense) in [
            (TRIMMED_WALL, TRIMMED, true),
            (TRIMMED_BACK_WALL, TRIMMED_BACK, false),
        ] {
            let what = format!("{schema}, sense {sense}");
            let arc = single(&model, product, RepresentationPurpose::Axis);
            assert_eq!(arc.curve, Some(curve), "{what}");
            assert!(!arc.closed, "{what}");
            assert!(arc.deviation.bound > 0.0, "{what}: a chorded arc");
            assert!(arc.deviation.meets_requested(), "{what}: {arc:?}");
            let got = points(&arc);
            assert_on_arc(&what, &got, arc.deviation.bound, centre, 1.0);
            assert_near(got[0], [3.0, 0.0, 0.0], &what);
            assert_near(got[got.len() - 1], [2.0, 1.0, 0.0], &what);
            // Strictly between the ends, every vertex is on the arc's side.
            for p in &got[1..got.len() - 1] {
                let a = angle(*p, centre);
                let inside = a > 0.0 && a < FRAC_PI_2;
                assert_eq!(inside, sense, "{what}: {p:?} at {a}");
            }
            if !sense {
                assert!(got.iter().any(|p| p[1] < -0.99), "{what}: through (2, -1)");
            }
        }
    }
}

/// An `IfcOffsetCurve2D` 0.1 to the left of the counter-clockwise quarter
/// arc is the concentric arc of radius 0.9, from (2.9, 0) to (2, 0.9),
/// certified (#506).
#[test]
fn an_offset_of_a_trimmed_circle_is_the_concentric_arc() {
    let centre = [2.0, 0.0];
    for schema in SCHEMAS {
        let model = fixture(schema);
        let arc = single(&model, OFFSET_WALL, RepresentationPurpose::Axis);
        assert_eq!(arc.curve, Some(OFFSET), "{schema}");
        assert!(arc.deviation.meets_requested(), "{schema}: {arc:?}");
        let got = points(&arc);
        assert_on_arc(schema, &got, arc.deviation.bound, centre, 0.9);
        assert_near(got[0], [2.9, 0.0, 0.0], schema);
        assert_near(got[got.len() - 1], [2.0, 0.9, 0.0], schema);
    }
}

/// A closed composite `FootPrint` of two polylines and a trimmed arc is
/// one closed polyline: the polylines' vertices as authored, each joint
/// one vertex, then the arc chorded within the certified bound (#506).
/// Its shoelace area is under the exact `1/2 + pi/4` m2, by no more than
/// the arc's length times the bound.
#[test]
fn a_composite_footprint_is_one_closed_polyline() {
    use std::f64::consts::PI;
    let (centre, radius) = ([0.5, 0.5], 0.5_f64.sqrt());
    for schema in SCHEMAS {
        let model = fixture(schema);
        let footprint = single(&model, COMPOSITE_WALL, RepresentationPurpose::FootPrint);
        assert_eq!(footprint.curve, Some(COMPOSITE), "{schema}");
        assert!(footprint.closed, "{schema}");
        let bound = footprint.deviation.bound;
        assert!(
            bound > 0.0 && footprint.deviation.meets_requested(),
            "{schema}: {footprint:?}"
        );
        let got = points(&footprint);
        assert_near(got[0], [0.0, 0.0, 0.0], schema);
        assert_near(got[1], [1.0, 0.0, 0.0], schema);
        assert_near(got[2], [1.0, 1.0, 0.0], schema);
        assert_eq!(got.first(), got.last(), "{schema}: ends where it starts");
        // The arc's vertices, joints included, lie on its circle through
        // (0, 1), the upper-left side.
        assert_on_arc(schema, &got[2..], bound, centre, radius);
        assert!(
            got[3..got.len() - 1].iter().all(|p| p[1] > p[0]),
            "{schema}"
        );
        let shoelace = 0.5
            * got
                .windows(2)
                .map(|w| w[0][0] * w[1][1] - w[1][0] * w[0][1])
                .sum::<f64>();
        let exact = 0.5 + PI / 4.0;
        assert!(
            shoelace <= exact && exact - shoelace <= PI * radius * bound,
            "{schema}: area {shoelace}, exact {exact}, bound {bound}"
        );
    }
}

/// The IFC4X3 alignment fixture
/// (`synthetic-surfaces/synthetic_ifc4x3_alignment_curves.ifc`): a
/// `FootPrint` composite of a 100 m line, a 60 m clothoid and a 50 m arc
/// of radius 200, and a gallery of six 60 m transition spirals as `Axis`
/// curves. Each is one certified polyline (#506): the clothoid and the
/// spirals are intrinsic curves, bounded by their curvature laws, their
/// vertices certified positions (axiolid/kernel#317). The joints are the
/// file's own segment placements, which it states to about 1e-14 m.
#[test]
fn ifc4x3_intrinsic_segments_are_certified_polylines() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../test/fixtures/synthetic-surfaces/synthetic_ifc4x3_alignment_curves.ifc");
    let model = StepCodec.read_path(&path).expect("fixture parses");
    let length = |got: &[[f64; 3]]| -> f64 {
        got.windows(2)
            .map(|w| ((w[1][0] - w[0][0]).hypot(w[1][1] - w[0][1])).hypot(w[1][2] - w[0][2]))
            .sum()
    };
    let near =
        |a: [f64; 3], b: [f64; 3], slack: f64| a.iter().zip(b).all(|(x, y)| (x - y).abs() <= slack);

    let footprint = single(&model, EntityId(137), RepresentationPurpose::FootPrint);
    assert_eq!(footprint.curve, Some(EntityId(35)));
    assert!(!footprint.closed);
    let bound = footprint.deviation.bound;
    assert!(
        bound > 0.0 && footprint.deviation.meets_requested(),
        "{footprint:?}"
    );
    let got = points(&footprint);
    assert_near(got[0], [0.0, 0.0, 0.0], "start");
    // The line ends, and the clothoid starts, at one shared vertex.
    let line_end = got
        .iter()
        .position(|p| near(*p, [100.0, 0.0, 0.0], 1e-9))
        .expect("the line's end is a vertex");
    assert!(got[..=line_end].iter().all(|p| p[1] == 0.0 && p[2] == 0.0));
    let arc_start = [159.865_140_552_005_58, 2.995_182_021_777_423_7, 0.0];
    let joint = got
        .iter()
        .position(|p| near(*p, arc_start, bound))
        .expect("the clothoid ends where the arc's placement starts");
    let end = got[got.len() - 1];
    assert!(
        near(
            end,
            [207.861_182_519_015_84, 16.537_198_808_408_853, 0.0],
            1e-6
        ),
        "{end:?}"
    );
    // The arc's vertices lie on its circle: radius 200, to the left of its
    // start tangent.
    let tangent = [0.988_771_077_936_042_2, 0.149_438_132_473_599_22];
    let centre = [
        arc_start[0] - 200.0 * tangent[1],
        arc_start[1] + 200.0 * tangent[0],
    ];
    for p in &got[joint + 1..] {
        let r = (p[0] - centre[0]).hypot(p[1] - centre[1]);
        assert!((r - 200.0).abs() < 1e-6, "{p:?} at {r}");
    }
    // Chords are no longer than the 210 m they span.
    let chorded = length(&got);
    assert!(
        chorded <= 210.0 + 1e-9 && chorded > 210.0 - 1e-2,
        "{chorded}"
    );

    let spirals = polylines(&model, EntityId(143), RepresentationPurpose::Axis);
    assert_eq!(spirals.len(), 6);
    for spiral in &spirals {
        assert!(spiral.deviation.meets_requested(), "{spiral:?}");
        let got = points(spiral);
        assert_near(got[0], [0.0, 0.0, 0.0], "a spiral's start");
        let chorded = length(&got);
        assert!(
            chorded <= 60.0 + 1e-9 && chorded > 60.0 - 1e-2,
            "{spiral:?}: {chorded}"
        );
    }
}

/// The kernel still refuses by name the forms it has no certified polyline
/// for, here an offset of an offset (axiolid/kernel#317). The refusal is
/// the backend's, with its reason; nothing is approximated in its place.
#[test]
fn an_offset_of_an_offset_is_refused_by_name() {
    for schema in SCHEMAS {
        let model = fixture(schema);
        let refused = compile_product_polylines(
            &model,
            OFFSET_OFFSET_WALL,
            RepresentationPurpose::Axis,
            Tolerance::MILLIMETRE,
        )
        .expect_err("an offset of an offset has no polyline");
        match &refused {
            GeometryError::CompilationRefused { entity, reason } => {
                assert_eq!(*entity, OFFSET_OFFSET_WALL, "{schema}");
                assert!(
                    reason.contains("UnsupportedInput")
                        && reason.contains("an offset of an offset curve"),
                    "{schema}: {reason}"
                );
            }
            other => panic!("{schema}: expected CompilationRefused, got {other:?}"),
        }
    }
}

#[test]
fn representation_purposes_round_trip_their_names() {
    for name in ["Body", "Plan", "Axis", "FootPrint", "Annotation", "Surface"] {
        let purpose = RepresentationPurpose::from_name(name).expect(name);
        assert_eq!(purpose.name(), name);
    }
    assert_eq!(RepresentationPurpose::from_name("body"), None);
    assert_eq!(RepresentationPurpose::Body.identifier(), None);
    assert_eq!(RepresentationPurpose::Axis.identifier(), Some("Axis"));
}

/// A dimension gives its lines; its text and fill area are skipped by
/// kind, not drawn and not a refusal (#512). The strict reader still
/// refuses the representation, and a text literal nested in a mapped
/// representation is lowered, so refused, rather than skipped.
#[test]
fn a_dimension_gives_its_curves_and_skips_its_text_and_fill_area() {
    for schema in SCHEMAS {
        let model = fixture(schema);
        let curves = compile_product_curves(
            &model,
            DIMENSION,
            RepresentationPurpose::Annotation,
            Tolerance::MILLIMETRE,
        )
        .expect("the curves compile")
        .expect("has the representation");
        let ids: Vec<_> = curves.polylines.iter().map(|p| p.curve).collect();
        assert_eq!(ids, [Some(EntityId(204)), Some(EntityId(205))], "{schema}");
        assert_near(points(&curves.polylines[1])[1], [1.0, 1.0, 0.0], schema);
        let skipped: Vec<_> = curves
            .skipped
            .iter()
            .map(|SkippedItem { item, kind, .. }| (*item, *kind))
            .collect();
        assert_eq!(
            skipped,
            [
                (EntityId(206), NonCurveItem::TextLiteral),
                (EntityId(209), NonCurveItem::AnnotationFillArea),
            ],
            "{schema}"
        );

        match compile_product_polylines(
            &model,
            DIMENSION,
            RepresentationPurpose::Annotation,
            Tolerance::MILLIMETRE,
        ) {
            Err(GeometryError::Unsupported { entity, .. }) => {
                assert_eq!(entity, EntityId(206), "{schema}");
            }
            other => panic!("{schema}: the strict reader refuses, got {other:?}"),
        }

        let text = compile_product_curves(
            &model,
            TEXT_ONLY,
            RepresentationPurpose::Annotation,
            Tolerance::MILLIMETRE,
        )
        .expect("text alone is no refusal")
        .expect("has the representation");
        assert!(text.polylines.is_empty(), "{schema}");
        assert_eq!(text.skipped.len(), 1, "{schema}");
        assert_eq!(text.skipped[0].item, EntityId(224), "{schema}");
        assert_eq!(text.skipped[0].kind, NonCurveItem::TextLiteral, "{schema}");

        match compile_product_curves(
            &model,
            MAPPED_TEXT,
            RepresentationPurpose::Annotation,
            Tolerance::MILLIMETRE,
        ) {
            Err(GeometryError::Unsupported { entity, .. }) => {
                assert_eq!(entity, EntityId(224), "{schema}");
            }
            other => panic!("{schema}: nested text is refused, got {other:?}"),
        }
    }
}
