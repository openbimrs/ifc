//! Light sources and the remaining surface-style elements.
//!
//! # Normalised ratios are bounded, and the bound is load-bearing
//!
//! `AmbientIntensity`, `Intensity` and `Transparency` are all
//! `IfcNormalisedRatioMeasure`, which IFC defines as 0.0 to 1.0
//! inclusive. A renderer handed 2.5 either clamps it or blows out the
//! image, and neither is what the file said. The existing
//! [`super::validate_ratio`] helper already enforces this for
//! shading, so the light sources reuse it rather than restating the
//! bound.
//!
//! # A spot light's two angles are not interchangeable
//!
//! `SpreadAngle` is the cone the light fills; `BeamWidthAngle` is the
//! cone at full intensity. Both are `IfcPositivePlaneAngleMeasure`, so
//! neither may be zero, and the schema does not require one to be
//! larger than the other -- some exporters write a beam wider than the
//! spread to mean "no falloff". Refusing that would reject valid
//! files, so the writer checks positivity and leaves the relationship
//! alone.

use ifc_model::{EntityId, Model, Transaction, Value};
use ifc_schema::Schema;

use crate::authoring::{build_named, invalid_authoring, validate_ratio, validate_ref};
use crate::StyleResult;

/// The four attributes every `IfcLightSource` carries.
#[derive(Debug, Clone, Copy)]
pub struct LightSourceDraft<'a> {
    /// `Name`.
    pub name: Option<&'a str>,
    /// `LightColour`: an `IfcColourRgb`. Required.
    pub light_colour: EntityId,
    /// `AmbientIntensity`, a normalised ratio.
    pub ambient_intensity: Option<f64>,
    /// `Intensity`, a normalised ratio.
    pub intensity: Option<f64>,
}

/// Attenuation over distance, as three coefficients.
///
/// The renderer divides by `constant + distance * d + quadric * d^2`.
/// All three being zero makes that divisor zero, so the combination is
/// refused.
#[derive(Debug, Clone, Copy)]
pub struct Attenuation {
    /// `ConstantAttenuation`.
    pub constant: f64,
    /// `DistanceAttenuation`.
    pub distance: f64,
    /// `QuadricAttenuation`.
    pub quadric: f64,
}

fn check_light(entity: &'static str, draft: &LightSourceDraft<'_>) -> StyleResult<()> {
    if let Some(value) = draft.ambient_intensity {
        validate_ratio(entity, "AmbientIntensity", value)?;
    }
    if let Some(value) = draft.intensity {
        validate_ratio(entity, "Intensity", value)?;
    }
    Ok(())
}

fn light_values<'a>(draft: &LightSourceDraft<'a>) -> Vec<(&'static str, Value)> {
    let mut values = vec![("LightColour", Value::Ref(draft.light_colour))];
    if let Some(name) = draft.name {
        values.push(("Name", Value::Text(name.into())));
    }
    if let Some(value) = draft.ambient_intensity {
        values.push(("AmbientIntensity", Value::Real(value)));
    }
    if let Some(value) = draft.intensity {
        values.push(("Intensity", Value::Real(value)));
    }
    values
}

fn check_attenuation(entity: &'static str, attenuation: &Attenuation) -> StyleResult<()> {
    let all = [
        ("ConstantAttenuation", attenuation.constant),
        ("DistanceAttenuation", attenuation.distance),
        ("QuadricAttenuation", attenuation.quadric),
    ];
    for (attribute, value) in all {
        if !value.is_finite() {
            return Err(invalid_authoring(
                entity,
                attribute,
                format!("expected a finite coefficient, got {value}"),
            ));
        }
    }
    if all.iter().all(|(_, value)| *value == 0.0) {
        return Err(invalid_authoring(
            entity,
            "ConstantAttenuation",
            "all three coefficients zero leaves no attenuation function",
        ));
    }
    Ok(())
}

/// Stage an `IfcLightSourceAmbient`: light with no position or direction.
///
/// # Errors
///
/// Refuses an out-of-range normalised ratio and a `light_colour` that
/// does not resolve to an `IfcColourRgb`.
pub fn create_light_source_ambient(
    tx: &mut Transaction,
    model: &Model,
    schema: &Schema,
    draft: LightSourceDraft<'_>,
) -> StyleResult<EntityId> {
    check_light("IfcLightSourceAmbient", &draft)?;
    validate_ref(tx, model, schema, draft.light_colour, "IfcColourRgb")?;
    Ok(tx.create(build_named(
        schema,
        "IfcLightSourceAmbient",
        light_values(&draft),
    )?))
}

/// Stage an `IfcLightSourceDirectional`: parallel rays, no position.
///
/// # Errors
///
/// Refuses an out-of-range normalised ratio, and references that do
/// not resolve to their declared types.
pub fn create_light_source_directional(
    tx: &mut Transaction,
    model: &Model,
    schema: &Schema,
    draft: LightSourceDraft<'_>,
    orientation: EntityId,
) -> StyleResult<EntityId> {
    check_light("IfcLightSourceDirectional", &draft)?;
    validate_ref(tx, model, schema, draft.light_colour, "IfcColourRgb")?;
    validate_ref(tx, model, schema, orientation, "IfcDirection")?;

    let mut values = light_values(&draft);
    values.push(("Orientation", Value::Ref(orientation)));
    Ok(tx.create(build_named(schema, "IfcLightSourceDirectional", values)?))
}

/// Stage an `IfcLightSourcePositional`: a point light with falloff.
///
/// # Errors
///
/// Refuses an out-of-range normalised ratio, a non-positive radius,
/// a non-finite or all-zero attenuation, and references that do not
/// resolve to their declared types.
pub fn create_light_source_positional(
    tx: &mut Transaction,
    model: &Model,
    schema: &Schema,
    draft: LightSourceDraft<'_>,
    point: PointLight,
) -> StyleResult<EntityId> {
    let PointLight {
        position,
        radius,
        attenuation,
    } = point;
    check_light("IfcLightSourcePositional", &draft)?;
    if !radius.is_finite() || radius <= 0.0 {
        return Err(invalid_authoring(
            "IfcLightSourcePositional",
            "Radius",
            format!("expected a positive length, got {radius}"),
        ));
    }
    check_attenuation("IfcLightSourcePositional", &attenuation)?;
    validate_ref(tx, model, schema, draft.light_colour, "IfcColourRgb")?;
    validate_ref(tx, model, schema, position, "IfcCartesianPoint")?;

    let mut values = light_values(&draft);
    values.extend([
        ("Position", Value::Ref(position)),
        ("Radius", Value::Real(radius)),
        ("ConstantAttenuation", Value::Real(attenuation.constant)),
        ("DistanceAttenuation", Value::Real(attenuation.distance)),
        ("QuadricAttenuation", Value::Real(attenuation.quadric)),
    ]);
    Ok(tx.create(build_named(schema, "IfcLightSourcePositional", values)?))
}

/// Where a light sits and how it falls off.
///
/// Position, radius and attenuation are not independent knobs: they
/// together describe one point emitter, and every entity carrying one
/// carries all three. Grouping them keeps the spot-light signature
/// honest rather than a seven-argument list where two adjacent `f64`
/// parameters invite transposition.
#[derive(Debug, Clone, Copy)]
pub struct PointLight {
    /// `Position`: an `IfcCartesianPoint`.
    pub position: EntityId,
    /// `Radius`: a positive length.
    pub radius: f64,
    /// The three attenuation coefficients.
    pub attenuation: Attenuation,
}

impl Default for Attenuation {
    /// No falloff: intensity is constant with distance.
    ///
    /// Deriving `Default` would give all three zeros, which makes the
    /// attenuation polynomial evaluate to zero at every distance: a
    /// light that emits nothing. The neutral value is constant = 1.
    fn default() -> Self {
        Self {
            constant: 1.0,
            distance: 0.0,
            quadric: 0.0,
        }
    }
}

/// The cone geometry of a spot light.
#[derive(Debug, Clone, Copy)]
pub struct SpotCone {
    /// `Orientation`: an `IfcDirection`.
    pub orientation: EntityId,
    /// `SpreadAngle`: the full cone, a positive plane angle.
    pub spread_angle: f64,
    /// `BeamWidthAngle`: the full-intensity cone, a positive angle.
    pub beam_width_angle: f64,
    /// `ConcentrationExponent`: falloff sharpness.
    pub concentration_exponent: Option<f64>,
}

/// Stage an `IfcLightSourceSpot`.
///
/// # Errors
///
/// Everything [`create_light_source_positional`] refuses, plus a
/// non-positive spread or beam width angle.
pub fn create_light_source_spot(
    tx: &mut Transaction,
    model: &Model,
    schema: &Schema,
    draft: LightSourceDraft<'_>,
    point: PointLight,
    cone: SpotCone,
) -> StyleResult<EntityId> {
    let PointLight {
        position,
        radius,
        attenuation,
    } = point;
    check_light("IfcLightSourceSpot", &draft)?;
    if !radius.is_finite() || radius <= 0.0 {
        return Err(invalid_authoring(
            "IfcLightSourceSpot",
            "Radius",
            format!("expected a positive length, got {radius}"),
        ));
    }
    check_attenuation("IfcLightSourceSpot", &attenuation)?;

    for (attribute, value) in [
        ("SpreadAngle", cone.spread_angle),
        ("BeamWidthAngle", cone.beam_width_angle),
    ] {
        if !value.is_finite() || value <= 0.0 {
            return Err(invalid_authoring(
                "IfcLightSourceSpot",
                attribute,
                format!("expected a positive plane angle, got {value}"),
            ));
        }
    }

    validate_ref(tx, model, schema, draft.light_colour, "IfcColourRgb")?;
    validate_ref(tx, model, schema, position, "IfcCartesianPoint")?;
    validate_ref(tx, model, schema, cone.orientation, "IfcDirection")?;

    let mut values = light_values(&draft);
    values.extend([
        ("Position", Value::Ref(position)),
        ("Radius", Value::Real(radius)),
        ("ConstantAttenuation", Value::Real(attenuation.constant)),
        ("DistanceAttenuation", Value::Real(attenuation.distance)),
        ("QuadricAttenuation", Value::Real(attenuation.quadric)),
        ("Orientation", Value::Ref(cone.orientation)),
        ("SpreadAngle", Value::Real(cone.spread_angle)),
        ("BeamWidthAngle", Value::Real(cone.beam_width_angle)),
    ]);
    if let Some(value) = cone.concentration_exponent {
        values.push(("ConcentrationExponent", Value::Real(value)));
    }
    Ok(tx.create(build_named(schema, "IfcLightSourceSpot", values)?))
}

/// Stage an `IfcSurfaceStyleLighting`: the four physical colour terms.
///
/// # Errors
///
/// Refuses any colour that does not resolve to an `IfcColourRgb`.
pub fn create_surface_style_lighting(
    tx: &mut Transaction,
    model: &Model,
    schema: &Schema,
    diffuse_transmission: EntityId,
    diffuse_reflection: EntityId,
    transmission: EntityId,
    reflectance: EntityId,
) -> StyleResult<EntityId> {
    for colour in [
        diffuse_transmission,
        diffuse_reflection,
        transmission,
        reflectance,
    ] {
        validate_ref(tx, model, schema, colour, "IfcColourRgb")?;
    }

    let values = vec![
        (
            "DiffuseTransmissionColour",
            Value::Ref(diffuse_transmission),
        ),
        ("DiffuseReflectionColour", Value::Ref(diffuse_reflection)),
        ("TransmissionColour", Value::Ref(transmission)),
        ("ReflectanceColour", Value::Ref(reflectance)),
    ];
    Ok(tx.create(build_named(schema, "IfcSurfaceStyleLighting", values)?))
}

/// Stage an `IfcSurfaceStyleRefraction`.
///
/// # Errors
///
/// Refuses a refraction index below 1.0: no physical medium refracts
/// less than a vacuum, and a value below one inverts the bend.
pub fn create_surface_style_refraction(
    tx: &mut Transaction,
    schema: &Schema,
    refraction_index: Option<f64>,
    dispersion_factor: Option<f64>,
) -> StyleResult<EntityId> {
    if let Some(value) = refraction_index {
        if !value.is_finite() || value < 1.0 {
            return Err(invalid_authoring(
                "IfcSurfaceStyleRefraction",
                "RefractionIndex",
                format!("expected at least 1.0, the vacuum index, got {value}"),
            ));
        }
    }
    if let Some(value) = dispersion_factor {
        if !value.is_finite() {
            return Err(invalid_authoring(
                "IfcSurfaceStyleRefraction",
                "DispersionFactor",
                format!("expected a finite factor, got {value}"),
            ));
        }
    }

    let mut values = Vec::new();
    if let Some(value) = refraction_index {
        values.push(("RefractionIndex", Value::Real(value)));
    }
    if let Some(value) = dispersion_factor {
        values.push(("DispersionFactor", Value::Real(value)));
    }
    Ok(tx.create(build_named(schema, "IfcSurfaceStyleRefraction", values)?))
}

/// The goniometric-only attributes of an `IfcLightSourceGoniometric`.
#[derive(Debug, Clone, Copy)]
pub struct GoniometricLight {
    /// `Position`, an `IfcAxis2Placement3D`.
    pub position: EntityId,
    /// `ColourAppearance`, an `IfcColourRgb`.
    pub colour_appearance: Option<EntityId>,
    /// `ColourTemperature`, in kelvin.
    pub colour_temperature: f64,
    /// `LuminousFlux`, in lumen.
    pub luminous_flux: f64,
    /// `LightEmissionSource`, an `IfcLightEmissionSourceEnum` token.
    pub emission_source: &'static str,
    /// `LightDistributionDataSource`: an `IfcLightIntensityDistribution`
    /// or an `IfcExternalReference`.
    pub distribution_data_source: EntityId,
}

/// Stage an `IfcLightSourceGoniometric`.
///
/// A photometrically defined luminaire: rather than a radius or a
/// cone, it carries a measured intensity distribution, which is what a
/// lighting calculation actually needs.
///
/// # Errors
///
/// Refuses an out-of-range intensity, a colour reference that is not an
/// `IfcColourRgb`, a non-positive colour temperature or luminous flux
/// (both are absolute physical measures), and an emission-source token
/// the schema does not declare.
pub fn create_light_source_goniometric(
    tx: &mut Transaction,
    model: &Model,
    schema: &Schema,
    draft: LightSourceDraft<'_>,
    light: GoniometricLight,
) -> StyleResult<EntityId> {
    const ENTITY: &str = "IfcLightSourceGoniometric";
    check_light(ENTITY, &draft)?;
    validate_ref(tx, model, schema, draft.light_colour, "IfcColourRgb")?;
    if let Some(colour) = light.colour_appearance {
        validate_ref(tx, model, schema, colour, "IfcColourRgb")?;
    }
    for (value, attribute) in [
        (light.colour_temperature, "ColourTemperature"),
        (light.luminous_flux, "LuminousFlux"),
    ] {
        // Both are absolute measures: zero kelvin or zero lumen is not
        // a dim light, it is a light that cannot exist.
        if !value.is_finite() || value <= 0.0 {
            return Err(invalid_authoring(
                ENTITY,
                attribute,
                format!("expected a positive measure, got {value}"),
            ));
        }
    }
    let declared = schema
        .attributes(ENTITY)
        .iter()
        .find(|a| a.name.eq_ignore_ascii_case("LightEmissionSource"))
        .and_then(|a| schema.type_def(&a.type_name))
        .is_some_and(|def| match &def.kind {
            ifc_schema::TypeKind::Enumeration(values) => values
                .iter()
                .any(|v| v.eq_ignore_ascii_case(light.emission_source)),
            _ => false,
        });
    if !declared {
        return Err(invalid_authoring(
            ENTITY,
            "LightEmissionSource",
            light.emission_source,
        ));
    }

    let mut values = light_values(&draft);
    values.push(("Position", Value::Ref(light.position)));
    if let Some(colour) = light.colour_appearance {
        values.push(("ColourAppearance", Value::Ref(colour)));
    }
    values.push(("ColourTemperature", Value::Real(light.colour_temperature)));
    values.push(("LuminousFlux", Value::Real(light.luminous_flux)));
    values.push((
        "LightEmissionSource",
        Value::Enum(light.emission_source.into()),
    ));
    values.push((
        "LightDistributionDataSource",
        Value::Ref(light.distribution_data_source),
    ));
    Ok(tx.create(build_named(schema, ENTITY, values)?))
}
