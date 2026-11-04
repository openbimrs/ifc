//! A Body split by its material layer stack (#455): one exact part per layer,
//! cut in the product's frame, the outer layers extending.
//!
//! Models are the generated `synthetic-layers/material_layers.ifc` (IFC4, in
//! millimetres); stacks and expected volumes are the hand-derived ones tabled
//! in `tools/gen_layer_fixtures.py`. Every wall is 5 m long and 2.8 m high,
//! 0.37 m thick; a volume that is off by a whole layer is a half space
//! applied with the wrong polarity.
#![cfg(feature = "compile-reference-backend")]

use axiolid_contracts::{Backend, BackendDescriptor, ExecutionOptions, GeomError, GeomResult};
use axiolid_core::Tolerance;
use axiolid_mesh::TriMesh;
use axiolid_mesh_compile_contract::MeshCompiler;
use axiolid_model::{GeometryGraph, GeometryNode, NodeId, SolidOperation};
use ifc_geometry::compile::{
    compile_product_layer_meshes, compile_product_layer_meshes_with, compile_product_mesh,
    compile_product_mesh_net, default_backend, LayerMeshes,
};
use ifc_geometry::lower::{lower_product_layers, LayerBody, LayerCut, LoweringSession};
use ifc_geometry::{layer_set_reference, units, DirectionSense, GeometryError, LayerSetDirection};
use ifc_model::{Codec, EntityId, Model};
use ifc_step::StepCodec;

const TOLERANCE: f64 = 1e-6;

fn fixture() -> Model {
    let path = format!(
        "{}/../../test/fixtures/synthetic-layers/material_layers.ifc",
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

/// The cut of a usage: direction, sense and offset as authored, thicknesses
/// in the fixture's millimetres, in set order.
fn cut(
    model: &Model,
    id: EntityId,
    direction: LayerSetDirection,
    sense: DirectionSense,
    offset: f64,
    thicknesses: &[f64],
) -> LayerCut {
    let scale = units::resolve(model).expect("units");
    let reference = layer_set_reference(model, &scale, id, direction).expect("reference");
    let stack = reference
        .boundaries(&scale, sense, offset, thicknesses)
        .expect("boundaries");
    LayerCut::from(&stack)
}

const CAVITY: [f64; 4] = [115.0, 100.0, 40.0, 115.0];
const FLOOR: [f64; 3] = [50.0, 30.0, 200.0];

fn cavity_cut(model: &Model, name: &str) -> LayerCut {
    cut(
        model,
        product(model, name),
        LayerSetDirection::Axis2,
        DirectionSense::Positive,
        -185.0,
        &CAVITY,
    )
}

fn volume(mesh: &TriMesh) -> f64 {
    if mesh.indices.is_empty() {
        return 0.0;
    }
    let base = mesh.positions[0];
    mesh.indices
        .as_chunks::<3>()
        .0
        .iter()
        .map(|t| {
            let [a, b, c] = [t[0], t[1], t[2]].map(|i| mesh.positions[i as usize] - base);
            a.dot(b.cross(c)) / 6.0
        })
        .sum()
}

fn volumes(layers: &LayerMeshes) -> Vec<Option<f64>> {
    layers
        .parts
        .iter()
        .map(|part| part.as_ref().map(|part| volume(&part.mesh)))
        .collect()
}

fn split(model: &Model, name: &str, cut: &LayerCut, body: LayerBody) -> LayerMeshes {
    compile_product_layer_meshes(
        model,
        product(model, name),
        cut,
        body,
        Tolerance::MILLIMETRE,
    )
    .unwrap_or_else(|error| panic!("{name} must split, got {error}"))
    .expect("the product has a body")
}

fn gross(model: &Model, name: &str) -> f64 {
    let mesh = compile_product_mesh(model, product(model, name), Tolerance::MILLIMETRE)
        .expect("gross compiles")
        .expect("body");
    volume(&mesh)
}

fn assert_volumes(actual: &[Option<f64>], expected: &[f64]) {
    assert_eq!(actual.len(), expected.len(), "{actual:?}");
    for (index, (a, e)) in actual.iter().zip(expected).enumerate() {
        let a = a.unwrap_or_else(|| panic!("layer {index} has no part"));
        assert!(
            (a - e).abs() <= TOLERANCE * e.abs().max(1.0),
            "layer {index}: volume {a}, expected {e} ({actual:?})"
        );
    }
}

fn total(volumes: &[Option<f64>]) -> f64 {
    volumes.iter().flatten().sum()
}

#[test]
fn a_cavity_wall_splits_into_its_layer_volumes() {
    let model = fixture();
    let layers = split(
        &model,
        "CavityWall",
        &cavity_cut(&model, "CavityWall"),
        LayerBody::Gross,
    );
    let parts = volumes(&layers);
    let area = 5.0 * 2.8;
    assert_volumes(
        &parts,
        &[area * 0.115, area * 0.100, area * 0.040, area * 0.115],
    );
    assert!((total(&parts) - gross(&model, "CavityWall")).abs() < 1e-9);
    assert!(layers.openings.is_empty());
    assert!(layers
        .parts
        .iter()
        .flatten()
        .all(|part| part.closure == ifc_geometry::compile::MeshClosure::Solid));
}

#[test]
fn a_reversed_wall_lists_the_same_volumes_in_reverse() {
    let model = fixture();
    let reversed = cut(
        &model,
        product(&model, "ReversedWall"),
        LayerSetDirection::Axis2,
        DirectionSense::Negative,
        185.0,
        &CAVITY,
    );
    let parts = volumes(&split(&model, "ReversedWall", &reversed, LayerBody::Gross));
    let area = 5.0 * 2.8;
    // Boundaries ascend; the layers follow them, so the last layer is first.
    assert_volumes(
        &parts,
        &[area * 0.115, area * 0.040, area * 0.100, area * 0.115],
    );
}

#[test]
fn a_clipped_wall_parts_add_up_to_its_gross_body() {
    let model = fixture();
    let parts = volumes(&split(
        &model,
        "ClippedWall",
        &cavity_cut(&model, "ClippedWall"),
        LayerBody::Gross,
    ));
    // Clipped at z = 2.4 m.
    let area = 5.0 * 2.4;
    assert_volumes(
        &parts,
        &[area * 0.115, area * 0.100, area * 0.040, area * 0.115],
    );
    assert!((total(&parts) - gross(&model, "ClippedWall")).abs() < 1e-9);
}

#[test]
fn a_floor_slab_splits_along_its_normal() {
    let model = fixture();
    let floor = cut(
        &model,
        product(&model, "FloorSlab"),
        LayerSetDirection::Axis3,
        DirectionSense::Negative,
        280.0,
        &FLOOR,
    );
    let parts = volumes(&split(&model, "FloorSlab", &floor, LayerBody::Gross));
    // Ascending boundaries: Concrete, Insulation, Screed; 4 x 3 m.
    assert_volumes(&parts, &[12.0 * 0.200, 12.0 * 0.030, 12.0 * 0.050]);
    assert!((total(&parts) - gross(&model, "FloorSlab")).abs() < 1e-9);
}

#[test]
fn a_tilted_slab_of_one_layer_is_its_body() {
    let model = fixture();
    let roof = cut(
        &model,
        product(&model, "RoofSlab"),
        LayerSetDirection::Axis3,
        DirectionSense::Positive,
        0.0,
        &[200.0],
    );
    let parts = volumes(&split(&model, "RoofSlab", &roof, LayerBody::Gross));
    assert_eq!(parts.len(), 1);
    let body = gross(&model, "RoofSlab");
    assert!(
        (parts[0].unwrap() - body).abs() < 1e-9,
        "{parts:?} != {body}"
    );
}

#[test]
fn a_body_thicker_than_its_stack_extends_the_outer_layers() {
    let model = fixture();
    let id = product(&model, "CavityWall");
    // The body spans y in [-0.185, 0.185]; the stack covers only [-0.07, 0.07].
    let inner = LayerCut::new([0.0, 1.0, 0.0], vec![-0.07, 0.0, 0.07]);
    let layers =
        compile_product_layer_meshes(&model, id, &inner, LayerBody::Gross, Tolerance::MILLIMETRE)
            .expect("splits")
            .expect("body");
    let parts = volumes(&layers);
    let area = 5.0 * 2.8;
    assert_volumes(&parts, &[area * (0.185 - 0.0), area * 0.185]);
    assert!((total(&parts) - gross(&model, "CavityWall")).abs() < 1e-9);
}

#[test]
fn a_layer_the_body_does_not_reach_is_an_empty_mesh_not_a_refusal() {
    let model = fixture();
    let id = product(&model, "CavityWall");
    // The body spans y in [-0.185, 0.185]. Layer 0 extends below 0.1 and takes
    // y < 0.1, layer 1 takes [0.1, 0.185], layer 2 (y > 0.3) lies beyond it.
    let beyond = LayerCut::new([0.0, 1.0, 0.0], vec![0.0, 0.1, 0.3, 0.5]);
    let layers =
        compile_product_layer_meshes(&model, id, &beyond, LayerBody::Gross, Tolerance::MILLIMETRE)
            .expect("splits")
            .expect("body");
    let parts = volumes(&layers);
    assert_eq!(parts.len(), 3);
    let body = gross(&model, "CavityWall");
    assert!((total(&parts) - body).abs() < 1e-9, "{parts:?}");
    let area = 5.0 * 2.8;
    assert!((parts[0].unwrap() - area * 0.285).abs() < 1e-6, "{parts:?}");
    assert!((parts[1].unwrap() - area * 0.085).abs() < 1e-6, "{parts:?}");
    assert!(parts[2].unwrap().abs() < 1e-9, "{parts:?}");
}

#[test]
fn a_zero_thick_layer_has_no_part_and_its_neighbour_extends() {
    let model = fixture();
    let id = product(&model, "CavityWall");
    let doubled = LayerCut::new([0.0, 1.0, 0.0], vec![-0.185, -0.07, -0.07, 0.07, 0.185]);
    let layers = compile_product_layer_meshes(
        &model,
        id,
        &doubled,
        LayerBody::Gross,
        Tolerance::MILLIMETRE,
    )
    .expect("splits")
    .expect("body");
    let parts = volumes(&layers);
    assert_eq!(parts[1], None);
    let area = 5.0 * 2.8;
    assert!((parts[0].unwrap() - area * 0.115).abs() < 1e-6);
    assert!((parts[2].unwrap() - area * 0.140).abs() < 1e-6);
    assert!((parts[3].unwrap() - area * 0.115).abs() < 1e-6);
}

#[test]
fn net_parts_add_up_to_the_net_body() {
    // A product with no opening: net is gross, and nothing is reported.
    let model = fixture();
    let layers = split(
        &model,
        "CavityWall",
        &cavity_cut(&model, "CavityWall"),
        LayerBody::Net(Default::default()),
    );
    let net =
        compile_product_mesh_net(&model, product(&model, "CavityWall"), Tolerance::MILLIMETRE)
            .expect("net")
            .expect("body");
    assert!((total(&volumes(&layers)) - volume(&net.mesh)).abs() < 1e-9);
    assert!(layers.openings.is_empty() && layers.taken_as_applied.is_empty());
}

fn degenerate(model: &Model, cut: &LayerCut) -> GeometryError {
    let id = product(model, "CavityWall");
    compile_product_layer_meshes(model, id, cut, LayerBody::Gross, Tolerance::MILLIMETRE)
        .expect_err("an invalid cut is refused")
}

#[test]
fn invalid_cuts_are_degenerate() {
    let model = fixture();
    let id = product(&model, "CavityWall");
    let cuts = [
        LayerCut::new([0.0, 2.0, 0.0], vec![0.0, 1.0]),
        LayerCut::new([f64::NAN, 0.0, 0.0], vec![0.0, 1.0]),
        LayerCut::new([0.0, 1.0, 0.0], vec![0.0]),
        LayerCut::new([0.0, 1.0, 0.0], vec![0.0, f64::INFINITY]),
        LayerCut::new([0.0, 1.0, 0.0], vec![0.1, 0.0]),
        LayerCut::new([0.0, 1.0, 0.0], vec![0.1, 0.1]),
    ];
    for cut in &cuts {
        match degenerate(&model, cut) {
            GeometryError::Degenerate { entity, .. } => assert_eq!(entity, id, "{cut:?}"),
            other => panic!("{cut:?}: expected Degenerate, got {other}"),
        }
    }
}

#[test]
fn lowering_attributes_the_parts_to_the_product_and_returns_one_per_layer() {
    let model = fixture();
    let id = product(&model, "CavityWall");
    let scale = units::resolve(&model).expect("units");
    let mut session = LoweringSession::new(&model, &scale);
    let layered = lower_product_layers(
        &mut session,
        id,
        &cavity_cut(&model, "CavityWall"),
        LayerBody::Gross,
    )
    .expect("lowers")
    .expect("body");
    assert_eq!(layered.parts.len(), 4);
    assert!(layered.parts.iter().all(Option::is_some));
    assert!(layered.net.is_none());
}

/// What the stub kernel refuses.
#[derive(Debug, Clone, Copy)]
enum Refuse {
    /// Any boolean whose tool is a half space on the lower side
    /// (`agreement = false`) of the boundary at this height along y: the cut
    /// that starts exactly one layer.
    LowerCutAt(f64),
    /// Any extrusion: the Body's own leaf.
    Extrusion,
}

/// The reference compiler, refusing one chosen piece of any graph.
#[derive(Debug)]
struct Stub(Refuse);

impl Backend for Stub {
    fn descriptor(&self) -> BackendDescriptor {
        default_backend().descriptor()
    }
}

fn reachable(graph: &GeometryGraph, root: NodeId) -> Vec<NodeId> {
    let mut seen = vec![root];
    let mut index = 0;
    while index < seen.len() {
        if let Some(node) = graph.get(seen[index]) {
            for child in node.references() {
                if !seen.contains(&child) {
                    seen.push(child);
                }
            }
        }
        index += 1;
    }
    seen
}

impl MeshCompiler for Stub {
    fn compile_mesh(
        &self,
        graph: &GeometryGraph,
        root: NodeId,
        options: &ExecutionOptions,
    ) -> GeomResult<TriMesh> {
        for id in reachable(graph, root) {
            let refused = match (self.0, graph.get(id)) {
                (
                    Refuse::LowerCutAt(y),
                    Some(GeometryNode::SolidOperation(SolidOperation::Boolean { right, .. })),
                ) => matches!(
                    graph.get(*right),
                    Some(GeometryNode::HalfSpace(half))
                        if !half.agreement && (half.boundary.origin.y - y).abs() < 1e-9
                ),
                (
                    Refuse::Extrusion,
                    Some(GeometryNode::SolidOperation(SolidOperation::Extrusion { .. })),
                ) => true,
                _ => false,
            };
            if refused {
                return Err(GeomError::InvalidInput(format!("stub refuses {id:?}")));
            }
        }
        default_backend().compile_mesh(graph, root, options)
    }
}

#[test]
fn a_backend_refusing_one_layer_cut_names_that_layer() {
    let model = fixture();
    let id = product(&model, "CavityWall");
    // The lower cut at y = 0.03 belongs to layer 2 alone.
    let error = compile_product_layer_meshes_with(
        &Stub(Refuse::LowerCutAt(0.03)),
        &model,
        id,
        &cavity_cut(&model, "CavityWall"),
        LayerBody::Gross,
        Tolerance::MILLIMETRE,
    )
    .expect_err("the stub refuses a cut");
    assert!(
        matches!(&error, GeometryError::LayerNotSplit { product, layer: 2, .. } if *product == id),
        "{error:?}"
    );
    assert_eq!(error.entity(), Some(id));
}

#[test]
fn a_backend_refusing_the_body_is_not_blamed_on_a_layer() {
    let model = fixture();
    let id = product(&model, "CavityWall");
    let error = compile_product_layer_meshes_with(
        &Stub(Refuse::Extrusion),
        &model,
        id,
        &cavity_cut(&model, "CavityWall"),
        LayerBody::Gross,
        Tolerance::MILLIMETRE,
    )
    .expect_err("the stub refuses the Body");
    assert!(
        matches!(&error, GeometryError::CompilationRefused { entity, .. } if *entity == id),
        "{error:?}"
    );
}

#[test]
fn a_net_body_in_world_coordinates_cannot_be_cut_by_a_host_frame_stack() {
    let path = format!(
        "{}/../../test/fixtures/synthetic-lowering/flush_openings_site_placements.ifc",
        env!("CARGO_MANIFEST_DIR")
    );
    let model = StepCodec
        .read_bytes(&std::fs::read(path).expect("fixture"))
        .expect("parses");
    // The ABSOLUTE wall's openings share no placement chain with it.
    let host = product(&model, "ABSOLUTE");
    let cut = LayerCut::new([0.0, 1.0, 0.0], vec![-0.125, 0.0, 0.125]);
    let error = compile_product_layer_meshes(
        &model,
        host,
        &cut,
        LayerBody::Net(Default::default()),
        Tolerance::MILLIMETRE,
    )
    .expect_err("a plane is not moved to another frame");
    assert!(
        matches!(&error, GeometryError::Unsupported { entity, .. } if *entity == host),
        "{error:?}"
    );
}
