//! Authoring rendering, texture maps, and photometric data.

use ifc_model::{Entity, EntityId, Model, Transaction, Value};
use ifc_schema::ifc4;
use ifc_style::{
    create_indexed_triangle_texture_map, create_light_distribution_data,
    create_light_intensity_distribution, create_surface_style_rendering, ColourOrFactor,
    SurfaceStyleRenderingDraft,
};

fn colour(tx: &mut Transaction) -> EntityId {
    tx.create(Entity::new("IFCCOLOURRGB", vec![Value::Null; 4]))
}

fn rendering(surface_colour: EntityId) -> SurfaceStyleRenderingDraft {
    SurfaceStyleRenderingDraft {
        surface_colour,
        transparency: None,
        diffuse: None,
        transmission: None,
        diffuse_transmission: None,
        reflection: None,
        specular: None,
        reflectance_method: "PHONG",
    }
}

/// A colour member and a factor member land in the same slot.
///
/// `IfcColourOrFactor` admits both, so the writer must keep them
/// distinguishable: a reference and a real are not interchangeable.
#[test]
fn a_colour_or_factor_slot_takes_either_form() {
    let mut model = Model::default();
    let mut tx = Transaction::new(&model);
    let surface = colour(&mut tx);
    let diffuse = colour(&mut tx);

    let mut as_colour = rendering(surface);
    as_colour.diffuse = Some(ColourOrFactor::Colour(diffuse));
    let by_colour =
        create_surface_style_rendering(&mut tx, &model, ifc4(), as_colour).expect("colour member");

    let mut as_factor = rendering(surface);
    as_factor.diffuse = Some(ColourOrFactor::Factor(0.5));
    let by_factor =
        create_surface_style_rendering(&mut tx, &model, ifc4(), as_factor).expect("factor member");
    tx.commit(&mut model).expect("commit");

    let staged = model.get(by_colour).expect("staged");
    assert_eq!(staged.type_name.as_ref(), "IFCSURFACESTYLERENDERING");
    assert_eq!(staged.attributes.len(), 9);
    assert_eq!(staged.attributes[2], Value::Ref(diffuse));
    assert_eq!(staged.attributes[8], Value::Enum("PHONG".into()));
    assert_eq!(
        model.get(by_factor).expect("staged").attributes[2],
        Value::Real(0.5),
    );
}

/// A factor outside [0, 1] and an undeclared method are refused.
#[test]
fn an_out_of_range_factor_or_unknown_method_is_refused() {
    let model = Model::default();
    let mut tx = Transaction::new(&model);
    let surface = colour(&mut tx);

    let mut high = rendering(surface);
    high.specular = Some(ColourOrFactor::Factor(1.5));
    assert!(
        create_surface_style_rendering(&mut tx, &model, ifc4(), high).is_err(),
        "a factor above 1 was accepted",
    );

    let mut bogus = rendering(surface);
    bogus.reflectance_method = "HOLOGRAPHIC";
    assert!(
        create_surface_style_rendering(&mut tx, &model, ifc4(), bogus).is_err(),
        "an undeclared reflectance method was accepted",
    );
}

/// The triangle map indexes with integers, not references.
#[test]
fn the_triangle_texture_map_indexes_with_integers() {
    let mut model = Model::default();
    let mut tx = Transaction::new(&model);
    let texture = tx.create(Entity::new("IFCIMAGETEXTURE", vec![Value::Null; 8]));
    let face_set = tx.create(Entity::new("IFCTRIANGULATEDFACESET", vec![Value::Null; 6]));
    let coords = tx.create(Entity::new("IFCTEXTUREVERTEXLIST", vec![Value::Null; 1]));

    let id = create_indexed_triangle_texture_map(
        &mut tx,
        ifc4(),
        &[texture],
        face_set,
        coords,
        &[[1, 2, 3], [2, 3, 4]],
    )
    .expect("triangle texture map");
    tx.commit(&mut model).expect("commit");

    let staged = model.get(id).expect("staged");
    assert_eq!(staged.type_name.as_ref(), "IFCINDEXEDTRIANGLETEXTUREMAP");
    assert_eq!(staged.attributes.len(), 4);
    assert_eq!(
        staged.attributes[3],
        Value::List(vec![
            Value::List(vec![
                Value::Integer(1),
                Value::Integer(2),
                Value::Integer(3)
            ]),
            Value::List(vec![
                Value::Integer(2),
                Value::Integer(3),
                Value::Integer(4)
            ]),
        ]),
        "TexCoordIndex is a list of integer triples",
    );
}

/// IfcPositiveInteger is one-based: a zero index is refused.
#[test]
fn a_zero_texture_index_is_refused() {
    let model = Model::default();
    let mut tx = Transaction::new(&model);
    let texture = tx.create(Entity::new("IFCIMAGETEXTURE", vec![Value::Null; 8]));
    let face_set = tx.create(Entity::new("IFCTRIANGULATEDFACESET", vec![Value::Null; 6]));
    let coords = tx.create(Entity::new("IFCTEXTUREVERTEXLIST", vec![Value::Null; 1]));

    assert!(
        create_indexed_triangle_texture_map(
            &mut tx,
            ifc4(),
            &[texture],
            face_set,
            coords,
            &[[0, 1, 2]],
        )
        .is_err(),
        "a zero-based index was accepted",
    );
}

/// A photometric row pairs each angle with one intensity.
#[test]
fn a_photometric_table_stages() {
    let mut model = Model::default();
    let mut tx = Transaction::new(&model);

    let row = create_light_distribution_data(
        &mut tx,
        ifc4(),
        0.0,
        &[0.0, 45.0, 90.0],
        &[1000.0, 800.0, 200.0],
    )
    .expect("distribution data");

    let table = create_light_intensity_distribution(&mut tx, ifc4(), "TYPE_C", &[row])
        .expect("intensity distribution");
    tx.commit(&mut model).expect("commit");

    let staged = model.get(row).expect("staged");
    assert_eq!(staged.type_name.as_ref(), "IFCLIGHTDISTRIBUTIONDATA");
    assert_eq!(staged.attributes.len(), 3);

    let staged = model.get(table).expect("staged");
    assert_eq!(staged.type_name.as_ref(), "IFCLIGHTINTENSITYDISTRIBUTION");
    assert_eq!(staged.attributes[0], Value::Enum("TYPE_C".into()));
    assert_eq!(staged.attributes[1], Value::List(vec![Value::Ref(row)]));
}

/// Mismatched angle and intensity lists are refused.
///
/// The two are read in parallel, so a row whose lengths disagree
/// pairs an angle with the wrong intensity rather than failing.
#[test]
fn a_ragged_photometric_row_is_refused() {
    let model = Model::default();
    let mut tx = Transaction::new(&model);

    assert!(
        create_light_distribution_data(&mut tx, ifc4(), 0.0, &[0.0, 45.0], &[1000.0]).is_err(),
        "a ragged row was accepted",
    );
    assert!(
        create_light_distribution_data(&mut tx, ifc4(), 0.0, &[], &[]).is_err(),
        "an empty LIST [1:?] was accepted",
    );
    assert!(
        create_light_intensity_distribution(&mut tx, ifc4(), "TYPE_C", &[]).is_err(),
        "an empty distribution table was accepted",
    );
}
