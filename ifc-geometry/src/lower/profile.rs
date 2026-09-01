//! Exact lowering of swept-area profile definitions.
//!
//! IFC units and profile-local placements are resolved here, but curves remain
//! exact. Tessellation is a geometry-kernel decision and never occurs in the
//! format adapter.

use axiolid_core::{Interval, Scalar, Transform2, Vec2};
use axiolid_curve::linear::Polyline2;
use axiolid_curve::{Curve2, Line2};
use axiolid_model::{GeometryNode, NodeId};
use axiolid_profile::CenterLineProfile;
use axiolid_profile::{
    CircleProfile, Contour, ContourProfile, EllipseProfile, Profile, ProfileSegment,
    RectangleProfile, SectionProfile,
};
use ifc_model::{EntityId, Model};

use crate::error::{GeometryError, GeometryResult};
use crate::lower::session::LoweringSession;
use crate::resource::operator::operator_transform;
use crate::slots::Slots;
use crate::transform::Transform;
use crate::units::UnitScale;

/// Concrete `IfcProfileDef` families represented exactly by the neutral profile model.
pub const IMPLEMENTED_PROFILES: &[&str] = &[
    "IFCARBITRARYCLOSEDPROFILEDEF",
    "IFCARBITRARYPROFILEDEFWITHVOIDS",
    "IFCASYMMETRICISHAPEPROFILEDEF",
    "IFCCENTERLINEPROFILEDEF",
    "IFCCIRCLEHOLLOWPROFILEDEF",
    "IFCCIRCLEPROFILEDEF",
    "IFCCOMPOSITEPROFILEDEF",
    "IFCCSHAPEPROFILEDEF",
    "IFCDERIVEDPROFILEDEF",
    "IFCELLIPSEPROFILEDEF",
    "IFCISHAPEPROFILEDEF",
    "IFCLSHAPEPROFILEDEF",
    "IFCMIRROREDPROFILEDEF",
    "IFCRECTANGLEHOLLOWPROFILEDEF",
    "IFCRECTANGLEPROFILEDEF",
    "IFCROUNDEDRECTANGLEPROFILEDEF",
    "IFCTSHAPEPROFILEDEF",
    "IFCTRAPEZIUMPROFILEDEF",
    "IFCUSHAPEPROFILEDEF",
    "IFCZSHAPEPROFILEDEF",
];

/// Concrete profile families blocked on a named neutral representation contract.
pub const PLANNED_PROFILES: &[(&str, &str)] = &[
    (
        "IFCARBITRARYOPENPROFILEDEF",
        "open profiles require a neutral exact open-path profile without implied area or width",
    ),
    (
        "IFCPROFILEDEF",
        "generic profile declaration carries no concrete geometry to lower",
    ),
];

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

/// Concrete profile families this lowerer does not yet build, with reasons.
///
/// Paired with `tests/schema_coverage.rs`, which fails if a concrete profile
/// appears in neither this table nor a match arm above. That is what makes the
/// gap visible: the committed corpus contains no steel sections, so a
/// corpus-shaped census reported full coverage while 13 families were absent.
///
/// A reason starting with `kernel:` needs a change in `axiolid-profile`; the
/// rest are IFC-side wiring.
pub const UNLOWERED: &[(&str, &str)] = &[];

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
    let profile = lower_profile(session.model(), id, session.units())?;
    let node = session.node_for(id, GeometryNode::Profile(profile))?;
    session.memoize(id, PROFILE, frame, node);
    Ok(node)
}

/// Lower one `IfcProfileDef` to an exact, format-neutral profile.
pub fn lower_profile(model: &Model, id: EntityId, units: &UnitScale) -> GeometryResult<Profile> {
    lower_profile_depth(model, id, units, 0)
}

/// Maximum profile nesting depth.
///
/// `IfcCompositeProfileDef` and `IfcDerivedProfileDef` both reference other
/// profiles, so a malicious or broken file can nest them without end. A
/// composite of derived sections is realistic; sixteen levels is not.
const MAX_PROFILE_DEPTH: usize = 16;

/// Lower a profile, tracking nesting depth for the recursive families.
fn lower_profile_depth(
    model: &Model,
    id: EntityId,
    units: &UnitScale,
    depth: usize,
) -> GeometryResult<Profile> {
    if depth > MAX_PROFILE_DEPTH {
        return Err(GeometryError::Unsupported {
            entity: id,
            type_name: "IFCPROFILEDEF".to_string(),
            detail: "profile nesting exceeded the depth budget",
        });
    }
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
        "IFCISHAPEPROFILEDEF" => i_shape(&slots, units)?,
        "IFCASYMMETRICISHAPEPROFILEDEF" => asymmetric_i(&slots, units)?,
        "IFCLSHAPEPROFILEDEF" => l_shape(&slots, units)?,
        "IFCTSHAPEPROFILEDEF" => t_shape(&slots, units)?,
        "IFCUSHAPEPROFILEDEF" => u_shape(&slots, units)?,
        "IFCCSHAPEPROFILEDEF" => c_shape(&slots, units)?,
        "IFCZSHAPEPROFILEDEF" => z_shape(&slots, units)?,
        "IFCTRAPEZIUMPROFILEDEF" => trapezium(&slots, units)?,
        "IFCELLIPSEPROFILEDEF" => ellipse(&slots, units)?,
        "IFCCOMPOSITEPROFILEDEF" => composite(model, &slots, units, depth)?,
        "IFCDERIVEDPROFILEDEF" => derived(model, id, &slots, units, depth, false)?,
        "IFCMIRROREDPROFILEDEF" => derived(model, id, &slots, units, depth, true)?,
        "IFCCENTERLINEPROFILEDEF" => center_line(model, &slots, units)?,
        // An open profile is a curve, not an area. The neutral profile model
        // is built on closed contours, so there is nothing to map it onto:
        // closing the curve would fabricate a face the file never described,
        // and silently sweeping it would produce a solid from a shape that
        // bounds no area. State that rather than emitting a generic gap.
        "IFCARBITRARYOPENPROFILEDEF" => {
            return Err(GeometryError::Unsupported {
                entity: id,
                type_name: type_name.to_string(),
                detail: PLANNED_PROFILES[0].1,
            });
        }
        "IFCPROFILEDEF" => {
            return Err(GeometryError::Unsupported {
                entity: id,
                type_name: type_name.to_string(),
                detail: PLANNED_PROFILES[1].1,
            });
        }
        other => {
            // A declared family reports its specific reason; anything else is
            // a family the crate does not know at all.
            let detail = UNLOWERED
                .iter()
                .find(|(name, _)| *name == other)
                .map(|(_, reason)| *reason)
                .unwrap_or("profile subtype is not lowered yet");
            return Err(GeometryError::Unsupported {
                entity: id,
                type_name: other.to_string(),
                detail,
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

/// Absolute attribute slots for the parameterized profile families.
///
/// Every index below was read from the IFC4 ADD2 TC1 schema, not inferred:
/// `IfcParameterizedProfileDef` contributes ProfileType, ProfileName and
/// Position, so subtype attributes start at slot 3.
mod section_slot {
    // IfcIShapeProfileDef
    pub const I_WIDTH: usize = 3;
    pub const I_DEPTH: usize = 4;
    pub const I_WEB: usize = 5;
    pub const I_FLANGE: usize = 6;
    pub const I_FILLET: usize = 7;
    pub const I_EDGE: usize = 8;
    pub const I_SLOPE: usize = 9;

    // IfcAsymmetricIShapeProfileDef
    pub const AI_BOTTOM_WIDTH: usize = 3;
    pub const AI_DEPTH: usize = 4;
    pub const AI_WEB: usize = 5;
    pub const AI_BOTTOM_FLANGE: usize = 6;
    pub const AI_BOTTOM_FILLET: usize = 7;
    pub const AI_TOP_WIDTH: usize = 8;
    pub const AI_TOP_FLANGE: usize = 9;
    pub const AI_TOP_FILLET: usize = 10;
    pub const AI_BOTTOM_EDGE: usize = 11;
    pub const AI_BOTTOM_SLOPE: usize = 12;
    pub const AI_TOP_EDGE: usize = 13;
    pub const AI_TOP_SLOPE: usize = 14;

    // IfcLShapeProfileDef
    pub const L_DEPTH: usize = 3;
    pub const L_WIDTH: usize = 4;
    pub const L_THICKNESS: usize = 5;
    pub const L_FILLET: usize = 6;
    pub const L_EDGE: usize = 7;
    pub const L_SLOPE: usize = 8;

    // IfcTShapeProfileDef
    pub const T_DEPTH: usize = 3;
    pub const T_WIDTH: usize = 4;
    pub const T_WEB: usize = 5;
    pub const T_FLANGE: usize = 6;
    pub const T_FILLET: usize = 7;
    pub const T_FLANGE_EDGE: usize = 8;
    pub const T_WEB_EDGE: usize = 9;
    pub const T_WEB_SLOPE: usize = 10;
    pub const T_FLANGE_SLOPE: usize = 11;

    // IfcUShapeProfileDef
    pub const U_DEPTH: usize = 3;
    pub const U_WIDTH: usize = 4;
    pub const U_WEB: usize = 5;
    pub const U_FLANGE: usize = 6;
    pub const U_FILLET: usize = 7;
    pub const U_EDGE: usize = 8;
    pub const U_SLOPE: usize = 9;

    // IfcCShapeProfileDef
    pub const C_DEPTH: usize = 3;
    pub const C_WIDTH: usize = 4;
    pub const C_WALL: usize = 5;
    pub const C_GIRTH: usize = 6;
    pub const C_FILLET: usize = 7;

    // IfcZShapeProfileDef
    pub const Z_DEPTH: usize = 3;
    pub const Z_FLANGE_WIDTH: usize = 4;
    pub const Z_WEB: usize = 5;
    pub const Z_FLANGE: usize = 6;
    pub const Z_FILLET: usize = 7;
    pub const Z_EDGE: usize = 8;

    // IfcEllipseProfileDef
    pub const E_SEMI1: usize = 3;
    pub const E_SEMI2: usize = 4;

    // IfcTrapeziumProfileDef
    pub const TZ_BOTTOM: usize = 3;
    pub const TZ_TOP: usize = 4;
    pub const TZ_Y: usize = 5;
    /// `IfcCenterLineProfileDef`: Curve at 2, Thickness at 3.
    /// Thickness is the FULL width across the path, not a half-width.
    pub const CL_CURVE: usize = 2;
    /// Full width across the centre line.
    pub const CL_THICKNESS: usize = 3;

    pub const TZ_OFFSET: usize = 6;

    // IfcCompositeProfileDef / IfcDerivedProfileDef
    pub const COMPOSITE_PROFILES: usize = 2;
    pub const DERIVED_PARENT: usize = 2;
    pub const DERIVED_OPERATOR: usize = 3;
}

/// Read an optional non-negative length, converting to kernel units.
fn opt_len(slots: &Slots<'_>, slot: usize, units: &UnitScale) -> Option<Scalar> {
    slots.opt_f64(slot).map(|v| units.length(v))
}

/// Read an optional plane angle, converting to kernel units.
///
/// Slopes are angles, not lengths: scaling one by the length factor turns a
/// 2 degree flange taper into radians-times-millimetres and silently deforms
/// the section.
fn opt_angle(slots: &Slots<'_>, slot: usize, units: &UnitScale) -> Option<Scalar> {
    slots.opt_f64(slot).map(|v| units.angle(v))
}

/// Lower a symmetric `IfcIShapeProfileDef`.
fn i_shape(slots: &Slots<'_>, units: &UnitScale) -> GeometryResult<Profile> {
    Ok(Profile::Section(SectionProfile::I {
        depth: units.length(slots.req_f64(section_slot::I_DEPTH, "OverallDepth")?),
        width: units.length(slots.req_f64(section_slot::I_WIDTH, "OverallWidth")?),
        web_thickness: units.length(slots.req_f64(section_slot::I_WEB, "WebThickness")?),
        flange_thickness: units.length(slots.req_f64(section_slot::I_FLANGE, "FlangeThickness")?),
        fillet_radius: opt_len(slots, section_slot::I_FILLET, units),
        flange_edge_radius: opt_len(slots, section_slot::I_EDGE, units),
        flange_slope: opt_angle(slots, section_slot::I_SLOPE, units),
    }))
}

/// Lower an `IfcAsymmetricIShapeProfileDef`.
///
/// Kept distinct from the symmetric variant on purpose: the top and bottom
/// flanges differ in width, thickness, fillet, edge radius and slope. Folding
/// this into `SectionProfile::I` would force a choice of one flange, and the
/// resulting section has the wrong area and the wrong second moment.
fn asymmetric_i(slots: &Slots<'_>, units: &UnitScale) -> GeometryResult<Profile> {
    let bottom_thickness =
        units.length(slots.req_f64(section_slot::AI_BOTTOM_FLANGE, "BottomFlangeThickness")?);
    Ok(Profile::Section(SectionProfile::AsymmetricI {
        depth: units.length(slots.req_f64(section_slot::AI_DEPTH, "OverallDepth")?),
        web_thickness: units.length(slots.req_f64(section_slot::AI_WEB, "WebThickness")?),
        bottom_flange_width: units
            .length(slots.req_f64(section_slot::AI_BOTTOM_WIDTH, "BottomFlangeWidth")?),
        bottom_flange_thickness: bottom_thickness,
        bottom_fillet_radius: opt_len(slots, section_slot::AI_BOTTOM_FILLET, units),
        bottom_flange_edge_radius: opt_len(slots, section_slot::AI_BOTTOM_EDGE, units),
        bottom_flange_slope: opt_angle(slots, section_slot::AI_BOTTOM_SLOPE, units),
        top_flange_width: units
            .length(slots.req_f64(section_slot::AI_TOP_WIDTH, "TopFlangeWidth")?),
        // TopFlangeThickness is optional and defaults to the bottom flange:
        // reading it as zero would produce a section with no top flange.
        top_flange_thickness: opt_len(slots, section_slot::AI_TOP_FLANGE, units),
        top_fillet_radius: opt_len(slots, section_slot::AI_TOP_FILLET, units),
        top_flange_edge_radius: opt_len(slots, section_slot::AI_TOP_EDGE, units),
        top_flange_slope: opt_angle(slots, section_slot::AI_TOP_SLOPE, units),
    }))
}

/// Lower an `IfcLShapeProfileDef`.
fn l_shape(slots: &Slots<'_>, units: &UnitScale) -> GeometryResult<Profile> {
    let depth = units.length(slots.req_f64(section_slot::L_DEPTH, "Depth")?);
    Ok(Profile::Section(SectionProfile::L {
        depth,
        // Width is optional and defaults to Depth, giving an equal-leg angle.
        width: opt_len(slots, section_slot::L_WIDTH, units),
        thickness: units.length(slots.req_f64(section_slot::L_THICKNESS, "Thickness")?),
        fillet_radius: opt_len(slots, section_slot::L_FILLET, units),
        edge_radius: opt_len(slots, section_slot::L_EDGE, units),
        leg_slope: opt_angle(slots, section_slot::L_SLOPE, units),
    }))
}

/// Lower an `IfcTShapeProfileDef`.
fn t_shape(slots: &Slots<'_>, units: &UnitScale) -> GeometryResult<Profile> {
    Ok(Profile::Section(SectionProfile::T {
        depth: units.length(slots.req_f64(section_slot::T_DEPTH, "Depth")?),
        flange_width: units.length(slots.req_f64(section_slot::T_WIDTH, "FlangeWidth")?),
        web_thickness: units.length(slots.req_f64(section_slot::T_WEB, "WebThickness")?),
        flange_thickness: units.length(slots.req_f64(section_slot::T_FLANGE, "FlangeThickness")?),
        fillet_radius: opt_len(slots, section_slot::T_FILLET, units),
        flange_edge_radius: opt_len(slots, section_slot::T_FLANGE_EDGE, units),
        web_edge_radius: opt_len(slots, section_slot::T_WEB_EDGE, units),
        web_slope: opt_angle(slots, section_slot::T_WEB_SLOPE, units),
        flange_slope: opt_angle(slots, section_slot::T_FLANGE_SLOPE, units),
    }))
}

/// Lower an `IfcUShapeProfileDef`.
fn u_shape(slots: &Slots<'_>, units: &UnitScale) -> GeometryResult<Profile> {
    Ok(Profile::Section(SectionProfile::U {
        depth: units.length(slots.req_f64(section_slot::U_DEPTH, "Depth")?),
        flange_width: units.length(slots.req_f64(section_slot::U_WIDTH, "FlangeWidth")?),
        web_thickness: units.length(slots.req_f64(section_slot::U_WEB, "WebThickness")?),
        flange_thickness: units.length(slots.req_f64(section_slot::U_FLANGE, "FlangeThickness")?),
        fillet_radius: opt_len(slots, section_slot::U_FILLET, units),
        edge_radius: opt_len(slots, section_slot::U_EDGE, units),
        flange_slope: opt_angle(slots, section_slot::U_SLOPE, units),
    }))
}

/// Lower an `IfcCShapeProfileDef`.
fn c_shape(slots: &Slots<'_>, units: &UnitScale) -> GeometryResult<Profile> {
    Ok(Profile::Section(SectionProfile::C {
        depth: units.length(slots.req_f64(section_slot::C_DEPTH, "Depth")?),
        width: units.length(slots.req_f64(section_slot::C_WIDTH, "Width")?),
        wall_thickness: units.length(slots.req_f64(section_slot::C_WALL, "WallThickness")?),
        // The returned lip. Dropping it turns a lipped channel into a plain
        // one, which is a different section with different buckling behaviour.
        girth: units.length(slots.req_f64(section_slot::C_GIRTH, "Girth")?),
        internal_fillet_radius: opt_len(slots, section_slot::C_FILLET, units),
    }))
}

/// Lower an `IfcZShapeProfileDef`.
fn z_shape(slots: &Slots<'_>, units: &UnitScale) -> GeometryResult<Profile> {
    Ok(Profile::Section(SectionProfile::Z {
        depth: units.length(slots.req_f64(section_slot::Z_DEPTH, "Depth")?),
        flange_width: units.length(slots.req_f64(section_slot::Z_FLANGE_WIDTH, "FlangeWidth")?),
        web_thickness: units.length(slots.req_f64(section_slot::Z_WEB, "WebThickness")?),
        flange_thickness: units.length(slots.req_f64(section_slot::Z_FLANGE, "FlangeThickness")?),
        fillet_radius: opt_len(slots, section_slot::Z_FILLET, units),
        edge_radius: opt_len(slots, section_slot::Z_EDGE, units),
    }))
}

/// Lower an `IfcTrapeziumProfileDef`.
fn trapezium(slots: &Slots<'_>, units: &UnitScale) -> GeometryResult<Profile> {
    Ok(Profile::Section(SectionProfile::Trapezium {
        bottom_x: units.length(slots.req_f64(section_slot::TZ_BOTTOM, "BottomXDim")?),
        top_x: units.length(slots.req_f64(section_slot::TZ_TOP, "TopXDim")?),
        y: units.length(slots.req_f64(section_slot::TZ_Y, "YDim")?),
        // TopXOffset is a plain IfcLengthMeasure and may be negative, so it
        // must not be read through a positive-length helper.
        top_offset: units.length(slots.req_f64(section_slot::TZ_OFFSET, "TopXOffset")?),
    }))
}

/// Lower an `IfcEllipseProfileDef`.
fn ellipse(slots: &Slots<'_>, units: &UnitScale) -> GeometryResult<Profile> {
    Ok(Profile::Ellipse(EllipseProfile {
        semi_axis_x: units.length(slots.req_f64(section_slot::E_SEMI1, "SemiAxis1")?),
        semi_axis_y: units.length(slots.req_f64(section_slot::E_SEMI2, "SemiAxis2")?),
    }))
}

/// Lower an `IfcCompositeProfileDef` into an ordered profile collection.
///
/// Order is preserved because it is the only identity a composite member has:
/// nothing else distinguishes two same-shaped members.
fn composite(
    model: &Model,
    slots: &Slots<'_>,
    units: &UnitScale,
    depth: usize,
) -> GeometryResult<Profile> {
    let refs = slots.req_ref_list(section_slot::COMPOSITE_PROFILES, "Profiles")?;
    let mut members = Vec::with_capacity(refs.len());
    for member in refs {
        members.push(lower_profile_depth(model, member, units, depth + 1)?);
    }
    Ok(Profile::Composite(members))
}

/// Lower an `IfcDerivedProfileDef` as a parent profile plus a 2D transform.
fn derived(
    model: &Model,
    id: EntityId,
    slots: &Slots<'_>,
    units: &UnitScale,
    depth: usize,
    mirrored: bool,
) -> GeometryResult<Profile> {
    let parent = slots.req_ref(section_slot::DERIVED_PARENT, "ParentProfile")?;
    let basis = lower_profile_depth(model, parent, units, depth + 1)?;

    let transform = (if mirrored {
        // IfcMirroredProfileDef derives its Operator in the schema, so no file
        // carries one: the mirror about the local y axis is implied by the
        // TYPE alone. Reading Operator here would find nothing and silently
        // yield an unmirrored profile, which is why this subtype cannot share
        // the parent's slot-reading path.
        Ok(Transform2::from_scale(Vec2::new(-1.0, 1.0)))
    } else {
        let operator = slots.req_ref(section_slot::DERIVED_OPERATOR, "Operator")?;
        let op_entity = model.get(operator).ok_or(GeometryError::MissingEntity {
            referrer: id,
            missing: operator,
        })?;
        // Reuse the shared operator reader rather than a second 2D-only
        // parser: it already handles the uniform and non-uniform forms.
        let full = operator_transform(model, operator, op_entity)?;
        flatten_to_2d(&full)
    })?;

    Ok(Profile::Derived {
        basis: Box::new(basis),
        transform,
    })
}

/// Reduce a 3D operator transform to the 2D transform a profile lives in.
///
/// A profile is defined in its own xy plane, and
/// `IfcCartesianTransformationOperator2D` is read through the shared 3D
/// reader. Any z component means the file used a 3D operator where the schema
/// requires a 2D one; refusing beats silently projecting geometry away.
fn flatten_to_2d(full: &Transform) -> GeometryResult<Transform2> {
    let m = full.to_geom().matrix3;
    let translation = full.to_geom().translation;
    let z_leak = m.z_axis.x.abs() + m.z_axis.y.abs() + m.x_axis.z.abs() + m.y_axis.z.abs();
    if z_leak > 1e-9 || translation.z.abs() > 1e-9 {
        return Err(GeometryError::Unsupported {
            entity: EntityId(0),
            type_name: "IFCCARTESIANTRANSFORMATIONOPERATOR2D".to_string(),
            detail: "profile operator has out-of-plane components",
        });
    }
    Ok(Transform2::from_cols(
        m.x_axis.truncate(),
        m.y_axis.truncate(),
        translation.truncate(),
    ))
}

/// Read an OPEN polyline path for a centre-line profile.
///
/// Deliberately not `curve_to_contour`: that reader closes the ring by
/// wrapping the last point back to the first and demands three distinct
/// points. A centre line is open, so closing it would add a segment the
/// source never stated and turn a two-point straight bar into a degenerate
/// zero-area triangle.
fn open_polyline_path(model: &Model, id: EntityId, units: &UnitScale) -> GeometryResult<Contour> {
    let entity = model.get(id).ok_or(GeometryError::MissingEntity {
        referrer: id,
        missing: id,
    })?;
    let type_name = entity.type_name.to_ascii_uppercase();
    if type_name != "IFCPOLYLINE" {
        return Err(GeometryError::Unsupported {
            entity: id,
            type_name,
            detail: "only polyline centre lines are lowered so far",
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
                detail: "centre line point is not at least 2D".to_string(),
            });
        }
        points.push(Vec2::new(
            units.length(coordinates[0]),
            units.length(coordinates[1]),
        ));
    }
    if points.len() < 2 {
        return Err(slots.degenerate("centre line has fewer than 2 points"));
    }

    // One polyline segment carrying the whole open path: the kernel offsets
    // the flattened points, so splitting it into per-edge lines here would
    // only add joins it would immediately have to re-derive.
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

/// Lower an `IfcCenterLineProfileDef`.
///
/// `Thickness` is the FULL width across the path, which the kernel stores
/// halved so both offset sides are symmetric by construction.
fn center_line(model: &Model, slots: &Slots<'_>, units: &UnitScale) -> GeometryResult<Profile> {
    let path = open_polyline_path(
        model,
        slots.req_ref(section_slot::CL_CURVE, "Curve")?,
        units,
    )?;
    let thickness = units.length(slots.req_f64(section_slot::CL_THICKNESS, "Thickness")?);
    Ok(Profile::CenterLine(CenterLineProfile::from_width(
        path, thickness,
    )))
}
