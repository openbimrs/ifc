//! The zero-length segment that closes every alignment layout.
//!
//! IFC4.3 concept template *Alignment Layout - Horizontal, Vertical and
//! Cant* (`IfcAlignment`, buildingSMART IFC4.3.x-development):
//!
//! > 1. A **zero-length segment** shall be added, at the end of the list of
//! >    segments for _IfcAlignmentSegment.DesignParameters_.
//! > 2. If the geometry definition is also present, then each of the
//! >    zero-length segments shall have a _IfcCurveSegment_ counterpart - of
//! >    length zero.
//!
//! The schema agrees: `SegmentLength` and `HorizontalLength` are
//! `IfcNonNegativeLengthMeasure`. The rule here:
//!
//! - a zero-length segment in LAST place is the layout's closing segment.
//!   It contributes no geometry and no curve piece. Its restated start
//!   (station, point, direction, height, cant) is still a seam and is
//!   checked against the end of the previous segment by the same rule as
//!   any other seam of its layout, wherever that rule is closed form;
//! - a zero-length segment anywhere else is refused
//!   ([`AlignmentError::SemanticViolation`]): it carries no geometry, and
//!   accepting it would only hide a seam;
//! - a layout whose only segment is the closing one is refused: it states
//!   no geometry at all.
//!
//! Zero means exactly zero after unit conversion, as the template states
//! ("of length zero"); a short positive segment is an ordinary segment.

use ifc_model::EntityId;

use crate::error::{AlignmentError, AlignmentResult};

/// The rule a zero-length segment before the end of its layout breaks.
pub(crate) const MISPLACED: &str =
    "a zero-length alignment segment is allowed only as the last segment of its layout";

/// The rule a layout of nothing but its closing segment breaks.
const ONLY_CLOSING: &str =
    "an alignment layout needs a segment of positive length before its closing zero-length segment";

/// A layout's segments split into the ones that carry geometry and the
/// optional closing zero-length segment.
///
/// `length` reads a segment's length along the layout (`SegmentLength` or
/// `HorizontalLength`), `entity` its id for the refusal. An empty slice is
/// returned unchanged, so the caller keeps its own emptiness refusal.
///
/// # Errors
///
/// [`AlignmentError::SemanticViolation`] naming a zero-length segment that
/// is not last, or a closing segment with nothing before it.
pub(crate) fn split_closing<T>(
    segments: &[T],
    length: impl Fn(&T) -> f64,
    entity: impl Fn(&T) -> EntityId,
) -> AlignmentResult<(&[T], Option<&T>)> {
    let Some((last, body)) = segments.split_last() else {
        return Ok((segments, None));
    };
    if let Some(misplaced) = body.iter().find(|segment| length(segment) == 0.0) {
        return Err(AlignmentError::SemanticViolation {
            entity: Some(entity(misplaced)),
            rule: MISPLACED,
        });
    }
    if length(last) != 0.0 {
        return Ok((segments, None));
    }
    if body.is_empty() {
        return Err(AlignmentError::SemanticViolation {
            entity: Some(entity(last)),
            rule: ONLY_CLOSING,
        });
    }
    Ok((body, Some(last)))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn split(lengths: &[f64]) -> AlignmentResult<(usize, Option<u64>)> {
        let segments: Vec<(u64, f64)> = lengths
            .iter()
            .enumerate()
            .map(|(i, l)| (i as u64 + 1, *l))
            .collect();
        split_closing(&segments, |s| s.1, |s| EntityId(s.0))
            .map(|(body, closing)| (body.len(), closing.map(|s| s.0)))
    }

    #[test]
    fn a_closing_zero_length_segment_is_split_off() {
        assert_eq!(split(&[10.0, 5.0, 0.0]), Ok((2, Some(3))));
        assert_eq!(split(&[10.0, 5.0]), Ok((2, None)));
        assert_eq!(split(&[]), Ok((0, None)));
    }

    #[test]
    fn a_zero_length_segment_before_the_end_is_refused() {
        assert_eq!(
            split(&[10.0, 0.0, 5.0, 0.0]),
            Err(AlignmentError::SemanticViolation {
                entity: Some(EntityId(2)),
                rule: MISPLACED,
            })
        );
    }

    #[test]
    fn a_layout_of_only_its_closing_segment_is_refused() {
        assert!(matches!(
            split(&[0.0]),
            Err(AlignmentError::SemanticViolation {
                entity: Some(EntityId(1)),
                ..
            })
        ));
    }
}
