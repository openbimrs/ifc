//! Boolean and CSG lowering: exact operation trees, never evaluated here.
//!
//! # Why this family proves the session design
//!
//! A boolean is the first family whose operands are themselves lowered
//! solids. Under the old one-graph-per-call shape its operands lived in two
//! unrelated graphs whose `NodeId`s could not legally be combined. Here both
//! operands append into the caller's session, so the operation node can refer
//! to them directly.

use axiolid_core::BooleanOperator;
use axiolid_model::{GeometryNode, NodeId, SolidOperation};
use ifc_model::EntityId;

use crate::error::{GeometryError, GeometryResult};
use crate::lower::session::LoweringSession;
use crate::transform::Transform;

mod slot {
    pub const OPERATOR: usize = 0;
    pub const FIRST_OPERAND: usize = 1;
    pub const SECOND_OPERAND: usize = 2;
}

/// Family label used for boolean memoization.
const BOOLEAN: &str = "boolean";

/// Map an IFC operator enumeration to the neutral operator.
///
/// `IfcBooleanClippingResult` is constrained by its where-rule to DIFFERENCE,
/// but the enumeration itself is shared with `IfcBooleanResult`, so the
/// mapping lives here once.
fn operator(entity: EntityId, token: &str) -> GeometryResult<BooleanOperator> {
    match token.trim_matches('.').to_ascii_uppercase().as_str() {
        "UNION" => Ok(BooleanOperator::Union),
        "INTERSECTION" => Ok(BooleanOperator::Intersection),
        "DIFFERENCE" => Ok(BooleanOperator::Difference),
        _ => Err(GeometryError::Degenerate {
            entity,
            type_name: "IFCBOOLEANRESULT".to_string(),
            detail: format!("unknown boolean operator {token}"),
        }),
    }
}

/// Append one `IfcBooleanResult` (or clipping result) and return its node.
///
/// Both operands are lowered into the same session before the operation node
/// is appended, which is exactly the ordering the append-only builder needs:
/// every reference is already a prior node.
pub fn lower_boolean_result_node(
    session: &mut LoweringSession<'_>,
    id: EntityId,
    frame: Transform,
) -> GeometryResult<NodeId> {
    if let Some(node) = session.memoized(id, BOOLEAN, frame) {
        return Ok(node);
    }

    session.enter(id, "boolean")?;
    let result = lower_operands(session, id, frame);
    session.exit(id);
    let node = result?;

    session.memoize(id, BOOLEAN, frame, node);
    Ok(node)
}

fn lower_operands(
    session: &mut LoweringSession<'_>,
    id: EntityId,
    frame: Transform,
) -> GeometryResult<NodeId> {
    let slots = session.slots(id)?;
    let operator_token = slots
        .opt_enum(slot::OPERATOR)
        .ok_or(GeometryError::MissingAttribute {
            entity: id,
            type_name: slots.type_name().to_string(),
            attribute: "Operator",
        })?
        .to_string();
    let operator = operator(id, &operator_token)?;
    let first = slots.req_ref(slot::FIRST_OPERAND, "FirstOperand")?;
    let second = slots.req_ref(slot::SECOND_OPERAND, "SecondOperand")?;

    let left = session.lower_operand(first, frame)?;
    let right = session.lower_operand(second, frame)?;

    session.node_for(
        id,
        GeometryNode::SolidOperation(SolidOperation::Boolean {
            left,
            right,
            operator,
        }),
    )
}
