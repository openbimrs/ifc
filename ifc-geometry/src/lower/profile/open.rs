//! Lower authored IFC open profiles without manufacturing area semantics.

use axiolid_core::{Interval, Vec2};
use axiolid_curve::linear::Polyline2;
use axiolid_curve::Curve2;
use axiolid_model::{GeometryNode, NodeId, OpenProfile};
use axiolid_profile::{Contour, ProfileSegment};
use ifc_model::{EntityId, Model};

use super::{slot, OPEN_PROFILE};
use crate::error::{GeometryError, GeometryResult};
use crate::lower::session::LoweringSession;
use crate::slots::Slots;
use crate::transform::Transform;
use crate::units::UnitScale;

/// Append one authored `IfcArbitraryOpenProfileDef` without implying area.
///
/// Area-profile callers deliberately use [`super::lower_profile_node`] instead.
/// This separate entry point prevents an open path from becoming a swept-area
/// input merely because IFC derives both declarations from `IfcProfileDef`.
pub fn lower_open_profile_node(
    session: &mut LoweringSession<'_>,
    id: EntityId,
) -> GeometryResult<NodeId> {
    let frame = Transform::identity();
    if let Some(node) = session.memoized(id, OPEN_PROFILE, frame) {
        return Ok(node);
    }

    let type_name = session.type_name(id)?;
    if type_name != "IFCARBITRARYOPENPROFILEDEF" {
        return Err(session.unsupported(
            id,
            &type_name,
            "only IfcArbitraryOpenProfileDef has authored open-path semantics",
        ));
    }

    let path_ref = session.slots(id)?.req_ref(slot::OUTER_CURVE, "Curve")?;
    let path_curve = open_curve2(session.model(), path_ref, session.units())?;
    let path = session.node_for(path_ref, GeometryNode::Curve2(path_curve))?;
    let node = session.node_for(id, GeometryNode::OpenProfile(OpenProfile::new(path)))?;
    session.memoize(id, OPEN_PROFILE, frame, node);
    Ok(node)
}

/// Extract the exact `Curve2` used by an authored open profile.
fn open_curve2(model: &Model, id: EntityId, units: &UnitScale) -> GeometryResult<Curve2> {
    let mut path = open_polyline_path(model, id, units)?;
    let segment = path
        .segments
        .pop()
        .expect("open_polyline_path always emits one segment");
    debug_assert!(path.segments.is_empty());
    Ok(segment.curve)
}

/// Read an open polyline path.
///
/// Deliberately not `curve_to_contour`: that reader closes the ring by wrapping
/// the last point back to the first and demands three distinct points. A centre
/// line is open, so closing it would add a segment the source never stated and
/// turn a two-point straight bar into a degenerate zero-area triangle.
pub(super) fn open_polyline_path(
    model: &Model,
    id: EntityId,
    units: &UnitScale,
) -> GeometryResult<Contour> {
    let entity = model.get(id).ok_or(GeometryError::MissingEntity {
        referrer: id,
        missing: id,
    })?;
    let type_name = entity.type_name.to_ascii_uppercase();
    if type_name != "IFCPOLYLINE" {
        return Err(GeometryError::Unsupported {
            entity: id,
            type_name,
            detail: "only polyline open paths are lowered so far",
        });
    }

    let slots = Slots::new(id, entity);
    let mut points = Vec::new();
    for point_id in slots.req_ref_list(0, "Points")? {
        let point = model.get(point_id).ok_or(GeometryError::MissingEntity {
            referrer: id,
            missing: point_id,
        })?;
        let coordinates = Slots::new(point_id, point).req_f64_list(0, "Coordinates")?;
        if coordinates.len() < 2 {
            return Err(GeometryError::Degenerate {
                entity: point_id,
                type_name: point.type_name.to_string(),
                detail: "open profile path point is not at least 2D".to_string(),
            });
        }
        points.push(Vec2::new(
            units.length(coordinates[0]),
            units.length(coordinates[1]),
        ));
    }
    if points.len() < 2 {
        return Err(slots.degenerate("open profile path has fewer than 2 points"));
    }
    // Axiolid's neutral OpenProfile contract rejects exact closure. A near-gap
    // remains authored-open; introducing a tolerance here would invent topology.
    if points[0] == *points.last().expect("length checked") {
        return Err(slots.degenerate("open profile path is geometrically closed"));
    }

    // One polyline segment carries the whole open path. Splitting it into
    // per-edge lines here would only add joins the kernel must re-derive.
    let last = (points.len() - 1) as f64;
    Ok(Contour::new(vec![ProfileSegment {
        curve: Curve2::Polyline(Polyline2 {
            points,
            closed: false,
        }),
        domain: Interval::new(0.0, last),
        same_sense: true,
    }]))
}
