//! Author IFC geometry entities from plain numbers.
//!
//! The write direction of this bridge (ADR 0011). Everything here is
//! kernel-free: arguments are `f64`, `[f64; 3]`, and index buffers, no
//! signature names an axiolid type, and the module compiles with
//! `--no-default-features`. That is what lets an application compute with
//! CGAL, OCCT, or Axiolid and still author through one API -- the kernel is
//! a source of numbers, never a dependency of the writer.
//!
//! IFC geometry entities are exact *descriptions*, not evaluations, so
//! writing them needs no evaluator. A cut is stored as an unevaluated
//! `IfcBooleanResult`: this module never tessellates, matching the standing
//! invariant of the read direction.
//!
//! Slot positions are not restated here. Each writer indexes the same
//! `pub(crate) mod slot` constants its reader uses, so a layout correction
//! lands once and serves both directions.

mod brep;
mod csg;
mod curve;
mod placement;
mod profile;
mod profile2;
mod solid;
mod std_profile;
mod surface;
mod swept;
mod tessellation;
mod transform;

pub use brep::{
    edge, edge_curve, edge_loop, face, face_based_surface_model, face_bound, face_outer_bound,
    face_surface, manifold_solid_brep, oriented_edge, poly_loop, shell, shell_based_surface_model,
    subedge, vertex_loop, vertex_point, BrepKind, ShellKind,
};
pub use csg::{
    block, bounding_box, boxed_half_space, cone, csg_solid, cylinder, half_space,
    polygonal_bounded_half_space, rectangular_pyramid, sphere,
};
pub use curve::{
    bspline_curve_with_knots, circle, composite_curve, composite_curve_segment, ellipse,
    indexed_poly_curve, line, offset_curve_2d, offset_curve_3d, rational_bspline_curve_with_knots,
    trimmed_curve, vector, KnotVector, PolyCurveSegment,
};
pub use placement::{axis2_placement_2d, axis2_placement_3d, cartesian_point, direction};
pub use profile::{
    arbitrary_closed_profile, circle_profile, polyline, rectangle_profile, ProfileType,
};
pub use profile2::{
    arbitrary_open_profile, arbitrary_profile_with_voids, center_line_profile, composite_profile,
    derived_profile, mirrored_profile, rounded_rectangle_profile,
};
pub use solid::{boolean_result, extruded_area_solid, revolved_area_solid};
pub use std_profile::{
    asymmetric_i_shape, c_shape, circle_hollow_profile, ellipse_profile, i_shape, l_shape,
    rectangle_hollow_profile, t_shape, trapezium_profile, u_shape, z_shape, AsymmetricIDims,
    AsymmetricIExtras, CShapeDims, FlangedDims, IShapeDims, IShapeExtras, LShapeExtras,
    ProfileHeader, RectangleHollowDims, RectangleHollowFillets, TShapeExtras, TrapeziumDims,
    UShapeExtras, ZShapeExtras,
};
pub use surface::{
    bspline_surface_with_knots, curve_bounded_plane, curve_bounded_surface,
    rational_bspline_surface_with_knots, rectangular_trimmed_surface, spherical_surface,
    toroidal_surface, SurfaceBasis, SurfaceKnots,
};
pub use swept::{
    axis1_placement, cylindrical_surface, extruded_area_solid_tapered,
    fixed_reference_swept_area_solid, plane, revolved_area_solid_tapered,
    surface_curve_swept_area_solid, surface_of_linear_extrusion, surface_of_revolution,
    swept_disk_solid, swept_disk_solid_polygonal, SweepTrim,
};
pub use tessellation::{
    cartesian_point_list_2d, cartesian_point_list_3d, indexed_polygonal_face,
    indexed_polygonal_face_with_voids, polygonal_face_set, triangulated_face_set,
    TriangulatedExtras,
};
pub use transform::{
    mapped_item, representation_map, topology_representation, transformation_operator_2d,
    transformation_operator_2d_non_uniform, transformation_operator_3d,
    transformation_operator_3d_non_uniform, Transform,
};

use ifc_model::{EntityId, Value};

use crate::error::GeometryError;

/// Build an [`GeometryError::InvalidAuthoredValue`].
fn invalid(
    type_name: &'static str,
    attribute: &'static str,
    detail: impl Into<String>,
) -> GeometryError {
    GeometryError::InvalidAuthoredValue {
        type_name,
        attribute,
        detail: detail.into(),
    }
}

/// Refuse NaN and infinity before they reach a file.
///
/// Every downstream consumer -- placement composition, profile bounds,
/// tessellation -- assumes finite input, and a NaN coordinate propagates
/// silently through all of them rather than failing where it was introduced.
fn require_finite(
    type_name: &'static str,
    attribute: &'static str,
    values: &[f64],
) -> Result<(), GeometryError> {
    if let Some(bad) = values.iter().position(|v| !v.is_finite()) {
        return Err(invalid(
            type_name,
            attribute,
            format!("value at index {bad} is not finite"),
        ));
    }
    Ok(())
}

/// A list of `IfcReal`/`IfcLengthMeasure` values.
fn reals(values: &[f64]) -> Value {
    Value::List(values.iter().copied().map(Value::Real).collect())
}

/// A list of entity references.
fn refs(ids: &[EntityId]) -> Value {
    Value::List(ids.iter().copied().map(Value::Ref).collect())
}
