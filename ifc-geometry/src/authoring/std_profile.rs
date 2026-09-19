//! Standard parameterized sections: the catalogue profiles.
//!
//! These are the sections a structural catalogue names -- an IPE beam,
//! an angle, a channel, a square hollow tube. Each is a fixed list of
//! dimensions, so authoring one is writing numbers into slots; no
//! curve construction and no evaluator is involved.
//!
//! Slots come from [`crate::lower::profile::section_slot`], the same
//! constants the lowerer reads. A layout correction lands once and
//! serves both directions.
//!
//! # Refusals are schema rules, not taste
//!
//! IFC declares the load-bearing dimensions as
//! `IfcPositiveLengthMeasure`, so zero is invalid rather than merely
//! degenerate, and each entity carries WHERE rules relating those
//! dimensions -- a web thinner than the flange it joins, a girth under
//! half the depth. This module enforces them at authoring time because
//! `ifc-validate` does not implement profile WHERE rules: a section
//! violating them is a record that parses and describes an impossible
//! shape.
//!
//! Fillet and edge radii are `IfcNonNegativeLengthMeasure`, where zero
//! is legal and means a sharp corner. They are checked for
//! non-negativity, never for positivity.

use ifc_model::{EntityId, Value};

use crate::error::GeometryError;
use crate::slots::profile_slot as base;

use super::profile::ProfileType;
use super::{invalid, require_finite};
/// The two attributes every parameterized profile carries.
///
/// Bundled so a section's own dimensions are not separated from each
/// other by two unrelated arguments at the call site.
#[derive(Debug, Clone, Copy, Default)]
pub struct ProfileHeader<'a> {
    /// `ProfileName`.
    pub name: Option<&'a str>,
    /// `Position : IfcAxis2Placement2D`.
    pub position: Option<EntityId>,
}

/// The four required dimensions of an I-section.
#[derive(Debug, Clone, Copy)]
pub struct IShapeDims {
    /// `OverallWidth`.
    pub width: f64,
    /// `OverallDepth`.
    pub depth: f64,
    /// `WebThickness`.
    pub web_thickness: f64,
    /// `FlangeThickness`.
    pub flange_thickness: f64,
}

/// The required dimensions of a T, U or Z section.
///
/// The three share a shape: a depth, a flange width, and the two
/// thicknesses. They differ only in which WHERE rules bound them.
#[derive(Debug, Clone, Copy)]
pub struct FlangedDims {
    /// `Depth`.
    pub depth: f64,
    /// `FlangeWidth`.
    pub flange_width: f64,
    /// `WebThickness`.
    pub web_thickness: f64,
    /// `FlangeThickness`.
    pub flange_thickness: f64,
}

/// The required dimensions of a C-section.
#[derive(Debug, Clone, Copy)]
pub struct CShapeDims {
    /// `Depth`.
    pub depth: f64,
    /// `Width`.
    pub width: f64,
    /// `WallThickness`.
    pub wall_thickness: f64,
    /// `Girth`.
    pub girth: f64,
}

/// The required dimensions of an asymmetric I-section.
#[derive(Debug, Clone, Copy)]
pub struct AsymmetricIDims {
    /// `BottomFlangeWidth`.
    pub bottom_flange_width: f64,
    /// `OverallDepth`.
    pub overall_depth: f64,
    /// `WebThickness`.
    pub web_thickness: f64,
    /// `BottomFlangeThickness`.
    pub bottom_flange_thickness: f64,
    /// `TopFlangeWidth`.
    pub top_flange_width: f64,
}

/// The required dimensions of a rectangular hollow section.
#[derive(Debug, Clone, Copy)]
pub struct RectangleHollowDims {
    /// `XDim`.
    pub x_dim: f64,
    /// `YDim`.
    pub y_dim: f64,
    /// `WallThickness`.
    pub wall_thickness: f64,
}

/// The two optional fillet radii of a rectangular hollow section.
#[derive(Debug, Clone, Copy, Default)]
pub struct RectangleHollowFillets {
    /// `InnerFilletRadius`.
    pub inner: Option<f64>,
    /// `OuterFilletRadius`.
    pub outer: Option<f64>,
}

/// The required dimensions of a trapezium.
#[derive(Debug, Clone, Copy)]
pub struct TrapeziumDims {
    /// `BottomXDim`.
    pub bottom_x_dim: f64,
    /// `TopXDim`.
    pub top_x_dim: f64,
    /// `YDim`.
    pub y_dim: f64,
    /// `TopXOffset`, which may be negative.
    pub top_x_offset: f64,
}

/// Fill the three `IfcParameterizedProfileDef` slots every section shares.
///
/// `ProfileType` is always `.AREA.`: each entity in this module bounds a
/// closed region, so `.CURVE.` would contradict the declaration.
pub(super) fn parameterized(
    type_name: &'static str,
    arity: usize,
    header: ProfileHeader<'_>,
) -> Vec<Value> {
    let mut attrs = vec![Value::Null; arity];
    debug_assert!(!type_name.is_empty());
    attrs[base::PROFILE_TYPE] = Value::Enum(ProfileType::Area.token().into());
    attrs[base::PROFILE_NAME] = header.name.map_or(Value::Null, |n| Value::Text(n.into()));
    attrs[base::POSITION] = header.position.map_or(Value::Null, Value::Ref);
    attrs
}

/// Require a strictly positive, finite `IfcPositiveLengthMeasure`.
pub(super) fn positive(
    type_name: &'static str,
    attribute: &'static str,
    value: f64,
) -> Result<(), GeometryError> {
    require_finite(type_name, attribute, &[value])?;
    if value <= 0.0 {
        return Err(invalid(
            type_name,
            attribute,
            format!("expected a positive length, got {value}"),
        ));
    }
    Ok(())
}

/// Require a finite, non-negative `IfcNonNegativeLengthMeasure`.
///
/// Zero is accepted: a fillet radius of zero is a sharp corner, which is
/// a real section, not a degenerate one.
pub(super) fn non_negative(
    type_name: &'static str,
    attribute: &'static str,
    value: Option<f64>,
) -> Result<(), GeometryError> {
    let Some(value) = value else { return Ok(()) };
    require_finite(type_name, attribute, &[value])?;
    if value < 0.0 {
        return Err(invalid(
            type_name,
            attribute,
            format!("expected a non-negative length, got {value}"),
        ));
    }
    Ok(())
}

/// Refuse a WHERE-rule violation.
pub(super) fn rule(
    type_name: &'static str,
    attribute: &'static str,
    detail: String,
) -> GeometryError {
    invalid(type_name, attribute, detail)
}

/// Write an optional length into its slot when present.
pub(super) fn put_opt(attrs: &mut [Value], index: usize, value: Option<f64>) {
    if let Some(value) = value {
        attrs[index] = Value::Real(value);
    }
}

mod closed_section;
mod flanged_section;

pub use closed_section::{
    circle_hollow_profile, ellipse_profile, rectangle_hollow_profile, trapezium_profile,
};
pub use flanged_section::{
    asymmetric_i_shape, c_shape, i_shape, l_shape, t_shape, u_shape, z_shape, AsymmetricIExtras,
    IShapeExtras, LShapeExtras, TShapeExtras, UShapeExtras, ZShapeExtras,
};
