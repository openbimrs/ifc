//! `IfcSweptSurface`: a surface made by moving a profile.
//!
//! Covers `IfcSurfaceOfLinearExtrusion` and `IfcSurfaceOfRevolution`.
//!
//! # `SweptCurve` is an `IfcProfileDef`, not an `IfcCurve`
//!
//! This is the trap. The attribute is named for a curve and typed as a
//! profile, and an `IfcProfileDef` carries a `ProfileType` of `AREA` or
//! `CURVE`. Only the *outer curve* of the profile is swept -- a swept surface
//! is a surface, not a solid, so an `AREA` profile's inner voids contribute
//! nothing. A consumer that hands the profile to the same code path used for
//! `IfcExtrudedAreaSolid` will build a solid where the file described a shell.
//!
//! # `Position` is optional and its absence is not the identity
//!
//! The profile is defined in its own 2D coordinate system; `Position` places
//! that system in 3D. When absent the profile's XY plane coincides with the
//! containing representation's XY plane. Treating an absent `Position` as an
//! error rejects valid files; treating it as "no transform at all" is right
//! only because the default *is* the identity in the parent's space.
//!
//! # Revolution: axis, not direction
//!
//! `AxisPosition` is an `IfcAxis1Placement` -- a point plus a direction. Both
//! halves matter: revolving about a line through the origin gives a different
//! surface from revolving about a parallel line one metre away. Code that
//! reads only the direction produces a surface of the right shape in the wrong
//! place, which is much harder to notice than a missing surface.

use crate::error::GeometryResult;
use crate::slots::Slots;
use ifc_model::{Entity, EntityId};

/// `IfcSweptSurface` family attribute slots.
///
/// From IFC4 ADD2 TC1: slots 0 and 1 are inherited from `IfcSweptSurface` by
/// both subtypes; each subtype's own attributes follow from slot 2.
mod slot {
    /// `SweptCurve`: `IfcProfileDef`, from `IfcSweptSurface`.
    pub const SWEPT_CURVE: usize = 0;
    /// `Position`: `OPTIONAL IfcAxis2Placement3D`, from `IfcSweptSurface`.
    pub const POSITION: usize = 1;
    /// `ExtrudedDirection` on `IfcSurfaceOfLinearExtrusion`.
    pub const EXTRUDED_DIRECTION: usize = 2;
    /// `Depth` on `IfcSurfaceOfLinearExtrusion`.
    pub const DEPTH: usize = 3;
    /// `AxisPosition` on `IfcSurfaceOfRevolution`.
    pub const AXIS_POSITION: usize = 2;
}

/// Shared accessors for both swept surfaces.
///
/// Split out so the two views cannot drift on the attributes they inherit.
#[derive(Debug, Clone, Copy)]
struct SweptBase<'m> {
    slots: Slots<'m>,
}

impl<'m> SweptBase<'m> {
    fn swept_curve_ref(&self) -> GeometryResult<EntityId> {
        self.slots.req_ref(slot::SWEPT_CURVE, "SweptCurve")
    }

    fn position_ref(&self) -> Option<EntityId> {
        self.slots.opt_ref(slot::POSITION)
    }
}

/// A borrowed view of an `IfcSurfaceOfLinearExtrusion`.
#[derive(Debug, Clone, Copy)]
pub struct SurfaceOfLinearExtrusion<'m> {
    base: SweptBase<'m>,
}

impl<'m> SurfaceOfLinearExtrusion<'m> {
    /// Wrap an entity known to be an `IfcSurfaceOfLinearExtrusion`.
    pub fn new(id: EntityId, entity: &'m Entity) -> Self {
        Self {
            base: SweptBase {
                slots: Slots::new(id, entity),
            },
        }
    }

    /// The entity id.
    pub fn id(&self) -> EntityId {
        self.base.slots.id()
    }

    /// The `IfcProfileDef` whose outer curve is swept.
    ///
    /// A profile, not a curve, despite the attribute name. Only its outer
    /// curve contributes; see the module docs.
    // TODO: `resource`/`profile` will provide the typed profile view.
    pub fn swept_curve_ref(&self) -> GeometryResult<EntityId> {
        self.base.swept_curve_ref()
    }

    /// The optional placement of the profile's coordinate system.
    ///
    /// `None` means the identity in the containing representation's space,
    /// which is a legal and common encoding, not a missing attribute.
    // TODO: `resource::placement` will provide the typed placement view.
    pub fn position_ref(&self) -> Option<EntityId> {
        self.base.position_ref()
    }

    /// The `IfcDirection` the profile is swept along.
    ///
    /// Given in the profile's own coordinate system, so it must be transformed
    /// by `Position` before use in model space.
    // TODO: `resource::direction` will provide the typed direction view.
    pub fn extruded_direction_ref(&self) -> GeometryResult<EntityId> {
        self.base
            .slots
            .req_ref(slot::EXTRUDED_DIRECTION, "ExtrudedDirection")
    }

    /// The extrusion depth, guaranteed non-zero.
    ///
    /// A zero depth collapses the surface to its generating curve. IFC4
    /// declares this `IfcLengthMeasure` rather than the positive variant, so a
    /// negative depth is legal and means the sweep runs the other way; only
    /// zero is rejected.
    pub fn depth(&self) -> GeometryResult<f64> {
        let depth = self.base.slots.req_f64(slot::DEPTH, "Depth")?;
        if depth == 0.0 {
            return Err(self
                .base
                .slots
                .degenerate("Depth is zero; the surface collapses to its generating curve"));
        }
        Ok(depth)
    }
}

/// A borrowed view of an `IfcSurfaceOfRevolution`.
#[derive(Debug, Clone, Copy)]
pub struct SurfaceOfRevolution<'m> {
    base: SweptBase<'m>,
}

impl<'m> SurfaceOfRevolution<'m> {
    /// Wrap an entity known to be an `IfcSurfaceOfRevolution`.
    pub fn new(id: EntityId, entity: &'m Entity) -> Self {
        Self {
            base: SweptBase {
                slots: Slots::new(id, entity),
            },
        }
    }

    /// The entity id.
    pub fn id(&self) -> EntityId {
        self.base.slots.id()
    }

    /// The `IfcProfileDef` whose outer curve is revolved.
    // TODO: `resource`/`profile` will provide the typed profile view.
    pub fn swept_curve_ref(&self) -> GeometryResult<EntityId> {
        self.base.swept_curve_ref()
    }

    /// The optional placement of the profile's coordinate system.
    // TODO: `resource::placement` will provide the typed placement view.
    pub fn position_ref(&self) -> Option<EntityId> {
        self.base.position_ref()
    }

    /// The `IfcAxis1Placement` giving the axis of revolution.
    ///
    /// Carries a location *and* a direction. Both are needed: the offset
    /// between the axis and the profile is what makes the surface a torus
    /// rather than a sphere.
    // TODO: `resource::placement` will provide the typed axis-placement view.
    pub fn axis_position_ref(&self) -> GeometryResult<EntityId> {
        self.base.slots.req_ref(slot::AXIS_POSITION, "AxisPosition")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ifc_model::Value;

    fn extrusion(position: Value, depth: Value) -> Entity {
        Entity::new(
            "IFCSURFACEOFLINEAREXTRUSION",
            vec![
                Value::Ref(EntityId(80)),
                position,
                Value::Ref(EntityId(82)),
                depth,
            ],
        )
    }

    #[test]
    fn extrusion_reads_inherited_swept_curve_before_its_own_direction_and_depth() {
        let e = extrusion(Value::Ref(EntityId(81)), Value::Real(3.0));
        let view = SurfaceOfLinearExtrusion::new(EntityId(1), &e);
        assert_eq!(view.swept_curve_ref().unwrap(), EntityId(80));
        assert_eq!(view.position_ref(), Some(EntityId(81)));
        assert_eq!(view.extruded_direction_ref().unwrap(), EntityId(82));
        assert_eq!(view.depth().unwrap(), 3.0);
    }

    /// An absent Position is the schema's own default, not a broken file.
    #[test]
    fn an_absent_position_is_reported_as_none_rather_than_as_an_error() {
        let e = extrusion(Value::Null, Value::Real(1.0));
        let view = SurfaceOfLinearExtrusion::new(EntityId(1), &e);
        assert_eq!(view.position_ref(), None);
        // The rest of the surface still reads, since Position is optional.
        assert_eq!(view.extruded_direction_ref().unwrap(), EntityId(82));
        assert_eq!(view.depth().unwrap(), 1.0);
    }

    /// IfcLengthMeasure, not the positive variant: reversal is legal.
    #[test]
    fn a_negative_depth_is_accepted_because_it_only_reverses_the_sweep() {
        let e = extrusion(Value::Ref(EntityId(81)), Value::Real(-2.0));
        assert_eq!(
            SurfaceOfLinearExtrusion::new(EntityId(1), &e)
                .depth()
                .unwrap(),
            -2.0
        );
    }

    #[test]
    fn a_zero_depth_is_degenerate() {
        let e = extrusion(Value::Ref(EntityId(81)), Value::Real(0.0));
        let err = SurfaceOfLinearExtrusion::new(EntityId(5), &e)
            .depth()
            .unwrap_err();
        assert!(err.to_string().contains("#5"), "got: {err}");
        assert!(err.to_string().contains("zero"), "got: {err}");
    }

    #[test]
    fn depth_reads_through_a_length_measure_wrapper() {
        let e = extrusion(
            Value::Ref(EntityId(81)),
            Value::Typed {
                type_name: "IFCLENGTHMEASURE".into(),
                value: Box::new(Value::Real(1.5)),
            },
        );
        assert_eq!(
            SurfaceOfLinearExtrusion::new(EntityId(1), &e)
                .depth()
                .unwrap(),
            1.5
        );
    }

    /// AxisPosition sits at slot 2, where the extrusion keeps
    /// ExtrudedDirection; reading the wrong slot would silently succeed.
    #[test]
    fn revolution_reads_its_axis_from_the_slot_after_the_inherited_two() {
        let e = Entity::new(
            "IFCSURFACEOFREVOLUTION",
            vec![
                Value::Ref(EntityId(90)),
                Value::Ref(EntityId(91)),
                Value::Ref(EntityId(92)),
            ],
        );
        let view = SurfaceOfRevolution::new(EntityId(1), &e);
        assert_eq!(view.swept_curve_ref().unwrap(), EntityId(90));
        assert_eq!(view.position_ref(), Some(EntityId(91)));
        assert_eq!(view.axis_position_ref().unwrap(), EntityId(92));
    }

    #[test]
    fn a_revolution_without_an_axis_reports_the_attribute_by_name() {
        let e = Entity::new(
            "IFCSURFACEOFREVOLUTION",
            vec![Value::Ref(EntityId(90)), Value::Null],
        );
        let err = SurfaceOfRevolution::new(EntityId(1), &e)
            .axis_position_ref()
            .unwrap_err();
        assert!(err.to_string().contains("AxisPosition"), "got: {err}");
    }

    #[test]
    fn a_swept_surface_without_its_profile_reports_the_attribute_by_name() {
        let e = Entity::new("IFCSURFACEOFLINEAREXTRUSION", vec![]);
        let err = SurfaceOfLinearExtrusion::new(EntityId(1), &e)
            .swept_curve_ref()
            .unwrap_err();
        assert!(err.to_string().contains("SweptCurve"), "got: {err}");
    }
}
