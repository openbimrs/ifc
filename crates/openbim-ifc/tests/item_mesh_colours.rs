//! Item colours and item meshes pair index by index (#656).
//!
//! `ColourResolver::item_colours` and
//! `ifc::geometry::compile::compile_product_item_meshes` walk a Body the same
//! way, so a viewer that colours per item zips them and finds the same item at
//! each index. The window #10 lists its pane (#40, glass) before its frame
//! (#30); #80 and #90 share one map whose representation lists the same two.

#![cfg(all(feature = "step", feature = "mesh"))]

use ifc::geometry::compile::{compile_product_item_meshes, Tolerance};
use ifc::geometry::geometric_products;
use ifc::{Codec, ColourError, ColourResolver, EntityId, Model, StepCodec};

const TOL: Tolerance = Tolerance::MILLIMETRE;

/// Frame and pane styles; the map's pane #140 is styled glass too.
const STYLES: &str = "#50=IFCCOLOURRGB('Frame',0.875,0.835,0.741);
#51=IFCSURFACESTYLERENDERING(#50,0.,$,$,$,$,$,$,.NOTDEFINED.);
#52=IFCSURFACESTYLE('Frame',.BOTH.,(#51));
#53=IFCSTYLEDITEM(#30,(#52),$);
#60=IFCCOLOURRGB('Glass',0.941,0.969,0.953);
#61=IFCSURFACESTYLERENDERING(#60,0.69,$,$,$,$,$,$,.NOTDEFINED.);
#62=IFCSURFACESTYLE('Glass',.BOTH.,(#61));
#63=IFCSTYLEDITEM(#40,(#62),$);
#64=IFCSTYLEDITEM(#140,(#62),$);";

fn window(items: &str, extra: &str) -> Model {
    let data = format!(
        "#1=IFCPROJECT('0YvctVUKr0kugbFTf53O9L',$,'P',$,$,$,$,(#2),#3);
#2=IFCGEOMETRICREPRESENTATIONCONTEXT($,'Model',3,1.E-05,#4,$);
#3=IFCUNITASSIGNMENT((#5));
#4=IFCAXIS2PLACEMENT3D(#6,$,$);
#5=IFCSIUNIT(*,.LENGTHUNIT.,$,.METRE.);
#6=IFCCARTESIANPOINT((0.,0.,0.));
#7=IFCDIRECTION((0.,0.,1.));
#8=IFCAXIS2PLACEMENT2D(#9,$);
#9=IFCCARTESIANPOINT((0.,0.));
#10=IFCWINDOW('1YvctVUKr0kugbFTf53O9L',$,'Window',$,$,#11,#12,$,1.2,1.,.WINDOW.,.NOTDEFINED.,$);
#11=IFCLOCALPLACEMENT($,#4);
#12=IFCPRODUCTDEFINITIONSHAPE($,$,(#16));
#16=IFCSHAPEREPRESENTATION(#2,'Body','SweptSolid',({items}));
#30=IFCEXTRUDEDAREASOLID(#31,#4,#7,1.2);
#31=IFCRECTANGLEPROFILEDEF(.AREA.,$,#8,1.,0.1);
#40=IFCEXTRUDEDAREASOLID(#41,#42,#7,1.);
#41=IFCRECTANGLEPROFILEDEF(.AREA.,$,#8,0.8,0.01);
#42=IFCAXIS2PLACEMENT3D(#43,$,$);
#43=IFCCARTESIANPOINT((0.,0.5,0.1));
{extra}
"
    );
    parse(&data)
}

fn parse(data: &str) -> Model {
    let text = format!(
        "ISO-10303-21;
HEADER;
FILE_DESCRIPTION((''),'2;1');
FILE_NAME('','',(''),(''),'','','');
FILE_SCHEMA(('IFC4'));
ENDSEC;
DATA;
{data}ENDSEC;
END-ISO-10303-21;
"
    );
    StepCodec
        .read_bytes(text.as_bytes())
        .expect("fixture parses")
}

const MAPPED: &str = "#70=IFCREPRESENTATIONMAP(#4,#71);
#71=IFCSHAPEREPRESENTATION(#2,'Body','SweptSolid',(#140,#130));
#130=IFCEXTRUDEDAREASOLID(#31,#4,#7,1.2);
#140=IFCEXTRUDEDAREASOLID(#41,#42,#7,1.);
#80=IFCWINDOW('2Yvct0000000000000000A',$,'A',$,$,#81,#82,$,$,$,$,$,$);
#81=IFCLOCALPLACEMENT($,#83);
#83=IFCAXIS2PLACEMENT3D(#84,$,$);
#84=IFCCARTESIANPOINT((5.,0.,0.));
#82=IFCPRODUCTDEFINITIONSHAPE($,$,(#85));
#85=IFCSHAPEREPRESENTATION(#2,'Body','MappedRepresentation',(#86));
#86=IFCMAPPEDITEM(#70,#87);
#87=IFCCARTESIANTRANSFORMATIONOPERATOR3D($,$,#6,$,$);
#90=IFCWINDOW('3Yvct0000000000000000B',$,'B',$,$,#91,#92,$,$,$,$,$,$);
#91=IFCLOCALPLACEMENT($,#93);
#93=IFCAXIS2PLACEMENT3D(#94,$,$);
#94=IFCCARTESIANPOINT((10.,0.,0.));
#92=IFCPRODUCTDEFINITIONSHAPE($,$,(#95));
#95=IFCSHAPEREPRESENTATION(#2,'Body','MappedRepresentation',(#96));
#96=IFCMAPPEDITEM(#70,#97);
#97=IFCCARTESIANTRANSFORMATIONOPERATOR3D($,$,#98,$,$);
#98=IFCCARTESIANPOINT((0.,0.,2.));";

fn ids_of(colours: &[(EntityId, Option<ifc::SurfaceColour>)]) -> Vec<EntityId> {
    colours.iter().map(|(item, _)| *item).collect()
}

fn transparency(colour: &Option<ifc::SurfaceColour>) -> Option<f64> {
    colour.as_ref().and_then(|colour| colour.transparency)
}

#[test]
fn colours_and_meshes_line_up_by_index() {
    let model = window(
        "#40,#30",
        &format!("{MAPPED}\n{STYLES}\n#54=IFCSTYLEDITEM(#130,(#52),$);"),
    );
    let resolver = ColourResolver::new(&model).expect("resolver");
    for product in [10, 80, 90] {
        let product = EntityId(product);
        let colours = resolver.item_colours(product).expect("colours");
        let meshes = compile_product_item_meshes(&model, product, TOL)
            .expect("meshes")
            .expect("body");
        assert_eq!(colours.len(), 2);
        assert_eq!(colours.len(), meshes.len());
        for ((item, _), mesh) in colours.iter().zip(&meshes) {
            assert_eq!(*item, mesh.item, "product {product:?}");
        }
        assert_eq!(transparency(&colours[0].1), Some(0.69), "glass first");
        assert_eq!(transparency(&colours[1].1), Some(0.0), "then frame");
        assert!(matches!(
            resolver.product_colour(product),
            Err(ColourError::ItemsDiffer { .. })
        ));
    }
}

/// Styling the mapped item itself reaches every leaf of that occurrence.
#[test]
fn a_style_on_the_mapped_item_reaches_every_leaf() {
    let model = window(
        "#40,#30",
        &format!("{MAPPED}\n{STYLES}\n#65=IFCSTYLEDITEM(#96,(#62),$);"),
    );
    let resolver = ColourResolver::new(&model).expect("resolver");
    let product = EntityId(90);
    let colours = resolver.item_colours(product).expect("colours");
    let meshes = compile_product_item_meshes(&model, product, TOL)
        .expect("meshes")
        .expect("body");
    assert_eq!(
        ids_of(&colours),
        meshes.iter().map(|m| m.item).collect::<Vec<_>>()
    );
    assert_eq!(colours.len(), 2);
    for (_, colour) in &colours {
        assert_eq!(transparency(colour), Some(0.69));
    }
}

/// Two map levels: the order still pairs, and the leaf is reached through both.
#[test]
fn nested_maps_line_up() {
    let model = window(
        "#180",
        "#180=IFCMAPPEDITEM(#181,#182);
#181=IFCREPRESENTATIONMAP(#4,#183);
#182=IFCCARTESIANTRANSFORMATIONOPERATOR3D(#184,#185,#6,$,#7);
#183=IFCSHAPEREPRESENTATION(#2,'Body','MappedRepresentation',(#190));
#184=IFCDIRECTION((0.,1.,0.));
#185=IFCDIRECTION((-1.,0.,0.));
#190=IFCMAPPEDITEM(#191,#192);
#191=IFCREPRESENTATIONMAP(#4,#193);
#192=IFCCARTESIANTRANSFORMATIONOPERATOR3D($,$,#194,$,$);
#193=IFCSHAPEREPRESENTATION(#2,'Body','SweptSolid',(#150,#155));
#194=IFCCARTESIANPOINT((0.,1.,0.));
#150=IFCEXTRUDEDAREASOLID(#151,#4,#7,1.);
#151=IFCRECTANGLEPROFILEDEF(.AREA.,$,#8,0.2,0.2);
#155=IFCEXTRUDEDAREASOLID(#151,#156,#7,1.);
#156=IFCAXIS2PLACEMENT3D(#157,$,$);
#157=IFCCARTESIANPOINT((0.,0.,1.5));",
    );
    let resolver = ColourResolver::new(&model).expect("resolver");
    let colours = resolver.item_colours(EntityId(10)).expect("colours");
    let meshes = compile_product_item_meshes(&model, EntityId(10), TOL)
        .expect("meshes")
        .expect("body");
    assert_eq!(colours.len(), 2);
    assert_eq!(
        ids_of(&colours),
        meshes.iter().map(|m| m.item).collect::<Vec<_>>()
    );
    assert_eq!(ids_of(&colours), vec![EntityId(150), EntityId(155)]);
    for mesh in &meshes {
        assert_eq!(mesh.mapped_by, vec![EntityId(180), EntityId(190)]);
    }
}

#[test]
fn the_colour_corpus_lines_up() {
    let mut total = 0;
    for name in [
        "material_colour.ifc",
        "material_colour_ifc2x3.ifc",
        "material_colour_ifc4x3.ifc",
    ] {
        let path = format!(
            "{}/../../test/fixtures/synthetic-colour/{name}",
            env!("CARGO_MANIFEST_DIR")
        );
        let model = StepCodec
            .read_bytes(&std::fs::read(path).expect("fixture"))
            .expect("parses");
        let resolver = ColourResolver::new(&model).expect("resolver");
        let mut compared = 0;
        for product in geometric_products(&model) {
            let (Ok(colours), Ok(Some(meshes))) = (
                resolver.item_colours(product),
                compile_product_item_meshes(&model, product, TOL),
            ) else {
                continue;
            };
            assert_eq!(
                ids_of(&colours),
                meshes.iter().map(|m| m.item).collect::<Vec<_>>(),
                "{name}: product {product:?}"
            );
            compared += 1;
        }
        assert!(compared > 0, "{name}: nothing compared");
        total += compared;
    }
    assert!(total >= TOTAL_MIN, "only {total} products compared");
}

/// Products the first run compared across the three fixtures.
const TOTAL_MIN: usize = 22;
