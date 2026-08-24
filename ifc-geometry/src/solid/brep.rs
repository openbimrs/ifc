//! Boundary representations: `IfcManifoldSolidBrep` and its subtypes.
//!
//! # The model
//!
//! A B-rep solid is one `Outer` closed shell, optionally with inner shells
//! that carve cavities out of it. Faceted breps use planar polygonal faces;
//! advanced breps use `IfcAdvancedFace` with analytic or NURBS surfaces and
//! properly curved edges.
//!
//! # The voids trap
//!
//! `IfcFacetedBrepWithVoids.Voids` and `IfcAdvancedBrepWithVoids.Voids` are
//! **inner** shells to be subtracted, not additional outer shells. Appending
//! them to the outer shell yields a solid with its cavities rendered as extra
//! surface -- visually plausible, volumetrically wrong, and it silently
//! corrupts any quantity takeoff computed from the mesh.
//!
//! # Shells are not resolved here
//!
//! `IfcClosedShell` belongs to `IfcTopologyResource`, owned elsewhere. These
//! views return shell `EntityId`s; walking into `CfsFaces` is the topology
//! layer's job.

use crate::error::GeometryResult;
use crate::slots::Slots;
use ifc_model::{Entity, EntityId};

/// `IfcManifoldSolidBrep` attribute slots.
///
/// EXPRESS (IFC4 ADD2 TC1): `IfcManifoldSolidBrep` declares `Outer` and its
/// supertype `IfcSolidModel` declares no explicit attributes, so `Outer` is
/// absolute slot 0 for every brep in the family.
mod slot {
    /// `Outer : IfcClosedShell`, declared on `IfcManifoldSolidBrep`.
    pub const OUTER: usize = 0;
    /// `Voids : SET [1:?] OF IfcClosedShell`, on the `WithVoids` subtypes.
    ///
    /// Absolute slot 1 in both `IfcFacetedBrepWithVoids` (which inherits only
    /// `Outer` through `IfcFacetedBrep`) and `IfcAdvancedBrepWithVoids`.
    pub const VOIDS: usize = 1;
}

/// `IfcManifoldSolidBrep`: the abstract brep, giving access to `Outer`.
///
/// Usable over any concrete brep because `Outer` sits at slot 0 in all of
/// them. `IfcFacetedBrep` and `IfcAdvancedBrep` add no attributes of their own,
/// so this view is the whole of their content.
#[derive(Debug, Clone, Copy)]
pub struct ManifoldSolidBrep<'m> {
    slots: Slots<'m>,
}

impl<'m> ManifoldSolidBrep<'m> {
    /// Wrap an entity assumed to be an `IfcManifoldSolidBrep` subtype.
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

    /// The `IfcClosedShell` reference bounding the solid from outside.
    pub fn outer(&self) -> GeometryResult<EntityId> {
        self.slots.req_ref(slot::OUTER, "Outer")
    }

    /// Does the concrete type carry `Voids`?
    ///
    /// Lets a caller branch without re-deriving the subtype lattice, and keeps
    /// the `WithVoids` string comparison in exactly one place.
    pub fn has_voids(&self) -> bool {
        let name = self.type_name();
        name.eq_ignore_ascii_case("IFCFACETEDBREPWITHVOIDS")
            || name.eq_ignore_ascii_case("IFCADVANCEDBREPWITHVOIDS")
    }
}

/// `IfcFacetedBrep`: a brep whose every face is a planar polygon.
///
/// Adds no attributes over [`ManifoldSolidBrep`]; the distinction is a promise
/// about the faces, which lets a consumer skip surface evaluation entirely.
#[derive(Debug, Clone, Copy)]
pub struct FacetedBrep<'m> {
    slots: Slots<'m>,
}

impl<'m> FacetedBrep<'m> {
    /// Wrap an entity assumed to be an `IfcFacetedBrep`.
    pub fn new(id: EntityId, entity: &'m Entity) -> Self {
        Self {
            slots: Slots::new(id, entity),
        }
    }

    /// The entity id.
    pub fn id(&self) -> EntityId {
        self.slots.id()
    }

    /// The inherited `IfcManifoldSolidBrep` attributes.
    pub fn base(&self) -> ManifoldSolidBrep<'m> {
        ManifoldSolidBrep { slots: self.slots }
    }
}

/// `IfcFacetedBrepWithVoids`: a faceted brep with internal cavities.
///
/// See the module docs: `Voids` are **subtracted** inner shells. They are also
/// each independently closed, so a void is a cavity, not a dent.
#[derive(Debug, Clone, Copy)]
pub struct FacetedBrepWithVoids<'m> {
    slots: Slots<'m>,
}

impl<'m> FacetedBrepWithVoids<'m> {
    /// Wrap an entity assumed to be an `IfcFacetedBrepWithVoids`.
    pub fn new(id: EntityId, entity: &'m Entity) -> Self {
        Self {
            slots: Slots::new(id, entity),
        }
    }

    /// The entity id.
    pub fn id(&self) -> EntityId {
        self.slots.id()
    }

    /// The inherited `IfcFacetedBrep` attributes.
    pub fn base(&self) -> FacetedBrep<'m> {
        FacetedBrep { slots: self.slots }
    }

    /// The `IfcClosedShell` references to subtract from the outer shell.
    ///
    /// Never concatenate these with `Outer`: they are holes, not more surface.
    pub fn voids(&self) -> GeometryResult<Vec<EntityId>> {
        self.slots.req_ref_list(slot::VOIDS, "Voids")
    }
}

/// `IfcAdvancedBrep`: a brep whose faces are `IfcAdvancedFace`.
///
/// The faces carry real surface geometry (planes, cylinders, B-splines) and
/// edges with curve geometry, so unlike [`FacetedBrep`] the boundaries are not
/// implied by their vertices. A consumer that treats the vertex loops as
/// polygons loses every curved edge, which is how a filleted steel section
/// renders as a crude prism.
#[derive(Debug, Clone, Copy)]
pub struct AdvancedBrep<'m> {
    slots: Slots<'m>,
}

impl<'m> AdvancedBrep<'m> {
    /// Wrap an entity assumed to be an `IfcAdvancedBrep`.
    pub fn new(id: EntityId, entity: &'m Entity) -> Self {
        Self {
            slots: Slots::new(id, entity),
        }
    }

    /// The entity id.
    pub fn id(&self) -> EntityId {
        self.slots.id()
    }

    /// The inherited `IfcManifoldSolidBrep` attributes.
    pub fn base(&self) -> ManifoldSolidBrep<'m> {
        ManifoldSolidBrep { slots: self.slots }
    }
}

/// `IfcAdvancedBrepWithVoids`: an advanced brep with internal cavities.
///
/// Same subtraction semantics as [`FacetedBrepWithVoids`], with the extra
/// schema requirement that every void shell's faces are also advanced faces.
#[derive(Debug, Clone, Copy)]
pub struct AdvancedBrepWithVoids<'m> {
    slots: Slots<'m>,
}

impl<'m> AdvancedBrepWithVoids<'m> {
    /// Wrap an entity assumed to be an `IfcAdvancedBrepWithVoids`.
    pub fn new(id: EntityId, entity: &'m Entity) -> Self {
        Self {
            slots: Slots::new(id, entity),
        }
    }

    /// The entity id.
    pub fn id(&self) -> EntityId {
        self.slots.id()
    }

    /// The inherited `IfcAdvancedBrep` attributes.
    pub fn base(&self) -> AdvancedBrep<'m> {
        AdvancedBrep { slots: self.slots }
    }

    /// The `IfcClosedShell` references to subtract from the outer shell.
    pub fn voids(&self) -> GeometryResult<Vec<EntityId>> {
        self.slots.req_ref_list(slot::VOIDS, "Voids")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::solid::testkit::{entity, refs};

    #[test]
    fn outer_shell_is_slot_zero_for_every_brep_subtype() {
        for name in [
            "IFCFACETEDBREP",
            "IFCADVANCEDBREP",
            "IFCFACETEDBREPWITHVOIDS",
            "IFCADVANCEDBREPWITHVOIDS",
        ] {
            let e = entity(name, vec![crate::solid::testkit::r(100), refs(&[200])]);
            let view = ManifoldSolidBrep::new(EntityId(1), &e);
            assert_eq!(view.outer().unwrap(), EntityId(100), "{name}");
        }
    }

    /// Voids are inner shells to subtract; they must stay separate from Outer
    /// or the cavity renders as extra outward surface.
    #[test]
    fn voids_are_kept_separate_from_the_outer_shell() {
        let e = entity(
            "IFCFACETEDBREPWITHVOIDS",
            vec![crate::solid::testkit::r(100), refs(&[201, 202])],
        );
        let view = FacetedBrepWithVoids::new(EntityId(1), &e);
        let outer = view.base().base().outer().unwrap();
        let voids = view.voids().unwrap();

        assert_eq!(outer, EntityId(100));
        assert_eq!(voids, vec![EntityId(201), EntityId(202)]);
        assert!(!voids.contains(&outer), "a void is never the outer shell");
    }

    #[test]
    fn advanced_brep_with_voids_uses_the_same_slot_layout() {
        let e = entity(
            "IFCADVANCEDBREPWITHVOIDS",
            vec![crate::solid::testkit::r(7), refs(&[8, 9])],
        );
        let view = AdvancedBrepWithVoids::new(EntityId(1), &e);
        assert_eq!(view.base().base().outer().unwrap(), EntityId(7));
        assert_eq!(view.voids().unwrap(), vec![EntityId(8), EntityId(9)]);
    }

    /// The void-carrying subtypes are distinguishable without the caller
    /// re-deriving the subtype lattice.
    #[test]
    fn only_the_with_voids_subtypes_report_carrying_voids() {
        let plain = entity("IFCFACETEDBREP", vec![crate::solid::testkit::r(1)]);
        let voided = entity(
            "IFCFACETEDBREPWITHVOIDS",
            vec![crate::solid::testkit::r(1), refs(&[2])],
        );
        assert!(!ManifoldSolidBrep::new(EntityId(1), &plain).has_voids());
        assert!(ManifoldSolidBrep::new(EntityId(1), &voided).has_voids());
    }

    #[test]
    fn a_brep_missing_its_outer_shell_reports_the_entity() {
        let e = entity("IFCFACETEDBREP", vec![]);
        let err = FacetedBrep::new(EntityId(42), &e)
            .base()
            .outer()
            .unwrap_err();
        assert_eq!(err.entity(), Some(EntityId(42)));
        assert!(err.to_string().contains("Outer"));
    }
}
