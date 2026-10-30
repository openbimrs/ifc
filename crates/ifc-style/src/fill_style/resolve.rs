//! Resolving an `IfcFillAreaStyle` into hatch line families (#597).
//!
//! What the standard fixes, and what this module reads from it (IFC4 ADD2
//! TC1 unless noted; the IFC2X3, IFC4X1, IFC4X2 and IFC4X3 ADD2 schemas agree
//! where the text says "all releases"):
//!
//! - `IfcFillAreaStyleHatching` (all releases) has a `HatchLineAppearance`
//!   curve style, a `StartOfNextHatchLine` distance, optional
//!   `PointOfReferenceHatchLine` and `PatternStart` points, which must be 2D
//!   (`PatternStart2D`, `RefHatchLine2D`), and a `HatchLineAngle`.
//! - `IfcHatchLineDistanceSelect` is `IfcPositiveLengthMeasure` or
//!   `IfcVector` from IFC4 on; IFC2X3 has `IfcOneDirectionRepeatFactor` (a
//!   vector in its `RepeatFactor`) in place of the vector, and its WR21
//!   forbids the `IfcTwoDirectionRepeatFactor` subtype.
//! - `IfcFillAreaStyle.FillStyles` is a non-empty set with at most one
//!   colour (`MaxOneColour`) and one externally defined hatch style
//!   (`MaxOneExtHatchStyle`); `ConsistentHatchStyleDef` forbids an external
//!   style next to anything else and hatching next to tiles. A colour next
//!   to hatching is the background; a colour alone is a solid fill.
//! - Hatch measures are in the project's global drawing units: lengths in
//!   the project length unit, `HatchLineAngle` in the project plane angle
//!   unit. The caller passes the factors ([`StyleUnits`]).
//! - A vector distance is read in the hatching's x/y frame: with the hatch
//!   direction `u = (cos a, sin a)` and its normal `n = (-sin a, cos a)`, the
//!   vector `v` splits into the perpendicular spacing `v . n` and the shift
//!   along the line `v . u`. A length distance is a perpendicular spacing
//!   with no shift. An absent `PointOfReferenceHatchLine` is the origin, the
//!   standard's own value.
//! - `IfcCurveStyle` states font, width and colour, each optional; an unset
//!   one stays `None`, never "continuous" or "black". The width is an
//!   `IfcPositiveLengthMeasure` or the `IfcDescriptiveMeasure` 'by layer'.
//!   `IfcCurveStyleFontAndScaling` multiplies its font's dashes by
//!   `CurveFontScaling` (> 0); its font slot is `CurveFont` up to IFC4X2 and
//!   `CurveStyleFont` in IFC4X3. A pattern's visible length is >= 0, its
//!   invisible length > 0.
//! - The predefined curve fonts other than 'continuous' give their lengths
//!   in a figure only, so they are refused, as are 'by layer' colours and
//!   widths, tiles and externally defined hatch styles.

use ifc_model::{EntityId, Value};

use crate::colour::colour_rgb_of;
use crate::error::{StyleError, StyleResult};
use crate::fill_style::{HatchLine, MaterialFill};
use crate::units::StyleUnits;
use crate::StyleView;

/// Resolve `style`, an `IfcFillAreaStyle` read from `styled_item`.
pub(crate) fn resolve_fill(
    view: StyleView<'_, '_>,
    style: EntityId,
    styled_item: EntityId,
    units: StyleUnits,
) -> StyleResult<MaterialFill> {
    let fill = view.fill_area_style(style)?;
    let style_type = entity_type(view, style)?;
    let (mut colours, mut external, mut hatchings, mut tiles) =
        (Vec::new(), Vec::new(), Vec::new(), Vec::new());
    for member in fill.fill_styles()? {
        let name = entity_type(view, member)?;
        let is_a = |ancestor| view.schema.is_a(&name, ancestor);
        if is_a("IfcColourSpecification") || is_a("IfcPreDefinedColour") {
            colours.push(member);
        } else if is_a("IfcExternallyDefinedHatchStyle") {
            external.push(member);
        } else if is_a("IfcFillAreaStyleHatching") {
            hatchings.push(member);
        } else if is_a("IfcFillAreaStyleTiles") {
            tiles.push(member);
        }
    }
    let breach = |rule: String| StyleError::InvalidValue {
        entity: style_type.clone(),
        id: style,
        attribute: "FillStyles",
        value: rule,
    };
    if colours.len() > 1 {
        return Err(breach(format!("MaxOneColour: {} colours", colours.len())));
    }
    if external.len() > 1 {
        return Err(breach(format!("MaxOneExtHatchStyle: {}", external.len())));
    }
    if !external.is_empty() && (!colours.is_empty() || !hatchings.is_empty() || !tiles.is_empty()) {
        return Err(breach(
            "ConsistentHatchStyleDef: external hatch style with other styles".to_owned(),
        ));
    }
    if !hatchings.is_empty() && !tiles.is_empty() {
        return Err(breach(
            "ConsistentHatchStyleDef: hatching with tiles".to_owned(),
        ));
    }
    let unsupported = |entity, construct: &str| StyleError::UnsupportedFill {
        style,
        entity,
        construct: construct.to_owned(),
    };
    if let Some(&tile) = tiles.first() {
        return Err(unsupported(tile, "IfcFillAreaStyleTiles"));
    }
    if let Some(&hatch) = external.first() {
        return Err(unsupported(hatch, "IfcExternallyDefinedHatchStyle"));
    }
    let background = colours
        .first()
        .map(|&colour| colour_rgb_of(view, style, colour))
        .transpose()?;
    let lines = hatchings
        .iter()
        .map(|&hatching| resolve_hatching(view, style, hatching, units))
        .collect::<StyleResult<Vec<_>>>()?;
    Ok(MaterialFill {
        style,
        styled_item,
        name: fill.name()?.map(str::to_owned),
        model_or_draughting: fill.model_or_draughting()?,
        background,
        lines,
    })
}

fn entity_type(view: StyleView<'_, '_>, id: EntityId) -> StyleResult<String> {
    view.model
        .get(id)
        .map(|entity| entity.type_name.to_string())
        .ok_or(StyleError::UnknownEntity { id })
}

fn resolve_hatching(
    view: StyleView<'_, '_>,
    style: EntityId,
    id: EntityId,
    units: StyleUnits,
) -> StyleResult<HatchLine> {
    const ENTITY: &str = "IfcFillAreaStyleHatching";
    let hatching = view.fill_area_style_hatching(id)?;
    let invalid = |attribute: &'static str, value: String| StyleError::InvalidValue {
        entity: ENTITY.to_owned(),
        id,
        attribute,
        value,
    };

    let angle = hatching.hatch_line_angle()?;
    if !angle.is_finite() {
        return Err(invalid("HatchLineAngle", format!("{angle}")));
    }
    let angle_rad = units.angle(id, angle)?;
    let (spacing, shift) = distance(view, style, id, angle_rad, units)?;

    let point = |attribute: &'static str, point: Option<EntityId>| {
        point
            .map(|point| {
                let coordinates = view
                    .record(point, "IfcCartesianPoint")?
                    .required_numbers("Coordinates", 1)?;
                match coordinates[..] {
                    [x, y] if x.is_finite() && y.is_finite() => {
                        Ok([units.length(point, x)?, units.length(point, y)?])
                    }
                    _ => Err(invalid(attribute, format!("{coordinates:?}"))),
                }
            })
            .transpose()
    };
    let origin = point(
        "PointOfReferenceHatchLine",
        hatching.point_of_reference_hatch_line()?,
    )?
    .unwrap_or([0.0, 0.0]);
    let pattern_start = point("PatternStart", hatching.pattern_start()?)?;

    let appearance = hatching.hatch_line_appearance()?;
    let curve = view.curve_style(appearance)?;
    let dashes = match curve.curve_font()?.map(Value::unwrap_typed) {
        None => None,
        Some(Value::Ref(font)) => Some(dashes(view, style, *font, units, 1.0)?),
        Some(other) => {
            return Err(StyleError::InvalidValue {
                entity: "IfcCurveStyle".to_owned(),
                id: appearance,
                attribute: "CurveFont",
                value: format!("{other:?}"),
            })
        }
    };
    let width = curve
        .curve_width()?
        .map(|width| curve_width(style, appearance, width, units))
        .transpose()?;
    let colour = curve
        .curve_colour()?
        .map(|colour| colour_rgb_of(view, style, colour))
        .transpose()?;

    Ok(HatchLine {
        angle_rad,
        spacing,
        shift,
        origin,
        pattern_start,
        dashes,
        colour,
        width,
    })
}

/// A number in `value`, bare or in its select wrapper.
fn number(value: &Value) -> Option<f64> {
    match value.unwrap_typed() {
        Value::Real(number) => Some(*number),
        Value::Integer(number) => Some(*number as f64),
        _ => None,
    }
}

/// `(spacing, shift)` in metres of `StartOfNextHatchLine` for a hatching at
/// `angle_rad`.
fn distance(
    view: StyleView<'_, '_>,
    style: EntityId,
    id: EntityId,
    angle_rad: f64,
    units: StyleUnits,
) -> StyleResult<(f64, f64)> {
    const ENTITY: &str = "IfcFillAreaStyleHatching";
    let hatching = view.fill_area_style_hatching(id)?;
    let record = view.record(id, ENTITY)?;
    let invalid = |value: String| StyleError::InvalidValue {
        entity: ENTITY.to_owned(),
        id,
        attribute: "StartOfNextHatchLine",
        value,
    };
    let start = hatching.start_of_next_hatch_line()?;
    let length = |value: &Value| match (number(value), value) {
        (Some(value), _) if value.is_finite() && value > 0.0 => Ok((units.length(id, value)?, 0.0)),
        (Some(value), _) => Err(StyleError::OutOfRange {
            entity: "IfcPositiveLengthMeasure",
            id,
            attribute: "StartOfNextHatchLine",
            value,
            minimum: 0.0,
            maximum: f64::INFINITY,
        }),
        (None, other) => Err(invalid(format!("{other:?}"))),
    };
    match start {
        Value::Typed { type_name, value } => {
            if type_name.eq_ignore_ascii_case("IFCPOSITIVELENGTHMEASURE") {
                length(value)
            } else {
                Err(invalid(format!("{start:?}")))
            }
        }
        Value::Real(_) | Value::Integer(_) => length(start),
        Value::Ref(target) => {
            let vector = if view.schema.entity("IfcOneDirectionRepeatFactor").is_some() {
                let actual = entity_type(view, *target)?;
                if view.schema.is_a(&actual, "IfcTwoDirectionRepeatFactor") {
                    return Err(invalid("WR21: a two-direction repeat factor".to_owned()));
                }
                record.check_reference(*target, "IfcOneDirectionRepeatFactor")?;
                view.record(*target, "IfcOneDirectionRepeatFactor")?
                    .required_ref("RepeatFactor", "IfcVector")?
            } else {
                record.check_reference(*target, "IfcVector")?;
                *target
            };
            vector_distance(view, style, id, vector, angle_rad, units)
        }
        other => Err(invalid(format!("{other:?}"))),
    }
}

/// Split the 2D `IfcVector` `vector` along and across the hatch direction.
fn vector_distance(
    view: StyleView<'_, '_>,
    style: EntityId,
    hatching: EntityId,
    vector: EntityId,
    angle_rad: f64,
    units: StyleUnits,
) -> StyleResult<(f64, f64)> {
    let record = view.record(vector, "IfcVector")?;
    let direction = record.required_ref("Orientation", "IfcDirection")?;
    let ratios = view
        .record(direction, "IfcDirection")?
        .required_numbers("DirectionRatios", 2)?;
    let invalid_direction = || StyleError::InvalidValue {
        entity: "IfcDirection".to_owned(),
        id: direction,
        attribute: "DirectionRatios",
        value: format!("{ratios:?}"),
    };
    let (dx, dy) = match ratios[..] {
        [x, y] => (x, y),
        [_, _, _] => {
            return Err(StyleError::UnsupportedFill {
                style,
                entity: vector,
                construct: "a 3D IfcVector".to_owned(),
            })
        }
        _ => return Err(invalid_direction()),
    };
    let norm = dx.hypot(dy);
    if !(norm.is_finite() && norm > 0.0) {
        return Err(invalid_direction());
    }
    let magnitude = record.required_number("Magnitude")?;
    if !(magnitude.is_finite() && magnitude >= 0.0) {
        return Err(StyleError::OutOfRange {
            entity: "IfcVector",
            id: vector,
            attribute: "Magnitude",
            value: magnitude,
            minimum: 0.0,
            maximum: f64::INFINITY,
        });
    }
    let length = units.length(vector, magnitude)?;
    let (vx, vy) = (length * dx / norm, length * dy / norm);
    let (sin, cos) = angle_rad.sin_cos();
    let (mut spacing, mut shift) = (-vx * sin + vy * cos, vx * cos + vy * sin);
    // The same family of lines results from the opposite vector.
    if spacing < 0.0 {
        spacing = -spacing;
        shift = -shift;
    }
    if length == 0.0 {
        return Err(StyleError::InvalidValue {
            entity: "IfcFillAreaStyleHatching".to_owned(),
            id: hatching,
            attribute: "StartOfNextHatchLine",
            value: "zero vector".to_owned(),
        });
    }
    if spacing <= 1e-9 * length {
        return Err(StyleError::InvalidValue {
            entity: "IfcFillAreaStyleHatching".to_owned(),
            id: hatching,
            attribute: "StartOfNextHatchLine",
            value: "vector along the hatch lines".to_owned(),
        });
    }
    Ok((spacing, shift))
}

/// `[visible, invisible]` lengths in metres of the curve font `font`, each
/// multiplied by `scale`; empty for 'continuous'.
fn dashes(
    view: StyleView<'_, '_>,
    style: EntityId,
    font: EntityId,
    units: StyleUnits,
    scale: f64,
) -> StyleResult<Vec<[f64; 2]>> {
    let name = entity_type(view, font)?;
    let is_a = |ancestor| view.schema.is_a(&name, ancestor);
    if is_a("IfcCurveStyleFont") {
        let mut pairs = Vec::new();
        for pattern in view.curve_style_font(font)?.patterns()? {
            let record = view.curve_style_font_pattern(pattern)?;
            let (visible, invisible) = (
                record.visible_segment_length()?,
                record.invisible_segment_length()?,
            );
            if !(visible.is_finite() && visible >= 0.0) {
                return Err(StyleError::OutOfRange {
                    entity: "IfcLengthMeasure",
                    id: pattern,
                    attribute: "VisibleSegmentLength",
                    value: visible,
                    minimum: 0.0,
                    maximum: f64::INFINITY,
                });
            }
            if !(invisible.is_finite() && invisible > 0.0) {
                return Err(StyleError::OutOfRange {
                    entity: "IfcPositiveLengthMeasure",
                    id: pattern,
                    attribute: "InvisibleSegmentLength",
                    value: invisible,
                    minimum: 0.0,
                    maximum: f64::INFINITY,
                });
            }
            pairs.push([
                units.length(pattern, visible)? * scale,
                units.length(pattern, invisible)? * scale,
            ]);
        }
        Ok(pairs)
    } else if is_a("IfcCurveStyleFontAndScaling") {
        let record = view.record(font, "IfcCurveStyleFontAndScaling")?;
        let slot = if record.has_attribute("CurveStyleFont") {
            "CurveStyleFont"
        } else {
            "CurveFont"
        };
        let inner = record.required_ref_select(
            slot,
            "IfcCurveStyleFontSelect",
            &["IfcCurveStyleFont", "IfcPreDefinedCurveFont"],
        )?;
        let scaling = record.positive("CurveFontScaling", "IfcPositiveRatioMeasure")?;
        dashes(view, style, inner, units, scale * scaling)
    } else if is_a("IfcDraughtingPreDefinedCurveFont") {
        let predefined = view
            .record(font, "IfcDraughtingPreDefinedCurveFont")?
            .required_text("Name")?;
        if predefined == "continuous" {
            Ok(Vec::new())
        } else {
            Err(StyleError::UnsupportedFill {
                style,
                entity: font,
                construct: format!("IfcDraughtingPreDefinedCurveFont '{predefined}'"),
            })
        }
    } else if is_a("IfcPreDefinedCurveFont") {
        Err(StyleError::UnsupportedFill {
            style,
            entity: font,
            construct: name.clone(),
        })
    } else {
        Err(StyleError::ReferenceType {
            target: font,
            expected: "IfcCurveFontOrScaledCurveFontSelect",
            actual: name.clone(),
        })
    }
}

/// The `CurveWidth` of `curve` in metres.
fn curve_width(
    style: EntityId,
    curve: EntityId,
    width: &Value,
    units: StyleUnits,
) -> StyleResult<f64> {
    let invalid = || StyleError::InvalidValue {
        entity: "IfcCurveStyle".to_owned(),
        id: curve,
        attribute: "CurveWidth",
        value: format!("{width:?}"),
    };
    let Value::Typed { type_name, value } = width else {
        return Err(invalid());
    };
    if type_name.eq_ignore_ascii_case("IFCPOSITIVELENGTHMEASURE") {
        let measure = number(value).ok_or_else(invalid)?;
        if !(measure.is_finite() && measure > 0.0) {
            return Err(StyleError::OutOfRange {
                entity: "IfcPositiveLengthMeasure",
                id: curve,
                attribute: "CurveWidth",
                value: measure,
                minimum: 0.0,
                maximum: f64::INFINITY,
            });
        }
        return units.length(curve, measure);
    }
    match (
        type_name.eq_ignore_ascii_case("IFCDESCRIPTIVEMEASURE"),
        &**value,
    ) {
        (true, Value::Text(text)) if &**text == "by layer" => Err(StyleError::UnsupportedFill {
            style,
            entity: curve,
            construct: "IfcDescriptiveMeasure 'by layer'".to_owned(),
        }),
        _ => Err(invalid()),
    }
}
