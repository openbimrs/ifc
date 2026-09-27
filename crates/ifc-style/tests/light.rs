//! `IfcLightSource` projections across IFC2x3, IFC4 ADD2 TC1, and IFC4X3 ADD2.
//!
//! Light sources are `IfcGeometricRepresentationItem` subtypes that carry no
//! shape, so `ifc-geometry` classifies them `non-shape` with `ifc-style` as
//! owner. These tests are what makes that ownership real: they prove the
//! attributes are actually readable, in every bundled schema, rather than
//! merely declared to be somebody's responsibility.

use ifc_model::{Codec, Entity, EntityId, Model, Value};
use ifc_schema::{ifc2x3, ifc4, ifc4x3, Schema};
use ifc_step::StepCodec;
use ifc_style::{LightSourceKind, StyleError, StyleView};
use std::sync::Arc;

fn named_entity(schema: &Schema, type_name: &str, fields: Vec<(&str, Value)>) -> Entity {
    let attributes = schema.attributes(type_name);
    assert!(
        !attributes.is_empty(),
        "{type_name} must exist in {}",
        schema.name()
    );
    let mut values = vec![Value::Null; attributes.len()];
    for (name, value) in fields {
        let index = attributes
            .iter()
            .position(|attribute| attribute.name.eq_ignore_ascii_case(name))
            .unwrap_or_else(|| panic!("{type_name}.{name} must exist in {}", schema.name()));
        values[index] = value;
    }
    Entity::new(type_name, values)
}

fn colour(model: &mut Model, schema: &Schema, rgb: [f64; 3]) -> EntityId {
    model.push(named_entity(
        schema,
        "IfcColourRgb",
        vec![
            ("Red", Value::Real(rgb[0])),
            ("Green", Value::Real(rgb[1])),
            ("Blue", Value::Real(rgb[2])),
        ],
    ))
}

fn point(model: &mut Model, schema: &Schema) -> EntityId {
    model.push(named_entity(
        schema,
        "IfcCartesianPoint",
        vec![(
            "Coordinates",
            Value::List(vec![Value::Real(1.0), Value::Real(2.0), Value::Real(3.0)]),
        )],
    ))
}

fn direction(model: &mut Model, schema: &Schema) -> EntityId {
    model.push(named_entity(
        schema,
        "IfcDirection",
        vec![(
            "DirectionRatios",
            Value::List(vec![Value::Real(0.0), Value::Real(0.0), Value::Real(-1.0)]),
        )],
    ))
}

/// The shared supertype attributes read identically in all three schemas.
///
/// `IfcLightSource` is abstract, so this exercises it through a concrete
/// subtype -- which is also the only way a real file can present one.
#[test]
fn shared_light_source_attributes_resolve_in_every_schema() {
    for schema in [ifc2x3(), ifc4(), ifc4x3()] {
        let mut model = Model::new();
        let light_colour = colour(&mut model, schema, [0.9, 0.8, 0.7]);
        let id = model.push(named_entity(
            schema,
            "IfcLightSourceAmbient",
            vec![
                ("Name", Value::Text(Arc::from("fill"))),
                ("LightColour", Value::Ref(light_colour)),
                ("AmbientIntensity", Value::Real(0.25)),
                ("Intensity", Value::Real(0.5)),
            ],
        ));
        let ambient = StyleView::new(&model, schema)
            .light_source_ambient(id)
            .unwrap();
        let source = ambient.light_source();
        assert_eq!(source.name().unwrap(), Some("fill"));
        assert_eq!(source.light_colour().unwrap(), light_colour);
        assert_eq!(source.ambient_intensity().unwrap(), Some(0.25));
        assert_eq!(source.intensity().unwrap(), Some(0.5));
        assert_eq!(source.kind().unwrap(), LightSourceKind::Ambient);
    }
}

/// An unauthored OPTIONAL intensity stays `None` instead of defaulting.
///
/// This is the difference between "the file said nothing" and "the file said
/// full brightness". A renderer may pick a default; the reader must not.
#[test]
fn absent_optional_intensity_is_none_not_a_default() {
    for schema in [ifc2x3(), ifc4(), ifc4x3()] {
        let mut model = Model::new();
        let light_colour = colour(&mut model, schema, [1.0, 1.0, 1.0]);
        let id = model.push(named_entity(
            schema,
            "IfcLightSourceAmbient",
            vec![("LightColour", Value::Ref(light_colour))],
        ));
        let source = StyleView::new(&model, schema)
            .light_source_ambient(id)
            .unwrap();
        let source = source.light_source();
        assert_eq!(source.name().unwrap(), None);
        assert_eq!(source.ambient_intensity().unwrap(), None);
        assert_eq!(source.intensity().unwrap(), None);
    }
}

/// A spot light reports both its own cone attributes and the positional
/// attenuation it inherits, without re-reading slots by index.
#[test]
fn spot_light_exposes_cone_and_inherited_attenuation() {
    for schema in [ifc2x3(), ifc4(), ifc4x3()] {
        let mut model = Model::new();
        let light_colour = colour(&mut model, schema, [1.0, 0.95, 0.8]);
        let position = point(&mut model, schema);
        let orientation = direction(&mut model, schema);
        let id = model.push(named_entity(
            schema,
            "IfcLightSourceSpot",
            vec![
                ("LightColour", Value::Ref(light_colour)),
                ("Position", Value::Ref(position)),
                ("Radius", Value::Real(0.15)),
                ("ConstantAttenuation", Value::Real(1.0)),
                ("DistanceAttenuation", Value::Real(0.0)),
                ("QuadricAttenuation", Value::Real(0.02)),
                ("Orientation", Value::Ref(orientation)),
                ("ConcentrationExponent", Value::Real(4.0)),
                ("SpreadAngle", Value::Real(0.6)),
                ("BeamWidthAngle", Value::Real(0.3)),
            ],
        ));
        let view = StyleView::new(&model, schema);
        let spot = view.light_source_spot(id).unwrap();
        assert_eq!(spot.orientation().unwrap(), orientation);
        assert_eq!(spot.concentration_exponent().unwrap(), Some(4.0));
        assert_eq!(spot.spread_angle().unwrap(), 0.6);
        assert_eq!(spot.beam_width_angle().unwrap(), 0.3);

        let positional = spot.positional();
        assert_eq!(positional.position().unwrap(), position);
        assert_eq!(positional.radius().unwrap(), 0.15);
        assert_eq!(positional.constant_attenuation().unwrap(), 1.0);
        assert_eq!(positional.distance_attenuation().unwrap(), 0.0);
        assert_eq!(positional.quadric_attenuation().unwrap(), 0.02);
        assert_eq!(spot.light_source().light_colour().unwrap(), light_colour);
        assert_eq!(spot.light_source().kind().unwrap(), LightSourceKind::Spot);

        // A spot IS a positional light, so the positional projection must
        // accept it rather than rejecting the subtype.
        assert_eq!(
            view.light_source_positional(id).unwrap().radius().unwrap(),
            0.15
        );
    }
}

/// `kind()` resolves through inheritance and never reports a supertype when a
/// more specific subtype is declared.
#[test]
fn light_source_kind_prefers_the_most_specific_subtype() {
    for schema in [ifc2x3(), ifc4(), ifc4x3()] {
        for (type_name, expected) in [
            ("IfcLightSourceAmbient", LightSourceKind::Ambient),
            ("IfcLightSourceDirectional", LightSourceKind::Directional),
            ("IfcLightSourcePositional", LightSourceKind::Positional),
            ("IfcLightSourceSpot", LightSourceKind::Spot),
            ("IfcLightSourceGoniometric", LightSourceKind::Goniometric),
        ] {
            let mut model = Model::new();
            let light_colour = colour(&mut model, schema, [1.0, 1.0, 1.0]);
            let id = model.push(named_entity(
                schema,
                type_name,
                vec![("LightColour", Value::Ref(light_colour))],
            ));
            assert_eq!(
                StyleView::new(&model, schema)
                    .light_source(id)
                    .unwrap()
                    .kind()
                    .unwrap(),
                expected,
                "{type_name} in {}",
                schema.name()
            );
        }
    }
}

/// The goniometric light's photometric attributes and validated select.
#[test]
fn goniometric_light_reports_photometry_and_validates_its_select() {
    for schema in [ifc2x3(), ifc4(), ifc4x3()] {
        let mut model = Model::new();
        let light_colour = colour(&mut model, schema, [1.0, 1.0, 1.0]);
        let appearance = colour(&mut model, schema, [0.98, 0.96, 0.9]);
        let origin = point(&mut model, schema);
        let placement = model.push(named_entity(
            schema,
            "IfcAxis2Placement3D",
            vec![("Location", Value::Ref(origin))],
        ));
        let data = model.push(named_entity(
            schema,
            "IfcLightDistributionData",
            vec![
                ("MainPlaneAngle", Value::Real(0.0)),
                (
                    "SecondaryPlaneAngle",
                    Value::List(vec![Value::Real(0.0), Value::Real(0.5)]),
                ),
                (
                    "LuminousIntensity",
                    Value::List(vec![Value::Real(120.0), Value::Real(80.0)]),
                ),
            ],
        ));
        let distribution = model.push(named_entity(
            schema,
            "IfcLightIntensityDistribution",
            vec![
                ("LightDistributionCurve", Value::Enum(Arc::from("TYPE_C"))),
                ("DistributionData", Value::List(vec![Value::Ref(data)])),
            ],
        ));
        let id = model.push(named_entity(
            schema,
            "IfcLightSourceGoniometric",
            vec![
                ("LightColour", Value::Ref(light_colour)),
                ("Position", Value::Ref(placement)),
                ("ColourAppearance", Value::Ref(appearance)),
                ("ColourTemperature", Value::Real(4000.0)),
                ("LuminousFlux", Value::Real(3200.0)),
                (
                    "LightEmissionSource",
                    Value::Enum(Arc::from("LIGHTEMITTINGDIODE")),
                ),
                ("LightDistributionDataSource", Value::Ref(distribution)),
            ],
        ));
        let view = StyleView::new(&model, schema);
        let light = view.light_source_goniometric(id).unwrap();
        assert_eq!(light.position().unwrap(), placement);
        assert_eq!(light.colour_appearance().unwrap(), Some(appearance));
        assert_eq!(light.colour_temperature().unwrap(), 4000.0);
        assert_eq!(light.luminous_flux().unwrap(), 3200.0);
        assert_eq!(light.light_emission_source().unwrap(), "LIGHTEMITTINGDIODE");
        assert_eq!(
            light.light_distribution_data_source().unwrap(),
            distribution
        );

        let distribution = view.light_intensity_distribution(distribution).unwrap();
        assert_eq!(distribution.light_distribution_curve().unwrap(), "TYPE_C");
        assert_eq!(distribution.distribution_data().unwrap(), vec![data]);

        let data = view.light_distribution_data(data).unwrap();
        assert_eq!(data.main_plane_angle().unwrap(), 0.0);
        assert_eq!(data.samples().unwrap(), vec![(0.0, 120.0), (0.5, 80.0)]);
    }
}

/// Parallel photometric lists of unequal length are a typed error, not a
/// silently truncated zip.
#[test]
fn mismatched_distribution_lists_are_refused() {
    let schema = ifc4();
    let mut model = Model::new();
    let id = model.push(named_entity(
        schema,
        "IfcLightDistributionData",
        vec![
            ("MainPlaneAngle", Value::Real(0.0)),
            (
                "SecondaryPlaneAngle",
                Value::List(vec![Value::Real(0.0), Value::Real(0.5)]),
            ),
            ("LuminousIntensity", Value::List(vec![Value::Real(120.0)])),
        ],
    ));
    let data = StyleView::new(&model, schema)
        .light_distribution_data(id)
        .unwrap();
    // Each list is individually well-formed...
    assert_eq!(data.secondary_plane_angles().unwrap().len(), 2);
    assert_eq!(data.luminous_intensities().unwrap().len(), 1);
    // ...but they cannot be paired.
    let error = data.samples().expect_err("unequal lists cannot pair");
    assert!(
        matches!(error, StyleError::InvalidValue { attribute, .. } if attribute == "SecondaryPlaneAngle"),
        "unexpected error: {error}"
    );
}

/// A `LIST [1:?]` that arrives empty is refused rather than yielding an empty
/// iterator that reads as a valid unlit luminaire.
#[test]
fn empty_mandatory_distribution_lists_are_refused() {
    let schema = ifc4();
    let mut model = Model::new();
    let data = model.push(named_entity(
        schema,
        "IfcLightDistributionData",
        vec![
            ("MainPlaneAngle", Value::Real(0.0)),
            ("SecondaryPlaneAngle", Value::List(Vec::new())),
            ("LuminousIntensity", Value::List(Vec::new())),
        ],
    ));
    let distribution = model.push(named_entity(
        schema,
        "IfcLightIntensityDistribution",
        vec![
            ("LightDistributionCurve", Value::Enum(Arc::from("TYPE_C"))),
            ("DistributionData", Value::List(Vec::new())),
        ],
    ));
    let view = StyleView::new(&model, schema);
    view.light_distribution_data(data)
        .unwrap()
        .secondary_plane_angles()
        .expect_err("LIST [1:?] must reject an empty list");
    view.light_intensity_distribution(distribution)
        .unwrap()
        .distribution_data()
        .expect_err("LIST [1:?] must reject an empty list");
}

/// Measures typed positive in EXPRESS reject zero and negative values.
#[test]
fn non_positive_positive_measures_are_refused() {
    let schema = ifc4();
    let mut model = Model::new();
    let light_colour = colour(&mut model, schema, [1.0, 1.0, 1.0]);
    let position = point(&mut model, schema);
    let orientation = direction(&mut model, schema);
    let id = model.push(named_entity(
        schema,
        "IfcLightSourceSpot",
        vec![
            ("LightColour", Value::Ref(light_colour)),
            ("Position", Value::Ref(position)),
            ("Radius", Value::Real(0.0)),
            ("ConstantAttenuation", Value::Real(1.0)),
            ("DistanceAttenuation", Value::Real(0.0)),
            ("QuadricAttenuation", Value::Real(0.0)),
            ("Orientation", Value::Ref(orientation)),
            ("SpreadAngle", Value::Real(-0.2)),
            ("BeamWidthAngle", Value::Real(0.3)),
        ],
    ));
    let spot = StyleView::new(&model, schema)
        .light_source_spot(id)
        .unwrap();
    // Attenuation coefficients are plain IfcReal: zero is legitimate there.
    assert_eq!(spot.positional().distance_attenuation().unwrap(), 0.0);
    // Radius and SpreadAngle are not.
    assert!(matches!(
        spot.positional().radius(),
        Err(StyleError::OutOfRange {
            entity: "IfcPositiveLengthMeasure",
            ..
        })
    ));
    assert!(matches!(
        spot.spread_angle(),
        Err(StyleError::OutOfRange {
            entity: "IfcPositivePlaneAngleMeasure",
            ..
        })
    ));
}

/// A reference whose target is outside the select is a typed refusal.
#[test]
fn distribution_data_source_select_membership_is_enforced() {
    let schema = ifc4();
    let mut model = Model::new();
    let light_colour = colour(&mut model, schema, [1.0, 1.0, 1.0]);
    let origin = point(&mut model, schema);
    let placement = model.push(named_entity(
        schema,
        "IfcAxis2Placement3D",
        vec![("Location", Value::Ref(origin))],
    ));
    let id = model.push(named_entity(
        schema,
        "IfcLightSourceGoniometric",
        vec![
            ("LightColour", Value::Ref(light_colour)),
            ("Position", Value::Ref(placement)),
            ("ColourTemperature", Value::Real(4000.0)),
            ("LuminousFlux", Value::Real(3200.0)),
            ("LightEmissionSource", Value::Enum(Arc::from("FLUORESCENT"))),
            // An IfcColourRgb is in neither select member.
            ("LightDistributionDataSource", Value::Ref(light_colour)),
        ],
    ));
    let error = StyleView::new(&model, schema)
        .light_source_goniometric(id)
        .unwrap()
        .light_distribution_data_source()
        .expect_err("a non-member reference must be refused");
    assert!(
        matches!(
            error,
            StyleError::ReferenceType {
                expected: "IfcLightDistributionDataSourceSelect",
                ..
            }
        ),
        "unexpected error: {error}"
    );
}

/// Reading a light as the wrong subtype is refused, so a caller cannot aim an
/// ambient light by accident.
#[test]
fn projecting_a_light_as_the_wrong_subtype_is_refused() {
    let schema = ifc4();
    let mut model = Model::new();
    let light_colour = colour(&mut model, schema, [1.0, 1.0, 1.0]);
    let id = model.push(named_entity(
        schema,
        "IfcLightSourceAmbient",
        vec![("LightColour", Value::Ref(light_colour))],
    ));
    let view = StyleView::new(&model, schema);
    assert!(matches!(
        view.light_source_spot(id),
        Err(StyleError::WrongEntityType {
            expected: "IfcLightSourceSpot",
            ..
        })
    ));
    assert!(matches!(
        view.light_source_directional(id),
        Err(StyleError::WrongEntityType {
            expected: "IfcLightSourceDirectional",
            ..
        })
    ));
    // The supertype projection still accepts it.
    assert!(view.light_source(id).is_ok());
}

/// A luminaire graph parsed from real IFC4 STEP text, not a hand-built model.
///
/// The hand-assembled models above prove slot resolution; this proves the
/// same graph survives the codec that real files arrive through, including
/// the `IfcLightDistributionDataSourceSelect` reference and the nested
/// photometric rows.
#[test]
fn goniometric_luminaire_survives_a_real_step_roundtrip() {
    const SOURCE: &[u8] = br#"ISO-10303-21;
HEADER;
FILE_DESCRIPTION(('Light source contract'),'2;1');
FILE_NAME('light.ifc','2026-09-14T00:00:00',(''),(''),'openbim','openbim','');
FILE_SCHEMA(('IFC4'));
ENDSEC;
DATA;
#1=IFCCOLOURRGB('warm',1.,0.9,0.8);
#2=IFCCARTESIANPOINT((0.,0.,3.));
#3=IFCDIRECTION((0.,0.,-1.));
#4=IFCDIRECTION((1.,0.,0.));
#5=IFCAXIS2PLACEMENT3D(#2,#3,#4);
#6=IFCLIGHTDISTRIBUTIONDATA(0.,(0.,0.7853981634,1.5707963268),(1000.,600.,120.));
#7=IFCLIGHTINTENSITYDISTRIBUTION(.TYPE_C.,(#6));
#8=IFCLIGHTSOURCEGONIOMETRIC('luminaire',#1,$,0.85,#5,$,3000.,4200.,.LIGHTEMITTINGDIODE.,#7);
#9=IFCLIGHTSOURCESPOT('spot',#1,0.1,0.9,#2,0.5,1.,0.,0.,#3,2.,0.6981317008,0.3490658504);
ENDSEC;
END-ISO-10303-21;
"#;

    // The angles below are the decimals authored in SOURCE, not math
    // constants: the assertions check the codec reproduces the file's own
    // digits, so substituting `FRAC_PI_4`/`FRAC_PI_2` would compare against a
    // value the file never contained.
    #[allow(clippy::approx_constant)]
    const MID_ANGLE: f64 = 0.7853981634;
    #[allow(clippy::approx_constant)]
    const EDGE_ANGLE: f64 = 1.5707963268;

    let codec = StepCodec;
    let model = codec.read_bytes(SOURCE).unwrap();

    let assert_contract = |model: &Model| {
        let view = StyleView::new(model, ifc4());

        let goniometric_id = model.of_type("IFCLIGHTSOURCEGONIOMETRIC").next().unwrap().0;
        let goniometric = view.light_source_goniometric(goniometric_id).unwrap();
        assert_eq!(
            goniometric.light_source().name().unwrap(),
            Some("luminaire")
        );
        assert_eq!(goniometric.light_source().intensity().unwrap(), Some(0.85));
        assert_eq!(goniometric.colour_temperature().unwrap(), 3000.0);
        assert_eq!(goniometric.luminous_flux().unwrap(), 4200.0);
        assert_eq!(
            goniometric.light_emission_source().unwrap(),
            "LIGHTEMITTINGDIODE"
        );
        assert_eq!(
            goniometric.light_source().kind().unwrap(),
            LightSourceKind::Goniometric
        );

        // The select resolves to the distribution, whose rows carry photometry.
        let distribution_id = goniometric.light_distribution_data_source().unwrap();
        let distribution = view.light_intensity_distribution(distribution_id).unwrap();
        assert_eq!(distribution.light_distribution_curve().unwrap(), "TYPE_C");
        let rows = distribution.distribution_data().unwrap();
        assert_eq!(rows.len(), 1);
        let row = view.light_distribution_data(rows[0]).unwrap();
        assert_eq!(row.main_plane_angle().unwrap(), 0.0);
        assert_eq!(
            row.samples().unwrap(),
            vec![(0.0, 1000.0), (MID_ANGLE, 600.0), (EDGE_ANGLE, 120.0)]
        );

        // A spot inherits positional attenuation and adds its own cone.
        let spot_id = model.of_type("IFCLIGHTSOURCESPOT").next().unwrap().0;
        let spot = view.light_source_spot(spot_id).unwrap();
        assert_eq!(spot.positional().radius().unwrap(), 0.5);
        assert_eq!(spot.positional().constant_attenuation().unwrap(), 1.0);
        assert_eq!(spot.concentration_exponent().unwrap(), Some(2.0));
        assert_eq!(spot.spread_angle().unwrap(), 0.6981317008);
        assert_eq!(spot.beam_width_angle().unwrap(), 0.3490658504);
        assert_eq!(
            spot.positional().light_source().kind().unwrap(),
            LightSourceKind::Spot
        );
    };

    assert_contract(&model);

    let bytes = codec.write_bytes(&model).unwrap();
    let reparsed = codec.read_bytes(&bytes).unwrap();
    assert_eq!(model.len(), reparsed.len());
    for (id, entity) in model.iter() {
        let other = reparsed.get(id).expect("entity must survive roundtrip");
        assert_eq!(entity.type_name, other.type_name);
        assert_eq!(entity.attributes, other.attributes);
    }
    assert_contract(&reparsed);
}
