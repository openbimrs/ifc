//! Fill patterns and the photometric light source.

use ifc_model::{Entity, EntityId, Model, Transaction, Value};
use ifc_schema::ifc4;
use ifc_style::{
    create_fill_area_style_hatching, create_fill_area_style_tiles, create_light_source_goniometric,
    GoniometricLight, HatchLineDistance, LightSourceDraft,
};

fn colour(tx: &mut Transaction) -> EntityId {
    tx.create(Entity::new("IFCCOLOURRGB", vec![Value::Null; 4]))
}

/// The hatching takes either form of its distance select.
#[test]
fn a_hatching_takes_a_length_or_a_vector() {
    let mut model = Model::default();
    let mut tx = Transaction::new(&model);
    let style = tx.create(Entity::new("IFCCURVESTYLE", vec![Value::Null; 5]));
    let vector = tx.create(Entity::new("IFCVECTOR", vec![Value::Null; 2]));

    let by_length = create_fill_area_style_hatching(
        &mut tx,
        &model,
        ifc4(),
        style,
        HatchLineDistance::Length(0.01),
        (None, None),
        0.785,
    )
    .expect("hatching by length");
    let by_vector = create_fill_area_style_hatching(
        &mut tx,
        &model,
        ifc4(),
        style,
        HatchLineDistance::Vector(vector),
        (None, None),
        0.0,
    )
    .expect("hatching by vector");
    tx.commit(&mut model).expect("commit");

    let staged = model.get(by_length).expect("staged");
    assert_eq!(staged.type_name.as_ref(), "IFCFILLAREASTYLEHATCHING");
    assert_eq!(staged.attributes.len(), 5);
    assert_eq!(staged.attributes[1], Value::Real(0.01));
    assert_eq!(
        model.get(by_vector).expect("staged").attributes[1],
        Value::Ref(vector),
    );
}

/// A non-positive hatch spacing is refused.
#[test]
fn a_zero_hatch_spacing_is_refused() {
    let model = Model::default();
    let mut tx = Transaction::new(&model);
    let style = tx.create(Entity::new("IFCCURVESTYLE", vec![Value::Null; 5]));

    for distance in [0.0, -0.01, f64::NAN] {
        assert!(
            create_fill_area_style_hatching(
                &mut tx,
                &model,
                ifc4(),
                style,
                HatchLineDistance::Length(distance),
                (None, None),
                0.0,
            )
            .is_err(),
            "spacing {distance} was accepted; lines would coincide or invert",
        );
    }
}

/// TilingPattern is LIST [2:2]: exactly two vectors, no more, no less.
///
/// Two vectors define the lattice the tile repeats on. One leaves the
/// second repetition direction undetermined.
#[test]
fn a_tiling_needs_exactly_two_vectors() {
    let mut model = Model::default();
    let mut tx = Transaction::new(&model);
    let v1 = tx.create(Entity::new("IFCVECTOR", vec![Value::Null; 2]));
    let v2 = tx.create(Entity::new("IFCVECTOR", vec![Value::Null; 2]));
    let tile = tx.create(Entity::new("IFCSTYLEDITEM", vec![Value::Null; 3]));

    for pattern in [vec![v1], vec![v1, v2, v1]] {
        assert!(
            create_fill_area_style_tiles(&mut tx, &model, ifc4(), &pattern, &[tile], 1.0).is_err(),
            "a {}-vector tiling pattern was accepted",
            pattern.len(),
        );
    }
    assert!(
        create_fill_area_style_tiles(&mut tx, &model, ifc4(), &[v1, v2], &[], 1.0).is_err(),
        "an empty SET [1:?] of tiles was accepted",
    );
    assert!(
        create_fill_area_style_tiles(&mut tx, &model, ifc4(), &[v1, v2], &[tile], 0.0).is_err(),
        "a zero tiling scale was accepted",
    );

    let id = create_fill_area_style_tiles(&mut tx, &model, ifc4(), &[v1, v2], &[tile], 0.5)
        .expect("tiles");
    tx.commit(&mut model).expect("commit");

    let staged = model.get(id).expect("staged");
    assert_eq!(staged.type_name.as_ref(), "IFCFILLAREASTYLETILES");
    assert_eq!(staged.attributes.len(), 3);
    assert_eq!(staged.attributes[2], Value::Real(0.5));
}

/// The goniometric light stages its ten slots.
#[test]
fn a_goniometric_light_stages() {
    let mut model = Model::default();
    let mut tx = Transaction::new(&model);
    let light_colour = colour(&mut tx);
    let position = tx.create(Entity::new("IFCAXIS2PLACEMENT3D", vec![Value::Null; 3]));
    let distribution = tx.create(Entity::new(
        "IFCLIGHTINTENSITYDISTRIBUTION",
        vec![Value::Null; 2],
    ));

    let id = create_light_source_goniometric(
        &mut tx,
        &model,
        ifc4(),
        LightSourceDraft {
            name: Some("Luminaire"),
            light_colour,
            ambient_intensity: None,
            intensity: None,
        },
        GoniometricLight {
            position,
            colour_appearance: None,
            colour_temperature: 4000.0,
            luminous_flux: 3200.0,
            emission_source: "LIGHTEMITTINGDIODE",
            distribution_data_source: distribution,
        },
    )
    .expect("goniometric light");
    tx.commit(&mut model).expect("commit");

    let staged = model.get(id).expect("staged");
    assert_eq!(staged.type_name.as_ref(), "IFCLIGHTSOURCEGONIOMETRIC");
    assert_eq!(staged.attributes.len(), 10);
    assert_eq!(
        staged.attributes[6],
        Value::Real(4000.0),
        "ColourTemperature"
    );
    assert_eq!(staged.attributes[9], Value::Ref(distribution));
}

/// Absolute physical measures cannot be zero.
#[test]
fn a_lightless_luminaire_is_refused() {
    let model = Model::default();
    let mut tx = Transaction::new(&model);
    let light_colour = colour(&mut tx);
    let position = tx.create(Entity::new("IFCAXIS2PLACEMENT3D", vec![Value::Null; 3]));
    let distribution = tx.create(Entity::new(
        "IFCLIGHTINTENSITYDISTRIBUTION",
        vec![Value::Null; 2],
    ));
    let draft = LightSourceDraft {
        name: None,
        light_colour,
        ambient_intensity: None,
        intensity: None,
    };
    let base = GoniometricLight {
        position,
        colour_appearance: None,
        colour_temperature: 4000.0,
        luminous_flux: 3200.0,
        emission_source: "LIGHTEMITTINGDIODE",
        distribution_data_source: distribution,
    };

    // Zero kelvin is not a dim light; it is a light that cannot exist.
    let mut cold = base;
    cold.colour_temperature = 0.0;
    assert!(
        create_light_source_goniometric(&mut tx, &model, ifc4(), draft, cold).is_err(),
        "a zero colour temperature was accepted",
    );
    let mut dark = base;
    dark.luminous_flux = 0.0;
    assert!(
        create_light_source_goniometric(&mut tx, &model, ifc4(), draft, dark).is_err(),
        "a zero luminous flux was accepted",
    );
    let mut bogus = base;
    bogus.emission_source = "PLASMA";
    assert!(
        create_light_source_goniometric(&mut tx, &model, ifc4(), draft, bogus).is_err(),
        "an undeclared emission source was accepted",
    );
}
