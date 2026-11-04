//! The source of a colour that all materials of a set state (#603): the set
//! and every material of it, or the one material of a set of one.

#![cfg(all(feature = "step", feature = "colour"))]

use ifc::{Codec, ColourResolver, ColourSource, EntityId, Model, StepCodec, SurfaceColour};

/// A second material, `Red twin`, styled with Red's `IfcSurfaceStyle`.
const RED_TWIN: &str = "#900=IFCMATERIAL('Red twin',$,$);\n\
    #901=IFCSTYLEDITEM($,(#27),$);\n\
    #902=IFCSTYLEDREPRESENTATION(#6,'Style','Material',(#901));\n\
    #903=IFCMATERIALDEFINITIONREPRESENTATION($,$,(#902),#900);\n";

/// `material_colour.ifc` with `edits` applied, then `RED_TWIN` added.
fn edited(edits: &[(&str, &str)]) -> Model {
    let path = format!(
        "{}/../../test/fixtures/synthetic-colour/material_colour.ifc",
        env!("CARGO_MANIFEST_DIR")
    );
    let mut text = std::fs::read_to_string(path).expect("fixture");
    for (from, to) in edits {
        assert!(text.contains(from), "fixture lacks {from}");
        text = text.replace(from, to);
    }
    let text = text.replace("ENDSEC;\nEND-ISO", &format!("{RED_TWIN}ENDSEC;\nEND-ISO"));
    StepCodec.read_bytes(text.as_bytes()).expect("parses")
}

fn named(model: &Model, kind: &str, slot: usize, name: &str) -> EntityId {
    model
        .of_type(kind)
        .find(|(_, entity)| entity.text(slot) == Some(name))
        .map(|(id, _)| id)
        .unwrap_or_else(|| panic!("no {kind} {name}"))
}

fn colour(model: &Model, product: &str) -> SurfaceColour {
    let id = model
        .iter()
        .find(|(_, entity)| entity.text(2) == Some(product) && entity.text(0).is_some())
        .map(|(id, _)| id)
        .unwrap_or_else(|| panic!("no product {product}"));
    ColourResolver::new(model)
        .unwrap()
        .product_colour(id)
        .unwrap_or_else(|error| panic!("{product}: {error}"))
        .unwrap_or_else(|| panic!("{product}: no colour"))
}

/// The set the source names is a material set or usage, and its materials
/// are `names`.
fn assert_set(model: &Model, colour: &SurfaceColour, names: &[&str]) {
    let ColourSource::MaterialSet {
        definition,
        materials,
    } = &colour.source
    else {
        panic!("not a material set: {colour:?}");
    };
    let mut expected: Vec<EntityId> = names
        .iter()
        .map(|name| named(model, "IFCMATERIAL", 0, name))
        .collect();
    expected.sort_unstable();
    assert_eq!(materials, &expected);
    let type_name = &model.get(*definition).expect("the set exists").type_name;
    assert!(
        type_name.starts_with("IFCMATERIAL") && type_name.contains("SET"),
        "{type_name}"
    );
}

#[test]
fn a_layer_set_that_agrees_names_the_set_and_every_material() {
    let model = edited(&[("#127=IFCMATERIALLAYER(#31,", "#127=IFCMATERIALLAYER(#900,")]);
    assert_set(&model, &colour(&model, "MULTI_LAYER"), &["Red", "Red twin"]);
}

#[test]
fn a_constituent_set_that_agrees_names_the_set_and_every_material() {
    let model = edited(&[
        ("(#221),#45);", "(#221),#951);"),
        (
            "ENDSEC;\nEND-ISO",
            "#949=IFCMATERIALCONSTITUENT('A',$,#24,$,$);\n\
         #950=IFCMATERIALCONSTITUENT('B',$,#900,$,$);\n\
         #951=IFCMATERIALCONSTITUENTSET('Pair',$,(#949,#950));\nENDSEC;\nEND-ISO",
        ),
    ]);
    assert_set(&model, &colour(&model, "PLAIN"), &["Red", "Red twin"]);
}

#[test]
fn a_set_of_one_material_keeps_the_material_source() {
    let model = edited(&[("#127=IFCMATERIALLAYER(#31,", "#127=IFCMATERIALLAYER(#24,")]);
    let got = colour(&model, "MULTI_LAYER");
    let red = named(&model, "IFCMATERIAL", 0, "Red");
    assert!(
        matches!(got.source, ColourSource::Material { material, .. } if material == red),
        "{got:?}"
    );
}
