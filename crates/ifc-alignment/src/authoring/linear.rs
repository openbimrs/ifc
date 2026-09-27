//! Staging the two linear supertypes.
//!
//! Both are concrete despite being supertypes: ONEOF constrains
//! which subtype an instance may also be, it does not bar writing
//! the supertype itself. A file states a linear element directly
//! when it carries no alignment-specific role.
//!
//! Neither adds an attribute of its own, so both use the seven
//! inherited `IfcProduct` slots.

use ifc_model::{Entity, EntityId, Transaction, Value};

use super::guid;
use crate::error::AlignmentError;
use crate::slot;

/// Stage an `IfcLinearElement`.
///
/// IFC4X3 only: IFC4 does not declare it, and `guid` resolving
/// against a schema without the entity is the caller's signal.
///
/// # Errors
///
/// Refuses a `GlobalId` that is not 22 characters.
pub fn linear_element(
    tx: &mut Transaction,
    global_id: &str,
    name: Option<&str>,
    placement: Option<EntityId>,
) -> Result<EntityId, AlignmentError> {
    const ENTITY: &str = "IFCLINEARELEMENT";
    let mut attrs = vec![Value::Null; slot::product::ARITY];
    attrs[slot::product::GLOBAL_ID] = guid(ENTITY, global_id)?;
    if let Some(name) = name {
        attrs[slot::product::NAME] = Value::Text(name.into());
    }
    if let Some(placement) = placement {
        attrs[slot::product::OBJECT_PLACEMENT] = Value::Ref(placement);
    }
    Ok(tx.create(Entity::new(ENTITY, attrs)))
}

/// Stage an `IfcLinearPositioningElement`.
///
/// `IfcPositioningElement.HasPlacement` makes the placement
/// required even though the slot is declared OPTIONAL: a
/// positioning element that positions nothing is the one thing
/// this entity cannot be. The sibling `linear_element` has no
/// such rule, which is why the two cannot share a path.
///
/// IFC4X3 only.
///
/// # Errors
///
/// Refuses a `GlobalId` that is not 22 characters, and refuses an
/// absent `placement`.
pub fn linear_positioning_element(
    tx: &mut Transaction,
    global_id: &str,
    name: Option<&str>,
    placement: EntityId,
) -> Result<EntityId, AlignmentError> {
    const ENTITY: &str = "IFCLINEARPOSITIONINGELEMENT";
    let mut attrs = vec![Value::Null; slot::product::ARITY];
    attrs[slot::product::GLOBAL_ID] = guid(ENTITY, global_id)?;
    if let Some(name) = name {
        attrs[slot::product::NAME] = Value::Text(name.into());
    }
    attrs[slot::product::OBJECT_PLACEMENT] = Value::Ref(placement);
    Ok(tx.create(Entity::new(ENTITY, attrs)))
}
