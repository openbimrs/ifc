//! Exact lowering of swept-area profile definitions.
//!
//! IFC units and profile-local placements are resolved here, but curves remain
//! exact. Tessellation is a geometry-kernel decision and never occurs in the
//! format adapter.

use axiolid_core::{Interval, Transform2, Vec2};
use axiolid_curve::{Curve2, Line2};
use axiolid_model::{GeometryNode, NodeId};
use axiolid_profile::{
    CircleProfile, Contour, ContourProfile, Profile, ProfileSegment, RectangleProfile,
};
use ifc_model::{EntityId, Model};

use crate::error::{GeometryError, GeometryResult};
use crate::lower::session::LoweringSession;
use crate::lower::Tolerance;
use crate::slots::Slots;
use crate::units::UnitScale;

mod slot {
    pub const POSITION: usize = 2;
    pub const X_DIM: usize = 3;
    pub const Y_DIM: usize = 4;
    pub const RADIUS: usize = 3;
    pub const OUTER_CURVE: usize = 2;
    pub const INNER_CURVES: usize = 3;
    pub const CIRCLE_WALL_THICKNESS: usize = 4;
    pub const RECT_WALL_THICKNESS: usize = 5;
    pub const RECT_INNER_RADIUS: usize = 6;
    pub const RECT_OUTER_RADIUS: usize = 7;
    pub const ROUNDED_RECT_RADIUS: usize = 5;
}

/// Family label used for profile memoization.
const PROFILE: &str = "profile";

/// Append one `IfcProfileDef` to a shared session and return its node.
///
/// Profiles are the most-shared geometry in a real model: one section
/// definition backs every beam of a type. Memoizing here is what keeps a
/// shared profile a single node instead of one copy per referencing solid.
/// The frame is the identity because a profile is defined in its own 2D space;
/// placement is applied by the referencing solid, not baked into the section.
pub fn lower_profile_node(
    session: &mut LoweringSession<'_>,
    id: EntityId,
) -> GeometryResult<NodeId> {
    let frame = crate::transform::Transform::identity();
    if let Some(node) = session.memoized(id, PROFILE, frame) {
        return Ok(node);
    }
    let profile = lower_profile(session.model(), id, session.units(), &session.tolerance())?;
    let node = session.node_for(id, GeometryNode::Profile(profile))?;
    session.memoize(id, PROFILE, frame, node);
    Ok(node)
}

/// Lower one `IfcProfileDef` to an exact, format-neutral profile.
///
/// `tol` remains in this early API for source compatibility but is deliberately
/// unused: exact circles and rounded corners must not change with tessellation
/// tolerance.
pub fn lower_profile(
    model: &Model,
    id: EntityId,
    units: &UnitScale,
    _tol: &Tolerance,
) -> GeometryResult<Profile> {
    let entity = model.get(id).ok_or(GeometryError::MissingEntity {
        referrer: id,
        missing: id,
    })?;
    let slots = Slots::new(id, entity);
    let type_name = entity.type_name.to_ascii_uppercase();

    let profile = match type_name.as_str() {
        "IFCRECTANGLEPROFILEDEF" => rectangle(&slots, units, None)?,
        "IFCROUNDEDRECTANGLEPROFILEDEF" => {
            let radius = units.length(slots.req_f64(slot::ROUNDED_RECT_RADIUS, "RoundingRadius")?);
            rectangle(&slots, units, Some(radius))?
        }
        "IFCRECTANGLEHOLLOWPROFILEDEF" => rectangle_hollow(&slots, units)?,
        "IFCCIRCLEPROFILEDEF" => circle(&slots, units, None)?,
        "IFCCIRCLEHOLLOWPROFILEDEF" => circle_hollow(&slots, units)?,
        "IFCARBITRARYCLOSEDPROFILEDEF" => arbitrary(model, &slots, units, false)?,
        "IFCARBITRARYPROFILEDEFWITHVOIDS" => arbitrary(model, &slots, units, true)?,
        other => {
            return Err(GeometryError::Unsupported {
                entity: id,
                type_name: other.to_string(),
                detail: "profile subtype is not lowered yet",
            });
        }
    };

    if type_name.contains("RECTANGLE") || type_name.contains("CIRCLE") {
        apply_parameterized_position(model, &slots, units, profile)
    } else {
        Ok(profile)
    }
}

fn rectangle(
    slots: &Slots<'_>,
    units: &UnitScale,
    outer_radius: Option<f64>,
) -> GeometryResult<Profile> {
    let x = units.length(slots.req_f64(slot::X_DIM, "XDim")?);
    let y = units.length(slots.req_f64(slot::Y_DIM, "YDim")?);
    if x <= 0.0 || y <= 0.0 || outer_radius.is_some_and(|radius| radius < 0.0) {
        return Err(slots.degenerate("rectangle dimensions and radius must be non-negative"));
    }
    Ok(Profile::Rectangle(RectangleProfile {
        x,
        y,
        thickness: None,
        outer_radius,
        inner_radius: None,
    }))
}

fn rectangle_hollow(slots: &Slots<'_>, units: &UnitScale) -> GeometryResult<Profile> {
    let x = units.length(slots.req_f64(slot::X_DIM, "XDim")?);
    let y = units.length(slots.req_f64(slot::Y_DIM, "YDim")?);
    let thickness = units.length(slots.req_f64(slot::RECT_WALL_THICKNESS, "WallThickness")?);
    if x <= 0.0 || y <= 0.0 || thickness <= 0.0 || 2.0 * thickness >= x.min(y) {
        return Err(slots.degenerate("wall thickness consumes the rectangular section"));
    }
    Ok(Profile::Rectangle(RectangleProfile {
        x,
        y,
        thickness: Some(thickness),
        inner_radius: slots
            .opt_f64(slot::RECT_INNER_RADIUS)
            .map(|value| units.length(value)),
        outer_radius: slots
            .opt_f64(slot::RECT_OUTER_RADIUS)
            .map(|value| units.length(value)),
    }))
}

fn circle(slots: &Slots<'_>, units: &UnitScale, thickness: Option<f64>) -> GeometryResult<Profile> {
    let radius = units.length(slots.req_f64(slot::RADIUS, "Radius")?);
    if radius <= 0.0 || thickness.is_some_and(|wall| wall <= 0.0 || wall >= radius) {
        return Err(slots.degenerate("circle radius or wall thickness is non-physical"));
    }
    Ok(Profile::Circle(CircleProfile { radius, thickness }))
}

fn circle_hollow(slots: &Slots<'_>, units: &UnitScale) -> GeometryResult<Profile> {
    let thickness = units.length(slots.req_f64(slot::CIRCLE_WALL_THICKNESS, "WallThickness")?);
    circle(slots, units, Some(thickness))
}

fn arbitrary(
    model: &Model,
    slots: &Slots<'_>,
    units: &UnitScale,
    with_voids: bool,
) -> GeometryResult<Profile> {
    let outer = curve_to_contour(
        model,
        slots.req_ref(slot::OUTER_CURVE, "OuterCurve")?,
        units,
    )?;
    let mut holes = Vec::new();
    if with_voids {
        for curve in slots.req_ref_list(slot::INNER_CURVES, "InnerCurves")? {
            holes.push(curve_to_contour(model, curve, units)?);
        }
    }
    Ok(Profile::Contour(ContourProfile { outer, holes }))
}

fn curve_to_contour(model: &Model, id: EntityId, units: &UnitScale) -> GeometryResult<Contour> {
    let entity = model.get(id).ok_or(GeometryError::MissingEntity {
        referrer: id,
        missing: id,
    })?;
    let type_name = entity.type_name.to_ascii_uppercase();
    if type_name != "IFCPOLYLINE" {
        return Err(GeometryError::Unsupported {
            entity: id,
            type_name,
            detail: "only polyline profile boundaries are lowered so far",
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
                detail: "profile boundary point is not at least 2D".to_string(),
            });
        }
        points.push(Vec2::new(
            units.length(coordinates[0]),
            units.length(coordinates[1]),
        ));
    }
    drop_closing_duplicate(&mut points);
    if points.len() < 3 {
        return Err(slots.degenerate("profile boundary has fewer than 3 distinct points"));
    }

    let segments = (0..points.len())
        .map(|index| {
            let origin = points[index];
            let next = points[(index + 1) % points.len()];
            ProfileSegment {
                curve: Curve2::Line(Line2 {
                    origin,
                    direction: next - origin,
                }),
                domain: Interval::UNIT,
                same_sense: true,
            }
        })
        .collect();
    Ok(Contour::new(segments))
}

fn drop_closing_duplicate(points: &mut Vec<Vec2>) {
    if points.len() >= 2 && points[0].distance(*points.last().expect("length checked")) < 1e-12 {
        points.pop();
    }
}

fn apply_parameterized_position(
    model: &Model,
    slots: &Slots<'_>,
    units: &UnitScale,
    profile: Profile,
) -> GeometryResult<Profile> {
    let Some(position_id) = slots.opt_ref(slot::POSITION) else {
        return Ok(profile);
    };
    let position = model.get(position_id).ok_or(GeometryError::MissingEntity {
        referrer: slots.id(),
        missing: position_id,
    })?;
    let position_slots = Slots::new(position_id, position);
    let location_id = position_slots.req_ref(0, "Location")?;
    let location = model.get(location_id).ok_or(GeometryError::MissingEntity {
        referrer: position_id,
        missing: location_id,
    })?;
    let coordinates = Slots::new(location_id, location).req_f64_list(0, "Coordinates")?;
    if coordinates.len() < 2 {
        return Err(position_slots.degenerate("2D placement location is not 2D"));
    }
    let origin = Vec2::new(units.length(coordinates[0]), units.length(coordinates[1]));
    let x = if let Some(direction_id) = position_slots.opt_ref(1) {
        let direction = model
            .get(direction_id)
            .ok_or(GeometryError::MissingEntity {
                referrer: position_id,
                missing: direction_id,
            })?;
        let ratios = Slots::new(direction_id, direction).req_f64_list(0, "DirectionRatios")?;
        if ratios.len() < 2 {
            return Err(position_slots.degenerate("2D reference direction is not 2D"));
        }
        Vec2::new(ratios[0], ratios[1])
            .try_normalize()
            .ok_or_else(|| position_slots.degenerate("2D reference direction has zero length"))?
    } else {
        Vec2::X
    };
    let y = Vec2::new(-x.y, x.x);
    Ok(Profile::Derived {
        basis: Box::new(profile),
        transform: Transform2::from_cols(x, y, origin),
    })
}
