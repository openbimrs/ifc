//! Seams between consecutive vertical segments, and grade breaks.
//!
//! IFC4.3 ADD2 `IfcAlignmentVerticalSegment`: "The transition at the
//! segment connection is not enforced to be tangential", and "Connectivity
//! between vertical segments is not necessarily tangential". The schema has
//! no attribute that would require it. Real exports carry such grade breaks
//! as authored design (buildingSMART IFC4.x-IF `BC003_ALX2`: 47 constant
//! grades meeting at 29 breaks with continuous height).
//!
//! So a seam whose height is continuous but whose grade changes is not an
//! error. It is a fact about the profile, reported here as
//! [`VerticalSeamKind::GradeBreak`]. A caller that needs a tangent profile
//! asks for it explicitly with
//! [`VerticalLayout::require_tangential`](super::VerticalLayout::require_tangential).

use ifc_model::EntityId;

/// How the grade continues across one vertical seam.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VerticalSeamKind {
    /// `StartGradient` equals the previous `EndGradient` up to rounding.
    Tangential,
    /// The grade changes at the seam: the profile has a kink there. Legal
    /// IFC4.3, and carried exactly by the composed elevation law, whose
    /// pieces each keep their own grade.
    GradeBreak,
}

/// One seam between two consecutive vertical segments of positive length.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct VerticalSeam {
    /// The segment that ends at this seam.
    pub previous: EntityId,
    /// The segment that starts at this seam.
    pub next: EntityId,
    /// `StartDistAlong` of `next`, in metres.
    pub distance_along: f64,
    /// The previous segment's `EndGradient`.
    pub incoming_gradient: f64,
    /// The next segment's `StartGradient`.
    pub outgoing_gradient: f64,
    /// Whether the grade continues or breaks.
    pub kind: VerticalSeamKind,
}
