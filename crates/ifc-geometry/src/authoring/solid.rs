//! Swept solids and booleans.
//!
//! A boolean is written *unevaluated*: this module records the operation and
//! its operands, it does not execute it. That preserves exact intent and
//! keeps the bridge free of computation in the write direction too (ADR
//! 0004, as amended by ADR 0011).

use ifc_model::{Entity, EntityId, Transaction, Value};

use crate::error::GeometryError;
use crate::solid::boolean::{slot as boolean_slot, IfcBooleanOperator};
use crate::solid::swept::{extruded_slot, revolved_slot, swept_area_slot};

use super::invalid;

/// Stage an `IfcExtrudedAreaSolid`.
///
/// # Errors
///
/// Refuses a non-positive or non-finite depth: EXPRESS declares `Depth` as
/// `IfcPositiveLengthMeasure`, and a zero-depth extrusion is a face claiming
/// to be a solid.
pub fn extruded_area_solid(
    tx: &mut Transaction,
    swept_area: EntityId,
    position: Option<EntityId>,
    extruded_direction: EntityId,
    depth: f64,
) -> Result<EntityId, GeometryError> {
    if !depth.is_finite() || depth <= 0.0 {
        return Err(invalid(
            "IFCEXTRUDEDAREASOLID",
            "Depth",
            format!("expected a positive length, got {depth}"),
        ));
    }
    let mut attrs = vec![Value::Null; 4];
    attrs[swept_area_slot::SWEPT_AREA] = Value::Ref(swept_area);
    attrs[swept_area_slot::POSITION] = position.map_or(Value::Null, Value::Ref);
    attrs[extruded_slot::EXTRUDED_DIRECTION] = Value::Ref(extruded_direction);
    attrs[extruded_slot::DEPTH] = Value::Real(depth);
    Ok(tx.create(Entity::new("IFCEXTRUDEDAREASOLID", attrs)))
}

/// Stage an `IfcRevolvedAreaSolid`.
///
/// `angle` is in radians, matching the unit-normalised form the reader
/// produces. A caller working in degrees converts before this point.
///
/// # Errors
///
/// Refuses a non-finite angle, and one outside `(0, 2*pi]`. Zero sweeps
/// nothing; beyond a full turn the solid would self-overlap, which IFC does
/// not define.
pub fn revolved_area_solid(
    tx: &mut Transaction,
    swept_area: EntityId,
    position: Option<EntityId>,
    axis: EntityId,
    angle: f64,
) -> Result<EntityId, GeometryError> {
    if !angle.is_finite() || angle <= 0.0 || angle > std::f64::consts::TAU {
        return Err(invalid(
            "IFCREVOLVEDAREASOLID",
            "Angle",
            format!("expected an angle in (0, 2pi] radians, got {angle}"),
        ));
    }
    let mut attrs = vec![Value::Null; 4];
    attrs[swept_area_slot::SWEPT_AREA] = Value::Ref(swept_area);
    attrs[swept_area_slot::POSITION] = position.map_or(Value::Null, Value::Ref);
    attrs[revolved_slot::AXIS] = Value::Ref(axis);
    attrs[revolved_slot::ANGLE] = Value::Real(angle);
    Ok(tx.create(Entity::new("IFCREVOLVEDAREASOLID", attrs)))
}

/// Stage an `IfcBooleanResult`.
///
/// The operation is **recorded, not executed**. The file states the operands
/// and the operator; evaluating them is a kernel concern belonging to a
/// consumer, so any kernel -- CGAL, OCCT, Axiolid -- can resolve the same
/// authored record later.
///
/// Operand order matters for `DIFFERENCE`: `first` minus `second`. The
/// operator type carries that distinction, so it is not restated here.
pub fn boolean_result(
    tx: &mut Transaction,
    operator: IfcBooleanOperator,
    first: EntityId,
    second: EntityId,
) -> Result<EntityId, GeometryError> {
    if first == second {
        return Err(invalid(
            "IFCBOOLEANRESULT",
            "SecondOperand",
            "both operands are the same entity",
        ));
    }
    let mut attrs = vec![Value::Null; 3];
    attrs[boolean_slot::OPERATOR] = Value::Enum(operator.as_token().into());
    attrs[boolean_slot::FIRST_OPERAND] = Value::Ref(first);
    attrs[boolean_slot::SECOND_OPERAND] = Value::Ref(second);
    Ok(tx.create(Entity::new("IFCBOOLEANRESULT", attrs)))
}
