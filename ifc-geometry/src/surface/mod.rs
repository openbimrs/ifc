//! Typed read-only views of the `IfcSurface` family.
//!
//! # The three branches
//!
//! `IfcSurface` is abstract and splits into:
//!
//! - [`elementary`]: analytic surfaces placed by an `IfcAxis2Placement3D`.
//!   Infinite. A file that means a finite piece wraps them.
//! - [`swept`]: a profile moved along a line or around an axis.
//! - [`bounded`]: finite patches, bounded either by curves or by a rectangle
//!   in parameter space.
//! - [`bspline`]: free-form NURBS patches.
//!
//! # Everything except the bounded branch is infinite
//!
//! `IfcPlane`, `IfcCylindricalSurface`, `IfcSphericalSurface` and
//! `IfcToroidalSurface` have no extent of their own; the sphere and torus are
//! closed and so are finite in area, but the plane and cylinder are not. They
//! appear as the `BasisSurface` of a bounded surface, as the base of an
//! `IfcHalfSpaceSolid`, or as the carrier of an `IfcFaceSurface`, and it is
//! that container which supplies the extent. Rendering a bare `IfcPlane`
//! produces a plane the size of the world.
//!
//! # Parameter units are not all lengths
//!
//! Every surface has a `(u, v)` parameterisation, and on the non-planar
//! elementary surfaces one or both parameters are *angles*. Applying the
//! model's length scale to them corrupts the geometry silently. See
//! [`ParameterKind`], which each elementary surface reports.
//!
//! # What is deliberately absent
//!
//! No evaluation, no tessellation, no NURBS basis functions. These views hand
//! a kernel the parameters; the kernel builds the surface.

pub mod bounded;
pub mod bspline;
pub mod elementary;
pub mod swept;

pub use bounded::{
    CurveBoundedPlane, CurveBoundedSurface, RectangularTrimmedSurface, TrimRectangle,
};
pub use bspline::{BSplineSurface, BSplineSurfaceForm, ControlPointGrid};
pub use elementary::{CylindricalSurface, ParameterKind, Plane, SphericalSurface, ToroidalSurface};
pub use swept::{SurfaceOfLinearExtrusion, SurfaceOfRevolution};

/// Which `IfcSurface` subtype an entity is.
///
/// Dispatch on the type name once, then construct the matching view. Returning
/// `None` rather than an error keeps "not a surface" separate from "a surface
/// we cannot read", which callers scanning a representation need.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SurfaceKind {
    /// `IfcPlane`.
    Plane,
    /// `IfcCylindricalSurface`.
    CylindricalSurface,
    /// `IfcSphericalSurface`.
    SphericalSurface,
    /// `IfcToroidalSurface`.
    ToroidalSurface,
    /// `IfcSurfaceOfLinearExtrusion`.
    SurfaceOfLinearExtrusion,
    /// `IfcSurfaceOfRevolution`.
    SurfaceOfRevolution,
    /// `IfcCurveBoundedPlane`.
    CurveBoundedPlane,
    /// `IfcCurveBoundedSurface`.
    CurveBoundedSurface,
    /// `IfcRectangularTrimmedSurface`.
    RectangularTrimmedSurface,
    /// `IfcBSplineSurface` and its non-knotted form.
    BSplineSurface,
    /// `IfcBSplineSurfaceWithKnots`.
    BSplineSurfaceWithKnots,
    /// `IfcRationalBSplineSurfaceWithKnots`.
    RationalBSplineSurfaceWithKnots,
}

impl SurfaceKind {
    /// Classify by IFC type name, `None` if not an `IfcSurface` subtype.
    pub fn classify(type_name: &str) -> Option<Self> {
        match type_name.to_ascii_uppercase().as_str() {
            "IFCPLANE" => Some(Self::Plane),
            "IFCCYLINDRICALSURFACE" => Some(Self::CylindricalSurface),
            "IFCSPHERICALSURFACE" => Some(Self::SphericalSurface),
            "IFCTOROIDALSURFACE" => Some(Self::ToroidalSurface),
            "IFCSURFACEOFLINEAREXTRUSION" => Some(Self::SurfaceOfLinearExtrusion),
            "IFCSURFACEOFREVOLUTION" => Some(Self::SurfaceOfRevolution),
            "IFCCURVEBOUNDEDPLANE" => Some(Self::CurveBoundedPlane),
            "IFCCURVEBOUNDEDSURFACE" => Some(Self::CurveBoundedSurface),
            "IFCRECTANGULARTRIMMEDSURFACE" => Some(Self::RectangularTrimmedSurface),
            "IFCBSPLINESURFACE" => Some(Self::BSplineSurface),
            "IFCBSPLINESURFACEWITHKNOTS" => Some(Self::BSplineSurfaceWithKnots),
            "IFCRATIONALBSPLINESURFACEWITHKNOTS" => Some(Self::RationalBSplineSurfaceWithKnots),
            _ => None,
        }
    }

    /// Is this an `IfcElementarySurface`?
    ///
    /// The distinguishing property is not "analytic" but *unbounded*: these
    /// are the surfaces that need a container to supply their extent.
    pub fn is_elementary(self) -> bool {
        matches!(
            self,
            Self::Plane | Self::CylindricalSurface | Self::SphericalSurface | Self::ToroidalSurface
        )
    }

    /// Is this an `IfcBoundedSurface`?
    ///
    /// The only branch that carries its own extent, so the only branch that
    /// can be meshed without asking its container how far to go.
    pub fn is_bounded(self) -> bool {
        matches!(
            self,
            Self::CurveBoundedPlane
                | Self::CurveBoundedSurface
                | Self::RectangularTrimmedSurface
                | Self::BSplineSurface
                | Self::BSplineSurfaceWithKnots
                | Self::RationalBSplineSurfaceWithKnots
        )
    }

    /// Is this an `IfcSweptSurface`?
    pub fn is_swept(self) -> bool {
        matches!(
            self,
            Self::SurfaceOfLinearExtrusion | Self::SurfaceOfRevolution
        )
    }

    /// Does this surface delegate its shape to another surface?
    ///
    /// True for the trimming and curve-bounding wrappers, whose `BasisSurface`
    /// must be resolved before anything can be evaluated. A consumer that does
    /// not follow the chain silently drops the geometry.
    pub fn has_basis_surface(self) -> bool {
        matches!(
            self,
            Self::CurveBoundedPlane | Self::CurveBoundedSurface | Self::RectangularTrimmedSurface
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_surface_entity_in_the_schema_is_classified() {
        let names = [
            "IFCPLANE",
            "IFCCYLINDRICALSURFACE",
            "IFCSPHERICALSURFACE",
            "IFCTOROIDALSURFACE",
            "IFCSURFACEOFLINEAREXTRUSION",
            "IFCSURFACEOFREVOLUTION",
            "IFCCURVEBOUNDEDPLANE",
            "IFCCURVEBOUNDEDSURFACE",
            "IFCRECTANGULARTRIMMEDSURFACE",
            "IFCBSPLINESURFACE",
            "IFCBSPLINESURFACEWITHKNOTS",
            "IFCRATIONALBSPLINESURFACEWITHKNOTS",
        ];
        for name in names {
            assert!(
                SurfaceKind::classify(name).is_some(),
                "{name} is unclassified"
            );
        }
    }

    /// A curve is not a surface: misclassifying would send it to a surface
    /// path that reads slot 0 as a placement.
    #[test]
    fn non_surface_entities_are_not_classified_as_surfaces() {
        for name in ["IFCPOLYLINE", "IFCCIRCLE", "IFCWALL", "IFCBSPLINECURVE"] {
            assert_eq!(SurfaceKind::classify(name), None, "{name}");
        }
    }

    #[test]
    fn type_names_are_matched_case_insensitively() {
        assert_eq!(SurfaceKind::classify("IfcPlane"), Some(SurfaceKind::Plane));
    }

    /// Only the bounded branch carries its own extent; everything elementary
    /// needs a container to say how far it goes.
    #[test]
    fn elementary_surfaces_are_never_self_bounding() {
        for kind in [
            SurfaceKind::Plane,
            SurfaceKind::CylindricalSurface,
            SurfaceKind::SphericalSurface,
            SurfaceKind::ToroidalSurface,
        ] {
            assert!(kind.is_elementary(), "{kind:?}");
            assert!(!kind.is_bounded(), "{kind:?}");
            assert!(!kind.is_swept(), "{kind:?}");
        }
    }

    #[test]
    fn bounded_surfaces_include_the_bspline_patches() {
        for kind in [
            SurfaceKind::CurveBoundedPlane,
            SurfaceKind::CurveBoundedSurface,
            SurfaceKind::RectangularTrimmedSurface,
            SurfaceKind::BSplineSurfaceWithKnots,
            SurfaceKind::RationalBSplineSurfaceWithKnots,
        ] {
            assert!(kind.is_bounded(), "{kind:?}");
            assert!(!kind.is_elementary(), "{kind:?}");
        }
    }

    #[test]
    fn swept_surfaces_are_the_two_profile_driven_kinds() {
        assert!(SurfaceKind::SurfaceOfLinearExtrusion.is_swept());
        assert!(SurfaceKind::SurfaceOfRevolution.is_swept());
        assert!(!SurfaceKind::Plane.is_swept());
    }

    /// A basis surface must be followed; a NURBS patch has no basis to follow.
    #[test]
    fn only_the_wrapping_surfaces_report_a_basis_surface() {
        assert!(SurfaceKind::RectangularTrimmedSurface.has_basis_surface());
        assert!(SurfaceKind::CurveBoundedPlane.has_basis_surface());
        assert!(SurfaceKind::CurveBoundedSurface.has_basis_surface());
        assert!(!SurfaceKind::BSplineSurfaceWithKnots.has_basis_surface());
        assert!(!SurfaceKind::Plane.has_basis_surface());
    }
}
