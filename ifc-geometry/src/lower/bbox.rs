//! `IfcBoundingBox` lowering.
//!
//! # The box is not world-aligned
//!
//! `IfcBoundingBox` is axis-aligned in ITS OWN representation's coordinate
//! system, which is usually the element's local placement and is routinely
//! rotated relative to the world. `GeometryNode::BoundingBox` is an
//! `Aabb`: world-axis-aligned by definition. Passing the corner and extents
//! straight through would therefore claim a box the file never described --
//! correct only when the placement happens to be axis-parallel.
//!
//! So the eight local corners are transformed and the world AABB is recomputed
//! from them. For a rotated element that yields a LARGER box than the local
//! extents, which is the honest answer: it is the tightest world-aligned box
//! that still contains the element.

use axiolid_core::{Aabb, Point3};
use axiolid_model::GeometryNode;
use ifc_model::EntityId;

use crate::error::GeometryResult;
use crate::lower::session::LoweringSession;
use crate::resource::point::CartesianPoint;
use crate::solid::bbox::BoundingBox;
use crate::transform::Transform;

/// Chain kind reported when a bounding box nests too deeply or cycles.
const KIND: &str = "bounding box";

/// Lower one `IfcBoundingBox` into a world-axis-aligned `Aabb`.
pub fn lower_bounding_box_node(
    session: &mut LoweringSession<'_>,
    id: EntityId,
    frame: Transform,
) -> GeometryResult<axiolid_model::NodeId> {
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
) -> GeometryResult<axiolid_model::NodeId> {
    let entity = session.entity(id, id)?;
    let view = BoundingBox::new(id, entity);
    let corner_ref = view.corner()?;
    let corner_entity = session.entity(id, corner_ref)?;
    let raw = CartesianPoint::new(corner_ref, corner_entity).coordinates_3d()?;
    let min_local = raw.map(|value| session.units().length(value));
    // checked_dimensions rejects zero, negative and NaN extents: all three are
    // malformed for IfcPositiveLengthMeasure, not degenerate-but-usable.
    let dims = view.checked_dimensions()?;
    let dims = dims.map(|value| session.units().length(value));

    // Every one of the eight corners, not just min and max: under rotation the
    // local minimum corner is generally NOT the world minimum corner, so a
    // two-point transform produces an inverted or truncated box.
    let mut bounds = Aabb::empty();
    for xi in [0.0, dims[0]] {
        for yi in [0.0, dims[1]] {
            for zi in [0.0, dims[2]] {
                let local = [min_local[0] + xi, min_local[1] + yi, min_local[2] + zi];
                bounds.extend(Point3::from_array(frame.apply(local)));
            }
        }
    }
    session.node_for(id, GeometryNode::BoundingBox(bounds))
}
