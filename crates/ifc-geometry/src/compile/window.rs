//! A kernel's refusal of a station's seam-snapping window (#423), named.
//!
//! A station on a basis whose seams lie at arc-length integrals is lowered
//! with Axiolid's seam-snapping window (`lower::station`, axiolid/kernel#294)
//! and the kernel snaps it when it resolves the station. Three refusals are
//! the window's own, each a typed `GeomError::UnsupportedInput` naming its
//! input: two seams within the window, a seam whose certified position
//! straddles its edge, and a seam whose distance cannot be certified. The
//! inputs are `axiolid-evaluate`'s constants, an execution provider this
//! crate does not link (ADR 0004), so they are repeated here and a test
//! pins them equal.

use axiolid_contracts::GeomError;

/// `axiolid_evaluate::station::SEAM_WINDOW_AMBIGUOUS`.
pub(crate) const AMBIGUOUS: &str =
    "a station whose seam-snapping window holds two seams of its basis: its seam side does not \
     say which one it lies on";

/// `axiolid_evaluate::station::SEAM_WINDOW_STRADDLED`.
pub(crate) const STRADDLED: &str =
    "a station whose seam-snapping window has a seam on its edge: the seam's certified position \
     straddles it, so whether the station lies within the window cannot be told";

/// `axiolid_evaluate::station::UNCERTIFIED_LENGTH`.
pub(crate) const UNCERTIFIED: &str =
    "a certified length of a station piece no bound here reaches: an offset of a banked curve, a \
     3D intrinsic curve, a chain's parametric piece or an elevated curve with an intrinsic \
     profile in a leaning frame; an offset by distances along an ellipse or a B-spline; a 3D \
     offset direction that is not vertical beside a plan-measured or intrinsic base; a base \
     placed in a frame that tilts +Z; a skewed frame; or a tolerance below the points' rounding";

/// Why the kernel refused a station's seam-snapping window (#423).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum SeamWindowRefusal {
    /// The window holds two seams of the basis, so the station's seam side
    /// does not say which one it lies on. The model's precision is coarser
    /// than the distance between two seams.
    Ambiguous,
    /// A seam's certified position straddles the window's edge, so whether
    /// the station lies on it within precision cannot be told.
    Straddled,
    /// A seam's distance cannot be certified (an offset of a banked or a 3D
    /// intrinsic curve, an offset by distances along an ellipse or a
    /// B-spline, and the other cases the kernel names).
    UncertifiedLength,
}

impl SeamWindowRefusal {
    /// The refusal `error` is, if it is one of a window's.
    pub(crate) fn of(error: &GeomError) -> Option<Self> {
        let GeomError::UnsupportedInput { input, .. } = error else {
            return None;
        };
        if *input == AMBIGUOUS {
            Some(Self::Ambiguous)
        } else if *input == STRADDLED {
            Some(Self::Straddled)
        } else if *input == UNCERTIFIED {
            Some(Self::UncertifiedLength)
        } else {
            None
        }
    }
}

impl std::fmt::Display for SeamWindowRefusal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Ambiguous => {
                "the window holds two seams of the basis curve, so the station's seam side does \
                 not say which one it lies on"
            }
            Self::Straddled => {
                "a seam's certified position straddles the window's edge, so whether the \
                 station lies on it within precision cannot be told"
            }
            Self::UncertifiedLength => {
                "the distance of a seam of the basis curve cannot be certified to the window"
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The inputs are the kernel's own (repeated because it is not linked).
    #[test]
    fn the_inputs_are_the_kernels() {
        use axiolid_evaluate::station::{
            SEAM_WINDOW_AMBIGUOUS, SEAM_WINDOW_STRADDLED, UNCERTIFIED_LENGTH,
        };
        assert_eq!(AMBIGUOUS, SEAM_WINDOW_AMBIGUOUS);
        assert_eq!(STRADDLED, SEAM_WINDOW_STRADDLED);
        assert_eq!(UNCERTIFIED, UNCERTIFIED_LENGTH);
    }
}
