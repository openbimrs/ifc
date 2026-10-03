//! A `CUBIC` transition compiles through the reference backend (#90, #315).
//!
//! An `IfcCurveSegment` over a 2D `IfcPolynomialCurve` lowers to its exact
//! Bezier trimmed at `TrimSelector::ArcLength(SegmentLength)`. The kernel,
//! not this crate, inverts that arc length. `axiolid-mesh-compile` 0.3.4 read
//! parameter selectors only and refused the trim by name; from 0.3.12 it
//! resolves it by quadrature.
//!
//! # The geometry
//!
//! A 0.05 m disk swept along IFC4.3's cubic parabola
//! `y = x^3 / (6 R L)` with `R = 100 m` and `L = 60 m`, cut from
//! `SegmentStart 0` by `SegmentLength = L`. The curve bends gently (its
//! tightest radius is about `R`), so the tube is `pi r^2 L` up to the disk's
//! tessellation, which a straight 60 m tube compiled at the same tolerance
//! shares. The ratio therefore checks the path length the kernel resolved.
//! Reading `L` as the parameter `x` instead would run past the arc length by
//! about 0.9 %, well outside the bound asserted here.
#![cfg(feature = "compile-reference-backend")]

use axiolid_core::Tolerance;
use axiolid_mesh::TriMesh;
use ifc_geometry::compile::compile_product_mesh;
use ifc_model::{Codec, EntityId, Model};
use ifc_step::StepCodec;

const PRODUCT: EntityId = EntityId(20);
const RADIUS: f64 = 100.0;
const LENGTH: f64 = 60.0;

/// A product whose body is a swept disk along `directrix`, in IFC4X3.
fn pipe(directrix: &str, records: &str) -> Model {
    let text = format!(
        "ISO-10303-21;
HEADER;
FILE_DESCRIPTION((''),'2;1');
FILE_NAME('','',(''),(''),'','','');
FILE_SCHEMA(('IFC4X3_ADD2'));
ENDSEC;
DATA;
#1=IFCPROJECT('0YvctVUKr0kugbFTf53O9L',$,'P',$,$,$,$,(#2),#3);
#2=IFCGEOMETRICREPRESENTATIONCONTEXT($,'Model',3,1.E-05,#4,$);
#3=IFCUNITASSIGNMENT((#5));
#4=IFCAXIS2PLACEMENT3D(#6,$,$);
#5=IFCSIUNIT(*,.LENGTHUNIT.,$,.METRE.);
#6=IFCCARTESIANPOINT((0.,0.,0.));
#20=IFCBUILDINGELEMENTPROXY('1YvctVUKr0kugbFTf53O9L',$,'Pipe',$,$,#21,#22,$,$);
#21=IFCLOCALPLACEMENT($,#4);
#22=IFCPRODUCTDEFINITIONSHAPE($,$,(#23));
#23=IFCSHAPEREPRESENTATION(#2,'Body','AdvancedSweptSolid',(#30));
#30=IFCSWEPTDISKSOLID(#40,0.05,$,$,$);
#40={directrix};
{records}
ENDSEC;
END-ISO-10303-21;
"
    );
    StepCodec
        .read_bytes(text.as_bytes())
        .unwrap_or_else(|e| panic!("synthetic pipe must parse: {e:?}"))
}

/// The CUBIC transition as a one-segment IFC4X3 composite.
fn cubic() -> Model {
    let cubic_term = 1.0 / (6.0 * RADIUS * LENGTH);
    pipe(
        "IFCCOMPOSITECURVE((#41),.F.)",
        &format!(
            "#41=IFCCURVESEGMENT(.DISCONTINUOUS.,#42,IFCLENGTHMEASURE(0.),\
             IFCLENGTHMEASURE({LENGTH:?}),#45);\n\
             #42=IFCAXIS2PLACEMENT2D(#43,#44);\n\
             #43=IFCCARTESIANPOINT((0.,0.));\n\
             #44=IFCDIRECTION((1.,0.));\n\
             #45=IFCPOLYNOMIALCURVE(#46,(0.,1.),(0.,0.,0.,{cubic_term:?}),$);\n\
             #46=IFCAXIS2PLACEMENT2D(#43,#44);"
        ),
    )
}

/// A straight tube of the same length, for the shared tessellation error.
fn straight() -> Model {
    pipe(
        "IFCPOLYLINE((#41,#42))",
        &format!(
            "#41=IFCCARTESIANPOINT((0.,0.,0.));\n\
             #42=IFCCARTESIANPOINT(({LENGTH:?},0.,0.));"
        ),
    )
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

fn compiled(model: &Model) -> TriMesh {
    compile_product_mesh(model, PRODUCT, Tolerance::MILLIMETRE)
        .unwrap_or_else(|e| panic!("the pipe must compile: {e}"))
        .expect("the pipe has a body")
}

/// The arc-length trim resolves: the tube is `SegmentLength` long.
#[test]
fn a_cubic_transition_compiles_to_its_segment_length() {
    let cubic = compiled(&cubic());
    let straight = signed_volume(&compiled(&straight()));
    let ratio = signed_volume(&cubic) / straight;
    assert!(
        (ratio - 1.0).abs() < 1e-3,
        "the CUBIC tube is {ratio} times a straight {LENGTH} m tube"
    );

    // The path ends where the arc length reaches L, short of x = L: the
    // parabola climbs, so its x-extent is less than its length.
    let max_x = cubic
        .positions
        .iter()
        .map(|p| p.x)
        .fold(f64::NEG_INFINITY, f64::max);
    assert!(
        max_x < LENGTH - 0.3,
        "the trim stops at arc length {LENGTH}, not at x = {LENGTH}: max x {max_x}"
    );
}
