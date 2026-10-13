//! Surface colour by item, by presentation layer and by material (#449),
//! against the generated `synthetic-colour` fixtures in IFC2X3, IFC4 and
//! IFC4X3, and against in-memory ambiguities.

use ifc_model::{Codec, Entity, EntityId, Model, Value};
use ifc_schema::{ifc4, Schema};
use ifc_step::StepCodec;
use ifc_style::{ColourSource, StyleError, StyleIndex, StyleView, SurfaceColour};

const RED: [f64; 3] = [0.8, 0.1, 0.1];
const GREEN: [f64; 3] = [0.1, 0.7, 0.2];
const BLUE: [f64; 3] = [0.1, 0.2, 0.9];
const YELLOW: [f64; 3] = [0.9, 0.8, 0.1];

fn fixture(name: &str) -> Model {
    let path = format!(
        "{}/../../test/fixtures/synthetic-colour/{name}",
        env!("CARGO_MANIFEST_DIR")
    );
    StepCodec
        .read_bytes(&std::fs::read(path).expect("fixture"))
        .expect("parses")
}

fn schema_of(model: &Model) -> &'static Schema {
    let version =
        ifc_schema::SchemaVersion::from_header_token(&model.header().schema[0]).expect("release");
    ifc_schema::for_version(version).expect("bundled")
}

/// The id of the entity of `type_name` whose attribute `slot` is the text
/// `name`.
fn named(model: &Model, type_name: &str, slot: usize, name: &str) -> EntityId {
    model
        .of_type(type_name)
        .find(|(_, entity)| entity.text(slot) == Some(name))
        .map(|(id, _)| id)
        .unwrap_or_else(|| panic!("no {type_name} named {name}"))
}

/// The one Body item of the product named `name`.
fn body_item(model: &Model, name: &str) -> EntityId {
    let product = named(model, "IFCBUILDINGELEMENTPROXY", 2, name);
    let shape = model.get(product).unwrap().reference(6).unwrap();
    let representation = model
        .get(shape)
        .unwrap()
        .attribute(2)
        .unwrap()
        .as_list()
        .unwrap()[0]
        .as_ref_id()
        .unwrap();
    model
        .get(representation)
        .unwrap()
        .attribute(3)
        .unwrap()
        .as_list()
        .unwrap()[0]
        .as_ref_id()
        .unwrap()
}

fn close(colour: &SurfaceColour, rgb: [f64; 3], transparency: Option<f64>) {
    for (got, want) in colour.rgb.iter().zip(rgb) {
        assert!((got - want).abs() < 1e-12, "{colour:?} is not {rgb:?}");
    }
    assert_eq!(colour.transparency, transparency, "{colour:?}");
}

#[test]
fn material_colour_resolves_in_every_release() {
    for name in [
        "material_colour.ifc",
        "material_colour_ifc4x3.ifc",
        "material_colour_ifc2x3.ifc",
    ] {
        let model = fixture(name);
        let index = StyleIndex::new(StyleView::new(&model, schema_of(&model))).unwrap();
        let red = named(&model, "IFCMATERIAL", 0, "Red");
        let colour = index.material_colour(red).unwrap().expect(name);
        close(&colour, RED, Some(0.25));
        assert!(matches!(
            colour.source,
            ColourSource::Material { material, .. } if material == red
        ));
        let green = named(&model, "IFCMATERIAL", 0, "Green");
        close(
            &index.material_colour(green).unwrap().expect(name),
            GREEN,
            Some(0.0),
        );
        // The material item carries no colour of its own.
        assert_eq!(
            index.item_colour(body_item(&model, "MATERIAL")).unwrap(),
            None
        );
    }
}

#[test]
fn a_material_without_a_representation_has_no_colour() {
    let model = fixture("material_colour.ifc");
    let index = StyleIndex::new(StyleView::new(&model, schema_of(&model))).unwrap();
    let plain = named(&model, "IFCMATERIAL", 0, "Plain");
    assert_eq!(index.material_colour(plain).unwrap(), None);
}

#[test]
fn the_item_and_the_layer_state_their_own_colours() {
    for name in [
        "material_colour.ifc",
        "material_colour_ifc4x3.ifc",
        "material_colour_ifc2x3.ifc",
    ] {
        let model = fixture(name);
        let index = StyleIndex::new(StyleView::new(&model, schema_of(&model))).unwrap();
        let item = body_item(&model, "ITEM_OVER_MATERIAL");
        let colour = index.item_colour(item).unwrap().expect(name);
        close(&colour, BLUE, Some(0.0));
        assert!(matches!(colour.source, ColourSource::StyledItem(_)));
    }
    let model = fixture("material_colour.ifc");
    let index = StyleIndex::new(StyleView::new(&model, schema_of(&model))).unwrap();
    let colour = index
        .item_colour(body_item(&model, "LAYER_OVER_MATERIAL"))
        .unwrap()
        .unwrap();
    close(&colour, YELLOW, Some(0.0));
    let layer = named(&model, "IFCPRESENTATIONLAYERWITHSTYLE", 0, "A-YELLOW");
    assert_eq!(colour.source, ColourSource::PresentationLayer(layer));
}

fn rgb(model: &mut Model, value: f64) -> EntityId {
    model.push(Entity::new(
        "IFCCOLOURRGB",
        vec![
            Value::Null,
            Value::Real(value),
            Value::Real(value),
            Value::Real(value),
        ],
    ))
}

fn surface_style(model: &mut Model, value: f64) -> EntityId {
    let colour = rgb(model, value);
    let shading = model.push(Entity::new(
        "IFCSURFACESTYLESHADING",
        vec![Value::Ref(colour), Value::Null],
    ));
    model.push(Entity::new(
        "IFCSURFACESTYLE",
        vec![
            Value::Null,
            Value::Enum("BOTH".into()),
            Value::List(vec![Value::Ref(shading)]),
        ],
    ))
}

#[test]
fn two_surface_colours_at_one_level_are_ambiguous() {
    let mut model = Model::new();
    let item = model.push(Entity::new("IFCREPRESENTATIONITEM", vec![]));
    let light = surface_style(&mut model, 0.9);
    let dark = surface_style(&mut model, 0.1);
    let styled = model.push(Entity::new(
        "IFCSTYLEDITEM",
        vec![
            Value::Ref(item),
            Value::List(vec![Value::Ref(light), Value::Ref(dark)]),
            Value::Null,
        ],
    ));
    let index = StyleIndex::new(StyleView::new(&model, ifc4())).unwrap();
    assert_eq!(
        index.item_colour(item),
        Err(StyleError::AmbiguousSurfaceColour {
            owner: styled,
            styles: vec![light, dark],
        })
    );
}

#[test]
fn a_curve_style_on_the_item_does_not_hide_the_layer_colour() {
    let mut model = Model::new();
    let item = model.push(Entity::new("IFCREPRESENTATIONITEM", vec![]));
    let curve = model.push(Entity::new(
        "IFCCURVESTYLE",
        vec![
            Value::Null,
            Value::Null,
            Value::Null,
            Value::Null,
            Value::Null,
        ],
    ));
    model.push(Entity::new(
        "IFCSTYLEDITEM",
        vec![
            Value::Ref(item),
            Value::List(vec![Value::Ref(curve)]),
            Value::Null,
        ],
    ));
    let style = surface_style(&mut model, 0.5);
    let layer = model.push(Entity::new(
        "IFCPRESENTATIONLAYERWITHSTYLE",
        vec![
            Value::Text("L".into()),
            Value::Null,
            Value::List(vec![Value::Ref(item)]),
            Value::Null,
            Value::Bool(true),
            Value::Bool(false),
            Value::Bool(false),
            Value::List(vec![Value::Ref(style)]),
        ],
    ));
    let index = StyleIndex::new(StyleView::new(&model, ifc4())).unwrap();
    let colour = index.item_colour(item).unwrap().unwrap();
    assert_eq!(colour.source, ColourSource::PresentationLayer(layer));
    assert_eq!(colour.transparency, None);
}
