//! Ordering and seam checks for `IfcAlignmentVertical`.
//!
//! The vertical counterpart of `CantLayout::resolve`: given the parent
//! `IfcAlignmentVertical`, resolve its nested segments in authored order
//! and check that they form one run. Every segment family is read,
//! including the ones no exact elevation law exists for yet; whether a
//! profile can be lowered is `profile_law`'s question, not this one's.

use ifc_model::{EntityId, Model};

use crate::curve::SeamTolerance;
use crate::error::{AlignmentError, AlignmentResult, ProfileSeam};
use crate::horizontal::AlignmentUnits;
use crate::vertical::segment::{read_vertical_segment, VerticalSegment};
use crate::view::AlignmentView;

/// One resolved, ordered vertical profile.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct VerticalLayout {
    /// The `IfcAlignmentVertical` entity this layout was resolved from.
    pub entity: EntityId,
    segments: Vec<VerticalSegment>,
}

impl VerticalLayout {
    /// Resolve `IfcAlignmentVertical#entity`'s nested `IfcAlignmentSegment`
    /// chain into an ordered profile.
    ///
    /// Checks what the segments state directly: they are contiguous and
    /// ascending in `StartDistAlong`, and each `StartGradient` equals the
    /// previous `EndGradient`, at the [`SeamTolerance`] the model declares. The height seam needs each segment's
    /// elevation law and is checked by `profile_law` when the profile is
    /// lowered.
    ///
    /// # Errors
    ///
    /// Refuses a model that is not IFC4X3, an entity that is not an
    /// `IfcAlignmentVertical`, an invalid declared `Precision`, a layout
    /// nesting no segment, a segment that
    /// does not read, a gap or overlap
    /// ([`AlignmentError::InvalidSegment`]), and a kink in grade
    /// ([`AlignmentError::ProfileDiscontinuity`]).
    pub fn resolve(
        model: &Model,
        entity: EntityId,
        units: AlignmentUnits,
    ) -> AlignmentResult<Self> {
        let view = AlignmentView::for_model(model)?;
        view.require(entity, "IfcAlignmentVertical")?;
        let ids = view.segment_chain(entity, "IfcAlignmentVerticalSegment")?;
        if ids.is_empty() {
            return Err(AlignmentError::SemanticViolation {
                entity: Some(entity),
                rule: "IfcAlignmentVertical must nest at least one IfcAlignmentSegment",
            });
        }
        // The same rule `vertical_profile_law` applies (#141): length seams
        // within the model's declared precision, gradients at rounding.
        let tolerance = SeamTolerance::for_model(model, units)?;
        let mut segments = Vec::with_capacity(ids.len());
        for id in ids {
            segments.push(read_vertical_segment(model, id, units)?);
        }
        for pair in segments.windows(2) {
            let [previous, next] = pair else {
                unreachable!("windows(2) yields pairs")
            };
            let previous_end = previous.start_dist_along + previous.horizontal_length;
            if !tolerance.same_length(next.start_dist_along, previous_end) {
                return Err(AlignmentError::InvalidSegment {
                    entity: next.entity,
                    detail: "vertical segments must be contiguous and ascending in StartDistAlong",
                });
            }
            if !tolerance.same_gradient(next.start_gradient, previous.end_gradient) {
                return Err(AlignmentError::ProfileDiscontinuity {
                    entity: next.entity,
                    previous: previous.entity,
                    seam: ProfileSeam::Gradient,
                    expected: previous.end_gradient,
                    actual: next.start_gradient,
                });
            }
        }
        Ok(Self { entity, segments })
    }

    /// Segments in authored (distance-along) order.
    #[must_use]
    pub fn segments(&self) -> &[VerticalSegment] {
        &self.segments
    }

    /// Distance along where the profile starts.
    #[must_use]
    pub fn start_dist_along(&self) -> f64 {
        self.segments[0].start_dist_along
    }

    /// Total distance-along span covered by this profile.
    #[must_use]
    pub fn length(&self) -> f64 {
        let last = &self.segments[self.segments.len() - 1];
        last.start_dist_along + last.horizontal_length - self.start_dist_along()
    }
}
