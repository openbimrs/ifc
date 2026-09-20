//! Texture authoring: image sources, coordinate maps and generators.
//!
//! The writer enforces cardinality and resolvable references. It does
//! not check that the vertex list matches the face's loop -- that
//! needs the face's geometry, which this crate deliberately never
//! inspects.

use ifc_model::{Entity, EntityId, Model, Transaction, Value};
use ifc_schema::ifc4x3;
use ifc_style::{
    create_image_texture, create_texture_coordinate_generator, create_texture_map,
    ImageTextureDraft,
};

fn texture(tx: &mut Transaction) -> EntityId {
    tx.create(Entity::new(
        "IFCIMAGETEXTURE",
        vec![
            Value::Bool(true),
            Value::Bool(true),
            Value::Null,
            Value::Null,
            Value::Null,
            Value::Text("texture.png".into()),
        ],
    ))
}

fn vertex(tx: &mut Transaction) -> EntityId {
    tx.create(Entity::new("IFCTEXTUREVERTEX", vec![Value::List(vec![])]))
}

/// An image texture without a URL references no image.
#[test]
fn a_texture_needs_somewhere_to_load_the_image_from() {
    let schema = ifc4x3();
    let model = Model::new();
    let mut tx = Transaction::new(&model);

    assert!(
        create_image_texture(
            &mut tx,
            &model,
            schema,
            ImageTextureDraft {
                repeat_s: true,
                repeat_t: true,
                mode: None,
                texture_transform: None,
                url_reference: "   ",
            },
        )
        .is_err(),
        "a blank URL names no image"
    );

    let id = create_image_texture(
        &mut tx,
        &model,
        schema,
        ImageTextureDraft {
            repeat_s: true,
            repeat_t: false,
            mode: Some("MODULATE"),
            texture_transform: None,
            url_reference: "../textures/brick.png",
        },
    )
    .expect("a referenced image is accepted");

    let mut model = model;
    tx.commit(&mut model).expect("commit");

    let entity = model.get(id).expect("staged");
    assert_eq!(entity.type_name.as_ref(), "IFCIMAGETEXTURE");
    assert_eq!(entity.attributes[0], Value::Bool(true), "RepeatS");
    assert_eq!(
        entity.attributes[1],
        Value::Bool(false),
        "RepeatT is distinct from RepeatS"
    );
    assert_eq!(
        entity.attributes[5],
        Value::Text("../textures/brick.png".into()),
        "URLReference"
    );
}

/// A texture map needs at least a triangle, and a face to map onto.
#[test]
fn a_texture_map_needs_three_vertices_and_a_face() {
    let schema = ifc4x3();
    let mut model = Model::new();
    let face = model.push(Entity::new("IFCFACE", vec![Value::List(vec![])]));
    let mut tx = Transaction::new(&model);

    let map = texture(&mut tx);
    let a = vertex(&mut tx);
    let b = vertex(&mut tx);
    let c = vertex(&mut tx);

    assert!(
        create_texture_map(&mut tx, &model, schema, &[], &[a, b, c], face).is_err(),
        "the record exists to carry textures"
    );
    assert!(
        create_texture_map(&mut tx, &model, schema, &[map], &[a, b], face).is_err(),
        "two vertices bound no area"
    );

    let id = create_texture_map(&mut tx, &model, schema, &[map], &[a, b, c], face)
        .expect("a triangle over a face is accepted");

    tx.commit(&mut model).expect("commit");

    let entity = model.get(id).expect("staged");
    assert_eq!(entity.type_name.as_ref(), "IFCTEXTUREMAP");
    assert_eq!(
        entity.attributes[0],
        Value::List(vec![Value::Ref(map)]),
        "Maps is slot 0, distinct from the vertex list"
    );
    assert_eq!(
        entity.attributes[1],
        Value::List(vec![Value::Ref(a), Value::Ref(b), Value::Ref(c)]),
        "Vertices is slot 1"
    );
    assert_eq!(entity.attributes[2], Value::Ref(face), "MappedTo is slot 2");
}

/// A texture map refuses a `MappedTo` that is not a face.
#[test]
fn a_texture_map_refuses_a_non_face_target() {
    let schema = ifc4x3();
    let mut model = Model::new();
    let not_a_face = model.push(Entity::new("IFCWALL", vec![Value::Null; 8]));
    let mut tx = Transaction::new(&model);

    let map = texture(&mut tx);
    let a = vertex(&mut tx);
    let b = vertex(&mut tx);
    let c = vertex(&mut tx);

    assert!(
        create_texture_map(&mut tx, &model, schema, &[map], &[a, b, c], not_a_face).is_err(),
        "texture coordinates belong to a face"
    );
}

/// A generator needs a mode: it is the function being applied.
#[test]
fn a_generator_needs_a_mode_and_finite_parameters() {
    let schema = ifc4x3();
    let model = Model::new();
    let mut tx = Transaction::new(&model);

    let map = texture(&mut tx);

    assert!(
        create_texture_coordinate_generator(&mut tx, schema, &[map], "  ", &[]).is_err(),
        "without a mode there is no function to apply"
    );
    assert!(
        create_texture_coordinate_generator(&mut tx, schema, &[], "SPHERE", &[]).is_err(),
        "the record exists to carry textures"
    );
    assert!(
        create_texture_coordinate_generator(&mut tx, schema, &[map], "SPHERE", &[f64::NAN])
            .is_err(),
        "a non-finite parameter is refused"
    );

    let id = create_texture_coordinate_generator(&mut tx, schema, &[map], "SPHERE", &[2.0, 0.5])
        .expect("a named mode with finite parameters is accepted");

    let mut model = model;
    tx.commit(&mut model).expect("commit");

    let entity = model.get(id).expect("staged");
    assert_eq!(entity.type_name.as_ref(), "IFCTEXTURECOORDINATEGENERATOR");
    assert_eq!(entity.attributes[1], Value::Text("SPHERE".into()), "Mode");
    assert_eq!(
        entity.attributes[2],
        Value::List(vec![Value::Real(2.0), Value::Real(0.5)]),
        "Parameter is slot 2"
    );
}
