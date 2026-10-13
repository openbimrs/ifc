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
    let model = StepCodec
        .read_bytes(&std::fs::read(path).unwrap())
        .unwrap();
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
        mesh.as_ref().unwrap().colour.as_ref().unwrap().as_ref().unwrap(),
        RED,
    );
}
