//! Profiles: the 2D sections that swept solids extrude.
//!
//! Slots come from [`crate::slots::profile_slot`], the same definition the
//! lowerer reads, so a layout correction serves both directions.

use ifc_model::{Entity, EntityId, Transaction, Value};

use crate::curve::polyline::polyline_slot;
use crate::error::GeometryError;
use crate::slots::profile_slot as slot;

use super::{invalid, refs, require_finite};

/// `IfcProfileTypeEnum`: whether the profile bounds an area or is a bare curve.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProfileType {
    /// `.AREA.` -- a closed section that can be swept into a solid.
    Area,
    /// `.CURVE.` -- an open section; sweeping it yields a surface, not a solid.
    Curve,
}

impl ProfileType {
    /// The EXPRESS token.
    pub(super) fn token(self) -> &'static str {
        match self {
            Self::Area => "AREA",
            Self::Curve => "CURVE",
        }
    }
}

/// Stage an `IfcPolyline`.
///
/// # Errors
///
/// Refuses fewer than two points: a polyline through one point has no
/// length and no direction, and every consumer would have to special-case it.
pub fn polyline(tx: &mut Transaction, points: &[EntityId]) -> Result<EntityId, GeometryError> {
    if points.len() < 2 {
        return Err(invalid(
            "IFCPOLYLINE",
            "Points",
            format!("expected at least 2 points, got {}", points.len()),
        ));
    }
    let mut attrs = vec![Value::Null];
    attrs[polyline_slot::POINTS] = refs(points);
    Ok(tx.create(Entity::new("IFCPOLYLINE", attrs)))
}

/// Stage an `IfcRectangleProfileDef`.
///
/// # Errors
///
/// Refuses a non-positive or non-finite dimension. IFC declares these as
/// `IfcPositiveLengthMeasure`, so zero is invalid, not merely degenerate.
pub fn rectangle_profile(
    tx: &mut Transaction,
    name: Option<&str>,
    position: Option<EntityId>,
    x_dim: f64,
    y_dim: f64,
) -> Result<EntityId, GeometryError> {
    positive("IFCRECTANGLEPROFILEDEF", "XDim", x_dim)?;
    positive("IFCRECTANGLEPROFILEDEF", "YDim", y_dim)?;
    let mut attrs = vec![Value::Null; 5];
    attrs[slot::PROFILE_TYPE] = Value::Enum(ProfileType::Area.token().into());
    attrs[slot::PROFILE_NAME] = name.map_or(Value::Null, |n| Value::Text(n.into()));
    attrs[slot::POSITION] = position.map_or(Value::Null, Value::Ref);
    attrs[slot::X_DIM] = Value::Real(x_dim);
    attrs[slot::Y_DIM] = Value::Real(y_dim);
    Ok(tx.create(Entity::new("IFCRECTANGLEPROFILEDEF", attrs)))
}

/// Stage an `IfcCircleProfileDef`.
///
/// # Errors
///
/// Refuses a non-positive or non-finite radius.
pub fn circle_profile(
    tx: &mut Transaction,
    name: Option<&str>,
    position: Option<EntityId>,
    radius: f64,
) -> Result<EntityId, GeometryError> {
    positive("IFCCIRCLEPROFILEDEF", "Radius", radius)?;
    let mut attrs = vec![Value::Null; 4];
    attrs[slot::PROFILE_TYPE] = Value::Enum(ProfileType::Area.token().into());
    attrs[slot::PROFILE_NAME] = name.map_or(Value::Null, |n| Value::Text(n.into()));
    attrs[slot::POSITION] = position.map_or(Value::Null, Value::Ref);
    attrs[slot::RADIUS] = Value::Real(radius);
    Ok(tx.create(Entity::new("IFCCIRCLEPROFILEDEF", attrs)))
}

/// Stage an `IfcArbitraryClosedProfileDef` from an outer curve.
///
/// The curve must already be closed. Closing it here would fabricate a
/// segment the caller never described -- the same reason the lowerer refuses
/// to close an open curve when reading.
pub fn arbitrary_closed_profile(
    tx: &mut Transaction,
    name: Option<&str>,
    outer_curve: EntityId,
) -> EntityId {
    let mut attrs = vec![Value::Null; 3];
    attrs[slot::PROFILE_TYPE] = Value::Enum(ProfileType::Area.token().into());
    attrs[slot::PROFILE_NAME] = name.map_or(Value::Null, |n| Value::Text(n.into()));
    attrs[slot::OUTER_CURVE] = Value::Ref(outer_curve);
    tx.create(Entity::new("IFCARBITRARYCLOSEDPROFILEDEF", attrs))
}

/// Refuse a measure IFC declares as `IfcPositiveLengthMeasure`.
fn positive(
    type_name: &'static str,
    attribute: &'static str,
    value: f64,
) -> Result<(), GeometryError> {
    require_finite(type_name, attribute, &[value])?;
    if value <= 0.0 {
        return Err(invalid(
            type_name,
            attribute,
            format!("expected a positive length, got {value}"),
        ));
    }
    Ok(())
}
