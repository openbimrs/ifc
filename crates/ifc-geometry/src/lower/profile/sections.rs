//! The `IfcParameterizedProfileDef` families mapped onto neutral profiles.
//!
//! Every value arrives in metres and radians from
//! [`crate::input::profile::describe_profile`]; this module renames fields
//! and nothing else. `Position` is applied by the caller, uniformly for every
//! family.

use axiolid_profile::{CircleProfile, EllipseProfile, Profile, RectangleProfile, SectionProfile};

use crate::error::{GeometryError, GeometryResult};
use crate::input::profile::{ProfileDescription, ProfileParameters};

/// Map one parameterised family onto its neutral profile.
pub(super) fn parameterized(
    description: &ProfileDescription,
    parameters: &ProfileParameters,
) -> GeometryResult<Profile> {
    let profile = match *parameters {
        ProfileParameters::Rectangle { x_dim, y_dim } => rectangle(x_dim, y_dim, None, None, None),
        ProfileParameters::RoundedRectangle {
            x_dim,
            y_dim,
            rounding_radius,
        } => rectangle(x_dim, y_dim, None, Some(rounding_radius), None),
        ProfileParameters::RectangleHollow {
            x_dim,
            y_dim,
            wall_thickness,
            inner_fillet_radius,
            outer_fillet_radius,
        } => rectangle(
            x_dim,
            y_dim,
            Some(wall_thickness),
            outer_fillet_radius,
            inner_fillet_radius,
        ),
        ProfileParameters::Circle { radius } => Profile::Circle(CircleProfile {
            radius,
            thickness: None,
        }),
        ProfileParameters::CircleHollow {
            radius,
            wall_thickness,
        } => Profile::Circle(CircleProfile {
            radius,
            thickness: Some(wall_thickness),
        }),
        ProfileParameters::Ellipse {
            semi_axis_1,
            semi_axis_2,
        } => Profile::Ellipse(EllipseProfile {
            semi_axis_x: semi_axis_1,
            semi_axis_y: semi_axis_2,
        }),
        ProfileParameters::IShape {
            overall_width,
            overall_depth,
            web_thickness,
            flange_thickness,
            fillet_radius,
            flange_edge_radius,
            flange_slope,
        } => Profile::Section(SectionProfile::I {
            depth: overall_depth,
            width: overall_width,
            web_thickness,
            flange_thickness,
            fillet_radius,
            flange_edge_radius,
            flange_slope,
        }),
        // Kept distinct from the symmetric variant on purpose: folding the two
        // flanges into `SectionProfile::I` would force a choice of one, and
        // the section would have the wrong area and second moment.
        ProfileParameters::AsymmetricIShape {
            bottom_flange_width,
            overall_depth,
            web_thickness,
            bottom_flange_thickness,
            bottom_flange_fillet_radius,
            top_flange_width,
            top_flange_thickness,
            top_flange_fillet_radius,
            bottom_flange_edge_radius,
            bottom_flange_slope,
            top_flange_edge_radius,
            top_flange_slope,
        } => Profile::Section(SectionProfile::AsymmetricI {
            depth: overall_depth,
            web_thickness,
            bottom_flange_width,
            bottom_flange_thickness,
            bottom_fillet_radius: bottom_flange_fillet_radius,
            bottom_flange_edge_radius,
            bottom_flange_slope,
            top_flange_width,
            top_flange_thickness,
            top_fillet_radius: top_flange_fillet_radius,
            top_flange_edge_radius,
            top_flange_slope,
        }),
        ProfileParameters::LShape {
            depth,
            width,
            thickness,
            fillet_radius,
            edge_radius,
            leg_slope,
        } => Profile::Section(SectionProfile::L {
            depth,
            width,
            thickness,
            fillet_radius,
            edge_radius,
            leg_slope,
        }),
        ProfileParameters::TShape {
            depth,
            flange_width,
            web_thickness,
            flange_thickness,
            fillet_radius,
            flange_edge_radius,
            web_edge_radius,
            web_slope,
            flange_slope,
        } => Profile::Section(SectionProfile::T {
            depth,
            flange_width,
            web_thickness,
            flange_thickness,
            fillet_radius,
            flange_edge_radius,
            web_edge_radius,
            web_slope,
            flange_slope,
        }),
        ProfileParameters::UShape {
            depth,
            flange_width,
            web_thickness,
            flange_thickness,
            fillet_radius,
            edge_radius,
            flange_slope,
        } => Profile::Section(SectionProfile::U {
            depth,
            flange_width,
            web_thickness,
            flange_thickness,
            fillet_radius,
            edge_radius,
            flange_slope,
        }),
        ProfileParameters::CShape {
            depth,
            width,
            wall_thickness,
            girth,
            internal_fillet_radius,
        } => Profile::Section(SectionProfile::C {
            depth,
            width,
            wall_thickness,
            girth,
            internal_fillet_radius,
        }),
        ProfileParameters::ZShape {
            depth,
            flange_width,
            web_thickness,
            flange_thickness,
            fillet_radius,
            edge_radius,
        } => Profile::Section(SectionProfile::Z {
            depth,
            flange_width,
            web_thickness,
            flange_thickness,
            fillet_radius,
            edge_radius,
        }),
        ProfileParameters::Trapezium {
            bottom_x_dim,
            top_x_dim,
            y_dim,
            top_x_offset,
        } => Profile::Section(SectionProfile::Trapezium {
            bottom_x: bottom_x_dim,
            top_x: top_x_dim,
            y: y_dim,
            top_offset: top_x_offset,
        }),
        // The caller handles every non-parameterised family before reaching
        // here; a new one must be routed there, not silently accepted.
        ProfileParameters::ArbitraryClosed { .. }
        | ProfileParameters::ArbitraryWithVoids { .. }
        | ProfileParameters::ArbitraryOpen { .. }
        | ProfileParameters::CenterLine { .. }
        | ProfileParameters::Composite { .. }
        | ProfileParameters::Derived { .. }
        | ProfileParameters::Mirrored { .. } => {
            return Err(GeometryError::Unsupported {
                entity: description.entity,
                type_name: description.type_name.clone(),
                detail: "profile family is not a parameterized section",
            })
        }
    };
    Ok(profile)
}

/// A rectangle, rounded rectangle or rectangular hollow section.
fn rectangle(
    x: f64,
    y: f64,
    thickness: Option<f64>,
    outer_radius: Option<f64>,
    inner_radius: Option<f64>,
) -> Profile {
    Profile::Rectangle(RectangleProfile {
        x,
        y,
        thickness,
        outer_radius,
        inner_radius,
    })
}
