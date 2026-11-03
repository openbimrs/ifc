//! A minimal STEP model: one layered product with a Body and an optional Axis.
//!
//! ```text
//! #11  IfcLocalPlacement -> #12 at (10, 20, 30), local X = (0, 1, 0)
//! #20  IfcShapeRepresentation 'Body' (body items)
//! #22  IfcShapeRepresentation 'Axis' (axis items), when given
//! #30  the product, of the class under test
//! ```
//!
//! Entity ids below 100 are the preamble; a test writes its own from `#100`.

use ifc_model::{Codec, Model};
use ifc_step::StepCodec;

/// Build an IFC4 model in millimetres.
pub fn model(
    product_type: &str,
    body_items: &str,
    axis_items: Option<&str>,
    entities: &str,
) -> Model {
    parse(&text(product_type, body_items, axis_items, entities))
}

/// [`model`], but the first representation is no `Body`.
pub fn without_body(product_type: &str, entities: &str) -> Model {
    parse(&text(product_type, "#104", None, entities).replace("'Body'", "'Box'"))
}

fn parse(text: &str) -> Model {
    StepCodec
        .read_bytes(text.as_bytes())
        .expect("test model parses")
}

fn text(product_type: &str, body_items: &str, axis_items: Option<&str>, entities: &str) -> String {
    let (axis_rep, shape) = match axis_items {
        Some(items) => (
            format!("#22=IFCSHAPEREPRESENTATION(#10,'Axis','Curve2D',({items}));"),
            "#20,#22",
        ),
        None => (String::new(), "#20"),
    };
    format!(
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
#11=IFCLOCALPLACEMENT($,#12);
#12=IFCAXIS2PLACEMENT3D(#13,$,#14);
#13=IFCCARTESIANPOINT((10.,20.,30.));
#14=IFCDIRECTION((0.,1.,0.));
#20=IFCSHAPEREPRESENTATION(#10,'Body','SweptSolid',({body_items}));
{axis_rep}
#21=IFCPRODUCTDEFINITIONSHAPE($,$,({shape}));
#30={product_type}('1',$,'p',$,$,#11,#21,$,$);
{entities}
ENDSEC;
END-ISO-10303-21;
",
    )
}

/// A straight wall axis from (0, y) to (5000, y), as `#40`.
pub fn axis(y: f64) -> String {
    format!(
        "#41=IFCCARTESIANPOINT((0.,{y}));
         #42=IFCCARTESIANPOINT((5000.,{y}));
         #40=IFCPOLYLINE((#41,#42));"
    )
}

/// An extrusion `#104` of a rectangle along `direction`, positioned by `#102`
/// (`location`, `axis`, `ref_direction` as STEP argument text).
pub fn extrusion(location: &str, axis: &str, ref_direction: &str, direction: &str) -> String {
    format!(
        "#100=IFCRECTANGLEPROFILEDEF(.AREA.,'R',$,200.,100.);
         #101=IFCCARTESIANPOINT(({location}));
         #102=IFCAXIS2PLACEMENT3D(#101,{axis},{ref_direction});
         #103=IFCDIRECTION(({direction}));
         #104=IFCEXTRUDEDAREASOLID(#100,#102,#103,2800.);"
    )
}

/// The plain extrusion `#104`: identity `Position` shifted by `(0, 0, z)`.
pub fn plain(z: f64) -> String {
    extrusion(&format!("0.,0.,{z}"), "$", "$", "0.,0.,1.")
}
