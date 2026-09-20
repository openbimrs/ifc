//! Textures: the image source, its coordinates, and generated maps.
//!
//! # Why a texture needs a face to mean anything
//!
//! `IfcTextureMap` carries `Vertices : LIST [3:?] OF IfcTextureVertex`
//! and `MappedTo : IfcFace`. The vertex list is a correspondence: the
//! nth texture vertex belongs to the nth vertex of the face's bound.
//! A list shorter than the face's loop leaves vertices unmapped, and a
//! longer one carries coordinates for vertices that do not exist.
//!
//! The writer cannot check that correspondence without walking the
//! face's loop, which is geometry this crate deliberately does not
//! depend on -- `ifc-style` may reference a representation item's id
//! but never inspects its shape. So the cardinality floor of three is
//! enforced here and the correspondence is left to the consumer that
//! owns the geometry. Claiming otherwise would be a writer asserting
//! something it cannot see.

use ifc_model::{EntityId, Model, Transaction, Value};
use ifc_schema::Schema;

use crate::authoring::{build_named, invalid_authoring, validate_ref};
use crate::StyleResult;

/// What to stage for an `IfcImageTexture`.
#[derive(Debug, Clone, Copy)]
pub struct ImageTextureDraft<'a> {
    /// `RepeatS`: does the texture tile along the s axis?
    pub repeat_s: bool,
    /// `RepeatT`: does the texture tile along the t axis?
    pub repeat_t: bool,
    /// `Mode`: how the texture combines with the underlying colour.
    pub mode: Option<&'a str>,
    /// `TextureTransform`: an `IfcCartesianTransformationOperator2D`.
    pub texture_transform: Option<EntityId>,
    /// `URLReference`: where the image lives.
    pub url_reference: &'a str,
}

/// Stage an `IfcImageTexture`.
///
/// # The URL is required and must not be blank
///
/// `URLReference` is the only attribute that says what the texture
/// actually is. An empty string satisfies the schema's type but names
/// no image, producing a texture that resolves to nothing -- so it is
/// refused rather than written.
///
/// # Errors
///
/// Refuses a blank `URLReference`, and a `TextureTransform` that does
/// not resolve to an `IfcCartesianTransformationOperator2D`.
pub fn create_image_texture(
    tx: &mut Transaction,
    model: &Model,
    schema: &Schema,
    draft: ImageTextureDraft<'_>,
) -> StyleResult<EntityId> {
    if draft.url_reference.trim().is_empty() {
        return Err(invalid_authoring(
            "IfcImageTexture",
            "URLReference",
            "expected a reference to an image, found nothing",
        ));
    }

    if let Some(transform) = draft.texture_transform {
        validate_ref(
            tx,
            model,
            schema,
            transform,
            "IfcCartesianTransformationOperator2D",
        )?;
    }

    let mut values = vec![
        ("RepeatS", Value::Bool(draft.repeat_s)),
        ("RepeatT", Value::Bool(draft.repeat_t)),
        ("URLReference", Value::Text(draft.url_reference.into())),
    ];
    if let Some(mode) = draft.mode {
        values.push(("Mode", Value::Text(mode.into())));
    }
    if let Some(transform) = draft.texture_transform {
        values.push(("TextureTransform", Value::Ref(transform)));
    }

    Ok(tx.create(build_named(schema, "IfcImageTexture", values)?))
}

/// Stage an `IfcTextureMap`: explicit texture coordinates on a face.
///
/// # Errors
///
/// Refuses fewer than three vertices, since a mapped area needs at
/// least a triangle; an empty `maps` list, since the record exists to
/// carry those textures; and a `mapped_to` that does not resolve to an
/// `IfcFace`.
pub fn create_texture_map(
    tx: &mut Transaction,
    model: &Model,
    schema: &Schema,
    maps: &[EntityId],
    vertices: &[EntityId],
    mapped_to: EntityId,
) -> StyleResult<EntityId> {
    if maps.is_empty() {
        return Err(invalid_authoring(
            "IfcTextureMap",
            "Maps",
            "expected at least one surface texture, found none",
        ));
    }
    if vertices.len() < 3 {
        return Err(invalid_authoring(
            "IfcTextureMap",
            "Vertices",
            format!("expected at least 3 vertices, found {}", vertices.len()),
        ));
    }

    validate_ref(tx, model, schema, mapped_to, "IfcFace")?;

    let values = vec![
        (
            "Maps",
            Value::List(maps.iter().copied().map(Value::Ref).collect()),
        ),
        (
            "Vertices",
            Value::List(vertices.iter().copied().map(Value::Ref).collect()),
        ),
        ("MappedTo", Value::Ref(mapped_to)),
    ];

    Ok(tx.create(build_named(schema, "IfcTextureMap", values)?))
}

/// Stage an `IfcTextureCoordinateGenerator`: coordinates computed, not listed.
///
/// `mode` names the generating function (`SPHERE`, `CYLINDER`, and so
/// on). Unlike [`create_texture_map`] there are no vertices: the
/// consumer derives them from the mode and parameters.
///
/// # Errors
///
/// Refuses an empty `maps` list, a blank `mode` -- which would leave
/// the generator with no function to apply -- and a non-finite
/// parameter.
pub fn create_texture_coordinate_generator(
    tx: &mut Transaction,
    schema: &Schema,
    maps: &[EntityId],
    mode: &str,
    parameters: &[f64],
) -> StyleResult<EntityId> {
    if maps.is_empty() {
        return Err(invalid_authoring(
            "IfcTextureCoordinateGenerator",
            "Maps",
            "expected at least one surface texture, found none",
        ));
    }
    if mode.trim().is_empty() {
        return Err(invalid_authoring(
            "IfcTextureCoordinateGenerator",
            "Mode",
            "expected a generating function, found nothing",
        ));
    }
    if let Some(value) = parameters.iter().find(|value| !value.is_finite()) {
        return Err(invalid_authoring(
            "IfcTextureCoordinateGenerator",
            "Parameter",
            format!("expected finite parameters, found {value}"),
        ));
    }

    let mut values = vec![
        (
            "Maps",
            Value::List(maps.iter().copied().map(Value::Ref).collect()),
        ),
        ("Mode", Value::Text(mode.into())),
    ];
    if !parameters.is_empty() {
        values.push((
            "Parameter",
            Value::List(parameters.iter().copied().map(Value::Real).collect()),
        ));
    }

    Ok(tx.create(build_named(
        schema,
        "IfcTextureCoordinateGenerator",
        values,
    )?))
}
