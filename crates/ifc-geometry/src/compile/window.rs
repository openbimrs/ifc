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
//!
//! A run's stations carry the same window (#429, axiolid/kernel#301), and
//! a run adds one refusal of its own: its stations, snapped, no longer
//! increase strictly in distance, a `GeomError::InvalidInput` whose text
//! `axiolid-mesh-compile` keeps private; it is repeated here, and a test
//! that compiles such a run pins it. An offset run continued to its
//! basis's end, which the kernel measures (#433, axiolid/kernel#303), adds
//! another: a stated station at or past that end, another private
//! `GeomError::InvalidInput` text repeated and pinned the same way.
//! Certifying a seam to the window's tolerance may also exceed the kernel's
//! panel budget, a `GeomError::BudgetExceeded` whose private resource name
//! is repeated here and pinned by a test that triggers it (#443).

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
     3D intrinsic curve with torsion or out of a level plane, a chain's parametric piece or an elevated curve with an intrinsic \
     profile in a leaning frame; a 3D offset direction that is not vertical beside a \
     plan-measured or intrinsic base; a base placed in a frame that tilts +Z; a skewed frame; or \
     a tolerance below the points' rounding";

/// The resource `axiolid-evaluate` names when certifying a piece's length
/// to a window's tolerance needs more panels than its budget allows
/// (`station::certify`, private there): a `GeomError::BudgetExceeded`.
pub(crate) const CERTIFICATION_BUDGET: &str = "certified length panels";

/// `axiolid-mesh-compile`'s refusal of an offset run whose stations, its
/// basis's ends resolved, do not increase (`station::RESOLVED_NOT_INCREASING`,
/// private there).
pub(crate) const PAST_BASIS_END: &str =
    "station: the run's stations, its stations at the basis's ends measured by the kernel, do \
     not increase strictly in distance: a stated station at or past the basis's end";

/// `axiolid-mesh-compile`'s refusal of a run whose snapped stations no
/// longer increase (`station::SNAPPED_NOT_INCREASING`, private there).
pub(crate) const NOT_INCREASING: &str =
    "station: the run's stations, snapped onto seams within their windows, no longer increase \
     strictly in distance: two snapped onto one seam, or one snapped past its neighbour";

/// Why the kernel refused a station's seam-snapping window (#423), or a
/// run's (#429).
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
    /// intrinsic curve, a tolerance below the points' rounding, and the
    /// other cases the kernel names). An offset by distances along an
    /// ellipse or a B-spline is certified since `axiolid-evaluate` 0.3.15
    /// (axiolid/kernel#307, #443).
    UncertifiedLength,
    /// A run's stations, snapped onto seams within their windows, no
    /// longer increase strictly in distance: two snapped onto one seam, or
    /// one past its neighbour (#429). The model's precision is coarser
    /// than the distance between them.
    NotIncreasing,
    /// An offset run continued to its basis's end, whose distance the
    /// kernel measures (#433), states a station at or past that end, so its
    /// stations do not increase strictly once the end is resolved. IFC4.3
    /// ADD2 (8.9.3.42.3) continues the offsets only where they stop short
    /// of the end; one stated within the model's precision of it is
    /// refused too, since this crate cannot tell it from the end.
    PastBasisEnd,
    /// Certifying a seam's distance to the window's tolerance (a thousandth
    /// of it) needs more panels than the kernel's budget allows: a piece
    /// whose length is a quadrature, before the window or reaching it,
    /// whose derivatives the kernel bounds only coarsely for so fine a
    /// tolerance (#443). Since `axiolid-evaluate` 0.3.15 refines its panels
    /// adaptively (axiolid/kernel#307), an ordinary curve certifies to
    /// `1e-8` relative and below within the budget.
    CertificationBudgetExceeded,
}

impl SeamWindowRefusal {
    /// The refusal `error` is, if it is one of a window's.
    pub(crate) fn of(error: &GeomError) -> Option<Self> {
        if let GeomError::BudgetExceeded { resource } = error {
            return (*resource == CERTIFICATION_BUDGET)
                .then_some(Self::CertificationBudgetExceeded);
        }
        if let GeomError::InvalidInput(detail) = error {
            return if detail == NOT_INCREASING {
                Some(Self::NotIncreasing)
            } else if detail == PAST_BASIS_END {
                Some(Self::PastBasisEnd)
            } else {
                None
            };
        }
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
            Self::NotIncreasing => {
                "the run's stations, snapped onto seams within the window, no longer increase \
                 strictly in distance"
            }
            Self::CertificationBudgetExceeded => {
                "the distance of a seam of the basis curve cannot be certified to the window \
                 within the kernel's panel budget"
            }
            Self::PastBasisEnd => {
                "the offset run states a station at or past its basis's end, which the kernel \
                 measures, so its stations do not increase strictly in distance"
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
        assert_eq!(
            crate::constraint::placement::derive::WINDOW_CERTIFICATION,
            axiolid_evaluate::station::window::WINDOW_CERTIFICATION
        );
    }
}
