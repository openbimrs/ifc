//! One mesh per material layer of a layered product (#455), on the generated
//! `synthetic-layers` fixtures. Expected values are the hand-derived ones
//! tabled in `tools/gen_layer_fixtures.py`: a wall is 5 m long and 2.8 m
//! high, its layers 115, 100, 40 and 115 mm thick.

#![cfg(all(
    feature = "step",
    feature = "material",
    feature = "mesh",
    feature = "geometry-select",
    feature = "schema"
))]

use ifc::geometry::compile::MeshClosure;
use ifc::{
    material_layer_meshes, Codec, EntityId, LayerBody, MaterialLayerMesh, MaterialLayerMeshes,
    Model, StepCodec,
};

const AREA: f64 = 5.0 * 2.8;
const THICKNESS: [f64; 4] = [0.115, 0.100, 0.040, 0.115];

fn path(name: &str) -> String {
    format!(
        "{}/../../test/fixtures/synthetic-layers/{name}",
        env!("CARGO_MANIFEST_DIR")
    )
}

fn parse(bytes: &[u8]) -> Model {
    StepCodec.read_bytes(bytes).expect("parses")
}

fn fixture(name: &str) -> Model {
    parse(&std::fs::read(path(name)).expect("fixture"))
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

fn volume(layer: &MaterialLayerMesh) -> f64 {
    let point = |i: u32| {
        let at = 3 * i as usize;
        [0, 1, 2].map(|k| f64::from(layer.positions[at + k]))
    };
    if layer.indices.is_empty() {
        return 0.0;
    }
    let base = point(layer.indices[0]);
    layer
        .indices
        .as_chunks::<3>()
        .0
        .iter()
        .map(|t| {
            let [a, b, c] = [t[0], t[1], t[2]].map(|i| {
                let p = point(i);
                [p[0] - base[0], p[1] - base[1], p[2] - base[2]]
            });
            let cross = [
                b[1] * c[2] - b[2] * c[1],
                b[2] * c[0] - b[0] * c[2],
                b[0] * c[1] - b[1] * c[0],
            ];
            (a[0] * cross[0] + a[1] * cross[1] + a[2] * cross[2]) / 6.0
        })
        .sum()
}

fn assert_volumes(meshes: &MaterialLayerMeshes, expected: &[f64]) {
    assert_eq!(meshes.layers.len(), expected.len());
    for (layer, expected) in meshes.layers.iter().zip(expected) {
        let actual = volume(layer);
        assert!(
            (actual - expected).abs() < 1e-4 * expected.max(1.0),
            "layer {}: volume {actual}, expected {expected}",
            layer.index
        );
        assert_eq!(layer.closure, MeshClosure::Solid);
    }
}

fn cavity(model: &Model, body: LayerBody) -> MaterialLayerMeshes {
    material_layer_meshes(model, product(model, "CavityWall"), body)
        .expect("splits")
        .expect("a layered wall")
}

#[test]
fn a_cavity_wall_has_a_mesh_and_a_material_per_layer() {
    let model = fixture("material_layers.ifc");
    let meshes = cavity(&model, LayerBody::Gross);
    assert_volumes(&meshes, &THICKNESS.map(|t| AREA * t));
    let materials: Vec<_> = meshes.layers.iter().map(|l| l.material).collect();
    assert_eq!(
        materials,
        [
            Some(material(&model, "Brick")),
            Some(material(&model, "MineralWool")),
            None,
            Some(material(&model, "Brick")),
        ]
    );
    for (layer, bounded) in meshes.layers.iter().zip(&meshes.bounds.layers) {
        assert_eq!((layer.index, layer.layer), (bounded.index, bounded.layer));
    }
    assert!(meshes.openings.is_empty());
}

#[test]
fn positions_are_in_the_bounds_frame_when_the_context_frame_is_the_identity() {
    let model = fixture("material_layers.ifc");
    let meshes = cavity(&model, LayerBody::Gross);
    for (layer, bounds) in meshes
        .layers
        .iter()
        .zip(meshes.bounds.boundaries.windows(2))
    {
        // The wall's thickness runs along its local y axis.
        let ys: Vec<f64> = layer
            .positions
            .as_chunks::<3>()
            .0
            .iter()
            .map(|p| f64::from(p[1]))
            .collect();
        let (low, high) = ys
            .iter()
            .fold((f64::MAX, f64::MIN), |(l, h), &y| (l.min(y), h.max(y)));
        assert!((low - bounds[0]).abs() < 1e-5 && (high - bounds[1]).abs() < 1e-5);
    }
}

/// The fixture's CavityWall with a 1.0 x 2.1 m opening cut through it, as an
/// `IfcOpeningElement` placed relative to the wall's own placement. The
/// fixture is in millimetres, and the opening's profile overhangs the wall's
/// 370 mm so the cut runs through every layer.
fn with_opening(name: &str) -> (Model, EntityId, EntityId) {
    let text = std::fs::read_to_string(path(name)).expect("fixture");
    let model = parse(text.as_bytes());
    let wall = product(&model, "CavityWall");
    let placement = model
        .get(wall)
        .and_then(|entity| entity.reference(5))
        .expect("an object placement")
        .0;
    let context = text
        .lines()
        .find(|line| line.contains("SUBCONTEXT('Body'"))
        .and_then(|line| {
            line.strip_prefix('#')?
                .split('=')
                .next()?
                .parse::<u64>()
                .ok()
        })
        .expect("a Body context");
    let first = model.iter().map(|(id, _)| id.0).max().expect("entities") + 1;
    let [opening, local, axis, point, shape, rep, solid, position, origin, direction, profile, rel] =
        std::array::from_fn(|k| first + k as u64);
    let added = format!(
        "#{opening}=IFCOPENINGELEMENT('2YvctVUKr0kugbFTf53O01',$,'Opening',$,$,#{local},#{shape},$,.OPENING.);
#{local}=IFCLOCALPLACEMENT(#{placement},#{axis});
#{axis}=IFCAXIS2PLACEMENT3D(#{point},$,$);
#{point}=IFCCARTESIANPOINT((1500.,0.,0.));
#{shape}=IFCPRODUCTDEFINITIONSHAPE($,$,(#{rep}));
#{rep}=IFCSHAPEREPRESENTATION(#{context},'Body','SweptSolid',(#{solid}));
#{solid}=IFCEXTRUDEDAREASOLID(#{profile},#{position},#{direction},2100.);
#{profile}=IFCRECTANGLEPROFILEDEF(.AREA.,$,$,1000.,500.);
#{position}=IFCAXIS2PLACEMENT3D(#{origin},$,$);
#{origin}=IFCCARTESIANPOINT((0.,0.,0.));
#{direction}=IFCDIRECTION((0.,0.,1.));
#{rel}=IFCRELVOIDSELEMENT('3YvctVUKr0kugbFTf53O02',$,$,$,#{wall},#{opening});
",
        wall = wall.0,
    );
    let end = text
        .rfind("ENDSEC;\nEND-ISO")
        .expect("the data section ends");
    let edited = format!("{}{added}{}", &text[..end], &text[end..]);
    (parse(edited.as_bytes()), wall, EntityId(opening))
}

#[test]
fn net_layers_lose_the_opening_and_gross_layers_do_not() {
    let (model, wall, opening) = with_opening("material_layers.ifc");
    let net = material_layer_meshes(&model, wall, LayerBody::Net(Default::default()))
        .expect("splits")
        .expect("a layered wall");
    assert_eq!(net.openings, vec![opening]);
    assert_volumes(&net, &THICKNESS.map(|t| (AREA - 1.0 * 2.1) * t));
    let gross = material_layer_meshes(&model, wall, LayerBody::default())
        .expect("splits")
        .expect("a layered wall");
    assert!(gross.openings.is_empty());
    assert_volumes(&gross, &THICKNESS.map(|t| AREA * t));
}

#[test]
fn other_releases_split_the_same_wall() {
    for name in ["material_layers_ifc2x3.ifc", "material_layers_ifc4x3.ifc"] {
        let model = fixture(name);
        let meshes = cavity(&model, LayerBody::Gross);
        assert_volumes(&meshes, &THICKNESS.map(|t| AREA * t));
        let slab = material_layer_meshes(&model, product(&model, "FloorSlab"), LayerBody::Gross)
            .expect("splits")
            .expect("a layered slab");
        assert_volumes(&slab, &[12.0 * 0.2, 12.0 * 0.03, 12.0 * 0.05]);
    }
}

#[test]
fn a_product_without_a_layer_set_usage_has_no_layer_meshes() {
    let model = fixture("material_layers.ifc");
    for name in ["Column", "PlainSetWall"] {
        let none = material_layer_meshes(&model, product(&model, name), LayerBody::Gross)
            .expect("not an error");
        assert!(none.is_none(), "{name}");
    }
}

/// The cavity fixture with its context's `WorldCoordinateSystem` replaced by
/// `frame`, STEP records for `#4` (its point) and `#5` (the placement).
fn with_context_frame(frame: &str) -> Model {
    let text = std::fs::read_to_string(path("material_layers.ifc")).expect("fixture");
    let plain = "#4=IFCCARTESIANPOINT((0.,0.,0.));\n#5=IFCAXIS2PLACEMENT3D(#4,$,$);";
    assert!(text.contains(plain));
    parse(text.replace(plain, frame).as_bytes())
}

/// Positions are in the frame `bounds` is stated in whatever the context's
/// frame is, and `world` carries them to world space (#706).
#[test]
fn positions_stay_in_the_bounds_frame_under_a_context_frame() {
    let plain = cavity(&fixture("material_layers.ifc"), LayerBody::Gross);
    // The wall is placed at (1, 2, 0) m. A context shifted 5 m along y puts
    // its origin at (1, 7, 0); one turned 180 degrees about z and shifted
    // 0.1 m along x maps (x, y) to (0.1 - x, -y).
    let cases = [
        (
            "#4=IFCCARTESIANPOINT((0.,5000.,0.));\n#5=IFCAXIS2PLACEMENT3D(#4,$,$);",
            [1.0, 7.0, 0.0],
        ),
        (
            "#4=IFCCARTESIANPOINT((100.,0.,0.));\n#900=IFCDIRECTION((-1.,0.,0.));\n\
             #901=IFCDIRECTION((0.,0.,1.));\n#5=IFCAXIS2PLACEMENT3D(#4,#901,#900);",
            [-0.9, -2.0, 0.0],
        ),
    ];
    for (frame, origin) in cases {
        let meshes = cavity(&with_context_frame(frame), LayerBody::Gross);
        // Bounds and positions are both in the object frame, so neither
        // depends on the context.
        assert_eq!(meshes.bounds.boundaries, plain.bounds.boundaries);
        for (layer, base) in meshes.layers.iter().zip(&plain.layers) {
            assert_eq!(layer.positions.len(), base.positions.len());
            for (a, b) in layer.positions.iter().zip(&base.positions) {
                assert!((a - b).abs() < 1e-4, "{a} != {b}");
            }
        }
        // `world` is the context frame above the placement.
        for (a, e) in meshes.world.origin.iter().zip(origin) {
            assert!(
                (a - e).abs() < 1e-9,
                "{:?} != {origin:?}",
                meshes.world.origin
            );
        }
    }
}
