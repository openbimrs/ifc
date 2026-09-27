//! The flanged catalogue sections: I, L, T, U, Z, C and the asymmetric I.
//!
//! Split from the closed sections purely for file size; the shared
//! helpers and the WHERE-rule discipline are identical.

use ifc_model::{Entity, EntityId, Transaction, Value};

use crate::error::GeometryError;
use crate::slots::section_slot as slot;

use super::super::require_finite;
use super::{
    non_negative, parameterized, positive, put_opt, rule, AsymmetricIDims, CShapeDims, FlangedDims,
    IShapeDims, ProfileHeader,
};

/// The optional radii and slope an I-section may carry.
///
/// Grouped rather than passed positionally: five trailing `Option<f64>`
/// arguments at a call site are indistinguishable from one another, and
/// transposing two of them writes a valid record describing the wrong
/// beam.
#[derive(Debug, Clone, Copy, Default)]
pub struct IShapeExtras {
    /// `FilletRadius`: the web-to-flange transition.
    pub fillet_radius: Option<f64>,
    /// `FlangeEdgeRadius`: the rounding at the flange tip.
    pub flange_edge_radius: Option<f64>,
    /// `FlangeSlope`, in radians.
    pub flange_slope: Option<f64>,
}

/// Stage an `IfcIShapeProfileDef`.
///
/// # Errors
///
/// Refuses a non-positive dimension, a negative radius, and each of the
/// entity's three WHERE rules: `ValidFlangeThickness`
/// (`2 * FlangeThickness < OverallDepth`), `ValidWebThickness`
/// (`WebThickness < OverallWidth`), and `ValidFilletRadius`, which bounds
/// the fillet by both the web clearance and the flange clearance.
pub fn i_shape(
    tx: &mut Transaction,
    header: ProfileHeader<'_>,
    dims: IShapeDims,
    extras: IShapeExtras,
) -> Result<EntityId, GeometryError> {
    const T: &str = "IFCISHAPEPROFILEDEF";
    let IShapeDims {
        width,
        depth,
        web_thickness,
        flange_thickness,
    } = dims;
    positive(T, "OverallWidth", width)?;
    positive(T, "OverallDepth", depth)?;
    positive(T, "WebThickness", web_thickness)?;
    positive(T, "FlangeThickness", flange_thickness)?;
    non_negative(T, "FilletRadius", extras.fillet_radius)?;
    non_negative(T, "FlangeEdgeRadius", extras.flange_edge_radius)?;
    if let Some(slope) = extras.flange_slope {
        require_finite(T, "FlangeSlope", &[slope])?;
    }
    if 2.0 * flange_thickness >= depth {
        return Err(rule(
            T,
            "FlangeThickness",
            format!("2 * {flange_thickness} is not below OverallDepth {depth}"),
        ));
    }
    if web_thickness >= width {
        return Err(rule(
            T,
            "WebThickness",
            format!("{web_thickness} is not below OverallWidth {width}"),
        ));
    }
    if let Some(fillet) = extras.fillet_radius {
        let web_room = (width - web_thickness) / 2.0;
        let flange_room = (depth - 2.0 * flange_thickness) / 2.0;
        if fillet > web_room || fillet > flange_room {
            return Err(rule(
                T,
                "FilletRadius",
                format!("{fillet} exceeds the room left by the web ({web_room}) or flanges ({flange_room})"),
            ));
        }
    }
    let mut attrs = parameterized(T, 10, header);
    attrs[slot::I_WIDTH] = Value::Real(width);
    attrs[slot::I_DEPTH] = Value::Real(depth);
    attrs[slot::I_WEB] = Value::Real(web_thickness);
    attrs[slot::I_FLANGE] = Value::Real(flange_thickness);
    put_opt(&mut attrs, slot::I_FILLET, extras.fillet_radius);
    put_opt(&mut attrs, slot::I_EDGE, extras.flange_edge_radius);
    put_opt(&mut attrs, slot::I_SLOPE, extras.flange_slope);
    Ok(tx.create(Entity::new(T, attrs)))
}
/// The optional radii and slope an L-section may carry.
#[derive(Debug, Clone, Copy, Default)]
pub struct LShapeExtras {
    /// `Width`: absent means a equal-legged angle of side `Depth`.
    pub width: Option<f64>,
    /// `FilletRadius`: the inner corner.
    pub fillet_radius: Option<f64>,
    /// `EdgeRadius`: the leg tips.
    pub edge_radius: Option<f64>,
    /// `LegSlope`, in radians.
    pub leg_slope: Option<f64>,
}

/// Stage an `IfcLShapeProfileDef`.
///
/// # Errors
///
/// Refuses a non-positive dimension, a negative radius, and the
/// `ValidThickness` rule: the thickness must be below the depth, and
/// below the width when one is stated. An equal-legged angle omits
/// `Width`, so the width half of the rule only applies when present.
pub fn l_shape(
    tx: &mut Transaction,
    header: ProfileHeader<'_>,
    depth: f64,
    thickness: f64,
    extras: LShapeExtras,
) -> Result<EntityId, GeometryError> {
    const T: &str = "IFCLSHAPEPROFILEDEF";
    positive(T, "Depth", depth)?;
    positive(T, "Thickness", thickness)?;
    if let Some(width) = extras.width {
        positive(T, "Width", width)?;
    }
    non_negative(T, "FilletRadius", extras.fillet_radius)?;
    non_negative(T, "EdgeRadius", extras.edge_radius)?;
    if let Some(slope) = extras.leg_slope {
        require_finite(T, "LegSlope", &[slope])?;
    }
    if thickness >= depth {
        return Err(rule(
            T,
            "Thickness",
            format!("{thickness} is not below Depth {depth}"),
        ));
    }
    if let Some(width) = extras.width {
        if thickness >= width {
            return Err(rule(
                T,
                "Thickness",
                format!("{thickness} is not below Width {width}"),
            ));
        }
    }
    let mut attrs = parameterized(T, 9, header);
    attrs[slot::L_DEPTH] = Value::Real(depth);
    attrs[slot::L_THICKNESS] = Value::Real(thickness);
    put_opt(&mut attrs, slot::L_WIDTH, extras.width);
    put_opt(&mut attrs, slot::L_FILLET, extras.fillet_radius);
    put_opt(&mut attrs, slot::L_EDGE, extras.edge_radius);
    put_opt(&mut attrs, slot::L_SLOPE, extras.leg_slope);
    Ok(tx.create(Entity::new(T, attrs)))
}
/// The optional radii and slopes a T-section may carry.
#[derive(Debug, Clone, Copy, Default)]
pub struct TShapeExtras {
    /// `FilletRadius`: the web-to-flange transition.
    pub fillet_radius: Option<f64>,
    /// `FlangeEdgeRadius`: the flange tips.
    pub flange_edge_radius: Option<f64>,
    /// `WebEdgeRadius`: the free end of the web.
    pub web_edge_radius: Option<f64>,
    /// `WebSlope`, in radians.
    pub web_slope: Option<f64>,
    /// `FlangeSlope`, in radians.
    pub flange_slope: Option<f64>,
}

/// Stage an `IfcTShapeProfileDef`.
///
/// # Errors
///
/// Refuses a non-positive dimension, a negative radius, and the two
/// WHERE rules: `FlangeThickness < Depth` and
/// `WebThickness < FlangeWidth`.
pub fn t_shape(
    tx: &mut Transaction,
    header: ProfileHeader<'_>,
    dims: FlangedDims,
    extras: TShapeExtras,
) -> Result<EntityId, GeometryError> {
    const T: &str = "IFCTSHAPEPROFILEDEF";
    let FlangedDims {
        depth,
        flange_width,
        web_thickness,
        flange_thickness,
    } = dims;
    positive(T, "Depth", depth)?;
    positive(T, "FlangeWidth", flange_width)?;
    positive(T, "WebThickness", web_thickness)?;
    positive(T, "FlangeThickness", flange_thickness)?;
    non_negative(T, "FilletRadius", extras.fillet_radius)?;
    non_negative(T, "FlangeEdgeRadius", extras.flange_edge_radius)?;
    non_negative(T, "WebEdgeRadius", extras.web_edge_radius)?;
    for (attribute, slope) in [
        ("WebSlope", extras.web_slope),
        ("FlangeSlope", extras.flange_slope),
    ] {
        if let Some(slope) = slope {
            require_finite(T, attribute, &[slope])?;
        }
    }
    if flange_thickness >= depth {
        return Err(rule(
            T,
            "FlangeThickness",
            format!("{flange_thickness} is not below Depth {depth}"),
        ));
    }
    if web_thickness >= flange_width {
        return Err(rule(
            T,
            "WebThickness",
            format!("{web_thickness} is not below FlangeWidth {flange_width}"),
        ));
    }
    let mut attrs = parameterized(T, 12, header);
    attrs[slot::T_DEPTH] = Value::Real(depth);
    attrs[slot::T_WIDTH] = Value::Real(flange_width);
    attrs[slot::T_WEB] = Value::Real(web_thickness);
    attrs[slot::T_FLANGE] = Value::Real(flange_thickness);
    put_opt(&mut attrs, slot::T_FILLET, extras.fillet_radius);
    put_opt(&mut attrs, slot::T_FLANGE_EDGE, extras.flange_edge_radius);
    put_opt(&mut attrs, slot::T_WEB_EDGE, extras.web_edge_radius);
    put_opt(&mut attrs, slot::T_WEB_SLOPE, extras.web_slope);
    put_opt(&mut attrs, slot::T_FLANGE_SLOPE, extras.flange_slope);
    Ok(tx.create(Entity::new(T, attrs)))
}
/// The optional radii and slope a U-section (channel) may carry.
#[derive(Debug, Clone, Copy, Default)]
pub struct UShapeExtras {
    /// `FilletRadius`: the web-to-flange transition.
    pub fillet_radius: Option<f64>,
    /// `EdgeRadius`: the flange tips.
    pub edge_radius: Option<f64>,
    /// `FlangeSlope`, in radians.
    pub flange_slope: Option<f64>,
}

/// Stage an `IfcUShapeProfileDef`.
///
/// # Errors
///
/// Refuses a non-positive dimension, a negative radius, and the two
/// WHERE rules. Note the flange bound is `Depth / 2`, not `Depth`: a
/// channel has two flanges, so a thickness at half the depth already
/// closes the section.
pub fn u_shape(
    tx: &mut Transaction,
    header: ProfileHeader<'_>,
    dims: FlangedDims,
    extras: UShapeExtras,
) -> Result<EntityId, GeometryError> {
    const T: &str = "IFCUSHAPEPROFILEDEF";
    let FlangedDims {
        depth,
        flange_width,
        web_thickness,
        flange_thickness,
    } = dims;
    positive(T, "Depth", depth)?;
    positive(T, "FlangeWidth", flange_width)?;
    positive(T, "WebThickness", web_thickness)?;
    positive(T, "FlangeThickness", flange_thickness)?;
    non_negative(T, "FilletRadius", extras.fillet_radius)?;
    non_negative(T, "EdgeRadius", extras.edge_radius)?;
    if let Some(slope) = extras.flange_slope {
        require_finite(T, "FlangeSlope", &[slope])?;
    }
    if flange_thickness >= depth / 2.0 {
        return Err(rule(
            T,
            "FlangeThickness",
            format!(
                "{flange_thickness} is not below half the Depth ({})",
                depth / 2.0
            ),
        ));
    }
    if web_thickness >= flange_width {
        return Err(rule(
            T,
            "WebThickness",
            format!("{web_thickness} is not below FlangeWidth {flange_width}"),
        ));
    }
    let mut attrs = parameterized(T, 10, header);
    attrs[slot::U_DEPTH] = Value::Real(depth);
    attrs[slot::U_WIDTH] = Value::Real(flange_width);
    attrs[slot::U_WEB] = Value::Real(web_thickness);
    attrs[slot::U_FLANGE] = Value::Real(flange_thickness);
    put_opt(&mut attrs, slot::U_FILLET, extras.fillet_radius);
    put_opt(&mut attrs, slot::U_EDGE, extras.edge_radius);
    put_opt(&mut attrs, slot::U_SLOPE, extras.flange_slope);
    Ok(tx.create(Entity::new(T, attrs)))
}

/// The optional radii a Z-section may carry.
#[derive(Debug, Clone, Copy, Default)]
pub struct ZShapeExtras {
    /// `FilletRadius`: the web-to-flange transitions.
    pub fillet_radius: Option<f64>,
    /// `EdgeRadius`: the flange tips.
    pub edge_radius: Option<f64>,
}

/// Stage an `IfcZShapeProfileDef`.
///
/// # Errors
///
/// Refuses a non-positive dimension, a negative radius, and the
/// `ValidFlangeThickness` rule (`FlangeThickness < Depth / 2`). A Z has
/// two opposed flanges, so the bound is half the depth.
pub fn z_shape(
    tx: &mut Transaction,
    header: ProfileHeader<'_>,
    dims: FlangedDims,
    extras: ZShapeExtras,
) -> Result<EntityId, GeometryError> {
    const T: &str = "IFCZSHAPEPROFILEDEF";
    let FlangedDims {
        depth,
        flange_width,
        web_thickness,
        flange_thickness,
    } = dims;
    positive(T, "Depth", depth)?;
    positive(T, "FlangeWidth", flange_width)?;
    positive(T, "WebThickness", web_thickness)?;
    positive(T, "FlangeThickness", flange_thickness)?;
    non_negative(T, "FilletRadius", extras.fillet_radius)?;
    non_negative(T, "EdgeRadius", extras.edge_radius)?;
    if flange_thickness >= depth / 2.0 {
        return Err(rule(
            T,
            "FlangeThickness",
            format!(
                "{flange_thickness} is not below half the Depth ({})",
                depth / 2.0
            ),
        ));
    }
    let mut attrs = parameterized(T, 9, header);
    attrs[slot::Z_DEPTH] = Value::Real(depth);
    attrs[slot::Z_FLANGE_WIDTH] = Value::Real(flange_width);
    attrs[slot::Z_WEB] = Value::Real(web_thickness);
    attrs[slot::Z_FLANGE] = Value::Real(flange_thickness);
    put_opt(&mut attrs, slot::Z_FILLET, extras.fillet_radius);
    put_opt(&mut attrs, slot::Z_EDGE, extras.edge_radius);
    Ok(tx.create(Entity::new(T, attrs)))
}
/// Stage an `IfcCShapeProfileDef`.
///
/// # Errors
///
/// Refuses a non-positive dimension, a negative radius, and the three
/// WHERE rules: `Girth < Depth / 2`, `WallThickness` below half of both
/// `Width` and `Depth`, and `InternalFilletRadius` within the room the
/// wall leaves on both axes.
pub fn c_shape(
    tx: &mut Transaction,
    header: ProfileHeader<'_>,
    dims: CShapeDims,
    internal_fillet_radius: Option<f64>,
) -> Result<EntityId, GeometryError> {
    const T: &str = "IFCCSHAPEPROFILEDEF";
    let CShapeDims {
        depth,
        width,
        wall_thickness,
        girth,
    } = dims;
    positive(T, "Depth", depth)?;
    positive(T, "Width", width)?;
    positive(T, "WallThickness", wall_thickness)?;
    positive(T, "Girth", girth)?;
    non_negative(T, "InternalFilletRadius", internal_fillet_radius)?;
    if girth >= depth / 2.0 {
        return Err(rule(
            T,
            "Girth",
            format!("{girth} is not below half the Depth ({})", depth / 2.0),
        ));
    }
    if wall_thickness >= width / 2.0 || wall_thickness >= depth / 2.0 {
        return Err(rule(
            T,
            "WallThickness",
            format!(
                "{wall_thickness} is not below half of both Width ({}) and Depth ({})",
                width / 2.0,
                depth / 2.0
            ),
        ));
    }
    if let Some(fillet) = internal_fillet_radius {
        let across_width = width / 2.0 - wall_thickness;
        let across_depth = depth / 2.0 - wall_thickness;
        if fillet > across_width || fillet > across_depth {
            return Err(rule(
                T,
                "InternalFilletRadius",
                format!(
                    "{fillet} exceeds the room the wall leaves ({across_width} by {across_depth})"
                ),
            ));
        }
    }
    let mut attrs = parameterized(T, 8, header);
    attrs[slot::C_DEPTH] = Value::Real(depth);
    attrs[slot::C_WIDTH] = Value::Real(width);
    attrs[slot::C_WALL] = Value::Real(wall_thickness);
    attrs[slot::C_GIRTH] = Value::Real(girth);
    put_opt(&mut attrs, slot::C_FILLET, internal_fillet_radius);
    Ok(tx.create(Entity::new(T, attrs)))
}
/// The optional attributes an asymmetric I-section may carry.
///
/// `TopFlangeThickness` is optional in the schema; when absent the top
/// flange is taken to match the bottom, and `ValidFlangeThickness` does
/// not apply.
#[derive(Debug, Clone, Copy, Default)]
pub struct AsymmetricIExtras {
    /// `TopFlangeThickness`.
    pub top_flange_thickness: Option<f64>,
    /// `BottomFlangeFilletRadius`.
    pub bottom_flange_fillet_radius: Option<f64>,
    /// `TopFlangeFilletRadius`.
    pub top_flange_fillet_radius: Option<f64>,
    /// `BottomFlangeEdgeRadius`.
    pub bottom_flange_edge_radius: Option<f64>,
    /// `TopFlangeEdgeRadius`.
    pub top_flange_edge_radius: Option<f64>,
    /// `BottomFlangeSlope`, in radians.
    pub bottom_flange_slope: Option<f64>,
    /// `TopFlangeSlope`, in radians.
    pub top_flange_slope: Option<f64>,
}

/// Stage an `IfcAsymmetricIShapeProfileDef`.
///
/// # Errors
///
/// Refuses a non-positive dimension, a negative radius, and the four
/// WHERE rules: the web must be narrower than both flanges, each fillet
/// is bounded by its own flange's clearance, and the two flange
/// thicknesses together must stay below the overall depth when a top
/// thickness is stated.
pub fn asymmetric_i_shape(
    tx: &mut Transaction,
    header: ProfileHeader<'_>,
    dims: AsymmetricIDims,
    extras: AsymmetricIExtras,
) -> Result<EntityId, GeometryError> {
    const T: &str = "IFCASYMMETRICISHAPEPROFILEDEF";
    let AsymmetricIDims {
        bottom_flange_width,
        overall_depth,
        web_thickness,
        bottom_flange_thickness,
        top_flange_width,
    } = dims;
    positive(T, "BottomFlangeWidth", bottom_flange_width)?;
    positive(T, "OverallDepth", overall_depth)?;
    positive(T, "WebThickness", web_thickness)?;
    positive(T, "BottomFlangeThickness", bottom_flange_thickness)?;
    positive(T, "TopFlangeWidth", top_flange_width)?;
    if let Some(top) = extras.top_flange_thickness {
        positive(T, "TopFlangeThickness", top)?;
    }
    non_negative(
        T,
        "BottomFlangeFilletRadius",
        extras.bottom_flange_fillet_radius,
    )?;
    non_negative(T, "TopFlangeFilletRadius", extras.top_flange_fillet_radius)?;
    non_negative(
        T,
        "BottomFlangeEdgeRadius",
        extras.bottom_flange_edge_radius,
    )?;
    non_negative(T, "TopFlangeEdgeRadius", extras.top_flange_edge_radius)?;
    for (attribute, slope) in [
        ("BottomFlangeSlope", extras.bottom_flange_slope),
        ("TopFlangeSlope", extras.top_flange_slope),
    ] {
        if let Some(slope) = slope {
            require_finite(T, attribute, &[slope])?;
        }
    }
    if web_thickness >= bottom_flange_width || web_thickness >= top_flange_width {
        return Err(rule(
            T,
            "WebThickness",
            format!(
                "{web_thickness} is not below both flange widths ({bottom_flange_width}, {top_flange_width})"
            ),
        ));
    }
    if let Some(top) = extras.top_flange_thickness {
        if bottom_flange_thickness + top >= overall_depth {
            return Err(rule(
                T,
                "TopFlangeThickness",
                format!(
                    "{bottom_flange_thickness} + {top} is not below OverallDepth {overall_depth}"
                ),
            ));
        }
    }
    for (attribute, fillet, width) in [
        (
            "BottomFlangeFilletRadius",
            extras.bottom_flange_fillet_radius,
            bottom_flange_width,
        ),
        (
            "TopFlangeFilletRadius",
            extras.top_flange_fillet_radius,
            top_flange_width,
        ),
    ] {
        if let Some(fillet) = fillet {
            let room = (width - web_thickness) / 2.0;
            if fillet > room {
                return Err(rule(
                    T,
                    attribute,
                    format!("{fillet} exceeds the flange clearance ({room})"),
                ));
            }
        }
    }
    let mut attrs = parameterized(T, 15, header);
    attrs[slot::AI_BOTTOM_WIDTH] = Value::Real(bottom_flange_width);
    attrs[slot::AI_DEPTH] = Value::Real(overall_depth);
    attrs[slot::AI_WEB] = Value::Real(web_thickness);
    attrs[slot::AI_BOTTOM_FLANGE] = Value::Real(bottom_flange_thickness);
    attrs[slot::AI_TOP_WIDTH] = Value::Real(top_flange_width);
    put_opt(&mut attrs, slot::AI_TOP_FLANGE, extras.top_flange_thickness);
    put_opt(
        &mut attrs,
        slot::AI_BOTTOM_FILLET,
        extras.bottom_flange_fillet_radius,
    );
    put_opt(
        &mut attrs,
        slot::AI_TOP_FILLET,
        extras.top_flange_fillet_radius,
    );
    put_opt(
        &mut attrs,
        slot::AI_BOTTOM_EDGE,
        extras.bottom_flange_edge_radius,
    );
    put_opt(&mut attrs, slot::AI_TOP_EDGE, extras.top_flange_edge_radius);
    put_opt(
        &mut attrs,
        slot::AI_BOTTOM_SLOPE,
        extras.bottom_flange_slope,
    );
    put_opt(&mut attrs, slot::AI_TOP_SLOPE, extras.top_flange_slope);
    Ok(tx.create(Entity::new(T, attrs)))
}
