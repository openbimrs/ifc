//! IFC4x3 vertical alignment segment parameters.

use ifc_model::value::Value;
use ifc_model::{EntityId, Model};

use crate::error::{AlignmentError, AlignmentResult};
use crate::horizontal::AlignmentUnits;

#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VerticalSegmentType {
    ConstantGradient,
    CircularArc,
    ParabolicArc,
    Clothoid,
    UserDefined,
    NotDefined,
    Other(String),
}

impl VerticalSegmentType {
    pub(crate) fn source_name(&self) -> &str {
        match self {
            Self::CircularArc => "CIRCULARARC",
            Self::Clothoid => "CLOTHOID",
            Self::ConstantGradient => "CONSTANTGRADIENT",
            Self::ParabolicArc => "PARABOLICARC",
            Self::UserDefined => "USERDEFINED",
            Self::NotDefined => "NOTDEFINED",
            Self::Other(name) => name,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct VerticalSegment {
    pub entity: EntityId,
    pub start_dist_along: f64,
    pub horizontal_length: f64,
    pub start_height: f64,
    pub start_gradient: f64,
    pub end_gradient: f64,
    pub radius_of_curvature: Option<f64>,
    pub predefined_type: VerticalSegmentType,
}

pub fn read_vertical_segment(
    model: &Model,
    id: EntityId,
    units: AlignmentUnits,
) -> AlignmentResult<VerticalSegment> {
    validate_units(units)?;
    let entity = model
        .get(id)
        .ok_or(AlignmentError::MissingEntity { entity: id })?;
    if !entity
        .type_name
        .eq_ignore_ascii_case("IFCALIGNMENTVERTICALSEGMENT")
    {
        return Err(AlignmentError::WrongType {
            entity: id,
            expected: "IFCALIGNMENTVERTICALSEGMENT",
            actual: entity.type_name.to_string(),
        });
    }
    // IFC4X3_ADD2: inherited StartTag/EndTag are slots 0..1; this declaration
    // contributes StartDistAlong through PredefinedType at slots 2..8.
    let values = &entity.attributes;
    let start_dist_along = length(number(values, id, 2, "StartDistAlong")?, units);
    let horizontal_length = length(number(values, id, 3, "HorizontalLength")?, units);
    let start_height = length(number(values, id, 4, "StartHeight")?, units);
    let start_gradient = number(values, id, 5, "StartGradient")?;
    let end_gradient = number(values, id, 6, "EndGradient")?;
    let radius_of_curvature =
        optional_number(values, id, 7, "RadiusOfCurvature")?.map(|value| length(value, units));
    let predefined_type = parse_type(enum_name(values, id, 8, "PredefinedType")?);

    let finite = [
        start_dist_along,
        horizontal_length,
        start_height,
        start_gradient,
        end_gradient,
    ]
    .into_iter()
    .all(f64::is_finite)
        && radius_of_curvature.is_none_or(f64::is_finite);
    if !finite || horizontal_length < 0.0 {
        return Err(AlignmentError::InvalidSegment {
            entity: id,
            detail: "vertical parameters must be finite and horizontal length non-negative",
        });
    }
    let needs_radius = matches!(
        predefined_type,
        VerticalSegmentType::CircularArc | VerticalSegmentType::ParabolicArc
    );
    if needs_radius != radius_of_curvature.is_some() {
        return Err(AlignmentError::InvalidSegment {
            entity: id,
            detail: "radius is required exactly for circular and parabolic vertical segments",
        });
    }

    Ok(VerticalSegment {
        entity: id,
        start_dist_along,
        horizontal_length,
        start_height,
        start_gradient,
        end_gradient,
        radius_of_curvature,
        predefined_type,
    })
}

fn parse_type(name: &str) -> VerticalSegmentType {
    match name.to_ascii_uppercase().as_str() {
        "CONSTANTGRADIENT" => VerticalSegmentType::ConstantGradient,
        "CIRCULARARC" => VerticalSegmentType::CircularArc,
        "PARABOLICARC" => VerticalSegmentType::ParabolicArc,
        "CLOTHOID" => VerticalSegmentType::Clothoid,
        "USERDEFINED" => VerticalSegmentType::UserDefined,
        "NOTDEFINED" => VerticalSegmentType::NotDefined,
        _ => VerticalSegmentType::Other(name.to_string()),
    }
}

fn validate_units(units: AlignmentUnits) -> AlignmentResult<()> {
    if !units.length_to_metres.is_finite() || units.length_to_metres <= 0.0 {
        return Err(AlignmentError::InvalidUnits {
            detail: "length factor must be finite and positive",
        });
    }
    Ok(())
}

fn length(value: f64, units: AlignmentUnits) -> f64 {
    value * units.length_to_metres
}

fn number(values: &[Value], id: EntityId, slot: usize, name: &'static str) -> AlignmentResult<f64> {
    match values.get(slot) {
        Some(Value::Real(value)) => Ok(*value),
        Some(Value::Integer(value)) => Ok(*value as f64),
        _ => Err(AlignmentError::InvalidAttribute {
            entity: id,
            index: slot,
            name,
        }),
    }
}

fn optional_number(
    values: &[Value],
    id: EntityId,
    slot: usize,
    name: &'static str,
) -> AlignmentResult<Option<f64>> {
    match values.get(slot) {
        Some(Value::Null) => Ok(None),
        Some(Value::Real(value)) => Ok(Some(*value)),
        Some(Value::Integer(value)) => Ok(Some(*value as f64)),
        _ => Err(AlignmentError::InvalidAttribute {
            entity: id,
            index: slot,
            name,
        }),
    }
}

fn enum_name<'a>(
    values: &'a [Value],
    id: EntityId,
    slot: usize,
    name: &'static str,
) -> AlignmentResult<&'a str> {
    match values.get(slot) {
        Some(Value::Enum(value)) => Ok(value),
        _ => Err(AlignmentError::InvalidAttribute {
            entity: id,
            index: slot,
            name,
        }),
    }
}
