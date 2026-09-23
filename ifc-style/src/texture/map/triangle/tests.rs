//! Tests for per-corner texture-coordinate resolution.
//!
//! The main fixture is the worked example in the IFC4 ADD2 TC1
//! specification, Figure 415, for `IfcIndexedTriangleTextureMap`: a 1x1x2
//! box with twelve triangles and eight texture vertices.

use ifc_model::{EntityId, Value};

use super::resolve;
use crate::StyleError;

const TYPE: &str = "IFCINDEXEDTRIANGLETEXTUREMAP";

fn index_rows(rows: &[[i64; 3]]) -> Value {
    Value::List(
        rows.iter()
            .map(|row| Value::List(row.iter().map(|&i| Value::Integer(i)).collect()))
            .collect(),
    )
}

fn pairs(rows: &[[f64; 2]]) -> Value {
    Value::List(
        rows.iter()
            .map(|[s, t]| Value::List(vec![Value::Real(*s), Value::Real(*t)]))
            .collect(),
    )
}

/// `IfcIndexedTriangleTextureMap.TexCoordIndex` from Figure 415.
const SPEC_INDEX: [[i64; 3]; 12] = [
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

/// `IfcTextureVertexList.TexCoordsList` from Figure 415.
const SPEC_COORDS: [[f64; 2]; 8] = [
    [0.0, -0.5],
    [1.0, -0.5],
    [0.0, 1.5],
    [1.0, 1.5],
    [0.0, 0.0],
    [0.0, 1.0],
    [1.0, 0.0],
    [1.0, 1.0],
];

fn spec(triangle_count: usize) -> Result<super::TriangleTextureCoordinates, StyleError> {
    resolve(
        EntityId(1),
        TYPE,
        &index_rows(&SPEC_INDEX),
        &pairs(&SPEC_COORDS),
        triangle_count,
    )
}

#[test]
fn the_specification_example_resolves_corner_by_corner() {
    let uv = spec(12).unwrap();
    assert_eq!(uv.covered(), 12);
    // Triangle 1 is texture vertices (1, 4, 3), one-based.
    assert_eq!(uv.triangle(0), Some([[0.0, -0.5], [1.0, 1.5], [0.0, 1.5]]));
}

/// The reason the result is per corner and not per position.
///
/// In Figure 415, `CoordIndex` triangle 1 is `(1,6,5)` and triangle 7 is
/// `(5,8,1)`: position 1 is the first corner of one and the third corner of
/// the other, and the two corners carry different texture coordinates. A
/// per-position UV array would have to pick one and smear the other face.
#[test]
fn one_position_carries_different_coordinates_in_different_triangles() {
    let uv = spec(12).unwrap();
    let in_first = uv.triangle(0).unwrap()[0];
    let in_seventh = uv.triangle(6).unwrap()[2];
    assert_eq!(in_first, [0.0, -0.5]);
    assert_eq!(in_seventh, [1.0, -0.5]);
    assert_ne!(in_first, in_seventh);
}

/// Real exports texture only some faces. The IfcTrafficSignLibrary signs map
/// 12 of 48 triangles, the front face.
#[test]
fn a_shorter_index_covers_only_the_leading_triangles() {
    let uv = resolve(
        EntityId(1),
        TYPE,
        &index_rows(&SPEC_INDEX[..2]),
        &pairs(&SPEC_COORDS),
        48,
    )
    .unwrap();
    assert_eq!(uv.covered(), 2);
    assert!(uv.triangle(1).is_some());
    assert_eq!(uv.triangle(2), None, "uncovered, not an error");
}

#[test]
fn more_mapped_triangles_than_the_face_set_has_is_rejected() {
    assert!(matches!(
        spec(11),
        Err(StyleError::InvalidValue {
            attribute: "TexCoordIndex",
            ..
        })
    ));
}

#[test]
fn an_index_outside_the_texture_vertex_list_is_rejected() {
    for bad in [0, 9, -1] {
        let result = resolve(
            EntityId(1),
            TYPE,
            &index_rows(&[[1, 2, bad]]),
            &pairs(&SPEC_COORDS),
            12,
        );
        assert!(
            matches!(
                result,
                Err(StyleError::InvalidValue {
                    attribute: "TexCoordIndex",
                    ..
                })
            ),
            "index {bad} must be rejected, got {result:?}"
        );
    }
}

#[test]
fn a_texture_vertex_that_is_not_a_pair_is_rejected() {
    let three = Value::List(vec![Value::List(vec![
        Value::Real(0.0),
        Value::Real(0.0),
        Value::Real(0.0),
    ])]);
    assert!(matches!(
        resolve(EntityId(1), TYPE, &index_rows(&[[1, 1, 1]]), &three, 12),
        Err(StyleError::InvalidValue {
            attribute: "TexCoordsList",
            ..
        })
    ));
}
