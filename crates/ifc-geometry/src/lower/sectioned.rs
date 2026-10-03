//! IFC4X3 sectioned sweeps along a directrix: typed refusals by name.
//!
//! `IfcSectionedSolidHorizontal` and `IfcSectionedSurface` sweep a list of
//! cross sections along a `Directrix`. Each section sits at an
//! `IfcAxis2PlacementLinear` whose `Location` is an
//! `IfcPointByDistanceExpression`: a distance (or parameter) along a basis
//! curve plus lateral and vertical offsets. IFC4.3 then defines the shape
//! between stations by linear interpolation "between profile points with the
//! same tag along the directrix", with the profile normal taken from the
//! placement rather than the directrix tangent.
//!
//! # Why both are refused
//!
//! Neither has an exact neutral carrier in Axiolid 0.3:
//!
//! - `SolidOperation::SectionedSpine` takes sections at RESOLVED frames
//!   (`Transform3`). A station here is a curve measure; resolving it to a
//!   frame is curve evaluation at a distance, which ADR 0004 keeps out of
//!   lowering (it is the opt-in `constraint::placement::derive` capability).
//!   Even with resolved frames, `SectionedSpine` states no horizontal frame
//!   law between stations and no tag-matched interpolation, so mapping onto
//!   it would substitute a different solid.
//! - Axiolid has no sectioned-SURFACE relation at all, and its sections are
//!   open profiles (`IfcOpenCrossProfileDef`), which `SectionedSpine`, a
//!   solid over area profiles, cannot take.
//!
//! Each refusal is [`crate::GeometryError::Unsupported`] naming the entity
//! and the missing neutral primitive. The open cross profile itself lowers
//! exactly through [`crate::lower::lower_open_profile_node`].
//!
//! `IfcSectionedSolid`, the parent of `IfcSectionedSolidHorizontal`, is
//! ABSTRACT in IFC4X3 ADD2 and so never appears in a file.

use axiolid_model::NodeId;
use ifc_model::EntityId;

use crate::error::GeometryResult;
use crate::lower::session::LoweringSession;

/// IFC type of the horizontal sectioned solid.
pub(crate) const SECTIONED_SOLID_HORIZONTAL_TYPE: &str = "IFCSECTIONEDSOLIDHORIZONTAL";
/// IFC type of the sectioned surface.
pub(crate) const SECTIONED_SURFACE_TYPE: &str = "IFCSECTIONEDSURFACE";

/// Why an `IfcSectionedSolidHorizontal` is refused.
pub(crate) const SECTIONED_SOLID_HORIZONTAL: &str =
    "kernel: sections stand at IfcAxis2PlacementLinear stations (a measure \
     along the directrix plus offsets) and are swept horizontally with \
     tag-matched linear interpolation; the neutral SectionedSpine takes only \
     resolved section frames, and resolving a station is curve evaluation";

/// Why an `IfcSectionedSurface` is refused.
pub(crate) const SECTIONED_SURFACE: &str =
    "kernel: no neutral sectioned-surface relation exists; its open sections \
     stand at IfcAxis2PlacementLinear stations along the directrix and are \
     joined by tag, and the neutral SectionedSpine is a solid over area \
     profiles";

/// Refuse an `IfcSectionedSolidHorizontal` with its named reason.
pub(crate) fn lower_sectioned_solid_horizontal_node(
    session: &mut LoweringSession<'_>,
    id: EntityId,
) -> GeometryResult<NodeId> {
    Err(session.unsupported(
        id,
        SECTIONED_SOLID_HORIZONTAL_TYPE,
        SECTIONED_SOLID_HORIZONTAL,
    ))
}

/// Refuse an `IfcSectionedSurface` with its named reason.
pub(crate) fn lower_sectioned_surface_node(
    session: &mut LoweringSession<'_>,
    id: EntityId,
) -> GeometryResult<NodeId> {
    Err(session.unsupported(id, SECTIONED_SURFACE_TYPE, SECTIONED_SURFACE))
}
