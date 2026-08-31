//! IFC4x3 cant alignment segment parameters.

use ifc_model::value::Value;
use ifc_model::{EntityId, Model};

use crate::error::{AlignmentError, AlignmentResult};
use crate::horizontal::AlignmentUnits;

#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CantSegmentType {
    BlossCurve,
    ConstantCant,
    CosineCurve,
    HelmertCurve,
    LinearTransition,
    SineCurve,
    VienneseBend,
    UserDefined,
    NotDefined,
    Other(String),
}

#[derive(Debug, Clone, PartialEq)]
pub struct CantSegment {
    pub entity: EntityId,
    pub start_dist_along: f64,
    pub horizontal_length: f64,
    pub start_cant_left: f64,
    pub end_cant_left: Option<f64>,
    pub start_cant_right: f64,
    pub end_cant_right: Option<f64>,
    pub predefined_type: CantSegmentType,
}

pub fn read_cant_segment(
    model: &Model,
    id: EntityId,
    units: AlignmentUnits,
) -> AlignmentResult<CantSegment> {
    validate_units(units)?;
    let entity = model
        .get(id)
        .ok_or(AlignmentError::MissingEntity { entity: id })?;
    if !entity
        .type_name
        .eq_ignore_ascii_case("IFCALIGNMENTCANTSEGMENT")
    {
        return Err(AlignmentError::WrongType {
            entity: id,
            expected: "IFCALIGNMENTCANTSEGMENT",
            actual: entity.type_name.to_string(),
        });
    }
    // IFC4X3_ADD2: inherited StartTag/EndTag are slots 0..1; this declaration
    // contributes StartDistAlong through PredefinedType at slots 2..8.
    let values = &entity.attributes;
    let start_dist_along = length(number(values, id, 2, "StartDistAlong")?, units);
    let horizontal_length = length(number(values, id, 3, "HorizontalLength")?, units);
    let start_cant_left = length(number(values, id, 4, "StartCantLeft")?, units);
    let end_cant_left =
        optional_number(values, id, 5, "EndCantLeft")?.map(|value| length(value, units));
    let start_cant_right = length(number(values, id, 6, "StartCantRight")?, units);
    let end_cant_right =
        optional_number(values, id, 7, "EndCantRight")?.map(|value| length(value, units));
    let predefined_type = parse_type(enum_name(values, id, 8, "PredefinedType")?);

    let finite = [
        start_dist_along,
        horizontal_length,
        start_cant_left,
        start_cant_right,
    ]
    .into_iter()
    .all(f64::is_finite)
        && end_cant_left.is_none_or(f64::is_finite)
        && end_cant_right.is_none_or(f64::is_finite);
    if !finite || horizontal_length < 0.0 {
        return Err(AlignmentError::InvalidSegment {
            entity: id,
            detail: "cant parameters must be finite and horizontal length non-negative",
        });
    }
    let ends_are_paired = end_cant_left.is_some() == end_cant_right.is_some();
    if !ends_are_paired {
        return Err(AlignmentError::InvalidSegment {
            entity: id,
            detail: "left and right end cant must both be supplied or both omitted",
        });
    }

    Ok(CantSegment {
        entity: id,
        start_dist_along,
        horizontal_length,
        start_cant_left,
        end_cant_left,
        start_cant_right,
        end_cant_right,
        predefined_type,
    })
}

fn parse_type(name: &str) -> CantSegmentType {
    match name.to_ascii_uppercase().as_str() {
        "BLOSSCURVE" => CantSegmentType::BlossCurve,
        "CONSTANTCANT" => CantSegmentType::ConstantCant,
        "COSINECURVE" => CantSegmentType::CosineCurve,
        "HELMERTCURVE" => CantSegmentType::HelmertCurve,
        "LINEARTRANSITION" => CantSegmentType::LinearTransition,
        "SINECURVE" => CantSegmentType::SineCurve,
        "VIENNESEBEND" => CantSegmentType::VienneseBend,
        "USERDEFINED" => CantSegmentType::UserDefined,
        "NOTDEFINED" => CantSegmentType::NotDefined,
        _ => CantSegmentType::Other(name.to_string()),
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
