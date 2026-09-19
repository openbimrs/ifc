//! The closed catalogue sections: ellipse, trapezium and the two hollow profiles.
//!
//! Split from the flanged sections purely for file size.

use ifc_model::{Entity, EntityId, Transaction, Value};

use crate::error::GeometryError;
use crate::slots::profile_slot as base;
use crate::slots::section_slot as slot;

use super::super::require_finite;
use super::{
    non_negative, parameterized, positive, put_opt, rule, ProfileHeader, RectangleHollowDims,
    RectangleHollowFillets, TrapeziumDims,
};

/// Stage an `IfcEllipseProfileDef`.
///
/// # Errors
///
/// Refuses a non-positive semi-axis. The schema states no WHERE rule:
/// `SemiAxis1` need not exceed `SemiAxis2`, so an ellipse taller than it
/// is wide is written as given rather than silently reordered.
pub fn ellipse_profile(
    tx: &mut Transaction,
    header: ProfileHeader<'_>,
    semi_axis_1: f64,
    semi_axis_2: f64,
) -> Result<EntityId, GeometryError> {
    const T: &str = "IFCELLIPSEPROFILEDEF";
    positive(T, "SemiAxis1", semi_axis_1)?;
    positive(T, "SemiAxis2", semi_axis_2)?;
    let mut attrs = parameterized(T, 5, header);
    attrs[slot::E_SEMI1] = Value::Real(semi_axis_1);
    attrs[slot::E_SEMI2] = Value::Real(semi_axis_2);
    Ok(tx.create(Entity::new(T, attrs)))
}

/// Stage an `IfcTrapeziumProfileDef`.
///
/// # Errors
///
/// Refuses a non-positive dimension. `TopXOffset` is an
/// `IfcLengthMeasure`, not a positive one: a negative offset leans the
/// top edge the other way and is legal, so it is only checked for
/// finiteness.
pub fn trapezium_profile(
    tx: &mut Transaction,
    header: ProfileHeader<'_>,
    dims: TrapeziumDims,
) -> Result<EntityId, GeometryError> {
    const T: &str = "IFCTRAPEZIUMPROFILEDEF";
    let TrapeziumDims {
        bottom_x_dim,
        top_x_dim,
        y_dim,
        top_x_offset,
    } = dims;
    positive(T, "BottomXDim", bottom_x_dim)?;
    positive(T, "TopXDim", top_x_dim)?;
    positive(T, "YDim", y_dim)?;
    require_finite(T, "TopXOffset", &[top_x_offset])?;
    let mut attrs = parameterized(T, 7, header);
    attrs[slot::TZ_BOTTOM] = Value::Real(bottom_x_dim);
    attrs[slot::TZ_TOP] = Value::Real(top_x_dim);
    attrs[slot::TZ_Y] = Value::Real(y_dim);
    attrs[slot::TZ_OFFSET] = Value::Real(top_x_offset);
    Ok(tx.create(Entity::new(T, attrs)))
}
/// Stage an `IfcRectangleHollowProfileDef`.
///
/// Inherits `XDim` and `YDim` from `IfcRectangleProfileDef`, so the
/// wall and fillet slots follow at 5, 6 and 7.
///
/// # Errors
///
/// Refuses a non-positive dimension, a negative radius, and the three
/// WHERE rules: the wall must be below half of both dimensions, the
/// inner fillet must fit the bore the wall leaves, and the outer fillet
/// must not exceed either half-dimension.
pub fn rectangle_hollow_profile(
    tx: &mut Transaction,
    header: ProfileHeader<'_>,
    dims: RectangleHollowDims,
    fillets: RectangleHollowFillets,
) -> Result<EntityId, GeometryError> {
    const T: &str = "IFCRECTANGLEHOLLOWPROFILEDEF";
    let RectangleHollowDims {
        x_dim,
        y_dim,
        wall_thickness,
    } = dims;
    let inner_fillet_radius = fillets.inner;
    let outer_fillet_radius = fillets.outer;
    positive(T, "XDim", x_dim)?;
    positive(T, "YDim", y_dim)?;
    positive(T, "WallThickness", wall_thickness)?;
    non_negative(T, "InnerFilletRadius", inner_fillet_radius)?;
    non_negative(T, "OuterFilletRadius", outer_fillet_radius)?;
    let half_x = x_dim / 2.0;
    let half_y = y_dim / 2.0;
    if wall_thickness >= half_x || wall_thickness >= half_y {
        return Err(rule(
            T,
            "WallThickness",
            format!("{wall_thickness} is not below half of both dimensions ({half_x}, {half_y})"),
        ));
    }
    if let Some(inner) = inner_fillet_radius {
        let bore_x = half_x - wall_thickness;
        let bore_y = half_y - wall_thickness;
        if inner > bore_x || inner > bore_y {
            return Err(rule(
                T,
                "InnerFilletRadius",
                format!("{inner} exceeds the bore the wall leaves ({bore_x} by {bore_y})"),
            ));
        }
    }
    if let Some(outer) = outer_fillet_radius {
        if outer > half_x || outer > half_y {
            return Err(rule(
                T,
                "OuterFilletRadius",
                format!("{outer} exceeds a half-dimension ({half_x}, {half_y})"),
            ));
        }
    }
    let mut attrs = parameterized(T, 8, header);
    attrs[base::X_DIM] = Value::Real(x_dim);
    attrs[base::Y_DIM] = Value::Real(y_dim);
    attrs[base::RECT_WALL_THICKNESS] = Value::Real(wall_thickness);
    put_opt(&mut attrs, base::RECT_INNER_RADIUS, inner_fillet_radius);
    put_opt(&mut attrs, base::RECT_OUTER_RADIUS, outer_fillet_radius);
    Ok(tx.create(Entity::new(T, attrs)))
}

/// Stage an `IfcCircleHollowProfileDef`.
///
/// # Errors
///
/// Refuses a non-positive dimension and the `WR1` rule: the wall must
/// be thinner than the radius, otherwise the bore vanishes and the
/// section is a solid disc described as a tube.
pub fn circle_hollow_profile(
    tx: &mut Transaction,
    header: ProfileHeader<'_>,
    radius: f64,
    wall_thickness: f64,
) -> Result<EntityId, GeometryError> {
    const T: &str = "IFCCIRCLEHOLLOWPROFILEDEF";
    positive(T, "Radius", radius)?;
    positive(T, "WallThickness", wall_thickness)?;
    if wall_thickness >= radius {
        return Err(rule(
            T,
            "WallThickness",
            format!("{wall_thickness} is not below the Radius {radius}"),
        ));
    }
    let mut attrs = parameterized(T, 5, header);
    attrs[base::RADIUS] = Value::Real(radius);
    attrs[base::CIRCLE_WALL_THICKNESS] = Value::Real(wall_thickness);
    Ok(tx.create(Entity::new(T, attrs)))
}
