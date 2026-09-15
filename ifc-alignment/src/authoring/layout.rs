//! Alignment roots and their layout children.
//!
//! `IfcAlignment`, `IfcAlignmentHorizontal`, `IfcAlignmentVertical` and
//! `IfcAlignmentCant` are all `IfcProduct` subtypes, so they carry the same
//! seven inherited attributes before adding their own.
//!
//! Nesting is deliberately not written here. `IfcRelNests` is a generic
//! relationship owned by `ifc-author`, and duplicating its construction in
//! this crate would give the same relationship two writers.

use ifc_model::{Entity, EntityId, Transaction, Value};

use super::{finite, guid, invalid};
use crate::error::AlignmentError;
use crate::slot;

/// Fill the seven inherited `IfcProduct` slots.
fn product_attrs(
    type_name: &'static str,
    arity: usize,
    global_id: &str,
    name: Option<&str>,
) -> Result<Vec<Value>, AlignmentError> {
    let mut attrs = vec![Value::Null; arity];
    attrs[slot::product::GLOBAL_ID] = guid(type_name, global_id)?;
    if let Some(name) = name {
        attrs[slot::product::NAME] = Value::Text(name.into());
    }
    Ok(attrs)
}

/// Stage an `IfcAlignment`.
///
/// # Errors
///
/// Refuses a `GlobalId` that is not 22 characters.
pub fn alignment(
    tx: &mut Transaction,
    global_id: &str,
    name: Option<&str>,
    predefined_type: Option<&'static str>,
) -> Result<EntityId, AlignmentError> {
    let mut attrs = product_attrs("IFCALIGNMENT", slot::alignment::ARITY, global_id, name)?;
    if let Some(token) = predefined_type {
        attrs[slot::alignment::PREDEFINED_TYPE] = Value::Enum(token.into());
    }
    Ok(tx.create(Entity::new("IFCALIGNMENT", attrs)))
}

/// Stage an `IfcAlignmentHorizontal`.
///
/// # Errors
///
/// Refuses a `GlobalId` that is not 22 characters.
pub fn horizontal_layout(
    tx: &mut Transaction,
    global_id: &str,
    name: Option<&str>,
) -> Result<EntityId, AlignmentError> {
    let attrs = product_attrs(
        "IFCALIGNMENTHORIZONTAL",
        slot::product::ARITY,
        global_id,
        name,
    )?;
    Ok(tx.create(Entity::new("IFCALIGNMENTHORIZONTAL", attrs)))
}

/// Stage an `IfcAlignmentVertical`.
///
/// # Errors
///
/// Refuses a `GlobalId` that is not 22 characters.
pub fn vertical_layout(
    tx: &mut Transaction,
    global_id: &str,
    name: Option<&str>,
) -> Result<EntityId, AlignmentError> {
    let attrs = product_attrs(
        "IFCALIGNMENTVERTICAL",
        slot::product::ARITY,
        global_id,
        name,
    )?;
    Ok(tx.create(Entity::new("IFCALIGNMENTVERTICAL", attrs)))
}

/// Stage an `IfcAlignmentCant`.
///
/// `rail_head_distance` is the gauge the cant values are measured against.
/// It is mandatory on this entity, and a non-positive value would make the
/// cant angle meaningless, so it is checked rather than written blindly.
///
/// # Errors
///
/// Refuses a `GlobalId` that is not 22 characters, and a rail head distance
/// that is not finite and positive.
pub fn cant_layout(
    tx: &mut Transaction,
    global_id: &str,
    name: Option<&str>,
    rail_head_distance: f64,
) -> Result<EntityId, AlignmentError> {
    finite("IFCALIGNMENTCANT", "RailHeadDistance", rail_head_distance)?;
    if rail_head_distance <= 0.0 {
        return Err(invalid(
            "IFCALIGNMENTCANT",
            "RailHeadDistance",
            "the distance between rail heads must be positive",
        ));
    }
    let mut attrs = product_attrs(
        "IFCALIGNMENTCANT",
        slot::cant_layout::ARITY,
        global_id,
        name,
    )?;
    attrs[slot::cant_layout::RAIL_HEAD_DISTANCE] = Value::Real(rail_head_distance);
    Ok(tx.create(Entity::new("IFCALIGNMENTCANT", attrs)))
}

/// Stage an `IfcAlignmentSegment` wrapping one parameter segment.
///
/// A layout does not nest parameter segments directly: it nests these
/// wrappers, each pointing at its `DesignParameters`. `AlignmentView`'s
/// traversal resolves that reference, so a layout built without wrappers
/// reads back as a malformed segment chain.
///
/// # Errors
///
/// Refuses a `GlobalId` that is not 22 characters.
pub fn alignment_segment(
    tx: &mut Transaction,
    global_id: &str,
    design_parameters: EntityId,
) -> Result<EntityId, AlignmentError> {
    let mut attrs = product_attrs("IFCALIGNMENTSEGMENT", slot::segment::ARITY, global_id, None)?;
    attrs[slot::segment::DESIGN_PARAMETERS] = Value::Ref(design_parameters);
    Ok(tx.create(Entity::new("IFCALIGNMENTSEGMENT", attrs)))
}
