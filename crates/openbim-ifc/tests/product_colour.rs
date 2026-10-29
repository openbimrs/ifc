//! The surface colour of each product's Body (#449): item style, then
//! presentation layer, then the material's styled representation, on the
//! generated `synthetic-colour` fixtures in IFC4, IFC4X3 and IFC2X3.

#![cfg(all(feature = "step", feature = "colour"))]

use ifc::{
    product_colours, Codec, ColourError, ColourResolver, ColourSource, EntityId, Model, StepCodec,
    SurfaceColour,
};

const RED: ([f64; 3], f64) = ([0.8, 0.1, 0.1], 0.25);
const GREEN: ([f64; 3], f64) = ([0.1, 0.7, 0.2], 0.0);
const GLASS: ([f64; 3], f64) = ([0.6, 0.8, 0.9], 0.6);
const BLUE: ([f64; 3], f64) = ([0.1, 0.2, 0.9], 0.0);
const YELLOW: ([f64; 3], f64) = ([0.9, 0.8, 0.1], 0.0);

fn fixture(name: &str) -> Model {
    let path = format!(
        "{}/../../test/fixtures/synthetic-colour/{name}",
        env!("CARGO_MANIFEST_DIR")
    );
    StepCodec
        .read_bytes(&std::fs::read(path).expect("fixture"))
        .expect("parses")
}

/// The product whose `Name` (slot 2) is `name`.
fn product(model: &Model, name: &str) -> EntityId {
    model
        .iter()
        .find(|(_, entity)| entity.text(2) == Some(name) && entity.text(0).is_some())
        .map(|(id, _)| id)
        .unwrap_or_else(|| panic!("no product {name}"))
}

/// The `IfcMaterial` named `name`.
fn material(model: &Model, name: &str) -> EntityId {
    model
        .of_type("IFCMATERIAL")
        .find(|(_, entity)| entity.text(0) == Some(name))
        .map(|(id, _)| id)
        .unwrap_or_else(|| panic!("no material {name}"))
}

fn assert_colour(colour: &SurfaceColour, (rgb, transparency): ([f64; 3], f64)) {
    for (got, want) in colour.rgb.iter().zip(rgb) {
        assert!((got - want).abs() < 1e-12, "{colour:?}");
    }
    assert_eq!(colour.transparency, Some(transparency), "{colour:?}");
}

fn colour(resolver: &ColourResolver<'_>, model: &Model, name: &str) -> SurfaceColour {
    resolver
        .product_colour(product(model, name))
        .unwrap_or_else(|error| panic!("{name}: {error}"))
        .unwrap_or_else(|| panic!("{name}: no colour"))
}

fn from_material(colour: &SurfaceColour, expected: EntityId) {
    assert!(
        matches!(colour.source, ColourSource::Material { material, .. } if material == expected),
        "{colour:?}"
    );
}

#[test]
fn ifc4_and_ifc4x3_resolve_item_layer_and_material_colours() {
    for name in ["material_colour.ifc", "material_colour_ifc4x3.ifc"] {
        let model = fixture(name);
        let resolver = ColourResolver::new(&model).expect(name);
        let red = material(&model, "Red");
        let green = material(&model, "Green");

        let only = colour(&resolver, &model, "MATERIAL");
        assert_colour(&only, RED);
        from_material(&only, red);

        let layered = colour(&resolver, &model, "LAYER_SET");
        assert_colour(&layered, GREEN);
        from_material(&layered, green);

        let item = colour(&resolver, &model, "ITEM_OVER_MATERIAL");
        assert_colour(&item, BLUE);
        assert!(matches!(item.source, ColourSource::StyledItem(_)));

        let layer = colour(&resolver, &model, "LAYER_OVER_MATERIAL");
        assert_colour(&layer, YELLOW);
        assert!(matches!(layer.source, ColourSource::PresentationLayer(_)));

        let profile = colour(&resolver, &model, "PROFILE_SET");
        assert_colour(&profile, GREEN);
        from_material(&profile, green);

        let frame = colour(&resolver, &model, "CONSTITUENT_FRAME");
        assert_colour(&frame, GREEN);
        from_material(&frame, green);

        let typed = colour(&resolver, &model, "TYPE_MATERIAL");
        assert_colour(&typed, GREEN);
        from_material(&typed, green);

        assert_eq!(
            resolver.product_colour(product(&model, "PLAIN")),
            Ok(None),
            "a material without a representation colours nothing"
        );
    }
}

#[test]
fn a_two_material_layer_set_is_ambiguous_not_its_first_layer() {
    for name in [
        "material_colour.ifc",
        "material_colour_ifc4x3.ifc",
        "material_colour_ifc2x3.ifc",
    ] {
        let model = fixture(name);
        let resolver = ColourResolver::new(&model).unwrap();
        let wall = product(&model, "MULTI_LAYER");
        let Err(ColourError::AmbiguousMaterial { materials, .. }) = resolver.product_colour(wall)
        else {
            panic!("{name}: two layers must be ambiguous");
        };
        assert_eq!(
            materials,
            [material(&model, "Red"), material(&model, "Green")]
        );
    }
}

#[test]
fn constituents_resolve_per_item_and_differing_items_refuse_the_product() {
    let model = fixture("material_colour.ifc");
    let resolver = ColourResolver::new(&model).unwrap();
    let two = product(&model, "CONSTITUENT_TWO_PARTS");
    let items = resolver.item_colours(two).unwrap();
    assert_eq!(items.len(), 2);
    assert_colour(items[0].1.as_ref().unwrap(), GREEN);
    assert_colour(items[1].1.as_ref().unwrap(), GLASS);
    from_material(items[1].1.as_ref().unwrap(), material(&model, "Glass"));
    assert_eq!(
        resolver.product_colour(two),
        Err(ColourError::ItemsDiffer {
            product: two,
            items: items.iter().map(|(item, _)| *item).collect(),
        })
    );
}

#[test]
fn ifc2x3_reads_its_own_layout() {
    let model = fixture("material_colour_ifc2x3.ifc");
    let resolver = ColourResolver::new(&model).unwrap();
    let red = material(&model, "Red");
    let green = material(&model, "Green");

    let only = colour(&resolver, &model, "MATERIAL");
    assert_colour(&only, RED);
    from_material(&only, red);
    let layered = colour(&resolver, &model, "LAYER_SET");
    assert_colour(&layered, GREEN);
    from_material(&layered, green);
    let item = colour(&resolver, &model, "ITEM_OVER_MATERIAL");
    assert_colour(&item, BLUE);
    assert!(matches!(item.source, ColourSource::StyledItem(_)));
    let listed = colour(&resolver, &model, "MATERIAL_LIST");
    assert_colour(&listed, GREEN);
    from_material(&listed, green);
}

#[test]
fn every_product_in_id_order() {
    let model = fixture("material_colour.ifc");
    let colours = product_colours(&model, None);
    assert_eq!(colours.len(), 10);
    assert!(colours.windows(2).all(|pair| pair[0].0 < pair[1].0));
    let resolved = colours
        .iter()
        .filter(|(_, colour)| matches!(colour, Ok(Some(_))))
        .count();
    // All but MULTI_LAYER, CONSTITUENT_TWO_PARTS and PLAIN.
    assert_eq!(resolved, 7);
}

#[test]
fn a_header_of_two_schemas_is_refused() {
    let mut model = fixture("material_colour.ifc");
    model.header_mut().schema = vec!["IFC4".into(), "IFC2X3".into()];
    assert!(matches!(
        ColourResolver::new(&model),
        Err(ColourError::UnsupportedSchema { .. })
    ));
}

#[test]
fn the_binding_geometry_fixture_has_no_colour() {
    // The fixture the hosts' mesh tests read states no style and no
    // material: every product resolves to none, as before #449.
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../test/fixtures/synthetic-bindings/binding_geometry.ifc"
    );
    let model = StepCodec.read_bytes(&std::fs::read(path).unwrap()).unwrap();
    for (product, colour) in product_colours(&model, None) {
        assert_eq!(colour, Ok(None), "{product}");
    }
}

#[cfg(feature = "mesh")]
#[test]
fn meshes_carry_their_colour() {
    let model = fixture("material_colour.ifc");
    for (product_id, mesh) in ifc::product_meshes(&model, None) {
        let mesh = mesh.expect("every cube meshes");
        assert!(!mesh.indices.is_empty());
        assert_eq!(
            mesh.colour,
            ColourResolver::new(&model)
                .unwrap()
                .product_colour(product_id)
        );
    }
    let only = product(&model, "MATERIAL");
    let (_, mesh) = &ifc::product_meshes(&model, Some(&[only]))[0];
    assert_colour(
        mesh.as_ref()
            .unwrap()
            .colour
            .as_ref()
            .unwrap()
            .as_ref()
            .unwrap(),
        RED,
    );
}

/// `material_colour.ifc` with the `MULTI_LAYER` wall's layers made of
/// `first` and `second`, after `extra` entities (ids from #900) are added.
fn layered(extra: &str, first: &str, second: &str) -> Model {
    let path = format!(
        "{}/../../test/fixtures/synthetic-colour/material_colour.ifc",
        env!("CARGO_MANIFEST_DIR")
    );
    let text = std::fs::read_to_string(path).expect("fixture");
    let text = replaced(
        &text,
        "#126=IFCMATERIALLAYER(#24,",
        &format!("#126=IFCMATERIALLAYER({first},"),
    );
    let text = replaced(
        &text,
        "#127=IFCMATERIALLAYER(#31,",
        &format!("#127=IFCMATERIALLAYER({second},"),
    );
    let text = replaced(
        &text,
        "ENDSEC;\nEND-ISO",
        &format!("{extra}ENDSEC;\nEND-ISO"),
    );
    StepCodec.read_bytes(text.as_bytes()).expect("parses")
}

/// `text` with `from` replaced by `to`; `from` must occur.
fn replaced(text: &str, from: &str, to: &str) -> String {
    assert!(text.contains(from), "fixture lacks {from}");
    text.replace(from, to)
}

/// `material_colour.ifc` with the PLAIN proxy made of a constituent set of
/// `first` and `second`, which no shape aspect names.
fn constituted(extra: &str, first: &str, second: &str) -> Model {
    let path = format!(
        "{}/../../test/fixtures/synthetic-colour/material_colour.ifc",
        env!("CARGO_MANIFEST_DIR")
    );
    let text = std::fs::read_to_string(path).expect("fixture");
    let text = replaced(&text, "(#221),#45);", "(#221),#951);");
    let set = format!(
        "{extra}#949=IFCMATERIALCONSTITUENT('A',$,{first},$,$);\n\
         #950=IFCMATERIALCONSTITUENT('B',$,{second},$,$);\n\
         #951=IFCMATERIALCONSTITUENTSET('Pair',$,(#949,#950));\n"
    );
    let text = replaced(&text, "ENDSEC;\nEND-ISO", &format!("{set}ENDSEC;\nEND-ISO"));
    StepCodec.read_bytes(text.as_bytes()).expect("parses")
}

/// A second material, `Red twin`, styled with Red's `IfcSurfaceStyle`.
const RED_TWIN: &str = "#900=IFCMATERIAL('Red twin',$,$);\n\
    #901=IFCSTYLEDITEM($,(#27),$);\n\
    #902=IFCSTYLEDREPRESENTATION(#6,'Style','Material',(#901));\n\
    #903=IFCMATERIALDEFINITIONREPRESENTATION($,$,(#902),#900);\n";

#[test]
fn a_layer_set_of_unstyled_materials_has_no_colour() {
    let model = layered("#904=IFCMATERIAL('Plain too',$,$);\n", "#45", "#904");
    let resolver = ColourResolver::new(&model).unwrap();
    assert_eq!(
        resolver.product_colour(product(&model, "MULTI_LAYER")),
        Ok(None)
    );
}

#[test]
fn a_layer_set_of_equal_styles_takes_their_colour() {
    let model = layered(RED_TWIN, "#24", "#900");
    let resolver = ColourResolver::new(&model).unwrap();
    let got = colour(&resolver, &model, "MULTI_LAYER");
    assert_colour(&got, RED);
    from_material(&got, material(&model, "Red"));
}

#[test]
fn a_layer_set_of_styled_and_unstyled_materials_is_ambiguous() {
    let model = layered("", "#24", "#45");
    let resolver = ColourResolver::new(&model).unwrap();
    assert!(matches!(
        resolver.product_colour(product(&model, "MULTI_LAYER")),
        Err(ColourError::AmbiguousMaterial { materials, .. })
            if materials == [material(&model, "Red"), material(&model, "Plain")]
    ));
}

#[test]
fn a_layer_set_of_different_styles_is_ambiguous() {
    let model = layered("", "#24", "#31");
    let resolver = ColourResolver::new(&model).unwrap();
    assert!(matches!(
        resolver.product_colour(product(&model, "MULTI_LAYER")),
        Err(ColourError::AmbiguousMaterial { materials, .. })
            if materials == [material(&model, "Red"), material(&model, "Green")]
    ));
}

#[test]
fn three_materials_with_one_odd_one_out_are_all_named() {
    let extra = format!("{RED_TWIN}#960=IFCMATERIALLAYER(#31,0.5,$,$,$,$,$);\n");
    // Layers: Red, Red twin; a third layer Green is added to the set.
    let model = layered(&extra, "#24", "#900");
    let text = StepCodec.write_bytes(&model).expect("writes");
    let text = replaced(
        &String::from_utf8(text).unwrap(),
        "(#126,#127)",
        "(#126,#127,#960)",
    );
    let model = StepCodec.read_bytes(text.as_bytes()).expect("parses");
    let resolver = ColourResolver::new(&model).unwrap();
    let Err(ColourError::AmbiguousMaterial { materials, .. }) =
        resolver.product_colour(product(&model, "MULTI_LAYER"))
    else {
        panic!("a differing third material must be ambiguous");
    };
    assert_eq!(
        materials,
        [
            material(&model, "Red"),
            material(&model, "Green"),
            material(&model, "Red twin")
        ]
    );
}

#[test]
fn an_unresolved_constituent_set_compares_its_materials() {
    let plain2 = "#904=IFCMATERIAL('Plain too',$,$);\n";
    let model = constituted(plain2, "#45", "#904");
    let resolver = ColourResolver::new(&model).unwrap();
    assert_eq!(resolver.product_colour(product(&model, "PLAIN")), Ok(None));

    let model = constituted(RED_TWIN, "#24", "#900");
    let resolver = ColourResolver::new(&model).unwrap();
    assert_colour(&colour(&resolver, &model, "PLAIN"), RED);

    let model = constituted("", "#24", "#31");
    let resolver = ColourResolver::new(&model).unwrap();
    assert!(matches!(
        resolver.product_colour(product(&model, "PLAIN")),
        Err(ColourError::AmbiguousMaterial { .. })
    ));
}

/// `material_colour.ifc` with `from` replaced by `to`.
fn edited(from: &str, to: &str) -> Model {
    let path = format!(
        "{}/../../test/fixtures/synthetic-colour/material_colour.ifc",
        env!("CARGO_MANIFEST_DIR")
    );
    let text = std::fs::read_to_string(path).expect("fixture");
    StepCodec
        .read_bytes(replaced(&text, from, to).as_bytes())
        .expect("parses")
}

/// A malformed record refuses the products that reach it and no others
/// (#608): the whole-model result is the unedited one except for them, and
/// asking for one product gives the same entry as asking for all.
#[test]
fn a_malformed_style_assignment_refuses_only_the_products_it_reaches() {
    let clean = fixture("material_colour.ifc");
    let expected = product_colours(&clean, None);
    for (from, to, reached, style) in [
        // The styled item of ITEM_OVER_MATERIAL's item: a bad `Styles`.
        (
            "#93=IFCSTYLEDITEM(#83,(#92),$);",
            "#93=IFCSTYLEDITEM(#83,'bad',$);",
            "ITEM_OVER_MATERIAL",
            true,
        ),
        // The shape aspect of CONSTITUENT_FRAME: a bad representation.
        (
            "#166=IFCSHAPEASPECT((#165),'Frame',$,.T.,#160);",
            "#166=IFCSHAPEASPECT((#165,'x'),'Frame',$,.T.,#160);",
            "CONSTITUENT_FRAME",
            false,
        ),
    ] {
        let model = edited(from, to);
        let hit = product(&model, reached);
        let all = product_colours(&model, None);
        assert_eq!(all.len(), expected.len());
        for ((id, got), (_, want)) in all.iter().zip(&expected) {
            if *id == hit {
                assert!(
                    if style {
                        matches!(got, Err(ColourError::Style(_)))
                    } else {
                        matches!(got, Err(ColourError::Geometry(_)))
                    },
                    "{reached}: {got:?}"
                );
            } else {
                assert_eq!(got, want, "{reached} edit moved product {id}");
            }
            assert_eq!(&product_colours(&model, Some(&[*id]))[0].1, got);
        }
        // The resolver itself accepts the model.
        assert!(ColourResolver::new(&model).is_ok());
    }
}
