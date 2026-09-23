//! Reading an `IfcIndexedTriangleTextureMap` back through the public view.
//!
//! Authored with this crate's own writers and read through `StyleView`, so
//! the test covers the write/read pair rather than a hand-built record.

use ifc_model::{Entity, EntityId, Model, Transaction, Value};
use ifc_schema::{ifc4, ifc4x3, Schema};
use ifc_style::{
    create_indexed_triangle_texture_map, create_texture_vertex_list, StyleError, StyleView,
};

struct Fixture {
    model: Model,
    texture: EntityId,
    map: EntityId,
}

fn fixture(schema: &Schema, index: &[[i64; 3]]) -> Fixture {
    let mut model = Model::new();
    let mut tx = Transaction::new(&model);
    let texture = tx.create(Entity::new("IFCIMAGETEXTURE", vec![Value::Null; 6]));
    let face_set = tx.create(Entity::new("IFCTRIANGULATEDFACESET", vec![Value::Null; 5]));
    let coords = create_texture_vertex_list(&mut tx, schema, &[[0.0, 0.0], [1.0, 0.0], [0.0, 1.0]])
        .expect("texture vertices");
    let map =
        create_indexed_triangle_texture_map(&mut tx, schema, &[texture], face_set, coords, index)
            .expect("texture map");
    tx.commit(&mut model).expect("commit");
    Fixture {
        model,
        texture,
        map,
    }
}

/// `Maps` is `LIST [1:?] OF IfcSurfaceTexture`. It used to be read as a single
/// reference, which failed on every conforming file, this crate's own output
/// included.
#[test]
fn maps_reads_the_list_the_writer_produces() {
    for schema in [ifc4(), ifc4x3()] {
        let f = fixture(schema, &[[1, 2, 3]]);
        let map = StyleView::new(&f.model, schema)
            .indexed_texture_map(f.map)
            .unwrap();
        assert_eq!(map.maps().unwrap(), [f.texture]);
    }
}

#[test]
fn triangle_coordinates_resolve_per_corner_through_the_view() {
    let schema = ifc4();
    let f = fixture(schema, &[[1, 2, 3], [3, 2, 1]]);
    let map = StyleView::new(&f.model, schema)
        .indexed_texture_map(f.map)
        .unwrap();
    let uv = map
        .triangle_coordinates(2)
        .unwrap()
        .expect("a triangle map with TexCoordIndex");
    assert_eq!(uv.covered(), 2);
    assert_eq!(uv.triangle(0), Some([[0.0, 0.0], [1.0, 0.0], [0.0, 1.0]]));
    assert_eq!(uv.triangle(1), Some([[0.0, 1.0], [1.0, 0.0], [0.0, 0.0]]));
}

/// Omitted `TexCoordIndex` has no defined meaning, so nothing is invented.
#[test]
fn an_omitted_index_yields_none() {
    let schema = ifc4();
    let f = fixture(schema, &[]);
    let map = StyleView::new(&f.model, schema)
        .indexed_texture_map(f.map)
        .unwrap();
    assert_eq!(map.tex_coord_index().unwrap(), None);
    assert_eq!(map.triangle_coordinates(1).unwrap(), None);
}

#[test]
fn more_mapped_triangles_than_the_face_set_declares_is_an_error() {
    let schema = ifc4();
    let f = fixture(schema, &[[1, 2, 3], [1, 2, 3]]);
    let map = StyleView::new(&f.model, schema)
        .indexed_texture_map(f.map)
        .unwrap();
    assert!(matches!(
        map.triangle_coordinates(1),
        Err(StyleError::InvalidValue {
            attribute: "TexCoordIndex",
            ..
        })
    ));
}
