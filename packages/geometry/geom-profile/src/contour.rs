//! Arbitrary closed contours.

use geom_curve::Curve2;

/// One oriented closed contour assembled from exact curve segments.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Contour {
    /// Consecutive segments. Validation checks endpoint continuity and closure.
    pub segments: Vec<Curve2>,
}

/// Arbitrary profile with one outer contour and zero or more holes.
#[derive(Debug, Clone, PartialEq)]
pub struct ContourProfile {
    /// Outer boundary.
    pub outer: Contour,
    /// Interior boundaries.
    pub holes: Vec<Contour>,
}
