//! Profile sweeps: `IfcSweptAreaSolid` and its extrusion/revolution subtypes.
//!
//! These four concrete types cover almost every solid in a real building model.

use super::{extruded_slot, revolved_slot, swept_area_slot};
use crate::error::GeometryResult;
use crate::slots::Slots;
use ifc_model::{Entity, EntityId};

/// The abstract `IfcSweptAreaSolid` view: profile plus optional placement.
///
/// Constructible over any subtype, because the inherited slots sit at the same
/// absolute positions in all of them. That is what lets a caller read the
/// profile of an extrusion, a revolution or a tapered sweep uniformly.
#[derive(Debug, Clone, Copy)]
pub struct SweptAreaSolid<'m> {
    slots: Slots<'m>,
}

impl<'m> SweptAreaSolid<'m> {
    /// Wrap an entity assumed to be an `IfcSweptAreaSolid` subtype.
    pub fn new(id: EntityId, entity: &'m Entity) -> Self {
        Self {
            slots: Slots::new(id, entity),
        }
    }

    /// Build from already-wrapped slots, for a subtype view delegating upward.
    pub(super) fn from_slots(slots: Slots<'m>) -> Self {
        Self { slots }
    }

    /// The entity id.
    pub fn id(&self) -> EntityId {
        self.slots.id()
    }

    /// The IFC type name, which names the concrete subtype.
    pub fn type_name(&self) -> &'m str {
        self.slots.type_name()
    }

    /// The `IfcProfileDef` reference giving the cross section.
    ///
    /// TODO: resolve through the profile module once it exists; this crate
    /// deliberately does not define a second, competing profile view.
    pub fn swept_area(&self) -> GeometryResult<EntityId> {
        self.slots.req_ref(swept_area_slot::SWEPT_AREA, "SweptArea")
    }

    /// The `IfcAxis2Placement3D` positioning the sweep, when present.
    ///
    /// Absent means the identity placement: the profile sits in the XY plane
    /// of the containing representation's coordinate system.
    pub fn position(&self) -> Option<EntityId> {
        self.slots.opt_ref(swept_area_slot::POSITION)
    }
}

/// `IfcExtrudedAreaSolid`: a profile swept along a straight direction.
///
/// The overwhelming majority of walls, slabs, columns and beams in a real model
/// are one of these, so this is the hot path of IFC geometry.
///
/// # The oblique-extrusion trap
///
/// `ExtrudedDirection` is expressed **in the `Position` coordinate system** and
/// is *not* required to be +Z. The schema only forbids it from being
/// perpendicular to the local Z axis. Assuming `[0, 0, 1]` produces a solid of
/// the right volume in the wrong place for every sheared extrusion in the file,
/// and sheared extrusions are common in roof and ramp geometry.
///
/// `Depth` is measured **along `ExtrudedDirection`**, not along Z, and is a
/// positive length measure: a zero or negative depth is a degenerate file.
#[derive(Debug, Clone, Copy)]
pub struct ExtrudedAreaSolid<'m> {
    slots: Slots<'m>,
}

impl<'m> ExtrudedAreaSolid<'m> {
    /// Wrap an entity assumed to be an `IfcExtrudedAreaSolid`.
    pub fn new(id: EntityId, entity: &'m Entity) -> Self {
        Self {
            slots: Slots::new(id, entity),
        }
    }

    /// The entity id.
    pub fn id(&self) -> EntityId {
        self.slots.id()
    }

    /// The inherited `IfcSweptAreaSolid` attributes.
    pub fn base(&self) -> SweptAreaSolid<'m> {
        SweptAreaSolid::from_slots(self.slots)
    }

    /// The `IfcDirection` reference giving the sweep direction.
    ///
    /// Expressed in the `Position` coordinate system. See the type docs: it is
    /// not necessarily +Z.
    pub fn extruded_direction(&self) -> GeometryResult<EntityId> {
        self.slots
            .req_ref(extruded_slot::EXTRUDED_DIRECTION, "ExtrudedDirection")
    }

    /// The sweep distance along `ExtrudedDirection`, in file length units.
    pub fn depth(&self) -> GeometryResult<f64> {
        self.slots.req_f64(extruded_slot::DEPTH, "Depth")
    }

    /// The depth, rejecting the non-positive values the schema forbids.
    ///
    /// Separate from [`Self::depth`] so a caller inspecting a file can still
    /// see the raw value while a caller building geometry gets a located error
    /// instead of a zero-volume solid.
    pub fn checked_depth(&self) -> GeometryResult<f64> {
        let depth = self.depth()?;
        if depth > 0.0 {
            Ok(depth)
        } else {
            Err(self
                .slots
                .degenerate(format!("Depth must be positive, found {depth}")))
        }
    }
}

/// `IfcExtrudedAreaSolidTapered`: an extrusion whose section morphs.
///
/// The solid is lofted between `SweptArea` at the start and `EndSweptArea` at
/// the depth. A kernel that ignores `EndSweptArea` silently produces a prism
/// where the file describes a taper, which is why this is a distinct view
/// rather than an optional field on [`ExtrudedAreaSolid`].
#[derive(Debug, Clone, Copy)]
pub struct ExtrudedAreaSolidTapered<'m> {
    slots: Slots<'m>,
}

impl<'m> ExtrudedAreaSolidTapered<'m> {
    /// Wrap an entity assumed to be an `IfcExtrudedAreaSolidTapered`.
    pub fn new(id: EntityId, entity: &'m Entity) -> Self {
        Self {
            slots: Slots::new(id, entity),
        }
    }

    /// The entity id.
    pub fn id(&self) -> EntityId {
        self.slots.id()
    }

    /// The inherited `IfcExtrudedAreaSolid` attributes.
    pub fn base(&self) -> ExtrudedAreaSolid<'m> {
        ExtrudedAreaSolid { slots: self.slots }
    }

    /// The `IfcProfileDef` reference for the section at full depth.
    pub fn end_swept_area(&self) -> GeometryResult<EntityId> {
        self.slots
            .req_ref(extruded_slot::END_SWEPT_AREA, "EndSweptArea")
    }
}

/// `IfcRevolvedAreaSolid`: a profile swept about an axis.
///
/// # The angle-unit trap
///
/// `Angle` is an `IfcPlaneAngleMeasure`, expressed in the **file's declared
/// angle unit**. That is very often degrees: IFC exporters routinely declare
/// the plane angle unit as `DEGREE` through a conversion-based unit. Treating a
/// `90` as radians yields fourteen full turns.
///
/// This view returns the raw number deliberately. Converting is
/// [`crate::units`]'s responsibility and must be applied exactly once.
#[derive(Debug, Clone, Copy)]
pub struct RevolvedAreaSolid<'m> {
    slots: Slots<'m>,
}

impl<'m> RevolvedAreaSolid<'m> {
    /// Wrap an entity assumed to be an `IfcRevolvedAreaSolid`.
    pub fn new(id: EntityId, entity: &'m Entity) -> Self {
        Self {
            slots: Slots::new(id, entity),
        }
    }

    /// The entity id.
    pub fn id(&self) -> EntityId {
        self.slots.id()
    }

    /// The inherited `IfcSweptAreaSolid` attributes.
    pub fn base(&self) -> SweptAreaSolid<'m> {
        SweptAreaSolid::from_slots(self.slots)
    }

    /// The `IfcAxis1Placement` reference giving the axis of revolution.
    ///
    /// Expressed in the `Position` coordinate system, and its own `Axis`
    /// attribute is itself optional, defaulting to +Z of that placement.
    pub fn axis(&self) -> GeometryResult<EntityId> {
        self.slots.req_ref(revolved_slot::AXIS, "Axis")
    }

    /// The sweep angle **in the file's angle unit**, unconverted.
    pub fn angle_raw(&self) -> GeometryResult<f64> {
        self.slots.req_f64(revolved_slot::ANGLE, "Angle")
    }
}

/// `IfcRevolvedAreaSolidTapered`: a revolution whose section morphs.
///
/// Same lofting caveat as [`ExtrudedAreaSolidTapered`]: ignoring
/// `EndSweptArea` produces a plain revolution, not the described solid.
#[derive(Debug, Clone, Copy)]
pub struct RevolvedAreaSolidTapered<'m> {
    slots: Slots<'m>,
}

impl<'m> RevolvedAreaSolidTapered<'m> {
    /// Wrap an entity assumed to be an `IfcRevolvedAreaSolidTapered`.
    pub fn new(id: EntityId, entity: &'m Entity) -> Self {
        Self {
            slots: Slots::new(id, entity),
        }
    }

    /// The entity id.
    pub fn id(&self) -> EntityId {
        self.slots.id()
    }

    /// The inherited `IfcRevolvedAreaSolid` attributes.
    pub fn base(&self) -> RevolvedAreaSolid<'m> {
        RevolvedAreaSolid { slots: self.slots }
    }

    /// The `IfcProfileDef` reference for the section at the end angle.
    pub fn end_swept_area(&self) -> GeometryResult<EntityId> {
        self.slots
            .req_ref(revolved_slot::END_SWEPT_AREA, "EndSweptArea")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::solid::testkit::{entity, n, r};
    use ifc_model::Value;

    fn extrusion(attrs: Vec<Value>) -> Entity {
        entity("IFCEXTRUDEDAREASOLID", attrs)
    }

    /// The inherited slots come FIRST; reading SweptArea from slot 2 is the
    /// classic local-index mistake.
    #[test]
    fn inherited_swept_area_slots_precede_the_subtype_own_slots() {
        let e = extrusion(vec![r(10), r(20), r(30), n(3.0)]);
        let view = ExtrudedAreaSolid::new(EntityId(1), &e);
        assert_eq!(view.base().swept_area().unwrap(), EntityId(10));
        assert_eq!(view.base().position(), Some(EntityId(20)));
        assert_eq!(view.extruded_direction().unwrap(), EntityId(30));
        assert_eq!(view.depth().unwrap(), 3.0);
    }

    /// Position is optional and its absence means identity, not an error.
    #[test]
    fn absent_position_is_reported_as_none_not_as_a_failure() {
        let e = extrusion(vec![r(10), Value::Null, r(30), n(3.0)]);
        let view = ExtrudedAreaSolid::new(EntityId(1), &e);
        assert_eq!(view.base().position(), None);
        assert!(view.base().swept_area().is_ok());
    }

    /// The direction is a reference to be resolved in the Position system; it
    /// is never assumed to be +Z, and a missing one is an error not a default.
    #[test]
    fn extruded_direction_is_a_reference_and_never_defaulted_to_z() {
        let e = extrusion(vec![r(10), r(20), r(99), n(1.0)]);
        assert_eq!(
            ExtrudedAreaSolid::new(EntityId(1), &e)
                .extruded_direction()
                .unwrap(),
            EntityId(99)
        );

        let missing = extrusion(vec![r(10), r(20), Value::Null, n(1.0)]);
        assert!(ExtrudedAreaSolid::new(EntityId(1), &missing)
            .extruded_direction()
            .is_err());
    }

    #[test]
    fn non_positive_depth_is_rejected_as_degenerate() {
        for bad in [0.0, -2.5] {
            let e = extrusion(vec![r(10), r(20), r(30), n(bad)]);
            let view = ExtrudedAreaSolid::new(EntityId(7), &e);
            let err = view.checked_depth().unwrap_err();
            assert_eq!(err.entity(), Some(EntityId(7)));
            assert!(view.depth().is_ok(), "raw depth stays readable");
        }
        let good = extrusion(vec![r(10), r(20), r(30), n(2.5)]);
        assert_eq!(
            ExtrudedAreaSolid::new(EntityId(7), &good)
                .checked_depth()
                .unwrap(),
            2.5
        );
    }

    #[test]
    fn tapered_extrusion_keeps_both_profiles_addressable() {
        let e = entity(
            "IFCEXTRUDEDAREASOLIDTAPERED",
            vec![r(10), r(20), r(30), n(3.0), r(40)],
        );
        let view = ExtrudedAreaSolidTapered::new(EntityId(1), &e);
        assert_eq!(view.base().base().swept_area().unwrap(), EntityId(10));
        assert_eq!(view.base().depth().unwrap(), 3.0);
        assert_eq!(view.end_swept_area().unwrap(), EntityId(40));
    }

    /// Angle is in the file's angle unit, so the view must hand back the
    /// literal without scaling it.
    #[test]
    fn revolution_angle_is_returned_raw_without_unit_conversion() {
        let e = entity("IFCREVOLVEDAREASOLID", vec![r(10), r(20), r(30), n(90.0)]);
        let view = RevolvedAreaSolid::new(EntityId(1), &e);
        assert_eq!(view.angle_raw().unwrap(), 90.0);
        assert_eq!(view.axis().unwrap(), EntityId(30));
        assert_eq!(view.base().swept_area().unwrap(), EntityId(10));
    }

    #[test]
    fn tapered_revolution_exposes_its_end_profile() {
        let e = entity(
            "IFCREVOLVEDAREASOLIDTAPERED",
            vec![r(10), r(20), r(30), n(45.0), r(50)],
        );
        let view = RevolvedAreaSolidTapered::new(EntityId(1), &e);
        assert_eq!(view.base().angle_raw().unwrap(), 45.0);
        assert_eq!(view.end_swept_area().unwrap(), EntityId(50));
    }

    /// Type name survives the view so diagnostics can name the subtype.
    #[test]
    fn abstract_view_reports_the_concrete_subtype_name() {
        let e = extrusion(vec![r(10), r(20), r(30), n(1.0)]);
        let view = SweptAreaSolid::new(EntityId(1), &e);
        assert_eq!(view.type_name(), "IFCEXTRUDEDAREASOLID");
        assert_eq!(view.id(), EntityId(1));
    }
}
