//! IFC4x3 horizontal alignment segment parameters.

use axiolid_core::Point2;
use ifc_model::value::Value;
use ifc_model::{Entity, EntityId, Model};

use crate::error::{AlignmentError, AlignmentResult};

/// Project unit conversion applied exactly once while reading a segment.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AlignmentUnits {
    /// Factor converting a length attribute's stored value to metres.
    pub length_to_metres: f64,
    /// Factor converting an angle attribute's stored value to radians.
    pub angle_to_radians: f64,
}

impl AlignmentUnits {
    fn validate(self) -> AlignmentResult<Self> {
        if !self.length_to_metres.is_finite() || self.length_to_metres <= 0.0 {
            return Err(AlignmentError::InvalidUnits {
                detail: "length factor must be finite and positive",
            });
        }
        if !self.angle_to_radians.is_finite() || self.angle_to_radians <= 0.0 {
            return Err(AlignmentError::InvalidUnits {
                detail: "angle factor must be finite and positive",
            });
        }
        Ok(self)
    }
}

/// IFC4x3 horizontal segment kind, preserving the declaration rather than guessing.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HorizontalSegmentType {
    /// `LINE`: a straight tangent segment (zero curvature throughout).
    Line,
    /// `CIRCULARARC`: constant-radius arc; `StartRadiusOfCurvature` and
    /// `EndRadiusOfCurvature` are equal.
    CircularArc,
    /// A transition-spiral `PredefinedType` token (e.g. `CLOTHOID`,
    /// `CUBIC`, `BLOSS`), preserved verbatim -- which spiral laws this crate
    /// can lower exactly is decided downstream, not by this type.
    Transition(String),
    /// `USERDEFINED`: an author-supplied law outside the enumerated set.
    UserDefined,
    /// `NOTDEFINED`: no horizontal segment kind was declared.
    NotDefined,
}

impl HorizontalSegmentType {
    /// The `PredefinedType` enumeration token this variant was read from.
    pub fn source_name(&self) -> &str {
        match self {
            Self::Line => "LINE",
            Self::CircularArc => "CIRCULARARC",
            Self::Transition(name) => name,
            Self::UserDefined => "USERDEFINED",
            Self::NotDefined => "NOTDEFINED",
        }
    }
}

/// Resolved IFC4x3 `IfcAlignmentHorizontalSegment` parameters in SI units.
#[derive(Debug, Clone, PartialEq)]
pub struct HorizontalSegment {
    /// The `IfcAlignmentHorizontalSegment` entity this was read from.
    pub entity: EntityId,
    /// `StartPoint`: the segment's start, resolved from the referenced
    /// `IfcCartesianPoint`.
    pub start_point: Point2,
    /// `StartDirection`: bearing at the segment start, in radians.
    pub start_direction: f64,
    /// `StartRadiusOfCurvature`.
    pub start_radius: f64,
    /// `EndRadiusOfCurvature`.
    pub end_radius: f64,
    /// `SegmentLength`: arc length along the segment. Must be positive.
    pub segment_length: f64,
    /// `GravityCenterLineHeight`, when authored (rail/cant-related, not
    /// used by exact lowering).
    pub gravity_center_line_height: Option<f64>,
    /// `PredefinedType`: which horizontal segment kind this is.
    pub segment_type: HorizontalSegmentType,
}

// IFC4X3_ADD2 EXPRESS order: inherited StartTag/EndTag occupy slots 0..1;
// IfcAlignmentHorizontalSegment declares the fields below in slots 2..8.
const START_POINT: usize = 2;
const START_DIRECTION: usize = 3;
const START_RADIUS: usize = 4;
const END_RADIUS: usize = 5;
const SEGMENT_LENGTH: usize = 6;
const GRAVITY_CENTER_LINE_HEIGHT: usize = 7;
const PREDEFINED_TYPE: usize = 8;

/// Read one `IfcAlignmentHorizontalSegment` referenced by `id`.
///
/// Fails if `units` is non-finite or non-positive, `id` is missing or not an
/// `IfcAlignmentHorizontalSegment`, `StartPoint` does not resolve to an
/// `IfcCartesianPoint` with at least two coordinates, an attribute is
/// missing or the wrong kind, any value is non-finite, or `SegmentLength`
/// is not positive.
pub fn read_horizontal_segment(
    model: &Model,
    id: EntityId,
    units: AlignmentUnits,
) -> AlignmentResult<HorizontalSegment> {
    let units = units.validate()?;
    let entity = model
        .get(id)
        .ok_or(AlignmentError::MissingEntity { entity: id })?;
    if !entity.is_type("IFCALIGNMENTHORIZONTALSEGMENT") {
        return Err(AlignmentError::WrongType {
            entity: id,
            expected: "IFCALIGNMENTHORIZONTALSEGMENT",
            actual: entity.type_name.to_string(),
        });
    }
    let point_ref = required_ref(entity, id, START_POINT, "StartPoint")?;
    let point = model
        .get(point_ref)
        .ok_or(AlignmentError::MissingEntity { entity: point_ref })?;
    if !point.is_type("IFCCARTESIANPOINT") {
        return Err(AlignmentError::WrongType {
            entity: point_ref,
            expected: "IFCCARTESIANPOINT",
            actual: point.type_name.to_string(),
        });
    }
    let coordinates =
        point
            .attribute(0)
            .and_then(Value::as_list)
            .ok_or(AlignmentError::InvalidAttribute {
                entity: point_ref,
                index: 0,
                name: "Coordinates",
            })?;
    if coordinates.len() < 2 {
        return Err(AlignmentError::InvalidAttribute {
            entity: point_ref,
            index: 0,
            name: "Coordinates",
        });
    }
    let x = coordinates[0]
        .unwrap_typed()
        .as_f64()
        .ok_or(AlignmentError::InvalidAttribute {
            entity: point_ref,
            index: 0,
            name: "Coordinates",
        })?
        * units.length_to_metres;
    let y = coordinates[1]
        .unwrap_typed()
        .as_f64()
        .ok_or(AlignmentError::InvalidAttribute {
            entity: point_ref,
            index: 0,
            name: "Coordinates",
        })?
        * units.length_to_metres;

    let start_direction =
        required_number(entity, id, START_DIRECTION, "StartDirection")? * units.angle_to_radians;
    let start_radius = required_number(entity, id, START_RADIUS, "StartRadiusOfCurvature")?
        * units.length_to_metres;
    let end_radius =
        required_number(entity, id, END_RADIUS, "EndRadiusOfCurvature")? * units.length_to_metres;
    let segment_length =
        required_number(entity, id, SEGMENT_LENGTH, "SegmentLength")? * units.length_to_metres;
    let gravity_center_line_height = optional_number(
        entity,
        id,
        GRAVITY_CENTER_LINE_HEIGHT,
        "GravityCenterLineHeight",
    )?
    .map(|value| value * units.length_to_metres);
    let source_type = required_enum(entity, id, PREDEFINED_TYPE, "PredefinedType")?;
    let segment_type = match source_type {
        "LINE" => HorizontalSegmentType::Line,
        "CIRCULARARC" => HorizontalSegmentType::CircularArc,
        "USERDEFINED" => HorizontalSegmentType::UserDefined,
        "NOTDEFINED" => HorizontalSegmentType::NotDefined,
        other => HorizontalSegmentType::Transition(other.to_owned()),
    };

    let values = [
        x,
        y,
        start_direction,
        start_radius,
        end_radius,
        segment_length,
    ];
    if values.iter().any(|value| !value.is_finite())
        || gravity_center_line_height.is_some_and(|value| !value.is_finite())
        || segment_length <= 0.0
    {
        return Err(AlignmentError::InvalidSegment {
            entity: id,
            detail: "coordinates and parameters must be finite and SegmentLength positive",
        });
    }

    Ok(HorizontalSegment {
        entity: id,
        start_point: Point2::new(x, y),
        start_direction,
        start_radius,
        end_radius,
        segment_length,
        gravity_center_line_height,
        segment_type,
    })
}

fn required_ref(
    entity: &Entity,
    id: EntityId,
    index: usize,
    name: &'static str,
) -> AlignmentResult<EntityId> {
    match entity.attribute(index) {
        None | Some(Value::Null) => Err(AlignmentError::MissingAttribute {
            entity: id,
            index,
            name,
        }),
        Some(Value::Ref(reference)) => Ok(*reference),
        _ => Err(AlignmentError::InvalidAttribute {
            entity: id,
            index,
            name,
        }),
    }
}

fn required_number(
    entity: &Entity,
    id: EntityId,
    index: usize,
    name: &'static str,
) -> AlignmentResult<f64> {
    match entity.attribute(index) {
        None | Some(Value::Null) => Err(AlignmentError::MissingAttribute {
            entity: id,
            index,
            name,
        }),
        Some(value) => value
            .unwrap_typed()
            .as_f64()
            .ok_or(AlignmentError::InvalidAttribute {
                entity: id,
                index,
                name,
            }),
    }
}

fn optional_number(
    entity: &Entity,
    id: EntityId,
    index: usize,
    name: &'static str,
) -> AlignmentResult<Option<f64>> {
    match entity.attribute(index) {
        None | Some(Value::Null) => Ok(None),
        Some(value) => {
            value
                .unwrap_typed()
                .as_f64()
                .map(Some)
                .ok_or(AlignmentError::InvalidAttribute {
                    entity: id,
                    index,
                    name,
                })
        }
    }
}

fn required_enum<'a>(
    entity: &'a Entity,
    id: EntityId,
    index: usize,
    name: &'static str,
) -> AlignmentResult<&'a str> {
    match entity.attribute(index) {
        Some(Value::Enum(value)) => Ok(value),
        None | Some(Value::Null) => Err(AlignmentError::MissingAttribute {
            entity: id,
            index,
            name,
        }),
        _ => Err(AlignmentError::InvalidAttribute {
            entity: id,
            index,
            name,
        }),
    }
}
