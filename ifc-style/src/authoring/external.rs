//! Externally defined styles, texture coordinate indices, and
//! the blob texture.
//!
//! # Why these sit apart from the presentation writers
//!
//! An externally defined style names a style held in another
//! system: a hatch pattern in a CAD library, a font the file does
//! not embed. The record carries only the reference, so nothing
//! here validates the target exists -- that is the point of the
//! entity, and inventing a check would refuse legitimate files.
//!
//! # The rules that are enforced
//!
//! `IfcBlobTexture` states both of the family's WHERE rules:
//!
//! ```text
//! RasterCodeByteStream  : BLENGTH(RasterCode) MOD 8 = 0;
//! SupportedRasterFormat : SELF.RasterFormat IN ['BMP','JPG','GIF','PNG'];
//! ```
//!
//! A raster code that is not a whole number of bytes cannot be
//! decoded by any reader, and a format outside the closed list
//! names a decoder the schema does not promise.

use ifc_model::{EntityId, Transaction, Value};
use ifc_schema::Schema;

use crate::error::StyleResult;

use super::{build_named, invalid_authoring, optional_reference, optional_text};

/// Which externally defined style is being named.
///
/// The three share one attribute shape, so a single writer covers
/// them; the tag keeps a caller from spelling the type name itself.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExternalStyleKind {
    /// `IfcExternallyDefinedHatchStyle`: a fill pattern held elsewhere.
    HatchStyle,
    /// `IfcExternallyDefinedSurfaceStyle`: a surface appearance held elsewhere.
    SurfaceStyle,
    /// `IfcExternallyDefinedTextFont`: a font the file does not embed.
    TextFont,
}

impl ExternalStyleKind {
    fn type_name(self) -> &'static str {
        match self {
            Self::HatchStyle => "IfcExternallyDefinedHatchStyle",
            Self::SurfaceStyle => "IfcExternallyDefinedSurfaceStyle",
            Self::TextFont => "IfcExternallyDefinedTextFont",
        }
    }
}

/// Stage an externally defined style reference.
///
/// All three attributes are optional in the schema, but a reference
/// naming nothing at all identifies no external resource: it would
/// round-trip as three nulls and resolve for no reader. At least one
/// of location, identification, or name must be given.
///
/// # Errors
///
/// Refuses a reference with no locating attribute set, and a blank
/// string in any attribute that is supplied.
pub fn create_externally_defined_style(
    tx: &mut Transaction,
    schema: &Schema,
    kind: ExternalStyleKind,
    location: Option<&str>,
    identification: Option<&str>,
    name: Option<&str>,
) -> StyleResult<EntityId> {
    let entity = kind.type_name();
    for (attribute, value) in [
        ("Location", location),
        ("Identification", identification),
        ("Name", name),
    ] {
        if value.is_some_and(|text| text.trim().is_empty()) {
            return Err(invalid_authoring(entity, attribute, "blank"));
        }
    }
    if location.is_none() && identification.is_none() && name.is_none() {
        return Err(invalid_authoring(entity, "Location", "no attribute set"));
    }
    let mut values = Vec::new();
    optional_text(&mut values, "Location", location);
    optional_text(&mut values, "Identification", identification);
    optional_text(&mut values, "Name", name);
    Ok(tx.create(build_named(schema, entity, values)?))
}

fn index_list(
    entity: &'static str,
    attribute: &'static str,
    indices: &[i64],
) -> StyleResult<Value> {
    if indices.len() < 3 {
        return Err(invalid_authoring(entity, attribute, indices.len()));
    }
    // IfcPositiveInteger is one-based: 0 is not a valid index and
    // accepting it would silently shift every lookup by one.
    if let Some(bad) = indices.iter().find(|index| **index < 1) {
        return Err(invalid_authoring(entity, attribute, bad));
    }
    Ok(Value::List(
        indices.iter().copied().map(Value::Integer).collect(),
    ))
}

/// Stage an `IfcTextureCoordinateIndices`.
///
/// # Errors
///
/// Refuses fewer than three indices, and any index below 1.
pub fn create_texture_coordinate_indices(
    tx: &mut Transaction,
    schema: &Schema,
    tex_coord_index: &[i64],
    tex_coords_of: EntityId,
) -> StyleResult<EntityId> {
    const ENTITY: &str = "IfcTextureCoordinateIndices";
    let values = vec![
        (
            "TexCoordIndex",
            index_list(ENTITY, "TexCoordIndex", tex_coord_index)?,
        ),
        ("TexCoordsOf", Value::Ref(tex_coords_of)),
    ];
    Ok(tx.create(build_named(schema, ENTITY, values)?))
}

/// Stage an `IfcTextureCoordinateIndicesWithVoids`.
///
/// `inner_tex_coord_indices` is `LIST [1:?] OF LIST [3:?] OF UNIQUE`:
/// each void is its own loop of at least three indices, and the
/// indices within one void must be distinct. A repeated index inside
/// a void collapses two corners of the hole onto one texture
/// coordinate, which parses and renders wrongly.
///
/// # Errors
///
/// Refuses an outer list with no voids, any void with fewer than
/// three indices, any index below 1, and a repeated index within a
/// single void.
pub fn create_texture_coordinate_indices_with_voids(
    tx: &mut Transaction,
    schema: &Schema,
    tex_coord_index: &[i64],
    tex_coords_of: EntityId,
    inner_tex_coord_indices: &[&[i64]],
) -> StyleResult<EntityId> {
    const ENTITY: &str = "IfcTextureCoordinateIndicesWithVoids";
    if inner_tex_coord_indices.is_empty() {
        return Err(invalid_authoring(ENTITY, "InnerTexCoordIndices", "empty"));
    }
    let mut inner = Vec::with_capacity(inner_tex_coord_indices.len());
    for void in inner_tex_coord_indices {
        let list = index_list(ENTITY, "InnerTexCoordIndices", void)?;
        // UNIQUE is declared on the inner list, so duplicates are
        // checked per void rather than across the whole set.
        for (position, index) in void.iter().enumerate() {
            if void[..position].contains(index) {
                return Err(invalid_authoring(ENTITY, "InnerTexCoordIndices", index));
            }
        }
        inner.push(list);
    }
    let values = vec![
        (
            "TexCoordIndex",
            index_list(ENTITY, "TexCoordIndex", tex_coord_index)?,
        ),
        ("TexCoordsOf", Value::Ref(tex_coords_of)),
        ("InnerTexCoordIndices", Value::List(inner)),
    ];
    Ok(tx.create(build_named(schema, ENTITY, values)?))
}

/// Attributes of an `IfcBlobTexture`.
#[derive(Debug, Clone, Copy, Default)]
pub struct BlobTextureDraft<'a> {
    /// `RepeatS`: whether the texture tiles along S.
    pub repeat_s: bool,
    /// `RepeatT`: whether the texture tiles along T.
    pub repeat_t: bool,
    /// `Mode`: the texture application mode.
    pub mode: Option<&'a str>,
    /// `TextureTransform`: a 2D operator applied to the coordinates.
    pub texture_transform: Option<EntityId>,
    /// `Parameter`: optional, but non-empty when present.
    pub parameter: &'a [&'a str],
    /// `RasterFormat`: one of BMP, JPG, GIF, PNG.
    pub raster_format: &'a str,
    /// `RasterCode`: the hex payload, a whole number of bytes.
    pub raster_code: &'a str,
}
/// Raster formats `SupportedRasterFormat` admits.
///
/// Closed and upper-case in the schema: 'png' is not 'PNG'.
const RASTER_FORMATS: &[&str] = &["BMP", "JPG", "GIF", "PNG"];

/// Stage an `IfcBlobTexture`, embedding the raster bytes in the file.
///
/// `raster_code` is the hex payload as it appears in STEP, without
/// the surrounding quotes. `RasterCodeByteStream` requires it to be
/// a whole number of bytes: STEP binary is bit-counted, so an odd
/// hex-digit count is a partial byte no decoder can complete.
///
/// # Errors
///
/// Refuses a raster format outside the closed list, a payload that is
/// not a whole number of bytes, a non-hex payload, an empty payload,
/// and an empty parameter list.
pub fn create_blob_texture(
    tx: &mut Transaction,
    schema: &Schema,
    draft: BlobTextureDraft<'_>,
) -> StyleResult<EntityId> {
    let BlobTextureDraft {
        repeat_s,
        repeat_t,
        mode,
        texture_transform,
        parameter,
        raster_format,
        raster_code,
    } = draft;
    const ENTITY: &str = "IfcBlobTexture";
    if !RASTER_FORMATS.contains(&raster_format) {
        return Err(invalid_authoring(ENTITY, "RasterFormat", raster_format));
    }
    if raster_code.is_empty() {
        return Err(invalid_authoring(ENTITY, "RasterCode", "empty"));
    }
    // BLENGTH counts bits; one hex digit is four bits, so a whole
    // byte is an even digit count.
    if raster_code.len() % 2 != 0 {
        return Err(invalid_authoring(ENTITY, "RasterCode", raster_code.len()));
    }
    if let Some(bad) = raster_code.chars().find(|c| !c.is_ascii_hexdigit()) {
        return Err(invalid_authoring(ENTITY, "RasterCode", bad));
    }
    Ok(tx.create(build_named(
        schema,
        ENTITY,
        blob_values(
            repeat_s,
            repeat_t,
            mode,
            texture_transform,
            parameter,
            raster_format,
            raster_code,
        )?,
    )?))
}

#[allow(clippy::too_many_arguments)]
fn blob_values(
    repeat_s: bool,
    repeat_t: bool,
    mode: Option<&str>,
    texture_transform: Option<EntityId>,
    parameter: &[&str],
    raster_format: &str,
    raster_code: &str,
) -> StyleResult<Vec<(&'static str, Value)>> {
    const ENTITY: &str = "IfcBlobTexture";
    // Parameter is OPTIONAL LIST [1:?]: absent is legal, present and
    // empty is not.
    let parameters = if parameter.is_empty() {
        Value::Null
    } else {
        for text in parameter {
            if text.trim().is_empty() {
                return Err(invalid_authoring(ENTITY, "Parameter", "blank"));
            }
        }
        Value::List(parameter.iter().map(|t| Value::Text((*t).into())).collect())
    };
    let mut values = vec![
        ("RepeatS", Value::Bool(repeat_s)),
        ("RepeatT", Value::Bool(repeat_t)),
        ("RasterFormat", Value::Text(raster_format.into())),
        ("RasterCode", Value::Binary(raster_code.into())),
    ];
    if !matches!(parameters, Value::Null) {
        values.push(("Parameter", parameters));
    }
    optional_text(&mut values, "Mode", mode);
    optional_reference(&mut values, "TextureTransform", texture_transform);
    Ok(values)
}

/// Stage an `IfcSurfaceStyleWithTextures`.
///
/// # Errors
///
/// Refuses an empty texture list: the single attribute is
/// `LIST [1:?]`, so a style with no textures is not a style.
pub fn create_surface_style_with_textures(
    tx: &mut Transaction,
    schema: &Schema,
    textures: &[EntityId],
) -> StyleResult<EntityId> {
    const ENTITY: &str = "IfcSurfaceStyleWithTextures";
    if textures.is_empty() {
        return Err(invalid_authoring(ENTITY, "Textures", "empty"));
    }
    let values = vec![(
        "Textures",
        Value::List(textures.iter().copied().map(Value::Ref).collect()),
    )];
    Ok(tx.create(build_named(schema, ENTITY, values)?))
}

/// Stage an `IfcIndexedPolygonalTextureMap`.
///
/// # Errors
///
/// Refuses an empty map list and an empty coordinate-index set,
/// both declared `[1:?]`.
pub fn create_indexed_polygonal_texture_map(
    tx: &mut Transaction,
    schema: &Schema,
    maps: &[EntityId],
    mapped_to: EntityId,
    tex_coords: EntityId,
    tex_coord_indices: &[EntityId],
) -> StyleResult<EntityId> {
    const ENTITY: &str = "IfcIndexedPolygonalTextureMap";
    if maps.is_empty() {
        return Err(invalid_authoring(ENTITY, "Maps", "empty"));
    }
    if tex_coord_indices.is_empty() {
        return Err(invalid_authoring(ENTITY, "TexCoordIndices", "empty"));
    }
    let values = vec![
        (
            "Maps",
            Value::List(maps.iter().copied().map(Value::Ref).collect()),
        ),
        ("MappedTo", Value::Ref(mapped_to)),
        ("TexCoords", Value::Ref(tex_coords)),
        (
            "TexCoordIndices",
            Value::List(tex_coord_indices.iter().copied().map(Value::Ref).collect()),
        ),
    ];
    Ok(tx.create(build_named(schema, ENTITY, values)?))
}

/// `IfcTextAlignment`: closed and lower-case.
const TEXT_ALIGNMENT: &[&str] = &["left", "right", "center", "justify"];

/// `IfcTextDecoration`: closed and lower-case.
const TEXT_DECORATION: &[&str] = &["none", "underline", "overline", "line-through", "blink"];

/// `IfcTextTransformation`: closed and lower-case.
const TEXT_TRANSFORMATION: &[&str] = &["capitalize", "uppercase", "lowercase", "none"];

fn closed(
    entity: &'static str,
    attribute: &'static str,
    value: Option<&str>,
    permitted: &[&str],
) -> StyleResult<Value> {
    let Some(text) = value else {
        return Ok(Value::Null);
    };
    // Case-sensitive: the schema lists 'none', so 'None' fails WR1.
    if !permitted.contains(&text) {
        return Err(invalid_authoring(entity, attribute, text));
    }
    Ok(Value::Text(text.into()))
}

/// A resolved `IfcSizeSelect` value.
///
/// The select admits six measure types, and a bare real is
/// ambiguous between them. The caller names which one it means so
/// the value is written typed rather than guessed.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SizeValue<'a> {
    /// `IfcDescriptiveMeasure`: a word such as 'small'.
    Descriptive(&'a str),
    /// `IfcLengthMeasure`: any finite length.
    Length(f64),
    /// `IfcPositiveLengthMeasure`: a length above zero.
    PositiveLength(f64),
    /// `IfcRatioMeasure`: any finite ratio.
    Ratio(f64),
    /// `IfcNormalisedRatioMeasure`: a ratio within 0 to 1.
    NormalisedRatio(f64),
    /// `IfcPositiveRatioMeasure`: a ratio above zero.
    PositiveRatio(f64),
}

impl SizeValue<'_> {
    fn into_value(self, entity: &'static str, attribute: &'static str) -> StyleResult<Value> {
        let (type_name, inner) = match self {
            Self::Descriptive(text) => {
                if text.trim().is_empty() {
                    return Err(invalid_authoring(entity, attribute, "blank"));
                }
                ("IFCDESCRIPTIVEMEASURE", Value::Text(text.into()))
            }
            Self::Length(v) => ("IFCLENGTHMEASURE", finite(entity, attribute, v)?),
            Self::PositiveLength(v) => {
                if v <= 0.0 {
                    return Err(invalid_authoring(entity, attribute, v));
                }
                ("IFCPOSITIVELENGTHMEASURE", finite(entity, attribute, v)?)
            }
            Self::Ratio(v) => ("IFCRATIOMEASURE", finite(entity, attribute, v)?),
            Self::NormalisedRatio(v) => {
                if !(0.0..=1.0).contains(&v) {
                    return Err(invalid_authoring(entity, attribute, v));
                }
                ("IFCNORMALISEDRATIOMEASURE", finite(entity, attribute, v)?)
            }
            Self::PositiveRatio(v) => {
                if v <= 0.0 {
                    return Err(invalid_authoring(entity, attribute, v));
                }
                ("IFCPOSITIVERATIOMEASURE", finite(entity, attribute, v)?)
            }
        };
        Ok(Value::Typed {
            type_name: type_name.into(),
            value: Box::new(inner),
        })
    }
}

fn finite(entity: &'static str, attribute: &'static str, value: f64) -> StyleResult<Value> {
    if !value.is_finite() {
        return Err(invalid_authoring(entity, attribute, value));
    }
    Ok(Value::Real(value))
}

/// Attributes of an `IfcTextStyleTextModel`.
#[derive(Debug, Clone, Copy, Default)]
pub struct TextModelDraft<'a> {
    /// `TextIndent`.
    pub text_indent: Option<SizeValue<'a>>,
    /// `TextAlign`, one of the closed lower-case alignment words.
    pub text_align: Option<&'a str>,
    /// `TextDecoration`, one of the closed lower-case decoration words.
    pub text_decoration: Option<&'a str>,
    /// `LetterSpacing`.
    pub letter_spacing: Option<SizeValue<'a>>,
    /// `WordSpacing`.
    pub word_spacing: Option<SizeValue<'a>>,
    /// `TextTransform`, one of the closed lower-case transform words.
    pub text_transform: Option<&'a str>,
    /// `LineHeight`.
    pub line_height: Option<SizeValue<'a>>,
}

/// Stage an `IfcTextStyleTextModel`.
///
/// Every attribute is optional, so this writer enforces the value
/// rules rather than presence: the three word-valued attributes are
/// closed lower-case lists, and each size is a resolved select.
///
/// # Errors
///
/// Refuses a word outside its closed list, and a size value that
/// violates the measure type the caller named.
pub fn create_text_style_text_model(
    tx: &mut Transaction,
    schema: &Schema,
    draft: TextModelDraft<'_>,
) -> StyleResult<EntityId> {
    const ENTITY: &str = "IfcTextStyleTextModel";
    let values = vec![
        ("TextIndent", size(ENTITY, "TextIndent", draft.text_indent)?),
        (
            "TextAlign",
            closed(ENTITY, "TextAlign", draft.text_align, TEXT_ALIGNMENT)?,
        ),
        (
            "TextDecoration",
            closed(
                ENTITY,
                "TextDecoration",
                draft.text_decoration,
                TEXT_DECORATION,
            )?,
        ),
        (
            "LetterSpacing",
            size(ENTITY, "LetterSpacing", draft.letter_spacing)?,
        ),
        (
            "WordSpacing",
            size(ENTITY, "WordSpacing", draft.word_spacing)?,
        ),
        (
            "TextTransform",
            closed(
                ENTITY,
                "TextTransform",
                draft.text_transform,
                TEXT_TRANSFORMATION,
            )?,
        ),
        ("LineHeight", size(ENTITY, "LineHeight", draft.line_height)?),
    ];
    Ok(tx.create(build_named(schema, ENTITY, values)?))
}

fn size(
    entity: &'static str,
    attribute: &'static str,
    value: Option<SizeValue<'_>>,
) -> StyleResult<Value> {
    value.map_or(Ok(Value::Null), |v| v.into_value(entity, attribute))
}

/// Stage an `IfcTextStyle`.
///
/// `text_font_style` is `IfcTextFontSelect` and is required: it
/// resolves to either a pre-defined text font or an externally
/// defined one, both of which this module can stage.
///
/// # Errors
///
/// Refuses a blank name.
pub fn create_text_style(
    tx: &mut Transaction,
    schema: &Schema,
    name: Option<&str>,
    text_character_appearance: Option<EntityId>,
    text_style: Option<EntityId>,
    text_font_style: EntityId,
    model_or_draughting: Option<bool>,
) -> StyleResult<EntityId> {
    const ENTITY: &str = "IfcTextStyle";
    if name.is_some_and(|text| text.trim().is_empty()) {
        return Err(invalid_authoring(ENTITY, "Name", "blank"));
    }
    let mut values = vec![("TextFontStyle", Value::Ref(text_font_style))];
    optional_text(&mut values, "Name", name);
    optional_reference(
        &mut values,
        "TextCharacterAppearance",
        text_character_appearance,
    );
    optional_reference(&mut values, "TextStyle", text_style);
    if let Some(flag) = model_or_draughting {
        values.push(("ModelOrDraughting", Value::Bool(flag)));
    }
    Ok(tx.create(build_named(schema, ENTITY, values)?))
}
