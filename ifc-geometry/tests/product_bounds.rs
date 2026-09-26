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
//! It is an extrusion, so it is bounded through the compiled mesh; the
//! triangulated slab is a mesh leaf and is bounded exactly.
#![cfg(feature = "compile-reference-backend")]

use std::ops::ControlFlow;

use axiolid_core::{Aabb, Point3, Ray3, Tolerance, Vec3};
use axiolid_spatial::{Bvh, SpatialIndex, SpatialItem};
use ifc_geometry::compile::{product_bounds, BoundsSource};
use ifc_geometry::error::GeometryError;
use ifc_model::{Codec, EntityId, Model};
use ifc_step::StepCodec;

const WALL: EntityId = EntityId(10);
const SLAB: EntityId = EntityId(30);
const SPACE: EntityId = EntityId(40);

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
",
        x = l(10.0),
        y = l(5.0),
        height = l(3.0),
        length = l(4.0),
        thickness = l(0.3),
        sx = l(-20.0),
        a = l(0.0),
        b = l(2.0),
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
        // Planar geometry: the tessellated box is the exact box.
        assert_eq!(bounds.source, BoundsSource::Tessellated);
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
