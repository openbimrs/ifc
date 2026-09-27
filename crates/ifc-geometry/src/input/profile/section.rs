//! Readers for the `IfcParameterizedProfileDef` families.
//!
//! Every slot comes from [`crate::slots::profile_slot`] and
//! [`crate::slots::section_slot`], the constants authoring writes through, so
//! reading and writing cannot disagree about an index. Each value is converted
//! once: lengths through [`UnitScale::length`], slopes through
//! [`UnitScale::angle`].

use super::types::ProfileParameters;
use crate::error::GeometryResult;
use crate::slots::{profile_slot as slot, section_slot, Slots};
use crate::units::UnitScale;

/// Required length, in metres.
fn len(
    slots: &Slots<'_>,
    index: usize,
    name: &'static str,
    units: &UnitScale,
) -> GeometryResult<f64> {
    Ok(units.length(slots.req_f64(index, name)?))
}

/// Optional length, in metres.
fn opt_len(slots: &Slots<'_>, index: usize, units: &UnitScale) -> Option<f64> {
    slots.opt_f64(index).map(|value| units.length(value))
}

/// Optional plane angle, in radians.
///
/// Slopes are angles, not lengths: scaling one by the length factor turns a
/// 2 degree flange taper into radians-times-millimetres and silently deforms
/// the section.
fn opt_angle(slots: &Slots<'_>, index: usize, units: &UnitScale) -> Option<f64> {
    slots.opt_f64(index).map(|value| units.angle(value))
}

/// `IfcRectangleProfileDef` and `IfcRoundedRectangleProfileDef`.
pub(super) fn rectangle(
    slots: &Slots<'_>,
    units: &UnitScale,
    rounded: bool,
) -> GeometryResult<ProfileParameters> {
    let x_dim = len(slots, slot::X_DIM, "XDim", units)?;
    let y_dim = len(slots, slot::Y_DIM, "YDim", units)?;
    let rounding_radius = if rounded {
        Some(len(
            slots,
            slot::ROUNDED_RECT_RADIUS,
            "RoundingRadius",
            units,
        )?)
    } else {
        None
    };
    if x_dim <= 0.0 || y_dim <= 0.0 || rounding_radius.is_some_and(|radius| radius < 0.0) {
        return Err(slots.degenerate("rectangle dimensions and radius must be non-negative"));
    }
    Ok(match rounding_radius {
        Some(rounding_radius) => ProfileParameters::RoundedRectangle {
            x_dim,
            y_dim,
            rounding_radius,
        },
        None => ProfileParameters::Rectangle { x_dim, y_dim },
    })
}

/// `IfcRectangleHollowProfileDef`.
pub(super) fn rectangle_hollow(
    slots: &Slots<'_>,
    units: &UnitScale,
) -> GeometryResult<ProfileParameters> {
    let x_dim = len(slots, slot::X_DIM, "XDim", units)?;
    let y_dim = len(slots, slot::Y_DIM, "YDim", units)?;
    let wall_thickness = len(slots, slot::RECT_WALL_THICKNESS, "WallThickness", units)?;
    if x_dim <= 0.0
        || y_dim <= 0.0
        || wall_thickness <= 0.0
        || 2.0 * wall_thickness >= x_dim.min(y_dim)
    {
        return Err(slots.degenerate("wall thickness consumes the rectangular section"));
    }
    Ok(ProfileParameters::RectangleHollow {
        x_dim,
        y_dim,
        wall_thickness,
        inner_fillet_radius: opt_len(slots, slot::RECT_INNER_RADIUS, units),
        outer_fillet_radius: opt_len(slots, slot::RECT_OUTER_RADIUS, units),
    })
}

/// `IfcCircleProfileDef` and `IfcCircleHollowProfileDef`.
pub(super) fn circle(
    slots: &Slots<'_>,
    units: &UnitScale,
    hollow: bool,
) -> GeometryResult<ProfileParameters> {
    let radius = len(slots, slot::RADIUS, "Radius", units)?;
    let wall_thickness = if hollow {
        Some(len(
            slots,
            slot::CIRCLE_WALL_THICKNESS,
            "WallThickness",
            units,
        )?)
    } else {
        None
    };
    if radius <= 0.0 || wall_thickness.is_some_and(|wall| wall <= 0.0 || wall >= radius) {
        return Err(slots.degenerate("circle radius or wall thickness is non-physical"));
    }
    Ok(match wall_thickness {
        Some(wall_thickness) => ProfileParameters::CircleHollow {
            radius,
            wall_thickness,
        },
        None => ProfileParameters::Circle { radius },
    })
}

/// `IfcEllipseProfileDef`.
pub(super) fn ellipse(slots: &Slots<'_>, units: &UnitScale) -> GeometryResult<ProfileParameters> {
    Ok(ProfileParameters::Ellipse {
        semi_axis_1: len(slots, section_slot::E_SEMI1, "SemiAxis1", units)?,
        semi_axis_2: len(slots, section_slot::E_SEMI2, "SemiAxis2", units)?,
    })
}

/// `IfcIShapeProfileDef`.
pub(super) fn i_shape(slots: &Slots<'_>, units: &UnitScale) -> GeometryResult<ProfileParameters> {
    Ok(ProfileParameters::IShape {
        overall_width: len(slots, section_slot::I_WIDTH, "OverallWidth", units)?,
        overall_depth: len(slots, section_slot::I_DEPTH, "OverallDepth", units)?,
        web_thickness: len(slots, section_slot::I_WEB, "WebThickness", units)?,
        flange_thickness: len(slots, section_slot::I_FLANGE, "FlangeThickness", units)?,
        fillet_radius: opt_len(slots, section_slot::I_FILLET, units),
        flange_edge_radius: opt_len(slots, section_slot::I_EDGE, units),
        flange_slope: opt_angle(slots, section_slot::I_SLOPE, units),
    })
}

/// `IfcAsymmetricIShapeProfileDef`.
///
/// Kept distinct from the symmetric variant on purpose: the top and bottom
/// flanges differ in width, thickness, fillet, edge radius and slope.
///
/// # IFC2X3 declares a different layout
///
/// In IFC2X3 TC1 the entity is a SUBTYPE of `IfcIShapeProfileDef` and adds
/// `TopFlangeWidth`, `TopFlangeThickness`, `TopFlangeFilletRadius` and
/// `CentreOfGravityInY` after the five inherited section attributes. Slots 3
/// to 10 therefore mean the same thing in both schemas, but slot 11 is
/// `CentreOfGravityInY` in IFC2X3 and `BottomFlangeEdgeRadius` in IFC4, and
/// slots 12 to 14 do not exist in IFC2X3. Reading the IFC4 layout from an
/// IFC2X3 file would report the centre of gravity as an edge radius, so the
/// declared schema selects the layout explicitly.
pub(super) fn asymmetric_i(
    slots: &Slots<'_>,
    units: &UnitScale,
    ifc2x3: bool,
) -> GeometryResult<ProfileParameters> {
    let (bottom_edge, bottom_slope, top_edge, top_slope) = if ifc2x3 {
        (None, None, None, None)
    } else {
        (
            opt_len(slots, section_slot::AI_BOTTOM_EDGE, units),
            opt_angle(slots, section_slot::AI_BOTTOM_SLOPE, units),
            opt_len(slots, section_slot::AI_TOP_EDGE, units),
            opt_angle(slots, section_slot::AI_TOP_SLOPE, units),
        )
    };
    Ok(ProfileParameters::AsymmetricIShape {
        bottom_flange_width: len(
            slots,
            section_slot::AI_BOTTOM_WIDTH,
            "BottomFlangeWidth",
            units,
        )?,
        overall_depth: len(slots, section_slot::AI_DEPTH, "OverallDepth", units)?,
        web_thickness: len(slots, section_slot::AI_WEB, "WebThickness", units)?,
        bottom_flange_thickness: len(
            slots,
            section_slot::AI_BOTTOM_FLANGE,
            "BottomFlangeThickness",
            units,
        )?,
        bottom_flange_fillet_radius: opt_len(slots, section_slot::AI_BOTTOM_FILLET, units),
        top_flange_width: len(slots, section_slot::AI_TOP_WIDTH, "TopFlangeWidth", units)?,
        // Optional, defaulting to the bottom flange: reading it as zero would
        // produce a section with no top flange.
        top_flange_thickness: opt_len(slots, section_slot::AI_TOP_FLANGE, units),
        top_flange_fillet_radius: opt_len(slots, section_slot::AI_TOP_FILLET, units),
        bottom_flange_edge_radius: bottom_edge,
        bottom_flange_slope: bottom_slope,
        top_flange_edge_radius: top_edge,
        top_flange_slope: top_slope,
    })
}

/// `IfcLShapeProfileDef`.
pub(super) fn l_shape(slots: &Slots<'_>, units: &UnitScale) -> GeometryResult<ProfileParameters> {
    Ok(ProfileParameters::LShape {
        depth: len(slots, section_slot::L_DEPTH, "Depth", units)?,
        width: opt_len(slots, section_slot::L_WIDTH, units),
        thickness: len(slots, section_slot::L_THICKNESS, "Thickness", units)?,
        fillet_radius: opt_len(slots, section_slot::L_FILLET, units),
        edge_radius: opt_len(slots, section_slot::L_EDGE, units),
        leg_slope: opt_angle(slots, section_slot::L_SLOPE, units),
    })
}

/// `IfcTShapeProfileDef`.
pub(super) fn t_shape(slots: &Slots<'_>, units: &UnitScale) -> GeometryResult<ProfileParameters> {
    Ok(ProfileParameters::TShape {
        depth: len(slots, section_slot::T_DEPTH, "Depth", units)?,
        flange_width: len(slots, section_slot::T_WIDTH, "FlangeWidth", units)?,
        web_thickness: len(slots, section_slot::T_WEB, "WebThickness", units)?,
        flange_thickness: len(slots, section_slot::T_FLANGE, "FlangeThickness", units)?,
        fillet_radius: opt_len(slots, section_slot::T_FILLET, units),
        flange_edge_radius: opt_len(slots, section_slot::T_FLANGE_EDGE, units),
        web_edge_radius: opt_len(slots, section_slot::T_WEB_EDGE, units),
        web_slope: opt_angle(slots, section_slot::T_WEB_SLOPE, units),
        flange_slope: opt_angle(slots, section_slot::T_FLANGE_SLOPE, units),
    })
}

/// `IfcUShapeProfileDef`.
pub(super) fn u_shape(slots: &Slots<'_>, units: &UnitScale) -> GeometryResult<ProfileParameters> {
    Ok(ProfileParameters::UShape {
        depth: len(slots, section_slot::U_DEPTH, "Depth", units)?,
        flange_width: len(slots, section_slot::U_WIDTH, "FlangeWidth", units)?,
        web_thickness: len(slots, section_slot::U_WEB, "WebThickness", units)?,
        flange_thickness: len(slots, section_slot::U_FLANGE, "FlangeThickness", units)?,
        fillet_radius: opt_len(slots, section_slot::U_FILLET, units),
        edge_radius: opt_len(slots, section_slot::U_EDGE, units),
        flange_slope: opt_angle(slots, section_slot::U_SLOPE, units),
    })
}

/// `IfcCShapeProfileDef`.
pub(super) fn c_shape(slots: &Slots<'_>, units: &UnitScale) -> GeometryResult<ProfileParameters> {
    Ok(ProfileParameters::CShape {
        depth: len(slots, section_slot::C_DEPTH, "Depth", units)?,
        width: len(slots, section_slot::C_WIDTH, "Width", units)?,
        wall_thickness: len(slots, section_slot::C_WALL, "WallThickness", units)?,
        // The returned lip. Dropping it turns a lipped channel into a plain
        // one, which is a different section with different buckling behaviour.
        girth: len(slots, section_slot::C_GIRTH, "Girth", units)?,
        internal_fillet_radius: opt_len(slots, section_slot::C_FILLET, units),
    })
}

/// `IfcZShapeProfileDef`.
pub(super) fn z_shape(slots: &Slots<'_>, units: &UnitScale) -> GeometryResult<ProfileParameters> {
    Ok(ProfileParameters::ZShape {
        depth: len(slots, section_slot::Z_DEPTH, "Depth", units)?,
        flange_width: len(slots, section_slot::Z_FLANGE_WIDTH, "FlangeWidth", units)?,
        web_thickness: len(slots, section_slot::Z_WEB, "WebThickness", units)?,
        flange_thickness: len(slots, section_slot::Z_FLANGE, "FlangeThickness", units)?,
        fillet_radius: opt_len(slots, section_slot::Z_FILLET, units),
        edge_radius: opt_len(slots, section_slot::Z_EDGE, units),
    })
}

/// `IfcTrapeziumProfileDef`.
pub(super) fn trapezium(slots: &Slots<'_>, units: &UnitScale) -> GeometryResult<ProfileParameters> {
    Ok(ProfileParameters::Trapezium {
        bottom_x_dim: len(slots, section_slot::TZ_BOTTOM, "BottomXDim", units)?,
        top_x_dim: len(slots, section_slot::TZ_TOP, "TopXDim", units)?,
        y_dim: len(slots, section_slot::TZ_Y, "YDim", units)?,
        // TopXOffset is a plain IfcLengthMeasure and may be negative, so it
        // must not be read through a positive-length check.
        top_x_offset: len(slots, section_slot::TZ_OFFSET, "TopXOffset", units)?,
    })
}
