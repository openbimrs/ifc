//! An interrupted wall, one body of several coaxial extrusions, split by its
//! layers (#705): every item is cut, so each layer holds a piece of each.
#![cfg(feature = "compile-reference-backend")]

use axiolid_core::Tolerance;
use axiolid_mesh::TriMesh;
use ifc_geometry::compile::compile_product_layer_meshes;
use ifc_geometry::lower::{LayerBody, LayerCut};
use ifc_geometry::{layer_set_reference, units, DirectionSense, LayerSetDirection};
use ifc_model::{Codec, EntityId, Model};
use ifc_step::StepCodec;

const PRODUCT: EntityId = EntityId(30);

/// A wall (millimetres) whose Body is two extrusions of a 2000 x 370 profile,
/// the second at `second_position`, both 2800 high.
fn interrupted_wall(second_position: &str) -> Model {
    let text = format!(
        "ISO-10303-21;
HEADER;
FILE_DESCRIPTION((''),'2;1');
FILE_NAME('','',(''),(''),'','','');
FILE_SCHEMA(('IFC4'));
ENDSEC;
DATA;
#1=IFCSIUNIT(*,.LENGTHUNIT.,.MILLI.,.METRE.);
#2=IFCSIUNIT(*,.PLANEANGLEUNIT.,$,.RADIAN.);
#3=IFCUNITASSIGNMENT((#1,#2));
#4=IFCPROJECT('0',$,'p',$,$,$,$,(#10),#3);
#5=IFCCARTESIANPOINT((0.,0.,0.));
#6=IFCAXIS2PLACEMENT3D(#5,$,$);
#10=IFCGEOMETRICREPRESENTATIONCONTEXT($,'Model',3,1.E-05,#6,$);
#11=IFCLOCALPLACEMENT($,#6);
#20=IFCSHAPEREPRESENTATION(#10,'Body','SweptSolid',(#104,#114));
#21=IFCPRODUCTDEFINITIONSHAPE($,$,(#20,#22));
#22=IFCSHAPEREPRESENTATION(#10,'Axis','Curve2D',(#40));
#30=IFCWALL('1',$,'w',$,$,#11,#21,$,$);
#41=IFCCARTESIANPOINT((0.,0.));
#42=IFCCARTESIANPOINT((5000.,0.));
#40=IFCPOLYLINE((#41,#42));
#100=IFCRECTANGLEPROFILEDEF(.AREA.,'R',$,2000.,370.);
#101=IFCAXIS2PLACEMENT3D(#5,$,$);
#103=IFCDIRECTION((0.,0.,1.));
#104=IFCEXTRUDEDAREASOLID(#100,#101,#103,2800.);
#112=IFCCARTESIANPOINT(({second_position}));
#113=IFCAXIS2PLACEMENT3D(#112,$,$);
#114=IFCEXTRUDEDAREASOLID(#100,#113,#103,2800.);
ENDSEC;
END-ISO-10303-21;
"
    );
    StepCodec.read_bytes(text.as_bytes()).expect("parses")
}

fn volume(mesh: &TriMesh) -> f64 {
    if mesh.indices.is_empty() {
        return 0.0;
    }
    let base = mesh.positions[0];
    mesh.indices
        .as_chunks::<3>()
        .0
        .iter()
        .map(|t| {
            let [a, b, c] = [t[0], t[1], t[2]].map(|i| mesh.positions[i as usize] - base);
            a.dot(b.cross(c)) / 6.0
        })
        .sum()
}

/// The two pieces sit at x = 0 and x = 3000, so they are disjoint and each
/// layer's volume is the sum of both cuts.
#[test]
fn every_item_of_an_interrupted_wall_is_split_by_the_layers() {
    let model = interrupted_wall("3000.,0.,0.");
    let scale = units::resolve(&model).expect("units");
    let reference =
        layer_set_reference(&model, &scale, PRODUCT, LayerSetDirection::Axis2).expect("reference");
    let stack = reference
        .boundaries(
            &scale,
            DirectionSense::Positive,
            -185.0,
            &[115.0, 100.0, 40.0, 115.0],
        )
        .expect("boundaries");
    assert_eq!(stack.boundaries.len(), 5);

    let layers = compile_product_layer_meshes(
        &model,
        PRODUCT,
        &LayerCut::from(&stack),
        LayerBody::Gross,
        Tolerance::MILLIMETRE,
    )
    .expect("splits")
    .expect("body");
    let volumes: Vec<f64> = layers
        .parts
        .iter()
        .map(|part| volume(&part.as_ref().expect("a part per layer").mesh))
        .collect();
    // Two pieces of 2 m x 2.8 m each, cut across the thickness.
    let area = 2.0 * 2.0 * 2.8;
    for (got, thickness) in volumes.iter().zip([0.115, 0.100, 0.040, 0.115]) {
        assert!(
            (got - area * thickness).abs() < 1e-6,
            "{volumes:?} against area {area}"
        );
    }
}
