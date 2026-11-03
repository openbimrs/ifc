//! The boundary planes of each material layer of a layered wall, slab or
//! plate (#455), on the generated `synthetic-layers` fixtures in IFC4,
//! IFC4X3 and IFC2X3. Expected values are the hand-derived ones tabled in
//! `tools/gen_layer_fixtures.py`.

#![cfg(all(
    feature = "step",
    feature = "material",
    feature = "geometry-select",
    feature = "schema"
))]

use ifc::geometry::GeometryError;
use ifc::{
    material_layer_bounds, Codec, EntityId, LayerBounds, LayerBoundsError, Model, StepCodec,
};

const TOLERANCE: f64 = 1e-12;
const ROOF: f64 = 0.866_025_403_784_438_6;

fn fixture(name: &str) -> Model {
    let path = format!(
        "{}/../../test/fixtures/synthetic-layers/{name}",
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

fn bounds(model: &Model, name: &str) -> Result<Option<LayerBounds>, LayerBoundsError> {
    material_layer_bounds(model, product(model, name))
}

fn close(actual: &[f64], expected: &[f64]) {
    assert_eq!(actual.len(), expected.len(), "{actual:?} != {expected:?}");
    for (a, e) in actual.iter().zip(expected) {
        assert!((a - e).abs() < TOLERANCE, "{actual:?} != {expected:?}");
    }
}

/// One expected stack: boundaries, then per layer (index, material, thickness).
struct Stack<'a> {
    normal: [f64; 3],
    boundaries: &'a [f64],
    layers: &'a [(usize, Option<&'a str>, f64)],
    reference: &'a str,
}

fn assert_stack(model: &Model, name: &str, expected: Stack<'_>) {
    let got = bounds(model, name)
        .unwrap_or_else(|error| panic!("{name}: {error}"))
        .unwrap_or_else(|| panic!("{name}: no usage"));
    assert_eq!(got.product, product(model, name));
    close(&got.normal, &expected.normal);
    close(&got.boundaries, expected.boundaries);
    assert_eq!(
        model.get(got.reference).map(|e| e.type_name.as_ref()),
        Some(expected.reference),
        "{name}"
    );
    assert_eq!(
        model.get(got.usage).map(|e| e.type_name.as_ref()),
        Some("IFCMATERIALLAYERSETUSAGE")
    );
    assert_eq!(
        model.get(got.layer_set).map(|e| e.type_name.as_ref()),
        Some("IFCMATERIALLAYERSET")
    );
    assert_eq!(got.layers.len(), expected.layers.len(), "{name}");
    for (layer, (index, material_name, thickness)) in got.layers.iter().zip(expected.layers) {
        assert_eq!(layer.index, *index, "{name}");
        assert_eq!(layer.material, material_name.map(|n| material(model, n)));
        assert!((layer.thickness - thickness).abs() < TOLERANCE, "{name}");
    }
}

const Y: [f64; 3] = [0.0, 1.0, 0.0];
const Z: [f64; 3] = [0.0, 0.0, 1.0];
const POLYLINE: &str = "IFCPOLYLINE";
const SOLID: &str = "IFCEXTRUDEDAREASOLID";

fn cavity(model: &Model, name: &str) {
    assert_stack(
        model,
        name,
        Stack {
            normal: Y,
            boundaries: &[-0.185, -0.07, 0.03, 0.07, 0.185],
            layers: &[
                (0, Some("Brick"), 0.115),
                (1, Some("MineralWool"), 0.1),
                (2, None, 0.04),
                (3, Some("Brick"), 0.115),
            ],
            reference: POLYLINE,
        },
    );
}

fn reversed(model: &Model) {
    assert_stack(
        model,
        "ReversedWall",
        Stack {
            normal: Y,
            boundaries: &[-0.185, -0.07, -0.03, 0.07, 0.185],
            layers: &[
                (3, Some("Brick"), 0.115),
                (2, None, 0.04),
                (1, Some("MineralWool"), 0.1),
                (0, Some("Brick"), 0.115),
            ],
            reference: POLYLINE,
        },
    );
}

fn floor(model: &Model) {
    assert_stack(
        model,
        "FloorSlab",
        Stack {
            normal: Z,
            boundaries: &[-0.28, -0.08, -0.05, 0.0],
            layers: &[
                (2, Some("Concrete"), 0.2),
                (1, Some("Insulation"), 0.03),
                (0, Some("Screed"), 0.05),
            ],
            reference: SOLID,
        },
    );
}

fn plain_set_and_column(model: &Model) {
    assert_eq!(bounds(model, "PlainSetWall"), Ok(None));
    assert_eq!(bounds(model, "Column"), Ok(None));
}

#[test]
fn ifc2x3_stacks_walls_and_slabs() {
    let model = fixture("material_layers_ifc2x3.ifc");
    cavity(&model, "CavityWall");
    reversed(&model);
    floor(&model);
    plain_set_and_column(&model);
}

#[test]
fn ifc4_and_ifc4x3_stack_every_layered_product() {
    for name in ["material_layers.ifc", "material_layers_ifc4x3.ifc"] {
        let model = fixture(name);
        cavity(&model, "CavityWall");
        cavity(&model, "ClippedWall");
        reversed(&model);
        floor(&model);
        assert_stack(
            &model,
            "OffsetAxisWall",
            Stack {
                normal: Y,
                boundaries: &[0.1, 0.3],
                layers: &[(0, Some("Concrete"), 0.2)],
                reference: POLYLINE,
            },
        );
        assert_stack(
            &model,
            "RoofSlab",
            Stack {
                normal: [0.0, -0.5, ROOF],
                boundaries: &[0.0, 0.2],
                layers: &[(0, Some("Concrete"), 0.2)],
                reference: SOLID,
            },
        );
        plain_set_and_column(&model);
    }
}

#[test]
fn unreadable_references_and_offset_layers_are_refused() {
    for name in ["material_layers.ifc", "material_layers_ifc4x3.ifc"] {
        let model = fixture(name);
        let curve = model
            .of_type("IFCTRIMMEDCURVE")
            .next()
            .map(|(id, _)| id)
            .expect("a trimmed curve");
        match bounds(&model, "CurvedWall") {
            Err(LayerBoundsError::Geometry(GeometryError::Unsupported { entity, .. })) => {
                assert_eq!(entity, curve);
            }
            other => panic!("{name}: {other:?}"),
        }
        match bounds(&model, "NoAxisWall") {
            Err(LayerBoundsError::Geometry(GeometryError::MissingAttribute {
                entity,
                attribute,
                ..
            })) => {
                assert_eq!(entity, product(&model, "NoAxisWall"));
                assert_eq!(attribute, "Axis representation");
            }
            other => panic!("{name}: {other:?}"),
        }
        match bounds(&model, "OffsetLayersWall") {
            Err(LayerBoundsError::LayerWithOffsets { usage, layer }) => {
                assert_eq!(
                    model.get(usage).map(|e| e.type_name.as_ref()),
                    Some("IFCMATERIALLAYERSETUSAGE")
                );
                assert_eq!(
                    model.get(layer).map(|e| e.type_name.as_ref()),
                    Some("IFCMATERIALLAYERWITHOFFSETS")
                );
            }
            other => panic!("{name}: {other:?}"),
        }
    }
}

#[test]
fn a_usage_inherited_from_the_type_is_stacked() {
    let path = format!(
        "{}/../../test/fixtures/synthetic-layers/material_layers.ifc",
        env!("CARGO_MANIFEST_DIR")
    );
    let text = std::fs::read_to_string(path).expect("fixture");
    // Move the CavityWall's material association onto a new wall type.
    let line = text
        .lines()
        .find(|line| line.contains("'CavityWall material'"))
        .expect("association");
    let wall = line
        .split(",(#")
        .nth(1)
        .and_then(|rest| rest.split(')').next())
        .expect("related wall")
        .to_string();
    let typed = text
        .replace(line, &line.replace(&format!("(#{wall})"), "(#9001)"))
        .replace(
            "ENDSEC;\nEND-ISO",
            &format!(
                "#9001=IFCWALLTYPE('2nFUheMFHOjfrnoQm9$Sst',$,'CavityType',$,$,$,$,$,$,.STANDARD.);\n\
                 #9002=IFCRELDEFINESBYTYPE('2nFUheMFHOjfrnoQm9$Ssu',$,$,$,(#{wall}),#9001);\n\
                 ENDSEC;\nEND-ISO"
            ),
        );
    let model = StepCodec.read_bytes(typed.as_bytes()).expect("parses");
    cavity(&model, "CavityWall");
}
