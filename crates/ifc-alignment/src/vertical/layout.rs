//! Ordering and seam checks for `IfcAlignmentVertical`.
//!
//! The vertical counterpart of `CantLayout::resolve`: given the parent
//! `IfcAlignmentVertical`, resolve its nested segments in authored order
//! and check that they form one run. Every segment family is read,
//! including the ones no exact elevation law exists for yet; whether a
//! profile can be lowered is `profile_law`'s question, not this one's.
//!
//! A grade break at a seam is legal IFC4.3 and is reported in
//! [`VerticalLayout::seams`], not refused (see `seam.rs`). The zero-length
//! segment IFC4.3 requires at the end of a layout is kept in
//! [`VerticalLayout::segments`] and checked for contiguity; it closes the
//! profile and has no seam of its own in [`VerticalLayout::seams`].

use ifc_model::{EntityId, Model};

use crate::curve::terminal::split_closing;
use crate::curve::SeamTolerance;
use crate::error::{AlignmentError, AlignmentResult, ProfileSeam};
use crate::horizontal::AlignmentUnits;
use crate::vertical::seam::{VerticalSeam, VerticalSeamKind};
use crate::vertical::segment::{read_vertical_segment, VerticalSegment};
use crate::view::AlignmentView;

/// One resolved, ordered vertical profile.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct VerticalLayout {
    /// The `IfcAlignmentVertical` entity this layout was resolved from.
    pub entity: EntityId,
    segments: Vec<VerticalSegment>,
    seams: Vec<VerticalSeam>,
}

impl VerticalLayout {
    /// Resolve `IfcAlignmentVertical#entity`'s nested `IfcAlignmentSegment`
    /// chain into an ordered profile.
    ///
    /// Checks what the segments state directly: they are contiguous and
    /// ascending in `StartDistAlong`, at the [`SeamTolerance`] the model
    /// declares, and a zero-length segment stands only in last place. The
    /// height seam needs each segment's elevation law and is checked by
    /// `profile_law` when the profile is lowered. A grade break is recorded
    /// in [`Self::seams`], not refused: IFC4.3 does not require vertical
    /// seams to be tangential.
    ///
    /// # Errors
    ///
    /// Refuses a model that is not IFC4X3, an entity that is not an
    /// `IfcAlignmentVertical`, an invalid declared `Precision`, a layout
    /// nesting no segment, a segment that does not read, a gap or overlap
    /// ([`AlignmentError::InvalidSegment`]), and a zero-length segment that
    /// is not last or is the only one ([`AlignmentError::SemanticViolation`]).
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
        let (body, _closing) = split_closing(&segments, |s| s.horizontal_length, |s| s.entity)?;
        let mut seams = Vec::with_capacity(body.len().saturating_sub(1));
        for (index, pair) in segments.windows(2).enumerate() {
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
            // The closing segment ends the profile: there is no grade after
            // it to break, so it has no seam of its own.
            if index + 1 < body.len() {
                let kind = if tolerance.same_gradient(next.start_gradient, previous.end_gradient) {
                    VerticalSeamKind::Tangential
                } else {
                    VerticalSeamKind::GradeBreak
                };
                seams.push(VerticalSeam {
                    previous: previous.entity,
                    next: next.entity,
                    distance_along: next.start_dist_along,
                    incoming_gradient: previous.end_gradient,
                    outgoing_gradient: next.start_gradient,
                    kind,
                });
            }
        }
        Ok(Self {
            entity,
            segments,
            seams,
        })
    }

    /// Every seam between consecutive segments of positive length, in
    /// order, with how the grade continues across it.
    #[must_use]
    pub fn seams(&self) -> &[VerticalSeam] {
        &self.seams
    }

    /// Demand a tangent profile: refuse the first grade break.
    ///
    /// IFC4.3 does not require vertical seams to be tangential, so
    /// [`Self::resolve`] accepts grade breaks. A consumer whose model needs
    /// a continuous grade (a design check, say) asks for it here.
    ///
    /// # Errors
    ///
    /// [`AlignmentError::ProfileDiscontinuity`] with
    /// [`ProfileSeam::Gradient`] at the first grade break.
    pub fn require_tangential(&self) -> AlignmentResult<()> {
        match self
            .seams
            .iter()
            .find(|seam| seam.kind == VerticalSeamKind::GradeBreak)
        {
            None => Ok(()),
            Some(seam) => Err(AlignmentError::ProfileDiscontinuity {
                entity: seam.next,
                previous: seam.previous,
                seam: ProfileSeam::Gradient,
                expected: seam.incoming_gradient,
                actual: seam.outgoing_gradient,
            }),
        }
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
