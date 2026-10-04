//! `IfcAxis2PlacementLinear` axes, and how a cross section stands at one.
//!
//! # A placement's axes
//!
//! IFC4.3 ADD2 8.9.3.4: `Axis` is "the exact direction of the local Z
//! Axis"; `RefDirection` is "the direction used to determine the direction
//! of the local X Axis", adjusted "to maintain orthogonality to the Axis
//! direction", and "if RefDirection is omitted, the direction is taken from
//! the curve tangent"; both "are relative to the curve used for linear
//! referencing ..., maintaining the relationship to the tangent of the
//! curve". Axiolid's `StationOrientation` is exactly that: components in
//! the base frame `(tangent, lateral, up)`, the axis exact, the reference
//! direction made perpendicular to it. So the ratios pass through
//! unchanged, and WR2 (not parallel) is checked here first so the refusal
//! names the entity.
//!
//! # A section's profile axes ([`section_orientation`], #344)
//!
//! Axiolid places a section's profile `x` along the oriented lateral (left),
//! `y` along the oriented up (`Axis`) and the normal along the oriented
//! tangent (`RefDirection`): profile X = `Axis x RefDirection`, in the
//! section plane. That is the reading adopted for both sectioned entities;
//! the printed IFC4.3 ADD2 text says otherwise in two sentences:
//!
//! - `IfcSectionedSolidHorizontal` (8.8.3.35.1): "The profile X axis is the
//!   direction of RefDirection ..., and the profile Y axis is the direction
//!   of Axis". With `RefDirection` defaulting to the tangent (8.9.3.4) that
//!   lays every default profile along the directrix.
//!   buildingSMART/IFC4.x-IF#147 (with a proposed text fix) and
//!   buildingSMART/IFC4.x-development#1010 document it as an error, and
//!   buildingSMART/IFC4.x-development PR #1163 rewrites it to profile Y =
//!   `Axis`, X = `Axis x RefDirection`: `RefDirection` is the section
//!   normal. Bonsai/IfcOpenShell and usBIM build the solid that way
//!   (IfcOpenShell/IfcOpenShell#6386).
//! - `IfcSectionedSurface` (8.8.3.37.1): "the X is derived from the cross
//!   product of Directrix and the Axis", which points RIGHT. Its own
//!   profile, `IfcOpenCrossProfileDef` (8.15.3.15.1), puts X "to the left
//!   of the Directrix (same direction as positive LateralOffset)", and a
//!   review on buildingSMART/IFC4.x-development PR #1162 asks to swap the
//!   operands to `Axis x Directrix`, the left
//!   (buildingSMART/IFC4.x-development#1151). The same sentence says "the
//!   profile normal is derived from the associated IfcAxis2PlacementLinear",
//!   so `RefDirection` is the normal here too.
//!
//! So the axes pass through unchanged for both, explicit `RefDirection`
//! included. openbimrs/ifc#344 tracks the upstream fix; when it lands
//! differently, [`section_orientation`] is the one place to change.

use axiolid_core::Vec3;
use axiolid_model::StationOrientation;
use ifc_model::EntityId;

use super::PLACEMENT;
use crate::error::GeometryResult;
use crate::lower::session::LoweringSession;

/// `Location` (inherited from `IfcPlacement`), `Axis`, `RefDirection`.
mod slot {
    pub const LOCATION: usize = 0;
    pub const AXIS: usize = 1;
    pub const REF_DIRECTION: usize = 2;
}

/// Which sectioned entity a section belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SectionOf {
    /// `IfcSectionedSolidHorizontal`.
    SolidHorizontal,
    /// `IfcSectionedSurface`.
    Surface,
}

/// An `IfcAxis2PlacementLinear`'s references.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct LinearPlacement {
    /// The placement entity.
    pub entity: EntityId,
    /// `Location`, an `IfcPointByDistanceExpression` by WR1.
    pub location: EntityId,
    /// `Axis`, when stated.
    pub axis: Option<EntityId>,
    /// `RefDirection`, when stated.
    pub ref_direction: Option<EntityId>,
}

/// Read `id` as an `IfcAxis2PlacementLinear`.
pub(crate) fn read_placement(
    session: &LoweringSession<'_>,
    id: EntityId,
) -> GeometryResult<LinearPlacement> {
    let kind = session.type_name(id)?;
    if kind != PLACEMENT {
        return Err(session.degenerate(
            id,
            &kind,
            "a cross section position must be an IfcAxis2PlacementLinear",
        ));
    }
    let slots = session.slots(id)?;
    Ok(LinearPlacement {
        entity: id,
        location: slots.req_ref(slot::LOCATION, "Location")?,
        axis: slots.opt_ref(slot::AXIS),
        ref_direction: slots.opt_ref(slot::REF_DIRECTION),
    })
}

/// A placement's `Axis` and `RefDirection` as a neutral orientation,
/// components passed through unchanged (see the module documentation).
pub(crate) fn placement_orientation(
    session: &LoweringSession<'_>,
    owner: EntityId,
    placement: &LinearPlacement,
) -> GeometryResult<StationOrientation> {
    let axis = placement
        .axis
        .map(|id| direction(session, owner, id, "Axis"))
        .transpose()?;
    let ref_direction = placement
        .ref_direction
        .map(|id| direction(session, owner, id, "RefDirection"))
        .transpose()?;
    let orientation = StationOrientation::new(axis, ref_direction);
    orientation.unit_axes().map_err(|why| {
        session.degenerate(
            owner,
            PLACEMENT,
            format!("WR2: Axis and RefDirection must not be parallel or anti-parallel ({why})"),
        )
    })?;
    Ok(orientation)
}

/// How a cross section stands at `placement`: the one place the profile
/// axis mapping lives (see the module documentation, #344). Both entities
/// read `Axis` as profile Y and `RefDirection` as the normal, Axiolid's own
/// reading, so the orientation passes through.
pub(crate) fn section_orientation(
    session: &LoweringSession<'_>,
    owner: EntityId,
    placement: &LinearPlacement,
) -> GeometryResult<StationOrientation> {
    placement_orientation(session, owner, placement)
}

/// A 3D `IfcDirection`'s ratios, unnormalised: the orientation normalises.
fn direction(
    session: &LoweringSession<'_>,
    owner: EntityId,
    id: EntityId,
    name: &str,
) -> GeometryResult<Vec3> {
    let ratios = session.slots(id)?.req_f64_list(0, "DirectionRatios")?;
    match ratios.as_slice() {
        [x, y, z] => Ok(Vec3::new(*x, *y, *z)),
        _ => Err(session.degenerate(
            owner,
            PLACEMENT,
            format!(
                "{name} has {} direction ratios; a linear placement's axes are components in the \
                 3D station frame (tangent, left, up)",
                ratios.len()
            ),
        )),
    }
}
