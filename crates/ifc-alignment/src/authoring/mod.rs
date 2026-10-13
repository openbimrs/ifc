//! Author IFC4X3 alignment entities from plain numbers.
//!
//! Mirrors ADR 0011: this is the write direction of the same bridge that
//! reads alignments, and it indexes `crate::slot` rather than restating a
//! single attribute position.
//!
//! Authoring records is independent of 3D composition. A file states its
//! horizontal layout, its vertical profile and its cant as separate
//! parameter segments; composing them into a centreline is a lowering
//! concern and does not gate writing the records.
//!
//! Values are plain `f64` in the units the caller declares in the file.
//! Nothing here converts units: the readers apply `AlignmentUnits` on the
//! way out, so applying a factor here too would double-scale.

mod layout;
mod linear;
mod referent;
mod segment;

pub use layout::{alignment, alignment_segment, cant_layout, horizontal_layout, vertical_layout};
pub use linear::{linear_element, linear_positioning_element};
pub use referent::{
    axis2_placement_linear, cartesian_point, linear_placement, point_by_distance, referent,
    stationing,
};
pub use segment::{
    cant_segment, horizontal_segment, vertical_segment, CantSegmentDraft, HorizontalSegmentDraft,
    VerticalSegmentDraft,
};

use ifc_model::guid::Guid;
use ifc_model::Value;

use crate::error::AlignmentError;

/// Refuse an authored value before anything is staged.
pub(crate) fn invalid(
    type_name: &'static str,
    attribute: &'static str,
    detail: impl Into<String>,
) -> AlignmentError {
    AlignmentError::InvalidAuthoredValue {
        type_name,
        attribute,
        detail: detail.into(),
    }
}

/// Refuse a non-finite measure. NaN and infinity parse back as garbage
/// rather than failing, so they are rejected at the boundary.
pub(crate) fn finite(
    type_name: &'static str,
    attribute: &'static str,
    value: f64,
) -> Result<(), AlignmentError> {
    if value.is_finite() {
        Ok(())
    } else {
        Err(invalid(
            type_name,
            attribute,
            format!("{value} is not finite"),
        ))
    }
}

/// An IFC GlobalId, checked by [`Guid::parse`]: 22 characters of the IFC
/// base-64 alphabet whose leading digit is `0`-`3`, so the text names a
/// 128-bit UUID. A wrong length, an alphabet the encoding does not use and
/// an out-of-range leading digit are all malformed.
pub(crate) fn guid(type_name: &'static str, value: &str) -> Result<Value, AlignmentError> {
    if Guid::parse(value).is_none() {
        return Err(invalid(
            type_name,
            "GlobalId",
            format!(
                "{value:?} is not a compressed IFC GlobalId (22 characters of \
                 0-9 A-Z a-z _ $, the first 0-3)"
            ),
        ));
    }
    Ok(Value::Text(value.into()))
}
