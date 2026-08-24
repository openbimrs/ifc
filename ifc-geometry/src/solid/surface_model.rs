//! Surface models and geometric sets: collections that are **not** solids.
//!
//! # Why these are grouped
//!
//! All four entities here are collections of geometry with no enclosed volume
//! guaranteed. That distinction is the point of the module: a
//! `IfcShellBasedSurfaceModel` looks like a brep and is not one, and code that
//! treats it as a solid computes volumes and material quantities from an open
//! surface.
//!
//! - [`ShellBasedSurfaceModel`] holds `IfcShell` (open **or** closed shells).
//!   Even a closed shell here carries no solid semantics: the entity is a
//!   surface model by declaration.
//! - [`FaceBasedSurfaceModel`] holds `IfcConnectedFaceSet`s, which are not
//!   required to be closed or even connected to each other.
//! - [`GeometricSet`] is a heterogeneous bag of points, curves and surfaces.
//! - [`GeometricCurveSet`] is a `GeometricSet` the schema forbids from
//!   containing surfaces -- so it is curves and points only.
//!
//! # Shells and face sets are not resolved here
//!
//! `IfcClosedShell`, `IfcOpenShell` and `IfcConnectedFaceSet` belong to
//! `IfcTopologyResource`, owned elsewhere. These views return `EntityId`s.

use crate::error::GeometryResult;
use crate::slots::Slots;
use ifc_model::{Entity, EntityId};

/// Surface model and geometric set slots.
///
/// EXPRESS (IFC4 ADD2 TC1): all four entities subtype
/// `IfcGeometricRepresentationItem`, which declares no explicit attributes, so
/// each one's single collection attribute is absolute slot 0.
/// `IfcGeometricCurveSet` adds nothing and inherits `Elements` at slot 0.
mod slot {
    /// `SbsmBoundary : SET [1:?] OF IfcShell`, on `IfcShellBasedSurfaceModel`.
    pub const SBSM_BOUNDARY: usize = 0;
    /// `FbsmFaces : SET [1:?] OF IfcConnectedFaceSet`, on the face-based
    /// surface model.
    pub const FBSM_FACES: usize = 0;
    /// `Elements : SET [1:?] OF IfcGeometricSetSelect`, on `IfcGeometricSet`.
    pub const ELEMENTS: usize = 0;
}

/// `IfcShellBasedSurfaceModel`: a surface described by one or more shells.
///
/// # Not a solid
///
/// `SbsmBoundary` is an `IfcShell` SELECT, so its members may be
/// `IfcClosedShell` or `IfcOpenShell`. Even when every shell is closed, the
/// entity declares a **surface model**, not a `IfcSolidModel`, and it is not a
/// legal boolean operand. Promoting it to a brep produces volume figures that a
/// quantity takeoff will happily report.
#[derive(Debug, Clone, Copy)]
pub struct ShellBasedSurfaceModel<'m> {
    slots: Slots<'m>,
}

impl<'m> ShellBasedSurfaceModel<'m> {
    /// Wrap an entity assumed to be an `IfcShellBasedSurfaceModel`.
    pub fn new(id: EntityId, entity: &'m Entity) -> Self {
        Self {
            slots: Slots::new(id, entity),
        }
    }

    /// The entity id.
    pub fn id(&self) -> EntityId {
        self.slots.id()
    }

    /// The `IfcShell` references bounding the model.
    ///
    /// Each is an `IfcClosedShell` or an `IfcOpenShell`; the caller must check
    /// the resolved type rather than assuming either.
    pub fn shells(&self) -> GeometryResult<Vec<EntityId>> {
        self.slots.req_ref_list(slot::SBSM_BOUNDARY, "SbsmBoundary")
    }
}

/// `IfcFaceBasedSurfaceModel`: a surface described by connected face sets.
///
/// The face sets need not be closed and need not connect to one another, so
/// this is the loosest surface container in the schema. It is frequently what
/// an exporter falls back to when it cannot produce a valid solid, which makes
/// its presence a useful signal about a file's quality.
#[derive(Debug, Clone, Copy)]
pub struct FaceBasedSurfaceModel<'m> {
    slots: Slots<'m>,
}

impl<'m> FaceBasedSurfaceModel<'m> {
    /// Wrap an entity assumed to be an `IfcFaceBasedSurfaceModel`.
    pub fn new(id: EntityId, entity: &'m Entity) -> Self {
        Self {
            slots: Slots::new(id, entity),
        }
    }

    /// The entity id.
    pub fn id(&self) -> EntityId {
        self.slots.id()
    }

    /// The `IfcConnectedFaceSet` references making up the model.
    pub fn face_sets(&self) -> GeometryResult<Vec<EntityId>> {
        self.slots.req_ref_list(slot::FBSM_FACES, "FbsmFaces")
    }
}

/// `IfcGeometricSet`: a heterogeneous collection of points, curves and
/// surfaces.
///
/// `Elements` is an `IfcGeometricSetSelect`, whose members may be `IfcPoint`,
/// `IfcCurve` or `IfcSurface`. The schema requires every member to share the
/// same dimensionality, but nothing more: a set may mix a curve and a surface
/// freely. Consumers must dispatch per element rather than sampling the first.
#[derive(Debug, Clone, Copy)]
pub struct GeometricSet<'m> {
    slots: Slots<'m>,
}

impl<'m> GeometricSet<'m> {
    /// Wrap an entity assumed to be an `IfcGeometricSet` or subtype.
    pub fn new(id: EntityId, entity: &'m Entity) -> Self {
        Self {
            slots: Slots::new(id, entity),
        }
    }

    /// The entity id.
    pub fn id(&self) -> EntityId {
        self.slots.id()
    }

    /// The IFC type name, naming the concrete subtype.
    pub fn type_name(&self) -> &'m str {
        self.slots.type_name()
    }

    /// The `IfcGeometricSetSelect` references in the set.
    ///
    /// Returned unresolved and unsorted: each may be a point, a curve or (on
    /// the base type only) a surface.
    pub fn elements(&self) -> GeometryResult<Vec<EntityId>> {
        self.slots.req_ref_list(slot::ELEMENTS, "Elements")
    }

    /// Is this the curves-and-points-only specialisation?
    pub fn is_curve_set(&self) -> bool {
        self.type_name()
            .eq_ignore_ascii_case("IFCGEOMETRICCURVESET")
    }
}

/// `IfcGeometricCurveSet`: a geometric set with no surfaces.
///
/// Adds no attributes; it is the EXPRESS `NoSurfaces` WHERE rule made into a
/// type. That guarantee is worth having because it lets a consumer skip surface
/// handling for the annotation and 2D-plan geometry these usually carry.
#[derive(Debug, Clone, Copy)]
pub struct GeometricCurveSet<'m> {
    slots: Slots<'m>,
}

impl<'m> GeometricCurveSet<'m> {
    /// Wrap an entity assumed to be an `IfcGeometricCurveSet`.
    pub fn new(id: EntityId, entity: &'m Entity) -> Self {
        Self {
            slots: Slots::new(id, entity),
        }
    }

    /// The entity id.
    pub fn id(&self) -> EntityId {
        self.slots.id()
    }

    /// The inherited `IfcGeometricSet` attributes.
    pub fn base(&self) -> GeometricSet<'m> {
        GeometricSet { slots: self.slots }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::solid::testkit::{entity, refs};

    /// A shell-based surface model is not a brep, even when its shells are
    /// closed; it declares no volume.
    #[test]
    fn shell_based_surface_model_exposes_shells_without_claiming_a_solid() {
        let e = entity("IFCSHELLBASEDSURFACEMODEL", vec![refs(&[10, 11])]);
        let view = ShellBasedSurfaceModel::new(EntityId(1), &e);
        assert_eq!(view.shells().unwrap(), vec![EntityId(10), EntityId(11)]);
    }

    #[test]
    fn face_based_surface_model_exposes_its_connected_face_sets() {
        let e = entity("IFCFACEBASEDSURFACEMODEL", vec![refs(&[20, 21, 22])]);
        let view = FaceBasedSurfaceModel::new(EntityId(1), &e);
        assert_eq!(
            view.face_sets().unwrap(),
            vec![EntityId(20), EntityId(21), EntityId(22)]
        );
    }

    #[test]
    fn geometric_set_elements_are_returned_unresolved_and_in_order() {
        let e = entity("IFCGEOMETRICSET", vec![refs(&[30, 31, 32])]);
        let view = GeometricSet::new(EntityId(1), &e);
        assert_eq!(
            view.elements().unwrap(),
            vec![EntityId(30), EntityId(31), EntityId(32)]
        );
        assert!(!view.is_curve_set());
    }

    /// The curve set shares the base layout exactly, so Elements is still
    /// slot 0 on the subtype.
    #[test]
    fn curve_set_inherits_elements_at_the_same_slot() {
        let e = entity("IFCGEOMETRICCURVESET", vec![refs(&[40, 41])]);
        let view = GeometricCurveSet::new(EntityId(1), &e);
        assert_eq!(
            view.base().elements().unwrap(),
            vec![EntityId(40), EntityId(41)]
        );
        assert!(view.base().is_curve_set());
    }

    #[test]
    fn an_empty_collection_slot_reports_the_missing_attribute() {
        let e = entity("IFCSHELLBASEDSURFACEMODEL", vec![]);
        let err = ShellBasedSurfaceModel::new(EntityId(5), &e)
            .shells()
            .unwrap_err();
        assert_eq!(err.entity(), Some(EntityId(5)));
        assert!(err.to_string().contains("SbsmBoundary"));
    }
}
