//! IFC4X3 sectioned sweeps along a directrix onto Axiolid's station-placed
//! sections (#307).
//!
//! `IfcSectionedSolidHorizontal` (IFC4.3 ADD2 8.8.3.35) and
//! `IfcSectionedSurface` (8.8.3.37) stand a list of cross sections at
//! `IfcAxis2PlacementLinear` positions along a `Directrix` and interpolate
//! linearly between them. They lower to `SolidOperation::SectionsAtStations`
//! and `SurfaceRelation::OpenSectionsAtStations` (`axiolid-model` 0.3.5,
//! ADR 0082): each section's profile node, its station (the position's
//! `IfcPointByDistanceExpression`), its orientation and, for the surface,
//! its tags. A kernel resolves the stations; nothing is evaluated here.
//!
//! # Mapping
//!
//! - **Stations.** Each position's `DistanceAlong` and offsets, read as for
//!   any station (`lower::station`, with its convention table). The
//!   position must measure along the `Directrix` itself; a station on
//!   another curve would not be the section's place along this one.
//! - **Frame.** `StationFrame::Section`: the default `IfcAxis2PlacementLinear`
//!   frame is the curve's own (tangent, left, up), which is where
//!   `OffsetVertical` is read (8.9.3.48.3). "Sweeping ... horizontally"
//!   states no other frame.
//! - **Profile axes.** `lower::station::axes`, the one place they live
//!   (#344): profile Y is `Axis`, the normal is `RefDirection` and profile
//!   X is `Axis x RefDirection`, the left lateral by default
//!   (8.15.3.15.1; buildingSMART/IFC4.x-IF#147, IFC4.x-development#1010,
//!   #1151, PRs #1162 and #1163). The printed sentences of 8.8.3.35.1 and
//!   8.8.3.37.1 say otherwise; `axes` cites both.
//! - **Between sections.** "linear interpolation between profile points
//!   with the same tag" (8.8.3.35.1), "linear interpolation is assumed"
//!   (8.8.3.37.1): Axiolid interpolates linearly in distance. A surface's
//!   `IfcOpenCrossProfileDef.Tags` become the section tags, matched as sets,
//!   an open section forwards or reversed. A solid's sections are untagged
//!   and matched by ring and vertex index: IFC tags a closed profile only
//!   through an `IfcIndexedPolyCurve`'s `TagList`, a boundary the profile
//!   lowering does not take, so no tagged closed profile reaches here.
//!
//! - **Across a tangent discontinuity** (#346). "If the directrix is not
//!   tangent continuous, the resulting solid is created by a miter at half
//!   angle between the two segments" (8.8.3.35.1; 8.8.3.37.1 says the same
//!   of the surface). Axiolid cuts a run of sections across a seam in the
//!   plane normal to the bisector of the incoming and outgoing tangents,
//!   which is that half-angle mitre (ADR 0082 amendment). A position within
//!   the model's precision of a seam is stored at the seam, where the
//!   kernel stands its section in the mitre plane (`lower::station`).
//!
//! # Refused by name
//!
//! Where IFC states a shape the neutral relation does not carry: the WHERE
//! rules (`NoLongitudinalOffsets`, `NoOffsets`, `CorrespondingSectionPositions`,
//! `SectionsSameType`), positions out of order or off the directrix, a
//! directrix that turns back on itself within the run ("very sharp edges
//! may result in nearly impossible miter", 8.8.3.35.1), a surface mixing
//! tagged and untagged sections or branching breaklines (sections with
//! different tags). `IfcSectionedSolid`, the parent, is ABSTRACT.

use axiolid_model::{
    GeometryNode, NodeId, SectionAtStation, SolidOperation, StationFrame, SurfaceRelation,
};
use ifc_model::EntityId;

use crate::error::GeometryResult;
use crate::input::profile::{describe_profile, ProfileParameters};
use crate::lower::profile::{lower_open_profile_node, lower_profile_node};
use crate::lower::session::LoweringSession;
use crate::lower::station::axes::{read_placement, section_orientation, SectionOf};
use crate::lower::station::read::distance_expression;
use crate::lower::station::Basis;
use crate::transform::Transform;

/// IFC type of the horizontal sectioned solid.
pub(crate) const SECTIONED_SOLID_HORIZONTAL_TYPE: &str = "IFCSECTIONEDSOLIDHORIZONTAL";
/// IFC type of the sectioned surface.
pub(crate) const SECTIONED_SURFACE_TYPE: &str = "IFCSECTIONEDSURFACE";

/// Family label used for memoization.
const KIND: &str = "sectioned";

/// Why a position on another curve is refused.
const OTHER_BASIS: &str =
    "a CrossSectionPositions member is measured along another BasisCurve than the Directrix";

/// Why mixed tagging is refused.
const MIXED_TAGS: &str =
    "some sections state Tags and others do not, so IFC4.3 ADD2 names no pairing between them";

/// Why branching breaklines are refused.
const BRANCHING: &str =
    "the sections carry different Tags (branching longitudinal breaklines, 8.8.3.37.1); the \
     neutral sectioned surface joins one set of tags through every section";

/// Why repeated or reordered tags are refused.
const TAG_ORDER: &str =
    "a section repeats a tag, or orders its tags other than the first section's or its \
     reverse, so joining by tag would cross the sheet";

/// Lower an `IfcSectionedSolidHorizontal` to `SectionsAtStations`.
pub(crate) fn lower_sectioned_solid_horizontal_node(
    session: &mut LoweringSession<'_>,
    id: EntityId,
    frame: Transform,
) -> GeometryResult<NodeId> {
    memoized(session, id, frame, |session| {
        sectioned(session, id, frame, SectionOf::SolidHorizontal)
    })
}

/// Lower an `IfcSectionedSurface` to `OpenSectionsAtStations`.
pub(crate) fn lower_sectioned_surface_node(
    session: &mut LoweringSession<'_>,
    id: EntityId,
    frame: Transform,
) -> GeometryResult<NodeId> {
    memoized(session, id, frame, |session| {
        sectioned(session, id, frame, SectionOf::Surface)
    })
}

/// Lower an `IfcSectionedSurface` with no memoization or cycle entry of its
/// own, for `lower::surface`, which holds both for every surface.
pub(crate) fn sectioned_surface(
    session: &mut LoweringSession<'_>,
    id: EntityId,
    frame: Transform,
) -> GeometryResult<NodeId> {
    sectioned(session, id, frame, SectionOf::Surface)
}

fn sectioned(
    session: &mut LoweringSession<'_>,
    id: EntityId,
    frame: Transform,
    of: SectionOf,
) -> GeometryResult<NodeId> {
    let (owner_type, positions_slot, sections_slot) = match of {
        SectionOf::SolidHorizontal => (SECTIONED_SOLID_HORIZONTAL_TYPE, 2, 1),
        SectionOf::Surface => (SECTIONED_SURFACE_TYPE, 1, 2),
    };
    let slots = session.slots(id)?;
    let directrix = slots.req_ref(0, "Directrix")?;
    let positions = slots.req_ref_list(positions_slot, "CrossSectionPositions")?;
    let profiles = slots.req_ref_list(sections_slot, "CrossSections")?;
    if positions.len() != profiles.len() {
        return Err(session.degenerate(
            id,
            owner_type,
            format!(
                "CorrespondingSectionPositions: {} sections and {} positions",
                profiles.len(),
                positions.len()
            ),
        ));
    }
    if profiles.len() < 2 {
        return Err(session.degenerate(id, owner_type, "CrossSections is LIST [2:?]"));
    }
    let first_type = session.type_name(profiles[0])?;
    for profile in &profiles[1..] {
        if session.type_name(*profile)? != first_type {
            return Err(session.degenerate(
                id,
                owner_type,
                "SectionsSameType: every cross section must be of one entity type",
            ));
        }
    }

    let basis = Basis::lower(session, id, owner_type, directrix, frame)?;
    let mut sections: Vec<SectionAtStation> = Vec::with_capacity(profiles.len());
    for (position, profile) in positions.iter().zip(&profiles) {
        let placement = read_placement(session, *position)?;
        let expression = distance_expression(session, id, owner_type, placement.location)?;
        if expression.basis != directrix {
            return Err(session.unsupported(id, owner_type, OTHER_BASIS));
        }
        match of {
            SectionOf::SolidHorizontal if expression.longitudinal.is_some() => {
                return Err(session.degenerate(
                    id,
                    owner_type,
                    "NoLongitudinalOffsets: a cross section position states OffsetLongitudinal",
                ))
            }
            SectionOf::Surface if expression.states_an_offset() => {
                return Err(session.degenerate(
                    id,
                    owner_type,
                    "NoOffsets: a cross section position states an offset",
                ))
            }
            _ => {}
        }
        basis.check_on_curve(session, id, owner_type, expression.distance)?;
        let distance = basis.run_distance(session, id, owner_type, expression.distance)?;
        if let Some(previous) = sections.last() {
            if distance <= previous.station.distance {
                return Err(session.degenerate(
                    id,
                    owner_type,
                    "CrossSectionPositions must be in strictly increasing order along the \
                     Directrix, by more than the model's precision at a tangent discontinuity",
                ));
            }
        }
        let orientation = section_orientation(session, id, &placement)?;
        let (node, tags) = match of {
            SectionOf::SolidHorizontal => (lower_profile_node(session, *profile)?, Vec::new()),
            SectionOf::Surface => (
                lower_open_profile_node(session, *profile)?,
                open_tags(session, *profile)?,
            ),
        };
        let mut station = expression.station();
        station.distance = distance;
        sections.push(
            SectionAtStation::new(node, station)
                .with_tags(tags)
                .with_orientation(orientation),
        );
    }
    check_tags(session, id, owner_type, &sections)?;
    let from = sections[0].station.distance;
    let to = sections[sections.len() - 1].station.distance;
    basis.check_run(session, id, owner_type, from, to)?;

    let node = match of {
        SectionOf::SolidHorizontal => {
            GeometryNode::SolidOperation(SolidOperation::SectionsAtStations {
                directrix: basis.node,
                sections,
                frame: StationFrame::Section,
            })
        }
        SectionOf::Surface => {
            GeometryNode::SurfaceRelation(SurfaceRelation::OpenSectionsAtStations {
                directrix: basis.node,
                sections,
                frame: StationFrame::Section,
            })
        }
    };
    session.node_for(id, node)
}

/// An `IfcOpenCrossProfileDef`'s `Tags`, or none.
fn open_tags(session: &LoweringSession<'_>, profile: EntityId) -> GeometryResult<Vec<String>> {
    let description = describe_profile(session.model(), session.units(), profile)?;
    Ok(match description.parameters {
        ProfileParameters::OpenCross {
            tags: Some(tags), ..
        } => tags,
        _ => Vec::new(),
    })
}

/// Every section tagged or none, one tag set, each in the first section's
/// order or its reverse, none repeated.
fn check_tags(
    session: &LoweringSession<'_>,
    id: EntityId,
    owner_type: &str,
    sections: &[SectionAtStation],
) -> GeometryResult<()> {
    let first = &sections[0].tags;
    if sections
        .iter()
        .any(|section| section.tags.is_empty() != first.is_empty())
    {
        return Err(session.unsupported(id, owner_type, MIXED_TAGS));
    }
    let sorted = |tags: &[String]| {
        let mut set = tags.to_vec();
        set.sort_unstable();
        set
    };
    let expected = sorted(first);
    if expected.windows(2).any(|pair| pair[0] == pair[1]) {
        return Err(session.unsupported(id, owner_type, TAG_ORDER));
    }
    for section in &sections[1..] {
        if sorted(&section.tags) != expected {
            return Err(session.unsupported(id, owner_type, BRANCHING));
        }
        if section.tags != *first && !section.tags.iter().eq(first.iter().rev()) {
            return Err(session.unsupported(id, owner_type, TAG_ORDER));
        }
    }
    Ok(())
}

fn memoized(
    session: &mut LoweringSession<'_>,
    id: EntityId,
    frame: Transform,
    build: impl FnOnce(&mut LoweringSession<'_>) -> GeometryResult<NodeId>,
) -> GeometryResult<NodeId> {
    if let Some(node) = session.memoized(id, KIND, frame) {
        return Ok(node);
    }
    session.enter(id, KIND)?;
    let result = build(session);
    session.exit(id);
    let node = result?;
    session.memoize(id, KIND, frame, node);
    Ok(node)
}
