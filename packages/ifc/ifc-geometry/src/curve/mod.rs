//! Typed read-only views of the `IfcCurve` family.
//!
//! # What is here and what is deliberately not
//!
//! These are *views*: newtypes over `(EntityId, &Entity)` that name the
//! attributes and check the invariants a file can violate. Nothing here
//! evaluates a curve, tessellates one, or implements NURBS basis functions.
//! A geometry kernel does that; this module's job is to hand it parameters it
//! can trust, and to fail loudly where the file cannot be trusted.
//!
//! # The taxonomy
//!
//! `IfcCurve` is abstract with three branches:
//!
//! ```text
//!   IfcCurve
//!   |- IfcLine                     unbounded, point + vector
//!   |- IfcConic                    unbounded, placement + radii
//!   |  |- IfcCircle
//!   |  '- IfcEllipse
//!   |- IfcOffsetCurve2D / 3D       relative to another curve
//!   |- IfcPcurve                   in a surface's parameter space
//!   |- IfcSurfaceCurve             on one or two surfaces
//!   |  |- IfcIntersectionCurve
//!   |  '- IfcSeamCurve
//!   '- IfcBoundedCurve             has a start and an end
//!      |- IfcPolyline
//!      |- IfcIndexedPolyCurve
//!      |- IfcTrimmedCurve
//!      |- IfcCompositeCurve
//!      |  '- IfcCompositeCurveOnSurface
//!      |     '- IfcBoundaryCurve
//!      |        '- IfcOuterBoundaryCurve
//!      '- IfcBSplineCurve
//!         '- IfcBSplineCurveWithKnots
//!            '- IfcRationalBSplineCurveWithKnots
//! ```
//!
//! **Bounded versus unbounded is the distinction that bites.** An `IfcCircle`
//! is the whole circle; an `IfcLine` is infinite in both directions. Neither
//! can be drawn on its own. They appear in files almost exclusively as the
//! `BasisCurve` of an [`trimmed::TrimmedCurve`] or as the `ParentCurve` of a
//! [`composite::CompositeCurveSegment`], which is where the bounds come from.
//! Code that meets an `IfcCircle` at the top of a representation and draws a
//! full circle has usually skipped a trim.
//!
//! # `IfcCurveSegment`
//!
//! Not in IFC4 ADD2 TC1. It was introduced in IFC4x3 for alignment geometry
//! and has no view here; [`CurveKind::classify`] returns `None` for it, so a
//! consumer meeting one in an IFC4x3 file gets an honest miss rather than a
//! wrong shape.
//!
//! # Coordinate and parameter units
//!
//! Curve coordinates are in the model's length unit, but `IfcParameterValue`
//! is not a length. On a conic it is an angle in the model's *plane-angle*
//! unit, which is degrees in a large minority of files. No accessor in this
//! module converts anything; see [`crate::units`].

pub mod bspline;
pub mod composite;
pub mod conic;
pub mod line;
pub mod offset;
pub mod polyline;
pub mod trimmed;

pub use bspline::{BSplineCurve, BSplineCurveForm, KnotType, KnotVector};
pub use composite::{CompositeCurve, CompositeCurveSegment, TransitionCode};
pub use conic::{Circle, Ellipse};
pub use line::Line;
pub use offset::{
    OffsetCurve2D, OffsetCurve3D, PCurve, PreferredSurfaceCurveRepresentation, SurfaceCurve,
    SurfaceCurveKind,
};
pub use polyline::{IndexedPolyCurve, PolySegment, Polyline};
pub use trimmed::{Trim, TrimPoint, TrimSpec, TrimmedCurve, TrimmingPreference};

/// Which concrete `IfcCurve` subtype an entity is.
///
/// # Why classify at all
///
/// A representation item's attribute is typed `IfcCurve`, so the file gives a
/// reference and nothing else. Dispatch has to happen on the type name, and
/// doing it in one place keeps the spelling of 15 IFC type names out of every
/// call site.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CurveKind {
    /// `IfcLine`: unbounded, needs trimming to be drawable.
    Line,
    /// `IfcCircle`: the *whole* circle unless trimmed.
    Circle,
    /// `IfcEllipse`: the whole ellipse unless trimmed.
    Ellipse,
    /// `IfcPolyline`: a chain of `IfcCartesianPoint` entities.
    Polyline,
    /// `IfcIndexedPolyCurve`: a point list plus 1-based index segments.
    IndexedPolyCurve,
    /// `IfcTrimmedCurve`: a bounded piece of some other curve.
    TrimmedCurve,
    /// `IfcCompositeCurve` or a subtype; see [`CompositeCurve::is_on_surface`].
    CompositeCurve,
    /// Any `IfcBSplineCurve` subtype.
    BSplineCurve,
    /// `IfcOffsetCurve2D`: offset in the plane.
    OffsetCurve2D,
    /// `IfcOffsetCurve3D`: offset in space, needs a reference direction.
    OffsetCurve3D,
    /// `IfcPcurve`: a curve in a surface's parameter space.
    PCurve,
    /// `IfcSurfaceCurve` or a subtype.
    SurfaceCurve,
}

impl CurveKind {
    /// Classify an IFC type name, or `None` if it is not an `IfcCurve`.
    ///
    /// Case-insensitive because STEP keywords are, and real files disagree
    /// about casing even within one export.
    pub fn classify(type_name: &str) -> Option<Self> {
        match type_name.to_ascii_uppercase().as_str() {
            "IFCLINE" => Some(Self::Line),
            "IFCCIRCLE" => Some(Self::Circle),
            "IFCELLIPSE" => Some(Self::Ellipse),
            "IFCPOLYLINE" => Some(Self::Polyline),
            "IFCINDEXEDPOLYCURVE" => Some(Self::IndexedPolyCurve),
            "IFCTRIMMEDCURVE" => Some(Self::TrimmedCurve),
            "IFCCOMPOSITECURVE"
            | "IFCCOMPOSITECURVEONSURFACE"
            | "IFCBOUNDARYCURVE"
            | "IFCOUTERBOUNDARYCURVE" => Some(Self::CompositeCurve),
            "IFCBSPLINECURVE" | "IFCBSPLINECURVEWITHKNOTS" | "IFCRATIONALBSPLINECURVEWITHKNOTS" => {
                Some(Self::BSplineCurve)
            }
            "IFCOFFSETCURVE2D" => Some(Self::OffsetCurve2D),
            "IFCOFFSETCURVE3D" => Some(Self::OffsetCurve3D),
            "IFCPCURVE" => Some(Self::PCurve),
            "IFCSURFACECURVE" | "IFCINTERSECTIONCURVE" | "IFCSEAMCURVE" => Some(Self::SurfaceCurve),
            _ => None,
        }
    }

    /// Is this curve bounded on its own, without an enclosing trim?
    ///
    /// The question to ask before drawing anything. `false` means the entity
    /// describes an infinite or closed-and-unbounded locus that only becomes a
    /// drawable edge inside an `IfcTrimmedCurve` or an
    /// `IfcCompositeCurveSegment`.
    ///
    /// `IfcPcurve` and `IfcSurfaceCurve` are `false` because their bounds come
    /// from the curve they wrap, which may itself be unbounded.
    pub fn is_bounded(&self) -> bool {
        matches!(
            self,
            Self::Polyline
                | Self::IndexedPolyCurve
                | Self::TrimmedCurve
                | Self::CompositeCurve
                | Self::BSplineCurve
        )
    }

    /// Is this a closed conic whose trimming is sense-dependent?
    ///
    /// True for `IfcCircle` and `IfcEllipse`. When one of these is the basis
    /// curve of an `IfcTrimmedCurve`, the trim pair plus `SenseAgreement`
    /// select one of four arcs; see [`trimmed`].
    pub fn is_closed_conic(&self) -> bool {
        matches!(self, Self::Circle | Self::Ellipse)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_ifc4_curve_type_name_classifies() {
        let names = [
            "IFCLINE",
            "IFCCIRCLE",
            "IFCELLIPSE",
            "IFCPOLYLINE",
            "IFCINDEXEDPOLYCURVE",
            "IFCTRIMMEDCURVE",
            "IFCCOMPOSITECURVE",
            "IFCCOMPOSITECURVEONSURFACE",
            "IFCBOUNDARYCURVE",
            "IFCOUTERBOUNDARYCURVE",
            "IFCBSPLINECURVE",
            "IFCBSPLINECURVEWITHKNOTS",
            "IFCRATIONALBSPLINECURVEWITHKNOTS",
            "IFCOFFSETCURVE2D",
            "IFCOFFSETCURVE3D",
            "IFCPCURVE",
            "IFCSURFACECURVE",
            "IFCINTERSECTIONCURVE",
            "IFCSEAMCURVE",
        ];
        for name in names {
            assert!(
                CurveKind::classify(name).is_some(),
                "{name} is an IfcCurve subtype but does not classify"
            );
        }
    }

    #[test]
    fn a_non_curve_entity_does_not_classify_as_a_curve() {
        assert_eq!(CurveKind::classify("IFCWALL"), None);
        assert_eq!(CurveKind::classify("IFCPLANE"), None);
        assert_eq!(CurveKind::classify("IFCCARTESIANPOINT"), None);
    }

    /// IfcCurveSegment is IFC4x3 only; claiming to handle it would substitute
    /// wrong geometry for alignment models.
    #[test]
    fn ifc4x3_curve_segment_is_an_honest_miss_rather_than_a_wrong_match() {
        assert_eq!(CurveKind::classify("IFCCURVESEGMENT"), None);
    }

    /// STEP keywords are case-insensitive and exports are inconsistent.
    #[test]
    fn classification_ignores_case() {
        assert_eq!(CurveKind::classify("IfcCircle"), Some(CurveKind::Circle));
        assert_eq!(
            CurveKind::classify("ifcpolyline"),
            Some(CurveKind::Polyline)
        );
    }

    /// Drawing an untrimmed IfcCircle as a full circle is a real and common
    /// bug; the boundedness flag is what prevents it.
    #[test]
    fn unbounded_curves_are_distinguished_from_drawable_ones() {
        for kind in [
            CurveKind::Line,
            CurveKind::Circle,
            CurveKind::Ellipse,
            CurveKind::OffsetCurve2D,
            CurveKind::OffsetCurve3D,
            CurveKind::PCurve,
            CurveKind::SurfaceCurve,
        ] {
            assert!(!kind.is_bounded(), "{kind:?} must not claim to be bounded");
        }
        for kind in [
            CurveKind::Polyline,
            CurveKind::IndexedPolyCurve,
            CurveKind::TrimmedCurve,
            CurveKind::CompositeCurve,
            CurveKind::BSplineCurve,
        ] {
            assert!(kind.is_bounded(), "{kind:?} is bounded");
        }
    }

    #[test]
    fn only_conics_are_flagged_as_sense_dependent_when_trimmed() {
        assert!(CurveKind::Circle.is_closed_conic());
        assert!(CurveKind::Ellipse.is_closed_conic());
        assert!(!CurveKind::Line.is_closed_conic());
        assert!(!CurveKind::BSplineCurve.is_closed_conic());
    }
}
