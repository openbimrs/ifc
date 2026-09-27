//! A minimal STEP model: one beam whose Body holds the given items.
//!
//! Shared by the body- and profile-description suites. Entity ids below 100
//! are the preamble; a test writes its own entities from `#100` upward.
//!
//! ```text
//! #11  IfcLocalPlacement -> #12 at (10, 20, 30), local X = product_x_axis
//! #20  IfcShapeRepresentation 'Body' (items)
//! #30  IfcBeam  (the product under test)
//! ```

use ifc_model::{Codec, EntityId, Model};
use ifc_step::StepCodec;

/// The beam every model carries.
pub const PRODUCT: EntityId = EntityId(30);

/// Build a model. `length_prefix` is the `IfcSIUnit` prefix (`$` for metres,
/// `.MILLI.` for millimetres); `product_x_axis` the placement's RefDirection.
pub fn model(
    schema: &str,
    length_prefix: &str,
    product_x_axis: &str,
    entities: &str,
    items: &str,
) -> Model {
    let text = format!(
        "ISO-10303-21;
HEADER;
FILE_DESCRIPTION((''),'2;1');
FILE_NAME('','',(''),(''),'','','');
FILE_SCHEMA(('{schema}'));
ENDSEC;
DATA;
#1=IFCSIUNIT(*,.LENGTHUNIT.,{length_prefix},.METRE.);
#2=IFCSIUNIT(*,.PLANEANGLEUNIT.,$,.RADIAN.);
#3=IFCUNITASSIGNMENT((#1,#2));
#4=IFCPROJECT('0',$,'p',$,$,$,$,(#10),#3);
#5=IFCCARTESIANPOINT((0.,0.,0.));
#6=IFCAXIS2PLACEMENT3D(#5,$,$);
#10=IFCGEOMETRICREPRESENTATIONCONTEXT($,'Model',3,1.E-05,#6,$);
#11=IFCLOCALPLACEMENT($,#12);
#12=IFCAXIS2PLACEMENT3D(#13,$,#14);
#13=IFCCARTESIANPOINT((10.,20.,30.));
#14=IFCDIRECTION(({product_x_axis}));
#20=IFCSHAPEREPRESENTATION(#10,'Body','SweptSolid',({items}));
#21=IFCPRODUCTDEFINITIONSHAPE($,$,(#20));
#30=IFCBEAM('1',$,'b',$,$,#11,#21,$,$);
{entities}
ENDSEC;
END-ISO-10303-21;
"
    );
    StepCodec
        .read_bytes(text.as_bytes())
        .expect("test model parses")
}

/// An IFC4 model in metres whose product placement is the identity rotation.
pub fn metres(entities: &str, items: &str) -> Model {
    model("IFC4", "$", "1.,0.,0.", entities, items)
}

/// Component-wise closeness.
pub fn close<const N: usize>(actual: [f64; N], expected: [f64; N]) -> bool {
    actual
        .iter()
        .zip(expected)
        .all(|(a, e)| (a - e).abs() < 1e-9)
}
