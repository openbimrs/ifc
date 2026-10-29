//! `StyleIndex` finds style records through the entities that reference
//! what is asked about (#608): the same answers with and without the
//! model's reverse index, a malformed record refusing only the lookups it
//! reaches, and several records read in ascending id.

use ifc_model::{Codec, Entity, EntityId, Model, Value};
use ifc_schema::{ifc4, Schema, SchemaVersion};
use ifc_step::StepCodec;
use ifc_style::{ColourSource, StyleError, StyleIndex, StyleView};

fn fixture(path: &str) -> Model {
    let path = format!("{}/../../test/fixtures/{path}", env!("CARGO_MANIFEST_DIR"));
    StepCodec
        .read_bytes(&std::fs::read(path).expect("fixture"))
        .expect("parses")
}

fn schema_of(model: &Model) -> &'static Schema {
    let version = SchemaVersion::from_header_token(&model.header().schema[0]).expect("release");
    ifc_schema::for_version(version).expect("bundled")
}

#[test]
fn lookups_agree_with_and_without_the_models_index() {
    let fixtures = [
        "synthetic-colour/material_colour.ifc",
        "synthetic-colour/material_colour_ifc2x3.ifc",
        "synthetic-colour/material_colour_ifc4x3.ifc",
        "synthetic-relations/cascade_ifc2x3.ifc",
        "synthetic-relations/cascade_ifc4.ifc",
        "synthetic-relations/cascade_ifc4x3.ifc",
    ];
    let mut coloured = 0;
    for name in fixtures {
        let bare = fixture(name);
        let indexed = fixture(name);
        indexed.reverse_index();
        let schema = schema_of(&bare);
        let without = StyleIndex::new(StyleView::new(&bare, schema)).expect(name);
        let with = StyleIndex::new(StyleView::new(&indexed, schema)).expect(name);
        // Every entity as an item and as a material: representation items
        // and materials are among them.
        for id in bare.ids() {
            let item = without.item_colour(id);
            assert_eq!(item, with.item_colour(id), "{name}: item #{id}");
            let material = without.material_colour(id);
            assert_eq!(material, with.material_colour(id), "{name}: material #{id}");
            coloured += usize::from(matches!(item, Ok(Some(_))))
                + usize::from(matches!(material, Ok(Some(_))));
        }
        assert!(indexed.reverse_index_if_built().is_some());
        assert!(
            bare.reverse_index_if_built().is_none(),
            "{name}: left an index"
        );
    }
    assert!(coloured >= 10, "the fixtures state {coloured} colours");
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

fn ref_list(ids: &[EntityId]) -> Value {
    Value::List(ids.iter().map(|id| Value::Ref(*id)).collect())
}

fn styled_item(model: &mut Model, item: Value, styles: Value) -> EntityId {
    model.push(Entity::new(
        "IFCSTYLEDITEM",
        vec![item, styles, Value::Null],
    ))
}

fn layer_attributes(items: Value, styles: &[EntityId]) -> Vec<Value> {
    vec![
        Value::Text("L".into()),
        Value::Null,
        items,
        Value::Null,
        Value::Bool(true),
        Value::Bool(false),
        Value::Bool(false),
        ref_list(styles),
    ]
}

/// Run `check` on a bare model and on one that keeps its index.
fn both(model: &Model, check: impl Fn(&StyleIndex<'_, '_>)) {
    let bare = model.clone();
    check(&StyleIndex::new(StyleView::new(&bare, ifc4())).expect("new never refuses"));
    assert!(bare.reverse_index_if_built().is_none(), "left an index");
    let indexed = model.clone();
    indexed.reverse_index();
    check(&StyleIndex::new(StyleView::new(&indexed, ifc4())).expect("new never refuses"));
}

#[test]
fn a_styled_item_naming_something_else_refuses_nothing() {
    let mut model = Model::new();
    let item = model.push(Entity::new("IFCREPRESENTATIONITEM", vec![]));
    let style = surface_style(&mut model, 0.4);
    let styled = styled_item(&mut model, Value::Ref(item), ref_list(&[style]));
    // An `Item` that is no reference, and a `Styles` that is no list: both
    // name nothing the lookup below asks about.
    styled_item(
        &mut model,
        Value::Text("not an item".into()),
        ref_list(&[style]),
    );
    styled_item(&mut model, Value::Null, Value::Text("not styles".into()));
    model.push(Entity::new(
        "IFCPRESENTATIONLAYERWITHSTYLE",
        layer_attributes(Value::Text("not items".into()), &[style]),
    ));
    model.push(Entity::new(
        "IFCMATERIALDEFINITIONREPRESENTATION",
        vec![
            Value::Null,
            Value::Null,
            Value::Null,
            Value::Text("m".into()),
        ],
    ));
    both(&model, |index| {
        let colour = index.item_colour(item).unwrap().unwrap();
        assert_eq!(colour.source, ColourSource::StyledItem(styled));
        assert_eq!(index.material_colour(item), Ok(None));
    });
}

#[test]
fn a_malformed_assignment_refuses_the_item_it_names() {
    let mut model = Model::new();
    let good = model.push(Entity::new("IFCREPRESENTATIONITEM", vec![]));
    let bad = model.push(Entity::new("IFCREPRESENTATIONITEM", vec![]));
    let style = surface_style(&mut model, 0.4);
    styled_item(&mut model, Value::Ref(good), ref_list(&[style]));
    let broken = styled_item(&mut model, Value::Ref(bad), Value::Text("styles".into()));
    let expected = StyleView::new(&model, ifc4())
        .styled_item(broken)
        .unwrap()
        .styles()
        .unwrap_err();
    both(&model, |index| {
        assert!(index.item_colour(good).unwrap().is_some());
        assert_eq!(index.item_colour(bad), Err(expected.clone()));
    });
}

#[test]
fn a_layer_listing_an_item_and_a_non_item_refuses_that_item() {
    let mut model = Model::new();
    let good = model.push(Entity::new("IFCREPRESENTATIONITEM", vec![]));
    let bad = model.push(Entity::new("IFCREPRESENTATIONITEM", vec![]));
    let style = surface_style(&mut model, 0.4);
    let layer = model.push(Entity::new(
        "IFCPRESENTATIONLAYERWITHSTYLE",
        layer_attributes(ref_list(&[good]), &[style]),
    ));
    let broken = model.push(Entity::new(
        "IFCPRESENTATIONLAYERWITHSTYLE",
        layer_attributes(
            Value::List(vec![Value::Ref(bad), Value::Text("x".into())]),
            &[style],
        ),
    ));
    let expected = StyleView::new(&model, ifc4())
        .presentation_layer(broken)
        .unwrap()
        .assigned_items()
        .unwrap_err();
    both(&model, |index| {
        let colour = index.item_colour(good).unwrap().unwrap();
        assert_eq!(colour.source, ColourSource::PresentationLayer(layer));
        assert_eq!(index.item_colour(bad), Err(expected.clone()));
    });
}

#[test]
fn a_definition_of_a_non_material_refuses_only_that_target() {
    let mut model = Model::new();
    let material = model.push(Entity::new(
        "IFCMATERIAL",
        vec![Value::Text("M".into()), Value::Null, Value::Null],
    ));
    let other = model.push(Entity::new("IFCREPRESENTATIONITEM", vec![]));
    model.push(Entity::new(
        "IFCMATERIALDEFINITIONREPRESENTATION",
        vec![Value::Null, Value::Null, Value::Null, Value::Ref(other)],
    ));
    both(&model, |index| {
        assert_eq!(index.material_colour(material), Ok(None));
        assert!(index.material_colour(other).is_err());
    });
}

#[test]
fn equal_colours_on_two_layers_name_the_lower_id_as_source() {
    let mut model = Model::new();
    let item = EntityId(1);
    model.insert(item, Entity::new("IFCREPRESENTATIONITEM", vec![]));
    let style = surface_style(&mut model, 0.5);
    // The higher id first: insertion order must not decide.
    for id in [50, 40] {
        model.insert(
            EntityId(id),
            Entity::new(
                "IFCPRESENTATIONLAYERWITHSTYLE",
                layer_attributes(ref_list(&[item]), &[style]),
            ),
        );
    }
    both(&model, |index| {
        let colour = index.item_colour(item).unwrap().unwrap();
        assert_eq!(colour.source, ColourSource::PresentationLayer(EntityId(40)));
    });
}

#[test]
fn equal_colours_on_two_definitions_name_the_lower_id_as_source() {
    let mut model = Model::new();
    let material = model.push(Entity::new(
        "IFCMATERIAL",
        vec![Value::Text("M".into()), Value::Null, Value::Null],
    ));
    let style = surface_style(&mut model, 0.5);
    let mut styled = Vec::new();
    for _ in 0..2 {
        let item = styled_item(&mut model, Value::Null, ref_list(&[style]));
        let representation = model.push(Entity::new(
            "IFCSTYLEDREPRESENTATION",
            vec![
                Value::Null,
                Value::Text("Style".into()),
                Value::Text("Material".into()),
                ref_list(&[item]),
            ],
        ));
        styled.push((item, representation));
    }
    // Define the second representation first, at the higher id.
    for (n, (_, representation)) in [(60, styled[1]), (55, styled[0])] {
        model.insert(
            EntityId(n),
            Entity::new(
                "IFCMATERIALDEFINITIONREPRESENTATION",
                vec![
                    Value::Null,
                    Value::Null,
                    ref_list(&[representation]),
                    Value::Ref(material),
                ],
            ),
        );
    }
    both(&model, |index| {
        let colour = index.material_colour(material).unwrap().unwrap();
        assert_eq!(
            colour.source,
            ColourSource::Material {
                material,
                styled_item: styled[0].0
            }
        );
    });
}

#[test]
fn two_styled_items_on_one_item_are_still_ambiguous() {
    let mut model = Model::new();
    let item = model.push(Entity::new("IFCREPRESENTATIONITEM", vec![]));
    let style = surface_style(&mut model, 0.5);
    styled_item(&mut model, Value::Ref(item), ref_list(&[style]));
    styled_item(&mut model, Value::Ref(item), ref_list(&[style]));
    both(&model, |index| {
        assert_eq!(
            index.item_colour(item),
            Err(StyleError::AmbiguousStyleAssignment { item, count: 2 })
        );
    });
}
