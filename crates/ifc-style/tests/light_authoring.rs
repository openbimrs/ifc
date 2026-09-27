//! Light sources and the remaining surface-style elements.
//!
//! Normalised ratios carry a schema bound of 0..=1. A renderer given
//! 2.0 does not fail, it clamps or blows out, so the writer is the
//! only place the bound can be enforced.

use ifc_model::{Entity, Model, Transaction, Value};
use ifc_schema::{ifc4x3, Schema};
use ifc_style::{
    create_light_source_ambient, create_light_source_directional, create_light_source_positional,
    create_light_source_spot, create_surface_style_lighting, create_surface_style_refraction,
    Attenuation, ColourRgbDraft, LightSourceDraft, PointLight, SpotCone,
};

use ifc_style::create_colour_rgb;

/// Stage a colour to hang the lights off.
fn colour(tx: &mut Transaction, schema: &Schema) -> ifc_model::EntityId {
    create_colour_rgb(
        tx,
        schema,
        ColourRgbDraft {
            name: None,
            red: 1.0,
            green: 1.0,
            blue: 1.0,
        },
    )
    .expect("a white colour is valid")
}

/// A plain light draft with both ratios set.
fn draft(colour: ifc_model::EntityId) -> LightSourceDraft<'static> {
    LightSourceDraft {
        name: Some("key"),
        light_colour: colour,
        ambient_intensity: Some(0.25),
        intensity: Some(0.75),
    }
}

/// The four light sources round-trip, each keeping its own slots.
#[test]
fn light_sources_round_trip() {
    let schema = ifc4x3();
    let mut model = Model::new();
    let mut tx = Transaction::new(&model);
    let c = colour(&mut tx, schema);

    let orientation = tx.create(Entity::new("IFCDIRECTION", vec![Value::Null]));
    let position = tx.create(Entity::new("IFCCARTESIANPOINT", vec![Value::Null]));

    let ambient = create_light_source_ambient(&mut tx, &model, schema, draft(c))
        .expect("an ambient light is accepted");
    let directional =
        create_light_source_directional(&mut tx, &model, schema, draft(c), orientation)
            .expect("a directional light is accepted");

    let point = PointLight {
        position,
        radius: 2.0,
        attenuation: Attenuation {
            constant: 1.0,
            distance: 0.5,
            quadric: 0.25,
        },
    };
    let positional = create_light_source_positional(&mut tx, &model, schema, draft(c), point)
        .expect("a positional light is accepted");

    tx.commit(&mut model).expect("commit");

    for (id, name) in [
        (ambient, "IFCLIGHTSOURCEAMBIENT"),
        (directional, "IFCLIGHTSOURCEDIRECTIONAL"),
        (positional, "IFCLIGHTSOURCEPOSITIONAL"),
    ] {
        let e = model.get(id).expect("staged");
        assert_eq!(e.type_name.as_ref(), name, "each light keeps its class");
        assert_eq!(e.attributes[1], Value::Ref(c), "LightColour at slot 1");
    }

    let e = model.get(directional).expect("staged");
    assert_eq!(
        e.attributes[4],
        Value::Ref(orientation),
        "Orientation at slot 4"
    );

    let e = model.get(positional).expect("staged");
    assert_eq!(e.attributes[4], Value::Ref(position), "Position at slot 4");
    assert_eq!(e.attributes[5], Value::Real(2.0), "Radius at slot 5");
    assert_eq!(e.attributes[6], Value::Real(1.0), "ConstantAttenuation");
    assert_eq!(e.attributes[7], Value::Real(0.5), "DistanceAttenuation");
    assert_eq!(e.attributes[8], Value::Real(0.25), "QuadricAttenuation");
}

/// A spot light keeps its cone angles distinct and after the shared
/// point-light slots.
#[test]
fn a_spot_light_keeps_its_cone() {
    let schema = ifc4x3();
    let mut model = Model::new();
    let mut tx = Transaction::new(&model);
    let c = colour(&mut tx, schema);
    let orientation = tx.create(Entity::new("IFCDIRECTION", vec![Value::Null]));
    let position = tx.create(Entity::new("IFCCARTESIANPOINT", vec![Value::Null]));

    let point = PointLight {
        position,
        radius: 3.0,
        attenuation: Attenuation::default(),
    };
    // Deliberately different angles: a transposition must be visible.
    let cone = SpotCone {
        orientation,
        concentration_exponent: Some(2.0),
        spread_angle: 0.4,
        beam_width_angle: 0.9,
    };
    let id = create_light_source_spot(&mut tx, &model, schema, draft(c), point, cone)
        .expect("a spot light is accepted");

    tx.commit(&mut model).expect("commit");

    let e = model.get(id).expect("staged");
    assert_eq!(e.attributes[9], Value::Ref(orientation), "Orientation at 9");
    assert_eq!(e.attributes[10], Value::Real(2.0), "ConcentrationExponent");
    assert_eq!(e.attributes[11], Value::Real(0.4), "SpreadAngle at 11");
    assert_eq!(e.attributes[12], Value::Real(0.9), "BeamWidthAngle at 12");
}

/// Every bounded measure is refused when it leaves its range.
#[test]
fn out_of_range_measures_are_refused() {
    let schema = ifc4x3();
    let model = Model::new();
    let mut tx = Transaction::new(&model);
    let c = colour(&mut tx, schema);
    let position = tx.create(Entity::new("IFCCARTESIANPOINT", vec![Value::Null]));
    let orientation = tx.create(Entity::new("IFCDIRECTION", vec![Value::Null]));

    let over = LightSourceDraft {
        name: None,
        light_colour: c,
        ambient_intensity: Some(1.5),
        intensity: None,
    };
    assert!(
        create_light_source_ambient(&mut tx, &model, schema, over).is_err(),
        "a normalised ratio above one is refused"
    );

    let under = LightSourceDraft {
        name: None,
        light_colour: c,
        ambient_intensity: None,
        intensity: Some(-0.1),
    };
    assert!(
        create_light_source_ambient(&mut tx, &model, schema, under).is_err(),
        "a negative intensity is refused"
    );

    let bad_radius = PointLight {
        position,
        radius: 0.0,
        attenuation: Attenuation::default(),
    };
    assert!(
        create_light_source_positional(&mut tx, &model, schema, draft(c), bad_radius).is_err(),
        "a zero radius is refused"
    );

    let good = PointLight {
        position,
        radius: 1.0,
        attenuation: Attenuation::default(),
    };
    let bad_cone = SpotCone {
        orientation,
        concentration_exponent: None,
        spread_angle: 0.0,
        beam_width_angle: 0.5,
    };
    assert!(
        create_light_source_spot(&mut tx, &model, schema, draft(c), good, bad_cone).is_err(),
        "a zero spread angle is refused"
    );
}

/// Lighting keeps its four colours in declared order, and refraction
/// accepts an entirely empty record.
#[test]
fn surface_style_lighting_and_refraction() {
    let schema = ifc4x3();
    let mut model = Model::new();
    let mut tx = Transaction::new(&model);

    // Four distinct colours so any transposition is visible.
    let mut mk = |r: f64| {
        create_colour_rgb(
            &mut tx,
            schema,
            ColourRgbDraft {
                name: None,
                red: r,
                green: 0.0,
                blue: 0.0,
            },
        )
        .expect("colour")
    };
    let (dt, dr, tr, rf) = (mk(0.1), mk(0.2), mk(0.3), mk(0.4));

    let lighting = create_surface_style_lighting(&mut tx, &model, schema, dt, dr, tr, rf)
        .expect("four colours are accepted");

    // Both attributes are optional: the schema permits an empty record.
    let empty = create_surface_style_refraction(&mut tx, schema, None, None)
        .expect("an empty refraction is legal");
    let full = create_surface_style_refraction(&mut tx, schema, Some(1.5), Some(0.02))
        .expect("a full refraction is accepted");

    assert!(
        create_surface_style_refraction(&mut tx, schema, Some(f64::NAN), None).is_err(),
        "a non-finite refraction index is refused"
    );

    tx.commit(&mut model).expect("commit");

    let e = model.get(lighting).expect("staged");
    assert_eq!(e.attributes[0], Value::Ref(dt), "DiffuseTransmissionColour");
    assert_eq!(e.attributes[1], Value::Ref(dr), "DiffuseReflectionColour");
    assert_eq!(e.attributes[2], Value::Ref(tr), "TransmissionColour");
    assert_eq!(e.attributes[3], Value::Ref(rf), "ReflectanceColour");

    let e = model.get(empty).expect("staged");
    assert_eq!(e.attributes[0], Value::Null, "an absent index stays null");

    let e = model.get(full).expect("staged");
    assert_eq!(e.attributes[0], Value::Real(1.5), "RefractionIndex");
    assert_eq!(e.attributes[1], Value::Real(0.02), "DispersionFactor");
}

/// Degenerate physical values are refused even when they are finite.
#[test]
fn degenerate_light_physics_is_refused() {
    let schema = ifc4x3();
    let model = Model::new();
    let mut tx = Transaction::new(&model);
    let c = colour(&mut tx, schema);
    let position = tx.create(Entity::new("IFCCARTESIANPOINT", vec![Value::Null]));
    let orientation = tx.create(Entity::new("IFCDIRECTION", vec![Value::Null]));

    // All three coefficients zero makes the falloff polynomial evaluate
    // to zero everywhere: a light that emits nothing at any distance.
    let dark = PointLight {
        position,
        radius: 1.0,
        attenuation: Attenuation {
            constant: 0.0,
            distance: 0.0,
            quadric: 0.0,
        },
    };
    assert!(
        create_light_source_positional(&mut tx, &model, schema, draft(c), dark).is_err(),
        "an all-zero attenuation emits nothing and is refused"
    );

    let good = PointLight {
        position,
        radius: 1.0,
        attenuation: Attenuation::default(),
    };
    let nan_cone = SpotCone {
        orientation,
        concentration_exponent: None,
        spread_angle: f64::NAN,
        beam_width_angle: 0.5,
    };
    assert!(
        create_light_source_spot(&mut tx, &model, schema, draft(c), good, nan_cone).is_err(),
        "a non-finite angle is refused, not just a non-positive one"
    );

    // Vacuum has index 1.0; nothing refracts less than that.
    assert!(
        create_surface_style_refraction(&mut tx, schema, Some(0.5), None).is_err(),
        "a refraction index below the vacuum index is refused"
    );
    assert!(
        create_surface_style_refraction(&mut tx, schema, Some(1.0), None).is_ok(),
        "the vacuum index itself is legal"
    );
}
