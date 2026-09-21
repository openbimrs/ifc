//! Curve, fill and text presentation styles plus texture coordinate data.

use ifc_model::{EntityId, Transaction, Value};
use ifc_schema::Schema;

use crate::error::StyleResult;

use super::{build_named, invalid_authoring, optional_text};

/// Stage an `IfcCurveStyleFontPattern`.
///
/// # Errors
///
/// Refuses a negative visible length or a non-positive invisible
/// length (VisibleLengthGreaterEqualZero, and the positive measure
/// type on InvisibleSegmentLength).
pub fn create_curve_style_font_pattern(
    tx: &mut Transaction,
    schema: &Schema,
    visible: f64,
    invisible: f64,
) -> StyleResult<EntityId> {
    const ENTITY: &str = "IfcCurveStyleFontPattern";
    if !visible.is_finite() || visible < 0.0 {
        return Err(invalid_authoring(
            ENTITY,
            "VisibleSegmentLength",
            format!("{visible}"),
        ));
    }
    if !invisible.is_finite() || invisible <= 0.0 {
        return Err(invalid_authoring(
            ENTITY,
            "InvisibleSegmentLength",
            format!("{invisible}"),
        ));
    }
    let values = vec![
        ("VisibleSegmentLength", Value::Real(visible)),
        ("InvisibleSegmentLength", Value::Real(invisible)),
    ];
    Ok(tx.create(build_named(schema, ENTITY, values)?))
}

/// Stage an `IfcCurveStyleFont`.
///
/// # Errors
///
/// Refuses an empty pattern list: `PatternList` is `LIST [1:?]`.
pub fn create_curve_style_font(
    tx: &mut Transaction,
    schema: &Schema,
    name: Option<&str>,
    patterns: &[EntityId],
) -> StyleResult<EntityId> {
    const ENTITY: &str = "IfcCurveStyleFont";
    if patterns.is_empty() {
        return Err(invalid_authoring(ENTITY, "PatternList", "empty"));
    }
    let mut values = Vec::new();
    optional_text(&mut values, "Name", name);
    values.push((
        "PatternList",
        Value::List(patterns.iter().copied().map(Value::Ref).collect()),
    ));
    Ok(tx.create(build_named(schema, ENTITY, values)?))
}

/// Stage an `IfcCurveStyleFontAndScaling`.
///
/// # Errors
///
/// Refuses a non-positive scaling factor: `CurveFontScaling` is an
/// `IfcPositiveRatioMeasure`.
pub fn create_curve_style_font_and_scaling(
    tx: &mut Transaction,
    schema: &Schema,
    name: Option<&str>,
    font: EntityId,
    scaling: f64,
) -> StyleResult<EntityId> {
    const ENTITY: &str = "IfcCurveStyleFontAndScaling";
    if !scaling.is_finite() || scaling <= 0.0 {
        return Err(invalid_authoring(
            ENTITY,
            "CurveFontScaling",
            format!("{scaling}"),
        ));
    }
    let mut values = Vec::new();
    optional_text(&mut values, "Name", name);
    values.push(("CurveStyleFont", Value::Ref(font)));
    values.push(("CurveFontScaling", Value::Real(scaling)));
    Ok(tx.create(build_named(schema, ENTITY, values)?))
}

/// Stage an `IfcCurveStyle`.
///
/// `curve_width` is an `IfcSizeSelect`. The schema admits only a
/// positive length, or the descriptive literal `by layer`; a bare
/// number carries no measure and is refused rather than guessed at.
///
/// # Errors
///
/// Refuses a style with no font, width or colour (IdentifiableCurveStyle)
/// and a width that is neither a positive length nor `by layer`
/// (MeasureOfWidth).
pub fn create_curve_style(
    tx: &mut Transaction,
    schema: &Schema,
    draft: CurveStyleDraft<'_>,
) -> StyleResult<EntityId> {
    const ENTITY: &str = "IfcCurveStyle";
    if draft.curve_font.is_none() && draft.curve_width.is_none() && draft.curve_colour.is_none() {
        return Err(invalid_authoring(
            ENTITY,
            "CurveFont",
            "a curve style must carry a font, a width or a colour",
        ));
    }
    let mut values = Vec::new();
    optional_text(&mut values, "Name", draft.name);
    if let Some(font) = draft.curve_font {
        values.push(("CurveFont", Value::Ref(font)));
    }
    if let Some(width) = draft.curve_width {
        values.push(("CurveWidth", width.into_value(ENTITY)?));
    }
    if let Some(colour) = draft.curve_colour {
        values.push(("CurveColour", Value::Ref(colour)));
    }
    if let Some(flag) = draft.model_or_draughting {
        values.push(("ModelOrDraughting", Value::Bool(flag)));
    }
    Ok(tx.create(build_named(schema, ENTITY, values)?))
}

/// The two curve widths `MeasureOfWidth` admits.
///
/// Modelled as a closed enum rather than a bare `f64`: the rule is not
/// "any number", and a caller that passes an unwrapped literal would
/// produce a file the schema rejects.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CurveWidth {
    /// An `IfcPositiveLengthMeasure`.
    PositiveLength(f64),
    /// The descriptive measure `by layer`, deferring width to the layer.
    ByLayer,
}

impl CurveWidth {
    fn into_value(self, entity: &'static str) -> StyleResult<Value> {
        match self {
            Self::PositiveLength(length) if length.is_finite() && length > 0.0 => {
                Ok(Value::Typed {
                    type_name: "IFCPOSITIVELENGTHMEASURE".into(),
                    value: Box::new(Value::Real(length)),
                })
            }
            Self::PositiveLength(length) => {
                Err(invalid_authoring(entity, "CurveWidth", format!("{length}")))
            }
            Self::ByLayer => Ok(Value::Typed {
                type_name: "IFCDESCRIPTIVEMEASURE".into(),
                value: Box::new(Value::Text("by layer".into())),
            }),
        }
    }
}

/// Attributes of an `IfcCurveStyle`.
#[derive(Debug, Clone, Copy, Default)]
pub struct CurveStyleDraft<'a> {
    /// `Name`.
    pub name: Option<&'a str>,
    /// `CurveFont`: an `IfcCurveStyleFont` or `IfcCurveStyleFontAndScaling`.
    pub curve_font: Option<EntityId>,
    /// `CurveWidth`.
    pub curve_width: Option<CurveWidth>,
    /// `CurveColour`: an `IfcColourRgb` or predefined colour.
    pub curve_colour: Option<EntityId>,
    /// `ModelOrDraughting`.
    pub model_or_draughting: Option<bool>,
}

/// Which `IfcFillStyleSelect` member a fill style is.
///
/// The caller states the kind rather than the writer resolving it: a
/// staged member cannot be read back out of a `Transaction`, so the
/// counting rules could not otherwise be checked before commit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FillStyleKind {
    /// An `IfcColour`: at most one per fill area style.
    Colour,
    /// An `IfcExternallyDefinedHatchStyle`: at most one.
    ExternalHatchStyle,
    /// An `IfcFillAreaStyleHatching`.
    Hatching,
    /// An `IfcFillAreaStyleTiles`.
    Tiles,
}

/// Stage an `IfcFillAreaStyle`.
///
/// # Errors
///
/// Refuses an empty style set, more than one colour (MaxOneColour), and
/// more than one externally defined hatch style (MaxOneExtHatchStyle).
pub fn create_fill_area_style(
    tx: &mut Transaction,
    schema: &Schema,
    name: Option<&str>,
    styles: &[(EntityId, FillStyleKind)],
    model_or_draughting: Option<bool>,
) -> StyleResult<EntityId> {
    const ENTITY: &str = "IfcFillAreaStyle";
    if styles.is_empty() {
        return Err(invalid_authoring(ENTITY, "FillStyles", "empty"));
    }
    for (kind, attribute) in [
        (FillStyleKind::Colour, "MaxOneColour"),
        (FillStyleKind::ExternalHatchStyle, "MaxOneExtHatchStyle"),
    ] {
        let found = styles.iter().filter(|(_, k)| *k == kind).count();
        if found > 1 {
            return Err(invalid_authoring(ENTITY, attribute, format!("{found}")));
        }
    }
    let mut values = Vec::new();
    optional_text(&mut values, "Name", name);
    values.push((
        "FillStyles",
        Value::List(styles.iter().map(|(id, _)| Value::Ref(*id)).collect()),
    ));
    if let Some(flag) = model_or_draughting {
        values.push(("ModelOrDraughting", Value::Bool(flag)));
    }
    Ok(tx.create(build_named(schema, ENTITY, values)?))
}

/// Stage an `IfcTextStyleForDefinedFont`.
pub fn create_text_style_for_defined_font(
    tx: &mut Transaction,
    schema: &Schema,
    colour: EntityId,
    background: Option<EntityId>,
) -> StyleResult<EntityId> {
    let mut values = vec![("Colour", Value::Ref(colour))];
    if let Some(background) = background {
        values.push(("BackgroundColour", Value::Ref(background)));
    }
    Ok(tx.create(build_named(schema, "IfcTextStyleForDefinedFont", values)?))
}

/// Stage an `IfcTextStyleFontModel`.
///
/// # Errors
///
/// Refuses a blank name, an empty font family, and a non-positive font
/// size. `MeasureOfFontSize` also requires the size to be an
/// `IfcLengthMeasure`, so it is written with that wrapper.
pub fn create_text_style_font_model(
    tx: &mut Transaction,
    schema: &Schema,
    name: &str,
    font_family: &[&str],
    font_size: f64,
) -> StyleResult<EntityId> {
    const ENTITY: &str = "IfcTextStyleFontModel";
    if name.trim().is_empty() {
        return Err(invalid_authoring(ENTITY, "Name", name));
    }
    if font_family.is_empty() {
        return Err(invalid_authoring(ENTITY, "FontFamily", "empty"));
    }
    if !font_size.is_finite() || font_size <= 0.0 {
        return Err(invalid_authoring(
            ENTITY,
            "FontSize",
            format!("{font_size}"),
        ));
    }
    let values = vec![
        ("Name", Value::Text(name.into())),
        (
            "FontFamily",
            Value::List(
                font_family
                    .iter()
                    .map(|family| Value::Text((*family).into()))
                    .collect(),
            ),
        ),
        (
            "FontSize",
            Value::Typed {
                type_name: "IFCLENGTHMEASURE".into(),
                value: Box::new(Value::Real(font_size)),
            },
        ),
    ];
    Ok(tx.create(build_named(schema, ENTITY, values)?))
}

/// The names `PreDefinedColourNames` admits, lower-case as the schema states.
pub const PREDEFINED_COLOUR_NAMES: &[&str] = &[
    "black", "red", "green", "blue", "yellow", "magenta", "cyan", "white", "by layer",
];

/// The names `PreDefinedCurveFontNames` admits.
pub const PREDEFINED_CURVE_FONT_NAMES: &[&str] = &[
    "continuous",
    "chain",
    "chain double dash",
    "dashed",
    "dotted",
    "by layer",
];

/// Stage an `IfcDraughtingPreDefinedColour`.
///
/// # Errors
///
/// Refuses a name outside `PREDEFINED_COLOUR_NAMES`. The comparison is
/// case-sensitive: the schema states the members in lower case, and a
/// reader matching them literally would miss `Black`.
pub fn create_draughting_predefined_colour(
    tx: &mut Transaction,
    schema: &Schema,
    name: &str,
) -> StyleResult<EntityId> {
    const ENTITY: &str = "IfcDraughtingPreDefinedColour";
    if !PREDEFINED_COLOUR_NAMES.contains(&name) {
        return Err(invalid_authoring(ENTITY, "Name", name));
    }
    let values = vec![("Name", Value::Text(name.into()))];
    Ok(tx.create(build_named(schema, ENTITY, values)?))
}

/// Stage an `IfcDraughtingPreDefinedCurveFont`.
///
/// # Errors
///
/// Refuses a name outside `PREDEFINED_CURVE_FONT_NAMES`.
pub fn create_draughting_predefined_curve_font(
    tx: &mut Transaction,
    schema: &Schema,
    name: &str,
) -> StyleResult<EntityId> {
    const ENTITY: &str = "IfcDraughtingPreDefinedCurveFont";
    if !PREDEFINED_CURVE_FONT_NAMES.contains(&name) {
        return Err(invalid_authoring(ENTITY, "Name", name));
    }
    let values = vec![("Name", Value::Text(name.into()))];
    Ok(tx.create(build_named(schema, ENTITY, values)?))
}

/// Stage an `IfcPixelTexture`.
///
/// `pixel` holds one binary literal per pixel, hex-encoded without the
/// STEP quotes. All five WHERE rules are checked here.
///
/// # Errors
///
/// Refuses a width or height below one (MinPixelInS, MinPixelInT), a
/// colour-component count outside 1..=4 (NumberOfColours), a pixel count
/// that is not `width * height` (SizeOfPixelList), and pixels that are
/// not whole bytes or differ in length from the first (
/// PixelAsByteAndSameLength).
pub fn create_pixel_texture(
    tx: &mut Transaction,
    schema: &Schema,
    draft: PixelTextureDraft<'_>,
) -> StyleResult<EntityId> {
    const ENTITY: &str = "IfcPixelTexture";
    if draft.width < 1 {
        return Err(invalid_authoring(
            ENTITY,
            "Width",
            format!("{}", draft.width),
        ));
    }
    if draft.height < 1 {
        return Err(invalid_authoring(
            ENTITY,
            "Height",
            format!("{}", draft.height),
        ));
    }
    if !(1..=4).contains(&draft.colour_components) {
        return Err(invalid_authoring(
            ENTITY,
            "ColourComponents",
            format!("{}", draft.colour_components),
        ));
    }
    let expected = i64::from(draft.width) * i64::from(draft.height);
    if draft.pixel.len() as i64 != expected {
        return Err(invalid_authoring(
            ENTITY,
            "SizeOfPixelList",
            format!("{} pixels for {expected} cells", draft.pixel.len()),
        ));
    }
    // BLENGTH counts bits: a hex literal carries four bits per digit, so a
    // whole number of bytes means an even digit count.
    let first = draft.pixel[0].len();
    for pixel in draft.pixel {
        if pixel.len() % 2 != 0 || pixel.len() != first {
            return Err(invalid_authoring(
                ENTITY,
                "PixelAsByteAndSameLength",
                pixel.to_string(),
            ));
        }
    }
    let mut values = vec![
        ("RepeatS", Value::Bool(draft.repeat_s)),
        ("RepeatT", Value::Bool(draft.repeat_t)),
    ];
    optional_text(&mut values, "Mode", draft.mode);
    values.extend([
        ("Width", Value::Integer(i64::from(draft.width))),
        ("Height", Value::Integer(i64::from(draft.height))),
        (
            "ColourComponents",
            Value::Integer(i64::from(draft.colour_components)),
        ),
        (
            "Pixel",
            Value::List(
                draft
                    .pixel
                    .iter()
                    .map(|pixel| Value::Binary((*pixel).into()))
                    .collect(),
            ),
        ),
    ]);
    Ok(tx.create(build_named(schema, ENTITY, values)?))
}

/// Attributes of an `IfcPixelTexture`.
#[derive(Debug, Clone, Copy)]
pub struct PixelTextureDraft<'a> {
    /// `RepeatS`.
    pub repeat_s: bool,
    /// `RepeatT`.
    pub repeat_t: bool,
    /// `Mode`.
    pub mode: Option<&'a str>,
    /// `Width`, at least one.
    pub width: i32,
    /// `Height`, at least one.
    pub height: i32,
    /// `ColourComponents`, in `1..=4`.
    pub colour_components: i32,
    /// `Pixel`: hex literals, one per cell, all the same length.
    pub pixel: &'a [&'a str],
}

/// Stage an `IfcTextureVertex`: one 2-tuple of parameter values.
///
/// # Errors
///
/// Refuses a non-finite coordinate. The pair arity is carried by the
/// argument type, so a wrong-length vertex cannot be expressed.
pub fn create_texture_vertex(
    tx: &mut Transaction,
    schema: &Schema,
    coordinates: [f64; 2],
) -> StyleResult<EntityId> {
    const ENTITY: &str = "IfcTextureVertex";
    for value in coordinates {
        if !value.is_finite() {
            return Err(invalid_authoring(ENTITY, "Coordinates", format!("{value}")));
        }
    }
    let values = vec![(
        "Coordinates",
        Value::List(coordinates.into_iter().map(Value::Real).collect()),
    )];
    Ok(tx.create(build_named(schema, ENTITY, values)?))
}

/// Stage an `IfcTextureVertexList`.
///
/// # Errors
///
/// Refuses an empty list and any non-finite coordinate.
pub fn create_texture_vertex_list(
    tx: &mut Transaction,
    schema: &Schema,
    coordinates: &[[f64; 2]],
) -> StyleResult<EntityId> {
    const ENTITY: &str = "IfcTextureVertexList";
    if coordinates.is_empty() {
        return Err(invalid_authoring(ENTITY, "TexCoordsList", "empty"));
    }
    for pair in coordinates {
        for value in pair {
            if !value.is_finite() {
                return Err(invalid_authoring(
                    ENTITY,
                    "TexCoordsList",
                    format!("{value}"),
                ));
            }
        }
    }
    let values = vec![(
        "TexCoordsList",
        Value::List(
            coordinates
                .iter()
                .map(|pair| Value::List(pair.iter().copied().map(Value::Real).collect()))
                .collect(),
        ),
    )];
    Ok(tx.create(build_named(schema, ENTITY, values)?))
}

/// Stage an `IfcColourRgbList`.
///
/// # Errors
///
/// Refuses an empty list and any channel outside `[0, 1]`: the inner
/// members are `IfcNormalisedRatioMeasure`, not free reals.
pub fn create_colour_rgb_list(
    tx: &mut Transaction,
    schema: &Schema,
    colours: &[[f64; 3]],
) -> StyleResult<EntityId> {
    const ENTITY: &str = "IfcColourRgbList";
    if colours.is_empty() {
        return Err(invalid_authoring(ENTITY, "ColourList", "empty"));
    }
    for colour in colours {
        for channel in colour {
            if !channel.is_finite() || *channel < 0.0 || *channel > 1.0 {
                return Err(invalid_authoring(
                    ENTITY,
                    "ColourList",
                    format!("{channel}"),
                ));
            }
        }
    }
    let values = vec![(
        "ColourList",
        Value::List(
            colours
                .iter()
                .map(|colour| Value::List(colour.iter().copied().map(Value::Real).collect()))
                .collect(),
        ),
    )];
    Ok(tx.create(build_named(schema, ENTITY, values)?))
}

/// Stage an `IfcIndexedColourMap`.
///
/// # Errors
///
/// Refuses an empty colour index, a non-positive index (the members are
/// `IfcPositiveInteger`, and STEP indices are one-based), and an opacity
/// outside `[0, 1]`.
pub fn create_indexed_colour_map(
    tx: &mut Transaction,
    schema: &Schema,
    mapped_to: EntityId,
    colours: EntityId,
    colour_index: &[i64],
    opacity: Option<f64>,
) -> StyleResult<EntityId> {
    const ENTITY: &str = "IfcIndexedColourMap";
    if colour_index.is_empty() {
        return Err(invalid_authoring(ENTITY, "ColourIndex", "empty"));
    }
    if let Some(index) = colour_index.iter().find(|index| **index < 1) {
        return Err(invalid_authoring(ENTITY, "ColourIndex", format!("{index}")));
    }
    if let Some(value) = opacity {
        if !value.is_finite() || !(0.0..=1.0).contains(&value) {
            return Err(invalid_authoring(ENTITY, "Opacity", format!("{value}")));
        }
    }
    let mut values = vec![("MappedTo", Value::Ref(mapped_to))];
    if let Some(value) = opacity {
        values.push(("Opacity", Value::Real(value)));
    }
    values.extend([
        ("Colours", Value::Ref(colours)),
        (
            "ColourIndex",
            Value::List(colour_index.iter().copied().map(Value::Integer).collect()),
        ),
    ]);
    Ok(tx.create(build_named(schema, ENTITY, values)?))
}

/// Stage an `IfcPresentationLayerAssignment`.
///
/// # Errors
///
/// Refuses a blank name and an empty item set (ApplicableItems).
pub fn create_presentation_layer_assignment(
    tx: &mut Transaction,
    schema: &Schema,
    name: &str,
    description: Option<&str>,
    assigned_items: &[EntityId],
    identifier: Option<&str>,
) -> StyleResult<EntityId> {
    const ENTITY: &str = "IfcPresentationLayerAssignment";
    if name.trim().is_empty() {
        return Err(invalid_authoring(ENTITY, "Name", name));
    }
    if assigned_items.is_empty() {
        return Err(invalid_authoring(ENTITY, "AssignedItems", "empty"));
    }
    let mut values = vec![("Name", Value::Text(name.into()))];
    optional_text(&mut values, "Description", description);
    values.push((
        "AssignedItems",
        Value::List(assigned_items.iter().copied().map(Value::Ref).collect()),
    ));
    optional_text(&mut values, "Identifier", identifier);
    Ok(tx.create(build_named(schema, ENTITY, values)?))
}
