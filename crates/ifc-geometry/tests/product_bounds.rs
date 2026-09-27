//! World-space product bounds, and indexing them with Axiolid's BVH (#36).
//!
//! The wall is 4 x 0.3 x 3, centred on its placement, translated to
//! (10, 5, 0) and rotated 30 degrees about Z. Its world AABB is therefore
//!
//! ```text
//! x = 10 ± (2·cos30 + 0.15·sin30)   = [8.19295, 11.80705]
//! y =  5 ± (2·sin30 + 0.15·cos30)   = [3.87010,  6.12990]
//! z = [0, 3]
//! ```
//!
//! computed by hand from the rectangle's corners, not by the code under test.
//! It is a polygonal extrusion, so it is bounded exactly from its vertices
//! (#98); so is the triangulated slab, a mesh leaf. The other bodies pin the
//! rest of that rule, each worked out by hand beside its test: an oblique
//! L-shaped extrusion under a 45 degree turn, a rotated block, and the
//! curved profiles and boolean that must still go through the compiled mesh.
#![cfg(feature = "compile-reference-backend")]

use std::ops::ControlFlow;

use axiolid_core::{Aabb, Point3, Ray3, Tolerance, Vec3};
use axiolid_spatial::{Bvh, SpatialIndex, SpatialItem};
use ifc_geometry::compile::{compile_product_mesh, product_bounds, BoundsSource};
use ifc_geometry::error::GeometryError;
use ifc_model::{Codec, EntityId, Model};
use ifc_step::StepCodec;

const WALL: EntityId = EntityId(10);
const SLAB: EntityId = EntityId(30);
const SPACE: EntityId = EntityId(40);
const OBLIQUE_L: EntityId = EntityId(50);
const BLOCK: EntityId = EntityId(70);
const CUT: EntityId = EntityId(80);
const ROUND: EntityId = EntityId(95);
const ROUNDED: EntityId = EntityId(110);
const TURNED_PROFILE: EntityId = EntityId(120);
const D_SHAPE: EntityId = EntityId(130);

fn fixture(unit: f64) -> Model {
    let prefix = if unit == 1.0 { "$" } else { ".MILLI." };
    let l = |v: f64| format!("{:?}", v * unit);
    let (sin, cos) = 30f64.to_radians().sin_cos();
    let data = format!(
        "#1=IFCPROJECT('0YvctVUKr0kugbFTf53O9L',$,'P',$,$,$,$,(#2),#3);
#2=IFCGEOMETRICREPRESENTATIONCONTEXT($,'Model',3,1.E-05,#4,$);
#3=IFCUNITASSIGNMENT((#5));
#4=IFCAXIS2PLACEMENT3D(#6,$,$);
#5=IFCSIUNIT(*,.LENGTHUNIT.,{prefix},.METRE.);
#6=IFCCARTESIANPOINT((0.,0.,0.));
#7=IFCDIRECTION((0.,0.,1.));
#10=IFCWALL('1YvctVUKr0kugbFTf53O9L',$,'Wall',$,$,#11,#12,$,$);
#11=IFCLOCALPLACEMENT($,#13);
#12=IFCPRODUCTDEFINITIONSHAPE($,$,(#16));
#13=IFCAXIS2PLACEMENT3D(#14,#7,#15);
#14=IFCCARTESIANPOINT(({x},{y},0.));
#15=IFCDIRECTION(({cos:?},{sin:?},0.));
#16=IFCSHAPEREPRESENTATION(#2,'Body','SweptSolid',(#17));
#17=IFCEXTRUDEDAREASOLID(#18,#4,#7,{height});
#18=IFCRECTANGLEPROFILEDEF(.AREA.,$,#19,{length},{thickness});
#19=IFCAXIS2PLACEMENT2D(#20,$);
#20=IFCCARTESIANPOINT((0.,0.));
#30=IFCSLAB('2YvctVUKr0kugbFTf53O9L',$,'Slab',$,$,#31,#32,$,$);
#31=IFCLOCALPLACEMENT($,#33);
#32=IFCPRODUCTDEFINITIONSHAPE($,$,(#34));
#33=IFCAXIS2PLACEMENT3D(#36,$,$);
#34=IFCSHAPEREPRESENTATION(#2,'Body','Tessellation',(#35));
#35=IFCTRIANGULATEDFACESET(#37,$,$,((1,2,3),(1,3,4)),$);
#36=IFCCARTESIANPOINT(({sx},0.,0.));
#37=IFCCARTESIANPOINTLIST3D((({a},{a},{a}),({b},{a},{a}),({b},{b},{a}),({a},{b},{a})));
#40=IFCSPACE('3YvctVUKr0kugbFTf53O9L',$,'Space',$,$,$,$,$,.ELEMENT.,.INTERNAL.,$);
#50=IFCWALL('4YvctVUKr0kugbFTf53O9L',$,'L',$,$,#51,#52,$,$);
#51=IFCLOCALPLACEMENT($,#53);
#52=IFCPRODUCTDEFINITIONSHAPE($,$,(#54));
#53=IFCAXIS2PLACEMENT3D(#55,#7,#56);
#54=IFCSHAPEREPRESENTATION(#2,'Body','SweptSolid',(#57));
#55=IFCCARTESIANPOINT((0.,0.,{ten}));
#56=IFCDIRECTION((1.,1.,0.));
#57=IFCEXTRUDEDAREASOLID(#58,#4,#59,{five});
#58=IFCARBITRARYCLOSEDPROFILEDEF(.AREA.,$,#61);
#59=IFCDIRECTION((0.,3.,4.));
#61=IFCPOLYLINE((#62,#63,#64,#65,#66,#67,#62));
#62=IFCCARTESIANPOINT(({a},{a}));
#63=IFCCARTESIANPOINT(({two},{a}));
#64=IFCCARTESIANPOINT(({two},{one}));
#65=IFCCARTESIANPOINT(({one},{one}));
#66=IFCCARTESIANPOINT(({one},{three}));
#67=IFCCARTESIANPOINT(({a},{three}));
#70=IFCWALL('5YvctVUKr0kugbFTf53O9L',$,'Block',$,$,#71,#72,$,$);
#71=IFCLOCALPLACEMENT($,#73);
#72=IFCPRODUCTDEFINITIONSHAPE($,$,(#74));
#73=IFCAXIS2PLACEMENT3D(#75,$,$);
#74=IFCSHAPEREPRESENTATION(#2,'Body','CSG',(#76));
#75=IFCCARTESIANPOINT(({hundred},0.,0.));
#76=IFCBLOCK(#77,{four},{two},{one});
#77=IFCAXIS2PLACEMENT3D(#78,#7,#79);
#78=IFCCARTESIANPOINT(({one},{two},{three}));
#79=IFCDIRECTION((0.,1.,0.));
#80=IFCWALL('6YvctVUKr0kugbFTf53O9L',$,'Cut',$,$,#81,#82,$,$);
#81=IFCLOCALPLACEMENT($,#4);
#82=IFCPRODUCTDEFINITIONSHAPE($,$,(#83));
#83=IFCSHAPEREPRESENTATION(#2,'Body','CSG',(#84));
#84=IFCBOOLEANRESULT(.DIFFERENCE.,#85,#86);
#85=IFCEXTRUDEDAREASOLID(#87,#4,#7,{two});
#86=IFCEXTRUDEDAREASOLID(#88,#89,#7,{four});
#87=IFCRECTANGLEPROFILEDEF(.AREA.,$,#19,{two},{two});
#88=IFCRECTANGLEPROFILEDEF(.AREA.,$,#90,{two},{four});
#89=IFCAXIS2PLACEMENT3D(#92,$,$);
#90=IFCAXIS2PLACEMENT2D(#91,$);
#91=IFCCARTESIANPOINT(({one},0.));
#92=IFCCARTESIANPOINT((0.,0.,{minus_one}));
#95=IFCWALL('7YvctVUKr0kugbFTf53O9L',$,'Round',$,$,#96,#97,$,$);
#96=IFCLOCALPLACEMENT($,#4);
#97=IFCPRODUCTDEFINITIONSHAPE($,$,(#98));
#98=IFCSHAPEREPRESENTATION(#2,'Body','SweptSolid',(#99));
#99=IFCEXTRUDEDAREASOLID(#100,#4,#7,{one});
#100=IFCCIRCLEPROFILEDEF(.AREA.,$,#19,{one});
#110=IFCWALL('8YvctVUKr0kugbFTf53O9L',$,'Rounded',$,$,#111,#112,$,$);
#111=IFCLOCALPLACEMENT($,#4);
#112=IFCPRODUCTDEFINITIONSHAPE($,$,(#113));
#113=IFCSHAPEREPRESENTATION(#2,'Body','SweptSolid',(#114));
#114=IFCEXTRUDEDAREASOLID(#115,#4,#7,{one});
#115=IFCROUNDEDRECTANGLEPROFILEDEF(.AREA.,$,#19,{two},{one},{fifth});
#120=IFCWALL('9YvctVUKr0kugbFTf53O9L',$,'Turned',$,$,#121,#122,$,$);
#121=IFCLOCALPLACEMENT($,#4);
#122=IFCPRODUCTDEFINITIONSHAPE($,$,(#123));
#123=IFCSHAPEREPRESENTATION(#2,'Body','SweptSolid',(#124));
#124=IFCEXTRUDEDAREASOLID(#125,#4,#7,{one});
#125=IFCRECTANGLEPROFILEDEF(.AREA.,$,#126,{two},{one});
#126=IFCAXIS2PLACEMENT2D(#127,#128);
#127=IFCCARTESIANPOINT(({three},0.));
#128=IFCDIRECTION((0.,1.));
#130=IFCWALL('AYvctVUKr0kugbFTf53O9L',$,'D',$,$,#131,#132,$,$);
#131=IFCLOCALPLACEMENT($,#4);
#132=IFCPRODUCTDEFINITIONSHAPE($,$,(#133));
#133=IFCSHAPEREPRESENTATION(#2,'Body','SweptSolid',(#134));
#134=IFCEXTRUDEDAREASOLID(#135,#4,#7,{one});
#135=IFCARBITRARYCLOSEDPROFILEDEF(.AREA.,$,#136);
#136=IFCCOMPOSITECURVE((#137,#138,#139),.F.);
#137=IFCCOMPOSITECURVESEGMENT(.CONTINUOUS.,.T.,#140);
#138=IFCCOMPOSITECURVESEGMENT(.CONTINUOUS.,.T.,#141);
#139=IFCCOMPOSITECURVESEGMENT(.CONTINUOUS.,.T.,#142);
#140=IFCPOLYLINE((#143,#144));
#141=IFCTRIMMEDCURVE(#147,(#144),(#145),.T.,.CARTESIAN.);
#142=IFCPOLYLINE((#145,#146,#143));
#143=IFCCARTESIANPOINT(({minus_one},{minus_one}));
#144=IFCCARTESIANPOINT(({one},{minus_one}));
#145=IFCCARTESIANPOINT(({one},{one}));
#146=IFCCARTESIANPOINT(({minus_one},{one}));
#147=IFCCIRCLE(#148,{one});
#148=IFCAXIS2PLACEMENT2D(#149,$);
#149=IFCCARTESIANPOINT(({one},0.));
",
        x = l(10.0),
        y = l(5.0),
        height = l(3.0),
        length = l(4.0),
        thickness = l(0.3),
        sx = l(-20.0),
        a = l(0.0),
        b = l(2.0),
        one = l(1.0),
        two = l(2.0),
        three = l(3.0),
        four = l(4.0),
        five = l(5.0),
        ten = l(10.0),
        hundred = l(100.0),
        minus_one = l(-1.0),
        fifth = l(0.2),
    );
    let text = format!(
        "ISO-10303-21;\nHEADER;\nFILE_DESCRIPTION((''),'2;1');\n\
         FILE_NAME('','',(''),(''),'','','');\nFILE_SCHEMA(('IFC4'));\n\
         ENDSEC;\nDATA;\n{data}ENDSEC;\nEND-ISO-10303-21;\n"
    );
    StepCodec
        .read_bytes(text.as_bytes())
        .expect("fixture parses")
}

fn wall_expected() -> Aabb {
    let (sin, cos) = 30f64.to_radians().sin_cos();
    let dx = 2.0 * cos + 0.15 * sin;
    let dy = 2.0 * sin + 0.15 * cos;
    Aabb {
        min: Point3::new(10.0 - dx, 5.0 - dy, 0.0),
        max: Point3::new(10.0 + dx, 5.0 + dy, 3.0),
    }
}

fn assert_close(actual: Aabb, expected: Aabb) {
    let off = (actual.min - expected.min)
        .abs()
        .max((actual.max - expected.max).abs())
        .max_element();
    assert!(off < 1e-9, "{actual:?} != {expected:?}");
}

#[test]
fn a_rotated_wall_is_bounded_in_world_coordinates() {
    for unit in [1.0, 1000.0] {
        let bounds = product_bounds(&fixture(unit), WALL, Tolerance::MILLIMETRE)
            .expect("bounds")
            .expect("the wall has a body");
        // A rectangle extrusion is bounded from its vertices, not a mesh.
        assert_eq!(bounds.source, BoundsSource::Exact);
        assert_close(bounds.aabb, wall_expected());
    }
}

#[test]
fn a_mesh_body_is_bounded_exactly_without_compiling() {
    let bounds = product_bounds(&fixture(1.0), SLAB, Tolerance::MILLIMETRE)
        .unwrap()
        .unwrap();
    assert_eq!(bounds.source, BoundsSource::Exact);
    assert_close(
        bounds.aabb,
        Aabb {
            min: Point3::new(-20.0, 0.0, 0.0),
            max: Point3::new(-18.0, 2.0, 0.0),
        },
    );
}

fn bounds_of(unit: f64, product: EntityId) -> ifc_geometry::compile::ProductBounds {
    product_bounds(&fixture(unit), product, Tolerance::MILLIMETRE)
        .expect("bounds")
        .expect("the product has a body")
}

fn aabb(min: [f64; 3], max: [f64; 3]) -> Aabb {
    Aabb {
        min: Point3::from_array(min),
        max: Point3::from_array(max),
    }
}

/// An L-shaped profile extruded obliquely, under a 45 degree turn.
///
/// Profile (0,0) (2,0) (2,1) (1,1) (1,3) (0,3). The direction (0, 3, 4) is
/// written unnormalised, so the offset is 5 * (0, 0.6, 0.8) = (0, 3, 4) and
/// the top face repeats the L at y + 3, z = 4. The turn
/// maps local (x, y) to c * (x - y, x + y) with c = 1/sqrt(2). Over the
/// twelve vertices x - y spans [-6, 2] and x + y spans [0, 7], and the
/// placement lifts z by 10. The L's missing corner (2, 3) matters: bounding
/// the profile's rectangle instead would reach x + y = 8.
#[test]
fn an_oblique_polygonal_extrusion_is_bounded_exactly_from_its_vertices() {
    let c = std::f64::consts::FRAC_1_SQRT_2;
    let expected = aabb([-6.0 * c, 0.0, 10.0], [2.0 * c, 7.0 * c, 14.0]);
    for unit in [1.0, 1000.0] {
        let bounds = bounds_of(unit, OBLIQUE_L);
        assert_eq!(bounds.source, BoundsSource::Exact);
        assert_close(bounds.aabb, expected);
    }
}

/// A 4 x 2 x 1 block, corner at (1, 2, 3), turned 90 degrees about Z,
/// in a product placed at x = 100.
///
/// `IfcBlock` extends from its `Position` along the positive axes. The turn
/// sends local x to world +y and local y to world -x, so the block spans
/// x in [1 - 2, 1], y in [2, 2 + 4], z in [3, 4], then shifts by 100 in x.
#[test]
fn a_rotated_block_is_bounded_exactly_from_its_corners() {
    let expected = aabb([99.0, 2.0, 3.0], [101.0, 6.0, 4.0]);
    for unit in [1.0, 1000.0] {
        let bounds = bounds_of(unit, BLOCK);
        assert_eq!(bounds.source, BoundsSource::Exact);
        assert_close(bounds.aabb, expected);
    }
}

/// The compiled block lands where the file put it, and where its exact box
/// says it is. The neutral block is centred, `IfcBlock` has a corner at its
/// `Position`; without the half-extent shift in lowering the mesh sits at
/// x [100, 102], z [2.5, 3.5].
#[test]
fn a_compiled_block_agrees_with_its_corner_anchor_and_its_exact_box() {
    let expected = aabb([99.0, 2.0, 3.0], [101.0, 6.0, 4.0]);
    for unit in [1.0, 1000.0] {
        let model = fixture(unit);
        let mesh = compile_product_mesh(&model, BLOCK, Tolerance::MILLIMETRE)
            .expect("the block compiles")
            .expect("the block has a body");
        assert_close(mesh.bounds(), expected);
        assert_close(mesh.bounds(), bounds_of(unit, BLOCK).aabb);
    }
}

/// A difference only shrinks its first operand, so it is never bounded
/// from the operand: the 2 x 2 x 2 box minus the tool covering x >= 0
/// leaves x in [-1, 0], which only the compiled result knows.
#[test]
fn a_boolean_difference_still_goes_through_the_compiled_mesh() {
    let bounds = bounds_of(1.0, CUT);
    assert_eq!(bounds.source, BoundsSource::Tessellated);
    assert_close(bounds.aabb, aabb([-1.0, -1.0, 0.0], [0.0, 1.0, 2.0]));
}

/// A 2 x 1 rectangle whose profile `Position` sits at (3, 0) and turns 90
/// degrees: its long side now runs along y, so x spans 3 +- 0.5 and y
/// spans +- 1, over a depth of 1.
#[test]
fn a_placed_profile_is_bounded_where_its_position_puts_it() {
    let bounds = bounds_of(1.0, TURNED_PROFILE);
    assert_eq!(bounds.source, BoundsSource::Exact);
    assert_close(bounds.aabb, aabb([2.5, -1.0, 0.0], [3.5, 1.0, 1.0]));
}

/// Curved outlines are not polygons: a circle, a rounded rectangle and a
/// contour with an arc (the D bulges to x = 2 between its corners at
/// x = 1) fall back to the compiled mesh.
#[test]
fn curved_profiles_still_go_through_the_compiled_mesh() {
    for product in [ROUND, ROUNDED, D_SHAPE] {
        let bounds = bounds_of(1.0, product);
        assert_eq!(bounds.source, BoundsSource::Tessellated, "{product}");
    }
}

#[test]
fn a_product_without_a_body_has_no_bounds() {
    assert_eq!(
        product_bounds(&fixture(1.0), SPACE, Tolerance::MILLIMETRE).unwrap(),
        None
    );
}

#[test]
fn a_missing_product_is_an_error_not_an_empty_box() {
    let result = product_bounds(&fixture(1.0), EntityId(999), Tolerance::MILLIMETRE);
    assert!(
        matches!(result, Err(GeometryError::MissingEntity { .. })),
        "{result:?}"
    );
}

/// The two crates compose: product boxes from here, the index from Axiolid.
#[test]
fn a_bvh_over_product_bounds_finds_the_product_at_a_probe() {
    let model = fixture(1.0);
    let items = [WALL, SLAB, SPACE].into_iter().filter_map(|product| {
        product_bounds(&model, product, Tolerance::MILLIMETRE)
            .expect("bounds")
            .map(|bounds| SpatialItem::new(product, bounds.aabb))
    });
    let bvh = Bvh::build(items);
    assert_eq!(bvh.len(), 2, "the space has no body and is not indexed");

    let probe = |point: Point3| {
        let mut hits = Vec::new();
        bvh.visit_aabb(&Aabb::from_point(point), &mut |key| {
            hits.push(*key);
            ControlFlow::Continue(())
        });
        hits
    };
    assert_eq!(probe(Point3::new(10.0, 5.0, 1.5)), [WALL]);
    assert_eq!(probe(Point3::new(-19.0, 1.0, 0.0)), [SLAB]);
    assert!(probe(Point3::new(0.0, 0.0, 50.0)).is_empty());

    // A ray along +x at the wall's mid-height meets the wall and nothing else.
    let mut hits = Vec::new();
    bvh.visit_ray(
        &Ray3 {
            origin: Point3::new(0.0, 5.0, 1.5),
            direction: Vec3::X,
        },
        &mut |hit| {
            hits.push(*hit.key);
            ControlFlow::Continue(())
        },
    );
    assert_eq!(hits, [WALL]);
}
