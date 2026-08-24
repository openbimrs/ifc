//! Legacy chord-tolerance policy retained for source compatibility.
//!
//! Active lowering preserves exact profiles and construction intent in
//! `axiolid_model::GeometryGraph`; it does not polygonize curves here. Existing
//! callers still pass this value through the pre-DAG lowering signatures, but
//! supported exact profile paths deliberately ignore it. New tessellation
//! providers consume explicit execution-time tolerance instead.
//!
//! Sagitta (chord height) remains the correct policy for any compatibility path
//! that must approximate a circular arc: it scales with radius and sweep rather
//! than imposing a global segment count.

/// Chord tolerance controlling curve approximation.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Tolerance {
    /// Maximum chord height in metres. Always positive.
    sagitta: f64,
    /// Hard cap on segments per full circle, so a tiny tolerance on a large
    /// radius cannot produce a million-point contour.
    max_segments: u32,
}

impl Tolerance {
    /// A default suited to building-scale geometry: 1 mm chord height.
    ///
    /// Chosen because it is below the tolerance of construction itself, so
    /// the approximation is not the limiting factor in any downstream
    /// measurement.
    pub fn building_scale() -> Self {
        Self {
            sagitta: 1e-3,
            max_segments: 512,
        }
    }

    /// A custom tolerance in metres.
    ///
    /// Returns `None` for non-positive or non-finite input rather than
    /// silently clamping: a zero tolerance means an infinite segment count,
    /// which is a caller bug worth surfacing.
    pub fn from_sagitta(metres: f64) -> Option<Self> {
        if !metres.is_finite() || metres <= 0.0 {
            return None;
        }
        Some(Self {
            sagitta: metres,
            max_segments: 512,
        })
    }

    /// The chord height in metres.
    pub fn sagitta(&self) -> f64 {
        self.sagitta
    }
}

impl Tolerance {
    /// Segments needed to approximate an arc of `radius` sweeping `angle`.
    ///
    /// From the sagitta relation for a circular segment:
    ///
    /// ```text
    /// s = r * (1 - cos(theta / 2))
    /// ```
    ///
    /// where `theta` is the angle subtended by ONE segment. Solving for
    /// theta and dividing the total sweep by it gives the count. When the
    /// requested sagitta exceeds the radius the arc is coarser than a
    /// triangle, so the result is clamped to a floor of 3 segments for a full
    /// circle: fewer cannot enclose area.
    pub fn segments_for_arc(&self, radius: f64, angle: f64) -> u32 {
        let sweep = angle.abs();
        if !radius.is_finite() || radius <= 0.0 || !sweep.is_finite() || sweep <= 0.0 {
            return 1;
        }
        // A sagitta at or beyond the radius carries no information; fall back
        // to the coarsest shape that still encloses area.
        let ratio = 1.0 - (self.sagitta / radius);
        if ratio <= -1.0 {
            return 3;
        }
        let per_segment = 2.0 * ratio.clamp(-1.0, 1.0).acos();
        if per_segment <= f64::EPSILON {
            return self.max_segments;
        }
        let needed = (sweep / per_segment).ceil();
        let scaled_cap = f64::from(self.max_segments) * (sweep / std::f64::consts::TAU);
        let cap = scaled_cap.max(3.0);
        (needed.clamp(1.0, cap) as u32).max(minimum_for(sweep))
    }
}

/// The fewest segments that can represent a sweep without degenerating.
///
/// A full circle needs at least 3 to enclose area; a partial arc needs only 1
/// chord, which is exact when the sweep is small.
fn minimum_for(sweep: f64) -> u32 {
    if sweep >= std::f64::consts::TAU - 1e-9 {
        3
    } else {
        1
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The sagitta relation must hold: with N segments on a circle of radius
    /// r, the actual chord height must not exceed the requested tolerance.
    /// This is the property the whole policy exists to guarantee.
    #[test]
    fn the_resulting_chord_height_respects_the_requested_tolerance() {
        let tol = Tolerance::from_sagitta(1e-3).unwrap();
        for radius in [0.05, 0.5, 5.0, 50.0] {
            let n = tol.segments_for_arc(radius, std::f64::consts::TAU);
            let per = std::f64::consts::TAU / f64::from(n);
            let actual = radius * (1.0 - (per / 2.0).cos());
            assert!(
                actual <= 1e-3 + 1e-12,
                "radius {radius}: {n} segments gives sagitta {actual}"
            );
        }
    }

    /// A bigger circle needs more segments at the same tolerance. This is the
    /// property a fixed segment count would violate.
    #[test]
    fn larger_radii_need_more_segments() {
        let tol = Tolerance::building_scale();
        let small = tol.segments_for_arc(0.1, std::f64::consts::TAU);
        let large = tol.segments_for_arc(10.0, std::f64::consts::TAU);
        assert!(large > small, "small={small} large={large}");
    }

    /// A coarser tolerance must not produce more segments.
    #[test]
    fn coarser_tolerance_never_refines_further() {
        let fine = Tolerance::from_sagitta(1e-4).unwrap();
        let coarse = Tolerance::from_sagitta(1e-2).unwrap();
        let r = 2.0;
        assert!(
            coarse.segments_for_arc(r, std::f64::consts::TAU)
                <= fine.segments_for_arc(r, std::f64::consts::TAU)
        );
    }
}

#[cfg(test)]
mod more_tests {
    use super::*;

    /// A full circle can never be fewer than 3 segments; a degenerate
    /// tolerance must not produce a line or a point.
    #[test]
    fn a_full_circle_never_degenerates_below_a_triangle() {
        let absurd = Tolerance::from_sagitta(1e6).unwrap();
        assert_eq!(absurd.segments_for_arc(1.0, std::f64::consts::TAU), 3);
    }

    /// A quarter arc needs fewer segments than a full circle at equal
    /// tolerance, because the cap scales with the sweep rather than being a
    /// flat per-curve number.
    #[test]
    fn partial_arcs_cost_less_than_full_circles() {
        let tol = Tolerance::building_scale();
        let quarter = tol.segments_for_arc(1.0, std::f64::consts::FRAC_PI_2);
        let full = tol.segments_for_arc(1.0, std::f64::consts::TAU);
        assert!(quarter < full, "quarter={quarter} full={full}");
    }

    /// Nonsense input is rejected at construction rather than clamped.
    #[test]
    fn a_non_positive_tolerance_is_refused() {
        assert!(Tolerance::from_sagitta(0.0).is_none());
        assert!(Tolerance::from_sagitta(-1.0).is_none());
        assert!(Tolerance::from_sagitta(f64::NAN).is_none());
    }

    /// Degenerate geometry must not hang or produce a huge contour.
    #[test]
    fn degenerate_radii_and_sweeps_are_bounded() {
        let tol = Tolerance::building_scale();
        assert_eq!(tol.segments_for_arc(0.0, 1.0), 1);
        assert_eq!(tol.segments_for_arc(-1.0, 1.0), 1);
        assert_eq!(tol.segments_for_arc(1.0, 0.0), 1);
        assert!(tol.segments_for_arc(1e9, std::f64::consts::TAU) <= 512);
    }
}
