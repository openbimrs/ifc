//! What a closed boundary curve is read for (#393).
//!
//! The `IfcCompositeCurve` and `IfcIndexedPolyCurve` readers serve two
//! consumers that read the curve the same way but differ in what they admit:
//!
//! - an arbitrary profile's `OuterCurve` and `InnerCurves`, which become a
//!   [`axiolid_profile::Contour`] and may keep exact arcs;
//! - `IfcPolygonalBoundedHalfSpace.PolygonalBoundary`, which also keeps
//!   exact arcs (#398: Axiolid's `SolidOperation::BoundedHalfSpace` takes a
//!   `Profile` contour of lines and arcs, axiolid/kernel#277, Axiolid ADR
//!   0084), but whose points must lie in `Position`'s XY plane
//!   (`BoundaryDim`), and whose segment joints are judged within the
//!   model's `Precision`.
//!
//! One reader with a role keeps the two from drifting apart: the traversal,
//! the `SameSense` handling, the collinear-arc fallback and the closure rules
//! are written once.

use crate::error::{GeometryError, GeometryResult};
use ifc_model::EntityId;

/// The consumer a boundary curve is lowered for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum BoundaryRole {
    /// An arbitrary profile's boundary: exact lines and arcs.
    Profile,
    /// A polygonal bounded half-space's boundary: exact lines and arcs in
    /// `Position`'s XY plane.
    HalfSpace,
}

impl BoundaryRole {
    /// The boundary, as error text names it.
    pub(crate) fn what(self) -> &'static str {
        match self {
            Self::Profile => "a profile boundary",
            Self::HalfSpace => "a polygonal half-space boundary",
        }
    }

    /// A boundary point's in-plane coordinates, in FILE units.
    ///
    /// A profile reads the first two coordinates of any point that has at
    /// least two. A half-space boundary keeps `IfcPolyline`'s rule: a 3D
    /// point is admitted only with `z == 0` exactly, because any other value
    /// puts it off the plane `BoundaryDim` requires and projecting it would
    /// silently move the clip; the coordinates must also be finite.
    pub(crate) fn planar(
        self,
        point: EntityId,
        type_name: &str,
        coordinates: &[f64],
    ) -> GeometryResult<[f64; 2]> {
        let degenerate = |detail: String| GeometryError::Degenerate {
            entity: point,
            type_name: type_name.to_owned(),
            detail,
        };
        let (x, y) = match (self, coordinates) {
            (Self::Profile, [x, y, ..]) => return Ok([*x, *y]),
            (Self::Profile, _) => {
                return Err(degenerate(
                    "profile boundary point is not at least 2D".to_owned(),
                ))
            }
            (Self::HalfSpace, [x, y]) => (*x, *y),
            (Self::HalfSpace, [x, y, z]) if *z == 0.0 => (*x, *y),
            (Self::HalfSpace, [_, _, z]) => {
                return Err(degenerate(format!(
                    "polygonal half-space boundary point has z = {z}; the boundary must lie \
                     in Position's XY plane (BoundaryDim)"
                )))
            }
            (Self::HalfSpace, other) => {
                return Err(degenerate(format!(
                    "Coordinates has {} entries, expected 2 or 3",
                    other.len()
                )))
            }
        };
        if !x.is_finite() || !y.is_finite() {
            return Err(degenerate(
                "polygonal half-space boundary point must be finite".to_owned(),
            ));
        }
        Ok([x, y])
    }
}
