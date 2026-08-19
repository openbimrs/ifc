//! `IfcBoundedSurface`: finite patches cut from infinite surfaces.
//!
//! Covers `IfcCurveBoundedPlane`, `IfcCurveBoundedSurface` and
//! `IfcRectangularTrimmedSurface`.
//!
//! # The two ways to bound a surface
//!
//! **By curves.** `IfcCurveBoundedPlane` and `IfcCurveBoundedSurface` name
//! boundary curves. The difference between them is subtle and load-bearing:
//! the *plane* variant separates `OuterBoundary` from `InnerBoundaries`
//! explicitly, so holes are unambiguous. The *surface* variant has a single
//! `Boundaries` set and an `ImplicitOuter` flag; when that flag is true, no
//! member is the outline and the surface's own natural parameter bounds serve
//! as the outer boundary, making every listed boundary a hole. A consumer that
//! treats the first member of `Boundaries` as the outline gets a hole-shaped
//! patch instead of a patch with a hole.
//!
//! **By parameter range.** `IfcRectangularTrimmedSurface` cuts a rectangle in
//! `(u, v)` space. Its `U1/U2` and `V1/V2` are *parameters*, which on a
//! cylinder or sphere means angles, so a length unit scale applied to them is
//! wrong. See [`crate::surface::elementary::ParameterKind`].
//!
//! # Why `Usense` and `Vsense` are not redundant
//!
//! On a closed (periodic) parameter direction, `U1 = 350deg` and `U2 = 10deg`
//! describe two different patches: the 20-degree sliver or the 340-degree
//! remainder. `Usense` picks which. IFC's own rule is that for a non-closed
//! direction the sense must agree with `U1 < U2`, but for a closed one both
//! orders are legal -- exactly the four-arc problem of `IfcTrimmedCurve`,
//! transposed to surfaces. Nothing here reorders the bounds.

use crate::error::GeometryResult;
use crate::slots::Slots;
use ifc_model::{Entity, EntityId};

/// `IfcCurveBoundedPlane` attribute slots, from IFC4 ADD2 TC1.
mod plane_slot {
    /// `BasisSurface`: an `IfcPlane`, not a general surface.
    pub const BASIS_SURFACE: usize = 0;
    /// `OuterBoundary`: the outline curve.
    pub const OUTER_BOUNDARY: usize = 1;
    /// `InnerBoundaries`: `SET [0:?] OF IfcCurve`, the holes.
    pub const INNER_BOUNDARIES: usize = 2;
}

/// `IfcCurveBoundedSurface` attribute slots, from IFC4 ADD2 TC1.
mod surface_slot {
    /// `BasisSurface`: any `IfcSurface`.
    pub const BASIS_SURFACE: usize = 0;
    /// `Boundaries`: `SET [1:?] OF IfcBoundaryCurve`.
    pub const BOUNDARIES: usize = 1;
    /// `ImplicitOuter`: whether the outer bound is the surface's own extent.
    pub const IMPLICIT_OUTER: usize = 2;
}

/// `IfcRectangularTrimmedSurface` attribute slots, from IFC4 ADD2 TC1.
mod trimmed_slot {
    /// `BasisSurface`: the surface being trimmed.
    pub const BASIS_SURFACE: usize = 0;
    /// `U1`: first u parameter.
    pub const U1: usize = 1;
    /// `V1`: first v parameter.
    pub const V1: usize = 2;
    /// `U2`: second u parameter.
    pub const U2: usize = 3;
    /// `V2`: second v parameter.
    pub const V2: usize = 4;
    /// `Usense`: does u run in increasing parameter order?
    pub const USENSE: usize = 5;
    /// `Vsense`: does v run in increasing parameter order?
    pub const VSENSE: usize = 6;
}

/// A borrowed view of an `IfcCurveBoundedPlane`.
#[derive(Debug, Clone, Copy)]
pub struct CurveBoundedPlane<'m> {
    slots: Slots<'m>,
}

impl<'m> CurveBoundedPlane<'m> {
    /// Wrap an entity known to be an `IfcCurveBoundedPlane`.
    pub fn new(id: EntityId, entity: &'m Entity) -> Self {
        Self {
            slots: Slots::new(id, entity),
        }
    }

    /// The entity id.
    pub fn id(&self) -> EntityId {
        self.slots.id()
    }

    /// The `IfcPlane` this patch lies in.
    ///
    /// Narrower than `IfcCurveBoundedSurface::BasisSurface`: the schema
    /// requires a plane here, so boundary curves are genuinely 2D.
    pub fn basis_surface_ref(&self) -> GeometryResult<EntityId> {
        self.slots
            .req_ref(plane_slot::BASIS_SURFACE, "BasisSurface")
    }

    /// The outline curve.
    ///
    /// Required and unambiguous, unlike the `IfcCurveBoundedSurface` case.
    pub fn outer_boundary_ref(&self) -> GeometryResult<EntityId> {
        self.slots
            .req_ref(plane_slot::OUTER_BOUNDARY, "OuterBoundary")
    }

    /// The hole curves; empty when the patch is solid.
    ///
    /// `SET [0:?]`, so an empty set and an absent attribute mean the same
    /// thing and neither is an error.
    pub fn inner_boundary_refs(&self) -> Vec<EntityId> {
        self.slots.opt_ref_list(plane_slot::INNER_BOUNDARIES)
    }
}

/// A borrowed view of an `IfcCurveBoundedSurface`.
#[derive(Debug, Clone, Copy)]
pub struct CurveBoundedSurface<'m> {
    slots: Slots<'m>,
}

impl<'m> CurveBoundedSurface<'m> {
    /// Wrap an entity known to be an `IfcCurveBoundedSurface`.
    pub fn new(id: EntityId, entity: &'m Entity) -> Self {
        Self {
            slots: Slots::new(id, entity),
        }
    }

    /// The entity id.
    pub fn id(&self) -> EntityId {
        self.slots.id()
    }

    /// The surface being bounded; any `IfcSurface`.
    pub fn basis_surface_ref(&self) -> GeometryResult<EntityId> {
        self.slots
            .req_ref(surface_slot::BASIS_SURFACE, "BasisSurface")
    }

    /// The boundary curves, at least one.
    ///
    /// These are `IfcBoundaryCurve` entities, so
    /// [`crate::curve::CompositeCurve::is_outer_boundary`] can tell an
    /// `IfcOuterBoundaryCurve` from a plain one -- which is how the outline is
    /// identified when [`Self::implicit_outer`] is false.
    pub fn boundary_refs(&self) -> GeometryResult<Vec<EntityId>> {
        let boundaries = self
            .slots
            .req_ref_list(surface_slot::BOUNDARIES, "Boundaries")?;
        if boundaries.is_empty() {
            return Err(self
                .slots
                .degenerate("Boundaries is empty; SET [1:?] requires a member"));
        }
        Ok(boundaries)
    }

    /// Is the outer boundary the surface's own natural extent?
    ///
    /// When true, **every** member of `Boundaries` is a hole and none is the
    /// outline. Defaults to `false` when absent, matching the more common and
    /// more conservative reading: an explicit outline is expected among the
    /// boundaries.
    pub fn implicit_outer(&self) -> bool {
        self.slots
            .opt_bool(surface_slot::IMPLICIT_OUTER)
            .unwrap_or(false)
    }
}

/// The parameter rectangle cut from a surface, with senses intact.
///
/// Kept as a struct rather than four loose numbers because the sense flags are
/// meaningless without the bounds they qualify, and a caller that fetches the
/// bounds without the senses will build the complementary patch on a periodic
/// surface.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TrimRectangle {
    /// First u parameter, as written.
    pub u1: f64,
    /// First v parameter, as written.
    pub v1: f64,
    /// Second u parameter, as written.
    pub u2: f64,
    /// Second v parameter, as written.
    pub v2: f64,
    /// Does u run from `u1` to `u2` in increasing parameter order?
    pub usense: bool,
    /// Does v run from `v1` to `v2` in increasing parameter order?
    pub vsense: bool,
}

impl TrimRectangle {
    /// Does the u range wrap through the surface's period?
    ///
    /// True when `u1 > u2` while `usense` claims increasing order, which is
    /// only consistent on a closed parameter direction. Useful as a check: a
    /// file asserting this for a plane is inconsistent, and a kernel that
    /// clamps rather than wraps will produce an empty patch.
    pub fn u_wraps(&self) -> bool {
        self.usense == (self.u1 > self.u2)
    }

    /// Does the v range wrap through the surface's period?
    pub fn v_wraps(&self) -> bool {
        self.vsense == (self.v1 > self.v2)
    }
}

/// A borrowed view of an `IfcRectangularTrimmedSurface`.
#[derive(Debug, Clone, Copy)]
pub struct RectangularTrimmedSurface<'m> {
    slots: Slots<'m>,
}

impl<'m> RectangularTrimmedSurface<'m> {
    /// Wrap an entity known to be an `IfcRectangularTrimmedSurface`.
    pub fn new(id: EntityId, entity: &'m Entity) -> Self {
        Self {
            slots: Slots::new(id, entity),
        }
    }

    /// The entity id.
    pub fn id(&self) -> EntityId {
        self.slots.id()
    }

    /// The surface being trimmed.
    pub fn basis_surface_ref(&self) -> GeometryResult<EntityId> {
        self.slots
            .req_ref(trimmed_slot::BASIS_SURFACE, "BasisSurface")
    }

    /// The full parameter rectangle including both sense flags.
    ///
    /// Rejects a zero-extent rectangle in either direction: `u1 == u2` gives a
    /// patch with no area, which reaches a mesher as a degenerate face rather
    /// than as an error.
    pub fn rectangle(&self) -> GeometryResult<TrimRectangle> {
        let u1 = self.slots.req_f64(trimmed_slot::U1, "U1")?;
        let v1 = self.slots.req_f64(trimmed_slot::V1, "V1")?;
        let u2 = self.slots.req_f64(trimmed_slot::U2, "U2")?;
        let v2 = self.slots.req_f64(trimmed_slot::V2, "V2")?;

        if u1 == u2 {
            return Err(self
                .slots
                .degenerate(format!("U1 and U2 are both {u1}; the patch has no extent")));
        }
        if v1 == v2 {
            return Err(self
                .slots
                .degenerate(format!("V1 and V2 are both {v1}; the patch has no extent")));
        }

        Ok(TrimRectangle {
            u1,
            v1,
            u2,
            v2,
            usense: self.slots.req_bool(trimmed_slot::USENSE, "Usense")?,
            vsense: self.slots.req_bool(trimmed_slot::VSENSE, "Vsense")?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ifc_model::Value;

    fn refs(ids: &[u64]) -> Value {
        Value::List(ids.iter().map(|i| Value::Ref(EntityId(*i))).collect())
    }

    fn trimmed(u1: f64, v1: f64, u2: f64, v2: f64, usense: bool, vsense: bool) -> Entity {
        Entity::new(
            "IFCRECTANGULARTRIMMEDSURFACE",
            vec![
                Value::Ref(EntityId(100)),
                Value::Real(u1),
                Value::Real(v1),
                Value::Real(u2),
                Value::Real(v2),
                Value::Bool(usense),
                Value::Bool(vsense),
            ],
        )
    }

    #[test]
    fn a_curve_bounded_plane_separates_its_outline_from_its_holes() {
        let e = Entity::new(
            "IFCCURVEBOUNDEDPLANE",
            vec![
                Value::Ref(EntityId(100)),
                Value::Ref(EntityId(101)),
                refs(&[102, 103]),
            ],
        );
        let view = CurveBoundedPlane::new(EntityId(1), &e);
        assert_eq!(view.basis_surface_ref().unwrap(), EntityId(100));
        assert_eq!(view.outer_boundary_ref().unwrap(), EntityId(101));
        assert_eq!(
            view.inner_boundary_refs(),
            vec![EntityId(102), EntityId(103)]
        );
    }

    /// SET [0:?]: no holes is normal, not a missing attribute.
    #[test]
    fn a_plane_with_no_holes_reports_an_empty_inner_boundary_list() {
        let e = Entity::new(
            "IFCCURVEBOUNDEDPLANE",
            vec![
                Value::Ref(EntityId(100)),
                Value::Ref(EntityId(101)),
                Value::List(vec![]),
            ],
        );
        assert!(CurveBoundedPlane::new(EntityId(1), &e)
            .inner_boundary_refs()
            .is_empty());

        let absent = Entity::new(
            "IFCCURVEBOUNDEDPLANE",
            vec![Value::Ref(EntityId(100)), Value::Ref(EntityId(101))],
        );
        assert!(CurveBoundedPlane::new(EntityId(1), &absent)
            .inner_boundary_refs()
            .is_empty());
    }

    /// With ImplicitOuter true, every listed boundary is a hole; reading the
    /// first as an outline inverts the patch.
    #[test]
    fn implicit_outer_makes_every_listed_boundary_a_hole() {
        let implicit = Entity::new(
            "IFCCURVEBOUNDEDSURFACE",
            vec![
                Value::Ref(EntityId(100)),
                refs(&[101, 102]),
                Value::Bool(true),
            ],
        );
        let view = CurveBoundedSurface::new(EntityId(1), &implicit);
        assert!(view.implicit_outer());
        assert_eq!(view.boundary_refs().unwrap().len(), 2);

        let explicit = Entity::new(
            "IFCCURVEBOUNDEDSURFACE",
            vec![
                Value::Ref(EntityId(100)),
                refs(&[101, 102]),
                Value::Bool(false),
            ],
        );
        assert!(!CurveBoundedSurface::new(EntityId(1), &explicit).implicit_outer());
    }

    /// The conservative default: expect an explicit outline among the
    /// boundaries rather than silently turning the outline into a hole.
    #[test]
    fn an_absent_implicit_outer_defaults_to_false() {
        let e = Entity::new(
            "IFCCURVEBOUNDEDSURFACE",
            vec![Value::Ref(EntityId(100)), refs(&[101])],
        );
        assert!(!CurveBoundedSurface::new(EntityId(1), &e).implicit_outer());
    }

    #[test]
    fn a_curve_bounded_surface_with_no_boundaries_is_degenerate() {
        let e = Entity::new(
            "IFCCURVEBOUNDEDSURFACE",
            vec![Value::Ref(EntityId(100)), Value::List(vec![])],
        );
        assert!(CurveBoundedSurface::new(EntityId(1), &e)
            .boundary_refs()
            .is_err());
    }

    /// U1,V1,U2,V2 is the declaration order; reading it as U1,U2,V1,V2 would
    /// swap a patch's width for its height and still typecheck.
    #[test]
    fn trim_parameters_are_read_in_u1_v1_u2_v2_declaration_order() {
        let e = trimmed(0.0, 1.0, 2.0, 3.0, true, true);
        let rect = RectangularTrimmedSurface::new(EntityId(1), &e)
            .rectangle()
            .unwrap();
        assert_eq!(rect.u1, 0.0);
        assert_eq!(rect.v1, 1.0);
        assert_eq!(rect.u2, 2.0);
        assert_eq!(rect.v2, 3.0);
    }

    /// Descending bounds on a periodic direction are legal and select the
    /// complementary patch, so they must not be sorted.
    #[test]
    fn descending_trim_bounds_are_preserved_not_normalised() {
        let e = trimmed(350.0, 0.0, 10.0, 1.0, true, true);
        let rect = RectangularTrimmedSurface::new(EntityId(1), &e)
            .rectangle()
            .unwrap();
        assert_eq!(rect.u1, 350.0);
        assert_eq!(rect.u2, 10.0);
        assert!(rect.u_wraps());
        assert!(!rect.v_wraps());
    }

    /// Sense flags distinguish patches that share their bounds, so flipping
    /// one must produce a different rectangle.
    #[test]
    fn the_sense_flags_distinguish_otherwise_identical_rectangles() {
        let a = trimmed(0.0, 0.0, 90.0, 1.0, true, true);
        let b = trimmed(0.0, 0.0, 90.0, 1.0, false, true);
        let rect_a = RectangularTrimmedSurface::new(EntityId(1), &a)
            .rectangle()
            .unwrap();
        let rect_b = RectangularTrimmedSurface::new(EntityId(1), &b)
            .rectangle()
            .unwrap();
        assert_ne!(rect_a, rect_b);
        assert!(!rect_a.u_wraps());
        assert!(rect_b.u_wraps());
    }

    #[test]
    fn a_zero_extent_trim_rectangle_is_degenerate_and_names_the_direction() {
        let flat_u = trimmed(1.0, 0.0, 1.0, 2.0, true, true);
        let err = RectangularTrimmedSurface::new(EntityId(4), &flat_u)
            .rectangle()
            .unwrap_err();
        assert!(err.to_string().contains("U1 and U2"), "got: {err}");

        let flat_v = trimmed(0.0, 5.0, 1.0, 5.0, true, true);
        let err = RectangularTrimmedSurface::new(EntityId(4), &flat_v)
            .rectangle()
            .unwrap_err();
        assert!(err.to_string().contains("V1 and V2"), "got: {err}");
    }

    #[test]
    fn trim_parameters_read_through_parameter_value_wrappers() {
        let e = Entity::new(
            "IFCRECTANGULARTRIMMEDSURFACE",
            vec![
                Value::Ref(EntityId(100)),
                Value::Typed {
                    type_name: "IFCPARAMETERVALUE".into(),
                    value: Box::new(Value::Real(0.0)),
                },
                Value::Real(0.0),
                Value::Typed {
                    type_name: "IFCPARAMETERVALUE".into(),
                    value: Box::new(Value::Real(1.0)),
                },
                Value::Real(1.0),
                Value::Bool(true),
                Value::Bool(true),
            ],
        );
        let rect = RectangularTrimmedSurface::new(EntityId(1), &e)
            .rectangle()
            .unwrap();
        assert_eq!(rect.u2, 1.0);
    }
}
