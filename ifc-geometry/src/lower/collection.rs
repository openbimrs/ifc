//! Loose geometry collections: sets and surface models.
//!
//! # Why these are collections and not solids
//!
//! `IfcShellBasedSurfaceModel` and `IfcFaceBasedSurfaceModel` describe
//! SURFACES, not volumes. Their shells may even all be closed, but the entity
//! still declares a surface model and is not a legal boolean operand.
//! Promoting one to a `BRep` would let a quantity takeoff report a volume the
//! file never claimed, so each shell lowers on its own and the members stay in
//! a `Collection`.
//!
//! `IfcGeometricSet` is looser still: its members are points, curves or
//! surfaces, mixed freely within one dimensionality. Each element therefore
//! dispatches individually rather than being assumed homogeneous.

use axiolid_model::{GeometryNode, NodeId};
use ifc_model::EntityId;

use crate::error::GeometryResult;
use crate::lower::curve::lower_curve_node;
use crate::lower::session::LoweringSession;
use crate::lower::surface::lower_surface_node;
use crate::select::is_a;
use crate::solid::surface_model::{FaceBasedSurfaceModel, GeometricSet, ShellBasedSurfaceModel};
use crate::transform::Transform;

/// Chain kind reported when a collection nests too deeply or cycles.
const KIND: &str = "geometric collection";

/// Lower a geometric set or surface model into a `Collection` node.
pub fn lower_collection_node(
    session: &mut LoweringSession<'_>,
    id: EntityId,
    frame: Transform,
) -> GeometryResult<NodeId> {
    if let Some(node) = session.memoized(id, KIND, frame) {
        return Ok(node);
    }
    session.enter(id, KIND)?;
    let result = build(session, id, frame);
    session.exit(id);
    let node = result?;
    session.memoize(id, KIND, frame, node);
    Ok(node)
}

fn build(
    session: &mut LoweringSession<'_>,
    id: EntityId,
    frame: Transform,
) -> GeometryResult<NodeId> {
    let entity = session.entity(id, id)?;
    let type_name = session.type_name(id)?.to_ascii_uppercase();
    let members = match type_name.as_str() {
        "IFCSHELLBASEDSURFACEMODEL" => ShellBasedSurfaceModel::new(id, entity).shells()?,
        "IFCFACEBASEDSURFACEMODEL" => FaceBasedSurfaceModel::new(id, entity).face_sets()?,
        "IFCGEOMETRICSET" | "IFCGEOMETRICCURVESET" => GeometricSet::new(id, entity).elements()?,
        other => return Err(session.unsupported(id, other, "geometric collection family")),
    };

    let mut nodes = Vec::with_capacity(members.len());
    for member in members {
        nodes.push(member_node(session, id, member, frame)?);
    }
    session.node_for(id, GeometryNode::Collection(nodes))
}

/// Lower one member, routing each family to the path that accepts it.
///
/// Three routes, because a collection's members are not all representation
/// items:
///
/// - Shells and connected face sets are topology, not items, so the item
///   dispatcher rejects them.
/// - Curves and surfaces ARE items but are deliberately not top-level in
///   `dispatch`: everywhere else they are reached through the solid that
///   sweeps or bounds them, and making them dispatchable globally would let a
///   bare curve stand in for a body representation. Inside an
///   `IfcGeometricSet` they are exactly the payload, so they route here and
///   only here.
/// - Everything else is an ordinary item.
///
/// Family tests go through the generated `is_a` supertype table rather than a
/// literal type list, so a curve subtype added to `lower_curve_node` is picked
/// up here without a second edit that could drift out of step.
fn member_node(
    session: &mut LoweringSession<'_>,
    referrer: EntityId,
    id: EntityId,
    frame: Transform,
) -> GeometryResult<NodeId> {
    let member_type = session.type_name(id)?.to_ascii_uppercase();
    if matches!(
        member_type.as_str(),
        "IFCCLOSEDSHELL" | "IFCOPENSHELL" | "IFCCONNECTEDFACESET"
    ) {
        return crate::lower::brep::lower_shell_node(session, referrer, id, frame);
    }
    if is_a(&member_type, "IFCCURVE") {
        return lower_curve_node(session, id, frame);
    }
    if is_a(&member_type, "IFCSURFACE") {
        return lower_surface_node(session, id, frame);
    }
    crate::lower::dispatch::lower_representation_item(session, id, frame)
}
