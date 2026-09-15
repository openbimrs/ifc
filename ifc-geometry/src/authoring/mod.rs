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

mod placement;
mod profile;
mod solid;

pub use placement::{axis2_placement_2d, axis2_placement_3d, cartesian_point, direction};
pub use profile::{
    arbitrary_closed_profile, circle_profile, polyline, rectangle_profile, ProfileType,
};
pub use solid::{boolean_result, extruded_area_solid, revolved_area_solid};

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
