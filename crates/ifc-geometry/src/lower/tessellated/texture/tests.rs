//! Texture coordinates on lowered triangulated face sets.
//!
//! The main fixture is IFC4 ADD2 TC1 Figure 415, verbatim: a 1x1x2 box whose
//! eight corners each carry three different texture coordinates. It is the
//! case a per-vertex channel cannot represent, so it is the case that proves
//! the channel is per corner.

use std::collections::BTreeSet;

use axiolid_mesh::{AttributeChannel, TriMesh};
use ifc_model::{EntityId, Model, Value};

use super::UV_CHANNEL;
use crate::error::GeometryError;
use crate::lower::session::LoweringSession;
use crate::lower::tessellated::lower_triangulated_face_set_node;
use crate::solid::testkit::{entity, int_grid, list, model, n, r};
use crate::transform::Transform;
use crate::units::UnitScale;

const COORD_INDEX: [[i64; 3]; 12] = [
    [1, 6, 5],
    [1, 2, 6],
    [6, 2, 7],
    [7, 2, 3],
    [7, 8, 6],
    [6, 8, 5],
    [5, 8, 1],
    [1, 8, 4],
    [4, 2, 1],
    [2, 4, 3],
    [4, 8, 7],
    [7, 3, 4],
];
const COORD_LIST: [[f64; 3]; 8] = [
    [0.0, 0.0, 0.0],
    [1.0, 0.0, 0.0],
    [1.0, 1.0, 0.0],
    [0.0, 1.0, 0.0],
    [0.0, 0.0, 2.0],
    [1.0, 0.0, 2.0],
    [1.0, 1.0, 2.0],
    [0.0, 1.0, 2.0],
];
const TEX_INDEX: [[i64; 3]; 12] = [
    [1, 4, 3],
    [1, 2, 4],
    [3, 1, 4],
    [4, 1, 2],
    [8, 7, 6],
    [6, 7, 5],
    [4, 3, 2],
    [2, 3, 1],
    [5, 8, 7],
    [8, 5, 6],
    [2, 4, 3],
    [3, 1, 2],
];
const TEX_LIST: [[f64; 2]; 8] = [
    [0.0, -0.5],
    [1.0, -0.5],
    [0.0, 1.5],
    [1.0, 1.5],
    [0.0, 0.0],
    [0.0, 1.0],
    [1.0, 0.0],
    [1.0, 1.0],
];

const FACE_SET: u64 = 2;

fn rows<const W: usize>(rows: &[[i64; W]]) -> Value {
    let slices: Vec<&[i64]> = rows.iter().map(|row| row.as_slice()).collect();
    int_grid(&slices)
}

/// Figure 415, with `tex_index` as the map's `TexCoordIndex` (`None` omits
/// it) and `extra` appended to the model.
fn figure_415(tex_index: Option<Value>, extra: Vec<(u64, ifc_model::Entity)>) -> Model {
    let points = list(
        COORD_LIST
            .iter()
            .map(|p| list(p.iter().map(|&v| n(v)).collect()))
            .collect(),
    );
    let tex = list(
        TEX_LIST
            .iter()
            .map(|p| list(p.iter().map(|&v| n(v)).collect()))
            .collect(),
    );
    let mut entities = vec![
        (1, entity("IFCCARTESIANPOINTLIST3D", vec![points])),
        (
            FACE_SET,
            entity(
                "IFCTRIANGULATEDFACESET",
                vec![
                    r(1),
                    Value::Null,
                    Value::Bool(true),
                    rows(&COORD_INDEX),
                    Value::Null,
                ],
            ),
        ),
        (3, entity("IFCTEXTUREVERTEXLIST", vec![tex])),
        (4, entity("IFCIMAGETEXTURE", vec![Value::Null; 6])),
        (
            5,
            entity(
                "IFCINDEXEDTRIANGLETEXTUREMAP",
                vec![
                    list(vec![r(4)]),
                    r(FACE_SET),
                    r(3),
                    tex_index.unwrap_or(Value::Null),
                ],
            ),
        ),
    ];
    entities.extend(extra);
    model(entities)
}

/// Figure 415's box with no texture map at all.
fn untextured() -> Model {
    let points = list(
        COORD_LIST
            .iter()
            .map(|p| list(p.iter().map(|&v| n(v)).collect()))
            .collect(),
    );
    model(vec![
        (1, entity("IFCCARTESIANPOINTLIST3D", vec![points])),
        (
            FACE_SET,
            entity(
                "IFCTRIANGULATEDFACESET",
                vec![
                    r(1),
                    Value::Null,
                    Value::Bool(true),
                    rows(&COORD_INDEX),
                    Value::Null,
                ],
            ),
        ),
    ])
}

fn lower(model: &Model) -> Result<TriMesh, GeometryError> {
    let scale = UnitScale::default();
    let mut session = LoweringSession::new(model, &scale);
    let node =
        lower_triangulated_face_set_node(&mut session, EntityId(FACE_SET), Transform::identity())?;
    let lowered = session.finish(node)?;
    match lowered.graph.get(lowered.root) {
        Some(axiolid_model::GeometryNode::TriMesh(mesh)) => Ok(mesh.clone()),
        other => panic!("expected a TriMesh, got {other:?}"),
    }
}

fn uv(mesh: &TriMesh) -> &AttributeChannel {
    mesh.attributes
        .iter()
        .find(|c| c.name == UV_CHANNEL)
        .expect("a uv channel")
}

/// Every corner resolves to the coordinate the spec pairs with it, and the
/// box stays a valid, closed mesh with shared positions.
#[test]
fn the_specification_box_carries_a_coordinate_per_corner() {
    let mesh = lower(&figure_415(Some(rows(&TEX_INDEX)), vec![])).unwrap();
    mesh.validate_structure().expect("valid mesh");
    assert_eq!(mesh.positions.len(), 8, "positions stay shared");
    assert!(
        axiolid_mesh::EdgeAdjacency::build(&mesh).is_closed_two_manifold(),
        "UV seams must not open the box"
    );
    let channel = uv(&mesh);
    assert!(channel.is_corner_indexed());
    for (corner, tex) in TEX_INDEX.iter().flatten().enumerate() {
        let want = TEX_LIST[(*tex - 1) as usize];
        assert_eq!(
            channel.at_corner(&mesh.indices, corner),
            Some(want.as_slice()),
            "corner {corner}"
        );
    }
}

/// The reason this is per corner: each box corner carries three values.
/// Flattening to one value per position loses two of them.
#[test]
fn one_position_carries_a_different_coordinate_in_each_face() {
    let mesh = lower(&figure_415(Some(rows(&TEX_INDEX)), vec![])).unwrap();
    let channel = uv(&mesh);
    for vertex in 0..8u32 {
        let seen: BTreeSet<[u64; 2]> = mesh
            .indices
            .iter()
            .enumerate()
            .filter(|(_, &v)| v == vertex)
            .map(|(corner, _)| {
                let value = channel.at_corner(&mesh.indices, corner).unwrap();
                [value[0].to_bits(), value[1].to_bits()]
            })
            .collect();
        assert_eq!(seen.len(), 3, "vertex {vertex}");
    }
}

/// A shorter index textures only the leading triangles; the rest are
/// explicitly unmapped rather than zero.
#[test]
fn a_shorter_index_leaves_the_trailing_triangles_unmapped() {
    let mesh = lower(&figure_415(Some(rows(&TEX_INDEX[..4])), vec![])).unwrap();
    mesh.validate_structure().expect("valid mesh");
    let channel = uv(&mesh);
    assert!(
        channel.at_corner(&mesh.indices, 11).is_some(),
        "triangle 4 mapped"
    );
    for corner in 12..36 {
        assert_eq!(
            channel.at_corner(&mesh.indices, corner),
            None,
            "corner {corner}"
        );
    }
}

/// An omitted index adds no channel: the schema does not define it.
#[test]
fn an_omitted_index_adds_no_channel() {
    let mesh = lower(&figure_415(None, vec![])).unwrap();
    assert!(mesh.attributes.is_empty());
}

/// An untextured face set is untouched.
#[test]
fn an_untextured_face_set_gets_no_channel() {
    assert!(lower(&untextured()).unwrap().attributes.is_empty());
}

/// More mapped triangles than the face set has is a broken map.
#[test]
fn more_mapped_triangles_than_the_face_set_has_is_an_error() {
    let mut long = TEX_INDEX.to_vec();
    long.push([1, 2, 3]);
    let err = lower(&figure_415(Some(rows(&long)), vec![])).unwrap_err();
    assert!(err.to_string().contains("maps 13 triangles"), "{err}");
}

/// 0 is not a 1-based index, and 9 is past the eight texture vertices.
#[test]
fn an_index_outside_the_texture_vertices_is_an_error() {
    for bad in [0, 9] {
        let mut index = TEX_INDEX;
        index[3][1] = bad;
        let err = lower(&figure_415(Some(rows(&index)), vec![])).unwrap_err();
        assert!(
            err.to_string().contains(&format!("holds {bad}")),
            "{bad}: {err}"
        );
    }
}

/// A second map on the same face set is kept under its own name, in file
/// order, not silently dropped.
#[test]
fn a_second_map_gets_its_own_channel() {
    let second = entity(
        "IFCINDEXEDTRIANGLETEXTUREMAP",
        vec![list(vec![r(4)]), r(FACE_SET), r(3), rows(&TEX_INDEX[..1])],
    );
    let mesh = lower(&figure_415(Some(rows(&TEX_INDEX)), vec![(6, second)])).unwrap();
    let names: Vec<&str> = mesh.attributes.iter().map(|c| c.name.as_str()).collect();
    assert_eq!(names, ["uv", "uv1"]);
    mesh.validate_structure().expect("valid mesh");
}

/// The product frame moves positions, never texture coordinates.
#[test]
fn the_frame_does_not_move_texture_coordinates() {
    let model = figure_415(Some(rows(&TEX_INDEX)), vec![]);
    let scale = UnitScale::default();
    let mut session = LoweringSession::new(&model, &scale);
    let frame = Transform::translation([10.0, 20.0, 30.0]);
    let node = lower_triangulated_face_set_node(&mut session, EntityId(FACE_SET), frame).unwrap();
    let lowered = session.finish(node).unwrap();
    let Some(axiolid_model::GeometryNode::TriMesh(mesh)) = lowered.graph.get(lowered.root) else {
        panic!("expected a TriMesh");
    };
    assert_eq!(
        uv(mesh).at_corner(&mesh.indices, 0),
        Some([0.0, -0.5].as_slice())
    );
}
