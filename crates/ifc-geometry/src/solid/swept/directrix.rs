//! Path-driven sweeps: directrix sweeps, swept disks, sectioned spines.
//!
//! What unites these is that a **curve** rather than a linear direction or an
//! axis drives the sweep, which is why the orientation rules differ per type
//! and why each one records its own frame convention.

use super::{directrix_slot, disk_slot, spine_slot};
use crate::error::GeometryResult;
use crate::slots::Slots;
use crate::solid::swept::area::SweptAreaSolid;
use ifc_model::{Entity, EntityId};

/// `IfcSurfaceCurveSweptAreaSolid`: a profile swept along a curve that lies on
/// a surface.
///
/// The directrix is a curve **on** `ReferenceSurface`, and the surface normal
/// at each point defines the profile's orientation. Sweeping along the curve
/// alone, ignoring the surface, twists the section wrongly on any
/// non-developable surface -- this is how curved facade mullions go wrong.
///
/// `StartParam`/`EndParam` may be absent, in which case the directrix must
/// itself be bounded or conic and is used in full.
#[derive(Debug, Clone, Copy)]
pub struct SurfaceCurveSweptAreaSolid<'m> {
    slots: Slots<'m>,
}

impl<'m> SurfaceCurveSweptAreaSolid<'m> {
    /// Wrap an entity assumed to be an `IfcSurfaceCurveSweptAreaSolid`.
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

    /// The `IfcCurve` reference the profile follows.
    pub fn directrix(&self) -> GeometryResult<EntityId> {
        self.slots.req_ref(directrix_slot::DIRECTRIX, "Directrix")
    }

    /// The parameter at which the sweep starts, if trimmed.
    pub fn start_param(&self) -> Option<f64> {
        self.slots.opt_f64(directrix_slot::START_PARAM)
    }

    /// The parameter at which the sweep ends, if trimmed.
    pub fn end_param(&self) -> Option<f64> {
        self.slots.opt_f64(directrix_slot::END_PARAM)
    }

    /// The `IfcSurface` reference the directrix lies on.
    pub fn reference_surface(&self) -> GeometryResult<EntityId> {
        self.slots
            .req_ref(directrix_slot::REFERENCE_SURFACE, "ReferenceSurface")
    }
}

/// `IfcFixedReferenceSweptAreaSolid`: a sweep whose section orientation is
/// pinned to a constant direction.
///
/// Unlike a Frenet sweep, the profile's frame is derived from a fixed
/// direction rather than the curve's own normal. That is exactly what keeps a
/// road cross section upright over a crest; substituting a Frenet frame rolls
/// the section with the curve's torsion.
#[derive(Debug, Clone, Copy)]
pub struct FixedReferenceSweptAreaSolid<'m> {
    slots: Slots<'m>,
}

impl<'m> FixedReferenceSweptAreaSolid<'m> {
    /// Wrap an entity assumed to be an `IfcFixedReferenceSweptAreaSolid`.
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

    /// The `IfcCurve` reference the profile follows.
    pub fn directrix(&self) -> GeometryResult<EntityId> {
        self.slots.req_ref(directrix_slot::DIRECTRIX, "Directrix")
    }

    /// The parameter at which the sweep starts, if trimmed.
    pub fn start_param(&self) -> Option<f64> {
        self.slots.opt_f64(directrix_slot::START_PARAM)
    }

    /// The parameter at which the sweep ends, if trimmed.
    pub fn end_param(&self) -> Option<f64> {
        self.slots.opt_f64(directrix_slot::END_PARAM)
    }

    /// The `IfcDirection` reference that fixes the section's orientation.
    pub fn fixed_reference(&self) -> GeometryResult<EntityId> {
        self.slots
            .req_ref(directrix_slot::FIXED_REFERENCE, "FixedReference")
    }
}

/// `IfcSweptDiskSolid`: a circular disk swept along a curve.
///
/// This is how pipes, ducts, cable trays and reinforcement bars are modelled.
///
/// # Radii
///
/// `InnerRadius` is optional and turns the solid into a tube. It must be
/// strictly smaller than `Radius`; a file that violates that describes a solid
/// with non-positive wall thickness, which is why [`Self::checked_radii`]
/// exists.
///
/// # Slot warning
///
/// This entity subtypes `IfcSolidModel` **directly**, so it has no `SweptArea`
/// and no `Position`. The directrix is absolute slot 0, not slot 2 -- assuming
/// the `IfcSweptAreaSolid` layout here shifts every attribute by two.
#[derive(Debug, Clone, Copy)]
pub struct SweptDiskSolid<'m> {
    slots: Slots<'m>,
}

impl<'m> SweptDiskSolid<'m> {
    /// Wrap an entity assumed to be an `IfcSweptDiskSolid`.
    pub fn new(id: EntityId, entity: &'m Entity) -> Self {
        Self {
            slots: Slots::new(id, entity),
        }
    }

    /// The entity id.
    pub fn id(&self) -> EntityId {
        self.slots.id()
    }

    /// The `IfcCurve` reference the disk is swept along.
    pub fn directrix(&self) -> GeometryResult<EntityId> {
        self.slots.req_ref(disk_slot::DIRECTRIX, "Directrix")
    }

    /// The outer radius, in file length units.
    pub fn radius(&self) -> GeometryResult<f64> {
        self.slots.req_f64(disk_slot::RADIUS, "Radius")
    }

    /// The inner radius, when the solid is a tube rather than a rod.
    pub fn inner_radius(&self) -> Option<f64> {
        self.slots.opt_f64(disk_slot::INNER_RADIUS)
    }

    /// The parameter at which the sweep starts, if trimmed.
    pub fn start_param(&self) -> Option<f64> {
        self.slots.opt_f64(disk_slot::START_PARAM)
    }

    /// The parameter at which the sweep ends, if trimmed.
    pub fn end_param(&self) -> Option<f64> {
        self.slots.opt_f64(disk_slot::END_PARAM)
    }

    /// The radii, rejecting a tube whose bore is at least its outside.
    pub fn checked_radii(&self) -> GeometryResult<(f64, Option<f64>)> {
        let radius = self.radius()?;
        if radius <= 0.0 {
            return Err(self
                .slots
                .degenerate(format!("Radius must be positive, found {radius}")));
        }
        let inner = self.inner_radius();
        if let Some(inner) = inner {
            if inner >= radius {
                return Err(self.slots.degenerate(format!(
                    "InnerRadius {inner} must be smaller than Radius {radius}"
                )));
            }
        }
        Ok((radius, inner))
    }
}

/// `IfcSweptDiskSolidPolygonal`: a swept disk whose directrix is a polyline
/// with optionally filleted corners.
///
/// `FilletRadius` absent means sharp corners. When present the schema requires
/// it to be at least the disk radius, so that the fillet does not pinch the
/// tube shut at a corner.
#[derive(Debug, Clone, Copy)]
pub struct SweptDiskSolidPolygonal<'m> {
    slots: Slots<'m>,
}

impl<'m> SweptDiskSolidPolygonal<'m> {
    /// Wrap an entity assumed to be an `IfcSweptDiskSolidPolygonal`.
    pub fn new(id: EntityId, entity: &'m Entity) -> Self {
        Self {
            slots: Slots::new(id, entity),
        }
    }

    /// The entity id.
    pub fn id(&self) -> EntityId {
        self.slots.id()
    }

    /// The inherited `IfcSweptDiskSolid` attributes.
    pub fn base(&self) -> SweptDiskSolid<'m> {
        SweptDiskSolid { slots: self.slots }
    }

    /// The corner fillet radius, when corners are rounded.
    pub fn fillet_radius(&self) -> Option<f64> {
        self.slots.opt_f64(disk_slot::FILLET_RADIUS)
    }
}

/// `IfcSectionedSpine`: cross sections positioned along a composite curve.
///
/// # Not a swept area solid
///
/// Despite the family resemblance it subtypes `IfcGeometricRepresentationItem`
/// directly, not `IfcSolidModel`. Slot 0 is `SpineCurve`, and there is no
/// inherited `SweptArea` or `Position`.
///
/// # The pairing invariant
///
/// `CrossSections` and `CrossSectionPositions` are parallel lists that the
/// schema requires to be the same length. Exporters do get this wrong, and a
/// plain zip silently truncates to the shorter one, producing a solid missing
/// its tail. [`Self::checked_sections`] reports the mismatch instead.
#[derive(Debug, Clone, Copy)]
pub struct SectionedSpine<'m> {
    slots: Slots<'m>,
}

impl<'m> SectionedSpine<'m> {
    /// Wrap an entity assumed to be an `IfcSectionedSpine`.
    pub fn new(id: EntityId, entity: &'m Entity) -> Self {
        Self {
            slots: Slots::new(id, entity),
        }
    }

    /// The entity id.
    pub fn id(&self) -> EntityId {
        self.slots.id()
    }

    /// The `IfcCompositeCurve` reference forming the spine.
    pub fn spine_curve(&self) -> GeometryResult<EntityId> {
        self.slots.req_ref(spine_slot::SPINE_CURVE, "SpineCurve")
    }

    /// The `IfcProfileDef` references, in spine order.
    pub fn cross_sections(&self) -> GeometryResult<Vec<EntityId>> {
        self.slots
            .req_ref_list(spine_slot::CROSS_SECTIONS, "CrossSections")
    }

    /// The `IfcAxis2Placement3D` references locating each cross section.
    pub fn cross_section_positions(&self) -> GeometryResult<Vec<EntityId>> {
        self.slots
            .req_ref_list(spine_slot::CROSS_SECTION_POSITIONS, "CrossSectionPositions")
    }

    /// Sections paired with their positions, rejecting a length mismatch.
    pub fn checked_sections(&self) -> GeometryResult<Vec<(EntityId, EntityId)>> {
        let sections = self.cross_sections()?;
        let positions = self.cross_section_positions()?;
        if sections.len() != positions.len() {
            return Err(self.slots.degenerate(format!(
                "CrossSections has {} entries but CrossSectionPositions has {}",
                sections.len(),
                positions.len()
            )));
        }
        Ok(sections.into_iter().zip(positions).collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::solid::testkit::{entity, list, n, r};
    use ifc_model::Value;

    #[test]
    fn surface_curve_sweep_exposes_directrix_surface_and_optional_trim() {
        let e = entity(
            "IFCSURFACECURVESWEPTAREASOLID",
            vec![r(10), r(20), r(30), n(0.0), n(1.0), r(60)],
        );
        let view = SurfaceCurveSweptAreaSolid::new(EntityId(1), &e);
        assert_eq!(view.base().swept_area().unwrap(), EntityId(10));
        assert_eq!(view.directrix().unwrap(), EntityId(30));
        assert_eq!(view.start_param(), Some(0.0));
        assert_eq!(view.end_param(), Some(1.0));
        assert_eq!(view.reference_surface().unwrap(), EntityId(60));
    }

    #[test]
    fn untrimmed_directrix_reports_absent_parameters_rather_than_zero() {
        let e = entity(
            "IFCSURFACECURVESWEPTAREASOLID",
            vec![r(10), r(20), r(30), Value::Null, Value::Null, r(60)],
        );
        let view = SurfaceCurveSweptAreaSolid::new(EntityId(1), &e);
        assert_eq!(view.start_param(), None);
        assert_eq!(view.end_param(), None);
    }

    /// Slot 5 is a surface on one sweep and a direction on the other;
    /// conflating them silently swaps a surface reference for a direction.
    #[test]
    fn fixed_reference_sweep_reads_a_direction_where_surface_sweep_reads_a_surface() {
        let attrs = vec![r(10), r(20), r(30), Value::Null, Value::Null, r(70)];
        let fixed = entity("IFCFIXEDREFERENCESWEPTAREASOLID", attrs.clone());
        let surface = entity("IFCSURFACECURVESWEPTAREASOLID", attrs);

        assert_eq!(
            FixedReferenceSweptAreaSolid::new(EntityId(1), &fixed)
                .fixed_reference()
                .unwrap(),
            EntityId(70)
        );
        assert_eq!(
            SurfaceCurveSweptAreaSolid::new(EntityId(1), &surface)
                .reference_surface()
                .unwrap(),
            EntityId(70)
        );
        assert_eq!(
            FixedReferenceSweptAreaSolid::new(EntityId(1), &fixed)
                .directrix()
                .unwrap(),
            EntityId(30)
        );
    }

    /// The disk solid subtypes IfcSolidModel directly: Directrix is slot 0,
    /// not slot 2 as it is on the IfcSweptAreaSolid branch.
    #[test]
    fn swept_disk_directrix_is_slot_zero_with_no_inherited_profile() {
        let e = entity(
            "IFCSWEPTDISKSOLID",
            vec![r(10), n(0.1), n(0.08), n(0.0), n(1.0)],
        );
        let view = SweptDiskSolid::new(EntityId(1), &e);
        assert_eq!(view.directrix().unwrap(), EntityId(10));
        assert_eq!(view.radius().unwrap(), 0.1);
        assert_eq!(view.inner_radius(), Some(0.08));
        assert_eq!(view.start_param(), Some(0.0));
        assert_eq!(view.end_param(), Some(1.0));
    }

    #[test]
    fn inner_radius_not_smaller_than_outer_is_degenerate() {
        let bad = entity("IFCSWEPTDISKSOLID", vec![r(10), n(0.1), n(0.1)]);
        let err = SweptDiskSolid::new(EntityId(5), &bad)
            .checked_radii()
            .unwrap_err();
        assert_eq!(err.entity(), Some(EntityId(5)));

        let tube = entity("IFCSWEPTDISKSOLID", vec![r(10), n(0.1), n(0.09)]);
        assert_eq!(
            SweptDiskSolid::new(EntityId(5), &tube)
                .checked_radii()
                .unwrap(),
            (0.1, Some(0.09))
        );

        let rod = entity("IFCSWEPTDISKSOLID", vec![r(10), n(0.1)]);
        assert_eq!(
            SweptDiskSolid::new(EntityId(5), &rod)
                .checked_radii()
                .unwrap(),
            (0.1, None)
        );
    }

    #[test]
    fn polygonal_disk_fillet_radius_is_optional() {
        let with_fillet = entity(
            "IFCSWEPTDISKSOLIDPOLYGONAL",
            vec![
                r(10),
                n(0.1),
                Value::Null,
                Value::Null,
                Value::Null,
                n(0.15),
            ],
        );
        let view = SweptDiskSolidPolygonal::new(EntityId(1), &with_fillet);
        assert_eq!(view.fillet_radius(), Some(0.15));
        assert_eq!(view.base().radius().unwrap(), 0.1);

        let sharp = entity("IFCSWEPTDISKSOLIDPOLYGONAL", vec![r(10), n(0.1)]);
        assert_eq!(
            SweptDiskSolidPolygonal::new(EntityId(1), &sharp).fillet_radius(),
            None
        );
    }

    #[test]
    fn sectioned_spine_pairs_each_section_with_its_own_placement() {
        let e = entity(
            "IFCSECTIONEDSPINE",
            vec![
                r(1),
                list(vec![r(10), r(11), r(12)]),
                list(vec![r(20), r(21), r(22)]),
            ],
        );
        let view = SectionedSpine::new(EntityId(9), &e);
        assert_eq!(view.spine_curve().unwrap(), EntityId(1));
        assert_eq!(
            view.checked_sections().unwrap(),
            vec![
                (EntityId(10), EntityId(20)),
                (EntityId(11), EntityId(21)),
                (EntityId(12), EntityId(22)),
            ]
        );
    }

    /// A plain zip would silently drop the unpaired tail; the file is
    /// malformed and must say so.
    #[test]
    fn mismatched_spine_list_lengths_are_reported_not_truncated() {
        let e = entity(
            "IFCSECTIONEDSPINE",
            vec![
                r(1),
                list(vec![r(10), r(11), r(12)]),
                list(vec![r(20), r(21)]),
            ],
        );
        let view = SectionedSpine::new(EntityId(9), &e);
        let err = view.checked_sections().unwrap_err();
        assert_eq!(err.entity(), Some(EntityId(9)));
        assert!(err.to_string().contains('3'));
    }
}
