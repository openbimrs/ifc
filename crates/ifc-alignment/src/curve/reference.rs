//! The cant-carrying 3D centreline: the `IfcSegmentedReferenceCurve` role.
//!
//! IFC4.3 ADD2 represents an alignment with cant as an
//! `IfcSegmentedReferenceCurve` whose base curve is the `IfcGradientCurve`:
//! the cant layout adds, as a function of distance along, a rotation of the
//! cross-section about the centreline tangent and a deviating elevation of
//! the rotation point ("Alignment Geometry - Horizontal, Vertical and Cant").
//!
//! The pinned neutral vocabulary (axiolid-curve 0.3) has no value for that:
//! `Curve3::Elevated` pairs a plan with an `ElevationLaw` and carries no
//! roll, and no other curve or relation attaches a bank angle law to a
//! curve. Inventing one here would put a kernel concept in an IFC crate, and
//! flattening cant into point frames would be sampling. So the composition
//! is a typed refusal until Axiolid has the primitive; the exact cant
//! itself is available as data through `CantLayout::frame_at_distance`.

use ifc_model::{EntityId, Model};

use super::assemble::LoweredAlignmentCurve;
use crate::cant::CantLayout;
use crate::error::{AlignmentError, AlignmentResult};
use crate::horizontal::AlignmentUnits;

/// Lower an `IfcAlignment` with cant to its exact cant-carrying centreline.
///
/// Always refuses today. The alignment and its cant layout are resolved
/// first, so a caller learns about a missing or ambiguous cant layout, or a
/// malformed cant segment, before the structural gap.
///
/// # Errors
///
/// - Everything [`CantLayout::for_alignment`] refuses: a wrong entity type,
///   no `IfcAlignmentCant` or several (`SemanticViolation`), a malformed or
///   discontinuous cant layout.
/// - Otherwise [`AlignmentError::Unsupported`] naming
///   `IfcSegmentedReferenceCurve` at the cant layout: the pinned neutral
///   curve vocabulary has no roll law to carry cant exactly.
pub fn lower_segmented_reference_curve(
    model: &Model,
    alignment: EntityId,
    units: AlignmentUnits,
) -> AlignmentResult<LoweredAlignmentCurve> {
    let cant = CantLayout::for_alignment(model, alignment, units)?;
    Err(AlignmentError::Unsupported {
        entity: cant.entity,
        type_name: "IfcSegmentedReferenceCurve".to_owned(),
        detail: "the pinned neutral curve vocabulary has no roll (bank angle) law to carry cant on the 3D centreline",
    })
}
