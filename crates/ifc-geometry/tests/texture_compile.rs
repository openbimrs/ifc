//! Texture coordinates survive compilation, not just lowering (#30).
//!
//! Lowering attaches the channel to the face set's `TriMesh`, but what a
//! caller receives is the mesh the compiler returns. The reference compiler
//! rebuilds meshes on some paths (placement, collections), so this checks
//! the whole product path: file -> lowered graph -> compiled mesh.
//!
//! Multi-item bodies (`Collection` root) and `IfcMappedItem` (`Instance`)
//! lost every channel in compilation until axiolid/kernel#115; the last two
//! tests pin both paths.
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

/// The #115 cases: textured item plus a second item (a `Collection`), and
/// the textured face set reached through a mirroring `IfcMappedItem`
/// (`Instance`). Before #115 both lost `uv` in compilation, silently.
fn with_body(items: &str, extra: &str) -> String {
    let body = "#13=IFCSHAPEREPRESENTATION(#2,'Body','Tessellation',(#20));";
    let new = format!("#13=IFCSHAPEREPRESENTATION(#2,'Body','Tessellation',({items}));");
    assert_eq!(FILE.matches(body).count(), 1, "fixture anchor");
    FILE.replace(body, &new)
        .replace("ENDSEC;\nEND-ISO", &format!("{extra}\nENDSEC;\nEND-ISO"))
}

fn compile(text: &str) -> axiolid_mesh::TriMesh {
    let model = StepCodec
        .read_bytes(text.as_bytes())
        .expect("fixture parses");
    compile_product_mesh(&model, EntityId(10), Tolerance::MILLIMETRE)
        .expect("compiles")
        .expect("has a body")
}

fn uv(mesh: &axiolid_mesh::TriMesh) -> &axiolid_mesh::AttributeChannel {
    mesh.attributes
        .iter()
        .find(|c| c.name == UV_CHANNEL)
        .unwrap_or_else(|| panic!("compiler dropped `{UV_CHANNEL}`: {:?}", mesh.attributes))
}

/// A second, untextured face set in the same body makes the root a
/// `Collection`. The textured triangles keep their values; the extra item's
/// triangles are unmapped, not zero-filled.
#[test]
fn a_second_untextured_item_keeps_the_first_items_uv() {
    let second = "#40=IFCTRIANGULATEDFACESET(#21,$,.T.,((1,3,2),(1,2,4),(2,3,4),(3,1,4)),$);";
    let mesh = compile(&with_body("#20,#40", second));
    let channel = uv(&mesh);
    mesh.validate_structure().expect("valid mesh");
    assert_eq!(mesh.indices.len(), (12 + 4) * 3, "both items compiled");
    assert_eq!(
        channel.at_corner(&mesh.indices, 0),
        Some([0.0, -0.5].as_slice())
    );
    let unmapped = (0..mesh.indices.len())
        .filter(|&c| channel.at_corner(&mesh.indices, c).is_none())
        .count();
    assert_eq!(
        unmapped,
        4 * 3,
        "exactly the second item's corners are unmapped"
    );
}

/// The textured face set reached through an `IfcMappedItem` whose target
/// mirrors (Axis2 opposing Y). Compilation swaps triangle corners 1 and 2
/// to keep the winding outward; each corner must still read its own
/// texture coordinate, so the value set per triangle is unchanged.
#[test]
fn a_mirrored_mapped_item_keeps_every_corners_uv() {
    let mapped = "#50=IFCSHAPEREPRESENTATION(#2,'Body','Tessellation',(#20));
#51=IFCREPRESENTATIONMAP(#4,#50);
#52=IFCDIRECTION((0.,-1.,0.));
#53=IFCCARTESIANTRANSFORMATIONOPERATOR3D($,#52,#6,$,$);
#54=IFCMAPPEDITEM(#51,#53);";
    let plain = compile(FILE);
    let mesh = compile(&with_body("#54", mapped));
    let (a, b) = (uv(&plain), uv(&mesh));
    mesh.validate_structure().expect("valid mesh");
    assert_eq!(mesh.indices.len(), plain.indices.len());
    for t in 0..mesh.indices.len() / 3 {
        let at = |m: &axiolid_mesh::TriMesh, c: &axiolid_mesh::AttributeChannel, k| {
            c.at_corner(&m.indices, 3 * t + k).map(<[f64]>::to_vec)
        };
        // Swapped corners: 0 stays, 1 and 2 trade places.
        assert_eq!(at(&mesh, b, 0), at(&plain, a, 0), "triangle {t} corner 0");
        assert_eq!(at(&mesh, b, 1), at(&plain, a, 2), "triangle {t} corner 1");
        assert_eq!(at(&mesh, b, 2), at(&plain, a, 1), "triangle {t} corner 2");
    }
}
