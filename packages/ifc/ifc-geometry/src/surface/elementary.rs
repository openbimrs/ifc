//! `IfcElementarySurface` subtypes: plane, cylinder, sphere, torus.
//!
//! # What they share
//!
//! Exactly one inherited attribute, `Position`, an `IfcAxis2Placement3D`.
//! Unlike `IfcConic::Position` this is *not* a select: an elementary surface
//! is always placed in 3D. The placement is not decoration -- it defines the
//! surface's parameter space, so two cylinders with the same radius and
//! different placements have different `(u, v)` meanings and their p-curves
//! are not interchangeable.
//!
//! # Parameterisation, and why it decides unit handling
//!
//! | Surface | u | v |
//! | --- | --- | --- |
//! | `IfcPlane` | length along local X | length along local Y |
//! | `IfcCylindricalSurface` | **angle** about local Z | length along local Z |
//! | `IfcSphericalSurface` | **angle** about local Z | **angle** from equator |
//! | `IfcToroidalSurface` | **angle** about local Z | **angle** about the tube |
//!
//! A consumer that scales every `IfcPcurve` coordinate by the model's length
//! unit will corrupt every non-planar surface: on a cylinder in millimetres it
//! multiplies an angle by 0.001. [`ParameterKind`] exists so that decision can
//! be made from data rather than from a comment.

use crate::error::GeometryResult;
use crate::slots::Slots;
use ifc_model::{Entity, EntityId};

/// `IfcElementarySurface` family attribute slots.
///
/// From IFC4 ADD2 TC1: slot 0 `Position` is inherited from
/// `IfcElementarySurface` by all four subtypes; radii follow from slot 1.
mod slot {
    /// `Position`: `IfcAxis2Placement3D`, from `IfcElementarySurface`.
    pub const POSITION: usize = 0;
    /// `Radius` on `IfcCylindricalSurface` and `IfcSphericalSurface`.
    pub const RADIUS: usize = 1;
    /// `MajorRadius` on `IfcToroidalSurface`: centre to tube centre.
    pub const MAJOR_RADIUS: usize = 1;
    /// `MinorRadius` on `IfcToroidalSurface`: the tube's own radius.
    pub const MINOR_RADIUS: usize = 2;
}

/// What a surface parameter means, and therefore how to convert it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParameterKind {
    /// A distance in the model's length unit.
    Length,
    /// An angle in the model's plane-angle unit, which may be degrees.
    Angle,
}

/// A borrowed view of an `IfcPlane`.
///
/// Infinite in both parameters. A file that means a finite patch wraps this in
/// an `IfcCurveBoundedPlane` or an `IfcRectangularTrimmedSurface`; a consumer
/// that renders a bare `IfcPlane` will fill the world.
#[derive(Debug, Clone, Copy)]
pub struct Plane<'m> {
    slots: Slots<'m>,
}

impl<'m> Plane<'m> {
    /// Wrap an entity known to be an `IfcPlane`.
    pub fn new(id: EntityId, entity: &'m Entity) -> Self {
        Self {
            slots: Slots::new(id, entity),
        }
    }

    /// The entity id.
    pub fn id(&self) -> EntityId {
        self.slots.id()
    }

    /// The `IfcAxis2Placement3D` whose Z axis is the plane normal.
    ///
    /// The normal direction matters beyond orientation: it decides which side
    /// of a half-space solid is solid.
    // TODO: `resource::placement` will provide the typed placement view.
    pub fn position_ref(&self) -> GeometryResult<EntityId> {
        self.slots.req_ref(slot::POSITION, "Position")
    }

    /// Both parameters of a plane are lengths.
    pub fn parameter_kinds(&self) -> (ParameterKind, ParameterKind) {
        (ParameterKind::Length, ParameterKind::Length)
    }
}

/// A borrowed view of an `IfcCylindricalSurface`.
#[derive(Debug, Clone, Copy)]
pub struct CylindricalSurface<'m> {
    slots: Slots<'m>,
}

impl<'m> CylindricalSurface<'m> {
    /// Wrap an entity known to be an `IfcCylindricalSurface`.
    pub fn new(id: EntityId, entity: &'m Entity) -> Self {
        Self {
            slots: Slots::new(id, entity),
        }
    }

    /// The entity id.
    pub fn id(&self) -> EntityId {
        self.slots.id()
    }

    /// The placement; local Z is the cylinder axis.
    // TODO: `resource::placement` will provide the typed placement view.
    pub fn position_ref(&self) -> GeometryResult<EntityId> {
        self.slots.req_ref(slot::POSITION, "Position")
    }

    /// The radius, guaranteed positive.
    pub fn radius(&self) -> GeometryResult<f64> {
        positive(&self.slots, slot::RADIUS, "Radius")
    }

    /// `u` is an angle about the axis, `v` a length along it.
    pub fn parameter_kinds(&self) -> (ParameterKind, ParameterKind) {
        (ParameterKind::Angle, ParameterKind::Length)
    }
}

/// A borrowed view of an `IfcSphericalSurface`.
#[derive(Debug, Clone, Copy)]
pub struct SphericalSurface<'m> {
    slots: Slots<'m>,
}

impl<'m> SphericalSurface<'m> {
    /// Wrap an entity known to be an `IfcSphericalSurface`.
    pub fn new(id: EntityId, entity: &'m Entity) -> Self {
        Self {
            slots: Slots::new(id, entity),
        }
    }

    /// The entity id.
    pub fn id(&self) -> EntityId {
        self.slots.id()
    }

    /// The placement; local Z runs through the poles.
    // TODO: `resource::placement` will provide the typed placement view.
    pub fn position_ref(&self) -> GeometryResult<EntityId> {
        self.slots.req_ref(slot::POSITION, "Position")
    }

    /// The radius, guaranteed positive.
    pub fn radius(&self) -> GeometryResult<f64> {
        positive(&self.slots, slot::RADIUS, "Radius")
    }

    /// Both parameters are angles: longitude and latitude.
    pub fn parameter_kinds(&self) -> (ParameterKind, ParameterKind) {
        (ParameterKind::Angle, ParameterKind::Angle)
    }
}

/// A borrowed view of an `IfcToroidalSurface`.
#[derive(Debug, Clone, Copy)]
pub struct ToroidalSurface<'m> {
    slots: Slots<'m>,
}

impl<'m> ToroidalSurface<'m> {
    /// Wrap an entity known to be an `IfcToroidalSurface`.
    pub fn new(id: EntityId, entity: &'m Entity) -> Self {
        Self {
            slots: Slots::new(id, entity),
        }
    }

    /// The entity id.
    pub fn id(&self) -> EntityId {
        self.slots.id()
    }

    /// The placement; local Z is the torus axis.
    // TODO: `resource::placement` will provide the typed placement view.
    pub fn position_ref(&self) -> GeometryResult<EntityId> {
        self.slots.req_ref(slot::POSITION, "Position")
    }

    /// Distance from the torus centre to the centre of the tube.
    pub fn major_radius(&self) -> GeometryResult<f64> {
        positive(&self.slots, slot::MAJOR_RADIUS, "MajorRadius")
    }

    /// The tube's own radius.
    pub fn minor_radius(&self) -> GeometryResult<f64> {
        positive(&self.slots, slot::MINOR_RADIUS, "MinorRadius")
    }

    /// Is the tube radius at least the major radius?
    ///
    /// IFC permits it and it is not an error: `minor >= major` gives a
    /// self-intersecting "spindle" or "apple" torus rather than a ring. Worth
    /// asking because a kernel that assumes a ring topology will produce
    /// inverted normals on the inner surface, and because it is far more often
    /// a units mistake in the file than a deliberate shape.
    pub fn is_self_intersecting(&self) -> GeometryResult<bool> {
        Ok(self.minor_radius()? >= self.major_radius()?)
    }

    /// Both parameters are angles: around the axis and around the tube.
    pub fn parameter_kinds(&self) -> (ParameterKind, ParameterKind) {
        (ParameterKind::Angle, ParameterKind::Angle)
    }
}

/// Read a radius that the schema declares `IfcPositiveLengthMeasure`.
///
/// STEP does not enforce constrained types, and zero radii reach a kernel as
/// divisions by zero on a surface thousands of entities from the cause.
fn positive(slots: &Slots<'_>, index: usize, name: &'static str) -> GeometryResult<f64> {
    let value = slots.req_f64(index, name)?;
    if value > 0.0 {
        Ok(value)
    } else {
        Err(slots.degenerate(format!("{name} must be positive, found {value}")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ifc_model::Value;

    fn surface(type_name: &str, radii: &[f64]) -> Entity {
        let mut attributes = vec![Value::Ref(EntityId(70))];
        attributes.extend(radii.iter().map(|r| Value::Real(*r)));
        Entity::new(type_name, attributes)
    }

    #[test]
    fn every_elementary_surface_reads_position_from_the_inherited_slot_zero() {
        let plane = surface("IFCPLANE", &[]);
        assert_eq!(
            Plane::new(EntityId(1), &plane).position_ref().unwrap(),
            EntityId(70)
        );

        let cylinder = surface("IFCCYLINDRICALSURFACE", &[2.0]);
        assert_eq!(
            CylindricalSurface::new(EntityId(1), &cylinder)
                .position_ref()
                .unwrap(),
            EntityId(70)
        );

        let sphere = surface("IFCSPHERICALSURFACE", &[3.0]);
        assert_eq!(
            SphericalSurface::new(EntityId(1), &sphere)
                .position_ref()
                .unwrap(),
            EntityId(70)
        );

        let torus = surface("IFCTOROIDALSURFACE", &[5.0, 1.0]);
        assert_eq!(
            ToroidalSurface::new(EntityId(1), &torus)
                .position_ref()
                .unwrap(),
            EntityId(70)
        );
    }

    #[test]
    fn cylinder_and_sphere_radii_are_read_from_the_slot_after_position() {
        let cylinder = surface("IFCCYLINDRICALSURFACE", &[2.5]);
        assert_eq!(
            CylindricalSurface::new(EntityId(1), &cylinder)
                .radius()
                .unwrap(),
            2.5
        );
        let sphere = surface("IFCSPHERICALSURFACE", &[4.0]);
        assert_eq!(
            SphericalSurface::new(EntityId(1), &sphere)
                .radius()
                .unwrap(),
            4.0
        );
    }

    #[test]
    fn torus_radii_are_read_in_major_then_minor_order() {
        let e = surface("IFCTOROIDALSURFACE", &[5.0, 1.0]);
        let view = ToroidalSurface::new(EntityId(1), &e);
        assert_eq!(view.major_radius().unwrap(), 5.0);
        assert_eq!(view.minor_radius().unwrap(), 1.0);
        assert!(!view.is_self_intersecting().unwrap());
    }

    /// A spindle torus is legal IFC and usually a units bug, so it is flagged
    /// rather than rejected.
    #[test]
    fn a_minor_radius_at_least_the_major_is_reported_as_self_intersecting() {
        let e = surface("IFCTOROIDALSURFACE", &[1.0, 2.0]);
        assert!(ToroidalSurface::new(EntityId(1), &e)
            .is_self_intersecting()
            .unwrap());
    }

    #[test]
    fn a_zero_radius_surface_is_degenerate_and_names_the_attribute() {
        let cylinder = surface("IFCCYLINDRICALSURFACE", &[0.0]);
        let err = CylindricalSurface::new(EntityId(8), &cylinder)
            .radius()
            .unwrap_err();
        assert!(err.to_string().contains("#8"), "got: {err}");
        assert!(err.to_string().contains("Radius"), "got: {err}");

        let torus = surface("IFCTOROIDALSURFACE", &[5.0, -1.0]);
        let err = ToroidalSurface::new(EntityId(1), &torus)
            .minor_radius()
            .unwrap_err();
        assert!(err.to_string().contains("MinorRadius"), "got: {err}");
    }

    /// Scaling an angular surface parameter by a length unit is a silent
    /// corruption, so each surface states which of its parameters are angles.
    #[test]
    fn only_the_plane_has_two_length_parameters() {
        let plane = surface("IFCPLANE", &[]);
        assert_eq!(
            Plane::new(EntityId(1), &plane).parameter_kinds(),
            (ParameterKind::Length, ParameterKind::Length)
        );

        let cylinder = surface("IFCCYLINDRICALSURFACE", &[1.0]);
        assert_eq!(
            CylindricalSurface::new(EntityId(1), &cylinder).parameter_kinds(),
            (ParameterKind::Angle, ParameterKind::Length)
        );

        let sphere = surface("IFCSPHERICALSURFACE", &[1.0]);
        assert_eq!(
            SphericalSurface::new(EntityId(1), &sphere).parameter_kinds(),
            (ParameterKind::Angle, ParameterKind::Angle)
        );

        let torus = surface("IFCTOROIDALSURFACE", &[5.0, 1.0]);
        assert_eq!(
            ToroidalSurface::new(EntityId(1), &torus).parameter_kinds(),
            (ParameterKind::Angle, ParameterKind::Angle)
        );
    }

    #[test]
    fn a_surface_missing_its_position_reports_the_attribute_by_name() {
        let e = Entity::new("IFCPLANE", vec![]);
        let err = Plane::new(EntityId(1), &e).position_ref().unwrap_err();
        assert!(err.to_string().contains("Position"), "got: {err}");
    }
}
