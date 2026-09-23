//! Texture coordinates survive compilation, not just lowering (#30).
//!
//! Lowering attaches the channel to the face set's `TriMesh`, but what a
//! caller receives is the mesh the compiler returns. The reference compiler
//! rebuilds meshes on some paths (placement, collections), so this checks
//! the whole product path: file -> lowered graph -> compiled mesh.
//!
//! Known limit, upstream: a body with two or more items (a `Collection`
//! root) or an `IfcMappedItem` (`Instance`) loses every channel in
//! compilation, silently -- axiolid/kernel#115. This test uses a single item
//! so it pins what works today; extend it when #115 lands.
#![cfg(feature = "compile-reference-backend")]

use axiolid_core::Tolerance;
use ifc_geometry::compile::compile_product_mesh;
use ifc_geometry::lower::tessellated::UV_CHANNEL;
use ifc_model::{Codec, EntityId};
use ifc_step::StepCodec;

/// One proxy holding Figure 415's box, placed at the origin with the
/// identity context, so the compiled mesh should be the face set itself.
const FILE: &str = "ISO-10303-21;
HEADER;
FILE_DESCRIPTION((''),'2;1');
FILE_NAME('','',(''),(''),'','','');
FILE_SCHEMA(('IFC4'));
ENDSEC;
DATA;
#1=IFCPROJECT('0YvctVUKr0kugbFTf53O9L',$,'P',$,$,$,$,(#2),#3);
#2=IFCGEOMETRICREPRESENTATIONCONTEXT($,'Model',3,1.E-05,#4,$);
#3=IFCUNITASSIGNMENT((#5));
#4=IFCAXIS2PLACEMENT3D(#6,$,$);
#5=IFCSIUNIT(*,.LENGTHUNIT.,$,.METRE.);
#6=IFCCARTESIANPOINT((0.,0.,0.));
#10=IFCBUILDINGELEMENTPROXY('1YvctVUKr0kugbFTf53O9L',$,'Box',$,$,#11,#12,$,$);
#11=IFCLOCALPLACEMENT($,#4);
#12=IFCPRODUCTDEFINITIONSHAPE($,$,(#13));
#13=IFCSHAPEREPRESENTATION(#2,'Body','Tessellation',(#20));
#20=IFCTRIANGULATEDFACESET(#21,$,.T.,((1,6,5),(1,2,6),(6,2,7),(7,2,3),(7,8,6),(6,8,5),(5,8,1),(1,8,4),(4,2,1),(2,4,3),(4,8,7),(7,3,4)),$);
#21=IFCCARTESIANPOINTLIST3D(((0.,0.,0.),(1.,0.,0.),(1.,1.,0.),(0.,1.,0.),(0.,0.,2.),(1.,0.,2.),(1.,1.,2.),(0.,1.,2.)));
#30=IFCTEXTUREVERTEXLIST(((0.,-0.5),(1.,-0.5),(0.,1.5),(1.,1.5),(0.,0.),(0.,1.),(1.,0.),(1.,1.)));
#31=IFCIMAGETEXTURE(.T.,.T.,$,$,$,'t.png');
#32=IFCINDEXEDTRIANGLETEXTUREMAP((#31),#20,#30,((1,4,3),(1,2,4),(3,1,4),(4,1,2),(8,7,6),(6,7,5),(4,3,2),(2,3,1),(5,8,7),(8,5,6),(2,4,3),(3,1,2)));
ENDSEC;
END-ISO-10303-21;
";

#[test]
fn a_textured_product_compiles_with_its_uv_channel() {
    let model = StepCodec
        .read_bytes(FILE.as_bytes())
        .expect("fixture parses");
    let mesh = compile_product_mesh(&model, EntityId(10), Tolerance::MILLIMETRE)
        .expect("compiles")
        .expect("has a body");
    let channel = mesh
        .attributes
        .iter()
        .find(|c| c.name == UV_CHANNEL)
        .unwrap_or_else(|| panic!("compiler dropped `{UV_CHANNEL}`: {:?}", mesh.attributes));
    assert!(channel.is_corner_indexed());
    mesh.validate_structure().expect("valid mesh");
    // Corner 0 of triangle (1,6,5) is texture vertex 1: (0, -0.5).
    assert_eq!(
        channel.at_corner(&mesh.indices, 0),
        Some([0.0, -0.5].as_slice())
    );
}
