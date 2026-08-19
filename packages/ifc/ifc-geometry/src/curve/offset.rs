//! Curves defined relative to another curve or to a surface.
//!
//! Covers `IfcOffsetCurve2D`, `IfcOffsetCurve3D`, `IfcPcurve` and
//! `IfcSurfaceCurve` (with its `IfcIntersectionCurve` and `IfcSeamCurve`
//! subtypes).
//!
//! # Why offsets are not just "the same curve, moved"
//!
//! An offset curve is not a translation. Every point moves perpendicular to
//! the basis curve's local tangent, so where the basis curve bends more
//! tightly than the offset distance the result self-intersects or vanishes.
//! That is why both offset entities carry `SelfIntersect` and why this module
//! surfaces it rather than dropping it: it is the file's only warning that
//! offsetting will not produce a simple curve.
//!
//! # The 2D/3D asymmetry
//!
//! `IfcOffsetCurve2D` has no reference direction: in the plane, "perpendicular
//! to the tangent" is unambiguous up to sign, and the sign comes from the
//! `Distance` value. `IfcOffsetCurve3D` needs `RefDirection` because in space
//! there is a whole circle of perpendiculars, and the offset direction is
//! `RefDirection` cross tangent. A consumer that treats the two entities
//! identically will place a 3D offset anywhere on that circle.
//!
//! # `IfcSurfaceCurve` carries the same curve twice
//!
//! `Curve3D` and `AssociatedGeometry` (one or two `IfcPcurve`s) describe the
//! *same* curve in different spaces, and they will not agree exactly.
//! `MasterRepresentation` says which one to believe. It matters most on a
//! seam curve, where the two p-curves are the two sides of a periodic
//! surface's seam and picking the wrong one puts an edge on the far side of a
//! cylinder.

use crate::error::GeometryResult;
use crate::slots::Slots;
use ifc_model::{Entity, EntityId};

/// `IfcOffsetCurve2D` attribute slots, from IFC4 ADD2 TC1.
mod offset_2d_slot {
    /// `BasisCurve`: the `IfcCurve` being offset.
    pub const BASIS_CURVE: usize = 0;
    /// `Distance`: `IfcLengthMeasure`; sign picks the side.
    pub const DISTANCE: usize = 1;
    /// `SelfIntersect`: `IfcLogical`.
    pub const SELF_INTERSECT: usize = 2;
}

/// `IfcOffsetCurve3D` attribute slots, from IFC4 ADD2 TC1.
///
/// Identical to the 2D form for slots 0-2, plus `RefDirection` at 3.
mod offset_3d_slot {
    /// `BasisCurve`: the `IfcCurve` being offset.
    pub const BASIS_CURVE: usize = 0;
    /// `Distance`: `IfcLengthMeasure`.
    pub const DISTANCE: usize = 1;
    /// `SelfIntersect`: `IfcLogical`.
    pub const SELF_INTERSECT: usize = 2;
    /// `RefDirection`: `IfcDirection` fixing which perpendicular is used.
    pub const REF_DIRECTION: usize = 3;
}

/// `IfcPcurve` attribute slots, from IFC4 ADD2 TC1.
mod pcurve_slot {
    /// `BasisSurface`: the `IfcSurface` the curve lives on.
    pub const BASIS_SURFACE: usize = 0;
    /// `ReferenceCurve`: a 2D curve in the surface's parameter space.
    pub const REFERENCE_CURVE: usize = 1;
}

/// `IfcSurfaceCurve` attribute slots, from IFC4 ADD2 TC1.
///
/// `IfcIntersectionCurve` and `IfcSeamCurve` add no explicit attributes, so
/// these indices serve all three.
mod surface_curve_slot {
    /// `Curve3D`: the curve in model space.
    pub const CURVE_3D: usize = 0;
    /// `AssociatedGeometry`: `LIST [1:2] OF IfcPcurve`.
    pub const ASSOCIATED_GEOMETRY: usize = 1;
    /// `MasterRepresentation`: `IfcPreferredSurfaceCurveRepresentation`.
    pub const MASTER_REPRESENTATION: usize = 2;
}

/// `IfcPreferredSurfaceCurveRepresentation`: which copy of the curve wins.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PreferredSurfaceCurveRepresentation {
    /// Believe `Curve3D`, the model-space curve.
    Curve3D,
    /// Believe the first `IfcPcurve` in `AssociatedGeometry`.
    PCurveS1,
    /// Believe the second `IfcPcurve` in `AssociatedGeometry`.
    ///
    /// Only meaningful when two are supplied; a file naming this with one
    /// p-curve is inconsistent.
    PCurveS2,
}

impl PreferredSurfaceCurveRepresentation {
    /// Parse the enumeration token, `None` if unrecognised.
    pub fn from_token(token: &str) -> Option<Self> {
        match token.to_ascii_uppercase().as_str() {
            "CURVE3D" => Some(Self::Curve3D),
            "PCURVE_S1" => Some(Self::PCurveS1),
            "PCURVE_S2" => Some(Self::PCurveS2),
            _ => None,
        }
    }

    /// Index into `AssociatedGeometry`, or `None` when 3D wins.
    ///
    /// Saves every caller from re-deriving that `PCURVE_S1` is element 0.
    pub fn pcurve_index(&self) -> Option<usize> {
        match self {
            Self::Curve3D => None,
            Self::PCurveS1 => Some(0),
            Self::PCurveS2 => Some(1),
        }
    }
}

/// A borrowed view of an `IfcOffsetCurve2D`.
#[derive(Debug, Clone, Copy)]
pub struct OffsetCurve2D<'m> {
    slots: Slots<'m>,
}

impl<'m> OffsetCurve2D<'m> {
    /// Wrap an entity known to be an `IfcOffsetCurve2D`.
    pub fn new(id: EntityId, entity: &'m Entity) -> Self {
        Self {
            slots: Slots::new(id, entity),
        }
    }

    /// The entity id.
    pub fn id(&self) -> EntityId {
        self.slots.id()
    }

    /// The curve being offset.
    pub fn basis_curve_ref(&self) -> GeometryResult<EntityId> {
        self.slots
            .req_ref(offset_2d_slot::BASIS_CURVE, "BasisCurve")
    }

    /// The offset distance; the sign selects which side.
    ///
    /// Zero is legal and means the curve coincides with its basis, which some
    /// exporters emit as a placeholder. Not treated as degenerate because the
    /// resulting geometry is perfectly well defined.
    pub fn distance(&self) -> GeometryResult<f64> {
        self.slots.req_f64(offset_2d_slot::DISTANCE, "Distance")
    }

    /// The asserted `SelfIntersect` flag; `None` for `.U.` or absent.
    pub fn self_intersect(&self) -> Option<bool> {
        self.slots.opt_bool(offset_2d_slot::SELF_INTERSECT)
    }
}

/// A borrowed view of an `IfcOffsetCurve3D`.
#[derive(Debug, Clone, Copy)]
pub struct OffsetCurve3D<'m> {
    slots: Slots<'m>,
}

impl<'m> OffsetCurve3D<'m> {
    /// Wrap an entity known to be an `IfcOffsetCurve3D`.
    pub fn new(id: EntityId, entity: &'m Entity) -> Self {
        Self {
            slots: Slots::new(id, entity),
        }
    }

    /// The entity id.
    pub fn id(&self) -> EntityId {
        self.slots.id()
    }

    /// The curve being offset.
    pub fn basis_curve_ref(&self) -> GeometryResult<EntityId> {
        self.slots
            .req_ref(offset_3d_slot::BASIS_CURVE, "BasisCurve")
    }

    /// The offset distance; the sign selects which side.
    pub fn distance(&self) -> GeometryResult<f64> {
        self.slots.req_f64(offset_3d_slot::DISTANCE, "Distance")
    }

    /// The asserted `SelfIntersect` flag; `None` for `.U.` or absent.
    pub fn self_intersect(&self) -> Option<bool> {
        self.slots.opt_bool(offset_3d_slot::SELF_INTERSECT)
    }

    /// The `IfcDirection` fixing which perpendicular the offset uses.
    ///
    /// Required by the schema and genuinely required by the geometry: without
    /// it the offset direction is only known up to rotation about the tangent.
    // TODO: `resource::direction` will provide a typed direction view.
    pub fn ref_direction_ref(&self) -> GeometryResult<EntityId> {
        self.slots
            .req_ref(offset_3d_slot::REF_DIRECTION, "RefDirection")
    }
}

/// A borrowed view of an `IfcPcurve`.
///
/// A curve defined in a surface's *parameter* space. `ReferenceCurve` is a 2D
/// curve whose coordinates are `(u, v)` values, not lengths, so applying a
/// length unit scale to them is wrong.
#[derive(Debug, Clone, Copy)]
pub struct PCurve<'m> {
    slots: Slots<'m>,
}

impl<'m> PCurve<'m> {
    /// Wrap an entity known to be an `IfcPcurve`.
    pub fn new(id: EntityId, entity: &'m Entity) -> Self {
        Self {
            slots: Slots::new(id, entity),
        }
    }

    /// The entity id.
    pub fn id(&self) -> EntityId {
        self.slots.id()
    }

    /// The surface whose parameter space the curve lives in.
    pub fn basis_surface_ref(&self) -> GeometryResult<EntityId> {
        self.slots
            .req_ref(pcurve_slot::BASIS_SURFACE, "BasisSurface")
    }

    /// The 2D curve in `(u, v)` parameter space.
    ///
    /// Its coordinates are surface parameters. For an `IfcPlane` those happen
    /// to be lengths; for an `IfcCylindricalSurface` the first is an angle.
    /// Unit conversion therefore depends on the basis surface, not on the
    /// curve.
    pub fn reference_curve_ref(&self) -> GeometryResult<EntityId> {
        self.slots
            .req_ref(pcurve_slot::REFERENCE_CURVE, "ReferenceCurve")
    }
}

/// A borrowed view of an `IfcSurfaceCurve` and its subtypes.
#[derive(Debug, Clone, Copy)]
pub struct SurfaceCurve<'m> {
    slots: Slots<'m>,
}

/// Which `IfcSurfaceCurve` subtype an entity is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SurfaceCurveKind {
    /// `IfcSurfaceCurve`: a curve lying on one or two surfaces.
    Plain,
    /// `IfcIntersectionCurve`: where two surfaces meet.
    ///
    /// Requires exactly two associated p-curves, one per surface.
    Intersection,
    /// `IfcSeamCurve`: the closing seam of a periodic surface.
    ///
    /// Its two p-curves lie on the *same* surface at the two ends of the
    /// periodic parameter range, which is why they look like duplicates.
    Seam,
}

impl<'m> SurfaceCurve<'m> {
    /// Wrap an entity known to be an `IfcSurfaceCurve` subtype.
    pub fn new(id: EntityId, entity: &'m Entity) -> Self {
        Self {
            slots: Slots::new(id, entity),
        }
    }

    /// The entity id.
    pub fn id(&self) -> EntityId {
        self.slots.id()
    }

    /// Which subtype this is, or `None` if the type name is not one of them.
    pub fn kind(&self) -> Option<SurfaceCurveKind> {
        match self.slots.entity().type_name.to_ascii_uppercase().as_str() {
            "IFCSURFACECURVE" => Some(SurfaceCurveKind::Plain),
            "IFCINTERSECTIONCURVE" => Some(SurfaceCurveKind::Intersection),
            "IFCSEAMCURVE" => Some(SurfaceCurveKind::Seam),
            _ => None,
        }
    }

    /// The model-space curve.
    pub fn curve_3d_ref(&self) -> GeometryResult<EntityId> {
        self.slots.req_ref(surface_curve_slot::CURVE_3D, "Curve3D")
    }

    /// The associated `IfcPcurve` references, one or two.
    ///
    /// Cardinality is checked because `IfcIntersectionCurve` and
    /// `IfcSeamCurve` both require exactly two, and a single-element list on
    /// either means the second surface was lost.
    pub fn associated_pcurve_refs(&self) -> GeometryResult<Vec<EntityId>> {
        let pcurves = self.slots.req_ref_list(
            surface_curve_slot::ASSOCIATED_GEOMETRY,
            "AssociatedGeometry",
        )?;
        if pcurves.is_empty() || pcurves.len() > 2 {
            return Err(self.slots.degenerate(format!(
                "AssociatedGeometry must hold 1 or 2 IfcPcurves, found {}",
                pcurves.len()
            )));
        }
        if matches!(
            self.kind(),
            Some(SurfaceCurveKind::Intersection) | Some(SurfaceCurveKind::Seam)
        ) && pcurves.len() != 2
        {
            return Err(self.slots.degenerate(format!(
                "{} requires exactly 2 associated IfcPcurves, found {}",
                self.slots.type_name(),
                pcurves.len()
            )));
        }
        Ok(pcurves)
    }

    /// Which representation is authoritative, defaulting to `Curve3D`.
    ///
    /// The 3D curve is the safe default: it exists on every surface curve,
    /// whereas a p-curve index may point past the end of a short list.
    pub fn master_representation(&self) -> PreferredSurfaceCurveRepresentation {
        self.slots
            .opt_enum(surface_curve_slot::MASTER_REPRESENTATION)
            .and_then(PreferredSurfaceCurveRepresentation::from_token)
            .unwrap_or(PreferredSurfaceCurveRepresentation::Curve3D)
    }

    /// The reference the `MasterRepresentation` actually names.
    ///
    /// Resolves the enumeration against the real list length, so a file
    /// naming `PCURVE_S2` with only one p-curve fails here rather than
    /// indexing out of bounds in a kernel.
    pub fn master_curve_ref(&self) -> GeometryResult<EntityId> {
        match self.master_representation().pcurve_index() {
            None => self.curve_3d_ref(),
            Some(index) => {
                let pcurves = self.associated_pcurve_refs()?;
                pcurves.get(index).copied().ok_or_else(|| {
                    self.slots.degenerate(format!(
                        "MasterRepresentation names p-curve {} but AssociatedGeometry holds {}",
                        index + 1,
                        pcurves.len()
                    ))
                })
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ifc_model::Value;

    fn offset_2d(distance: f64) -> Entity {
        Entity::new(
            "IFCOFFSETCURVE2D",
            vec![
                Value::Ref(EntityId(30)),
                Value::Real(distance),
                Value::Bool(false),
            ],
        )
    }

    fn offset_3d(distance: f64) -> Entity {
        Entity::new(
            "IFCOFFSETCURVE3D",
            vec![
                Value::Ref(EntityId(30)),
                Value::Real(distance),
                Value::Bool(false),
                Value::Ref(EntityId(31)),
            ],
        )
    }

    fn surface_curve(type_name: &str, pcurves: &[u64], preference: &str) -> Entity {
        Entity::new(
            type_name,
            vec![
                Value::Ref(EntityId(40)),
                Value::List(pcurves.iter().map(|i| Value::Ref(EntityId(*i))).collect()),
                Value::Enum(preference.into()),
            ],
        )
    }

    /// The sign of Distance is the only thing selecting the offset side, so it
    /// must survive unchanged.
    #[test]
    fn offset_distance_keeps_its_sign() {
        let e = offset_2d(-0.5);
        assert_eq!(
            OffsetCurve2D::new(EntityId(1), &e).distance().unwrap(),
            -0.5
        );
    }

    /// A zero offset is well-defined geometry, not corruption.
    #[test]
    fn a_zero_offset_distance_is_accepted() {
        let e = offset_2d(0.0);
        assert_eq!(OffsetCurve2D::new(EntityId(1), &e).distance().unwrap(), 0.0);
    }

    /// The 3D form needs RefDirection to pick one of infinitely many
    /// perpendiculars; the 2D form has no such slot.
    #[test]
    fn only_the_3d_offset_carries_a_reference_direction() {
        let e = offset_3d(1.0);
        let view = OffsetCurve3D::new(EntityId(1), &e);
        assert_eq!(view.basis_curve_ref().unwrap(), EntityId(30));
        assert_eq!(view.ref_direction_ref().unwrap(), EntityId(31));
        assert_eq!(view.self_intersect(), Some(false));
    }

    #[test]
    fn a_3d_offset_missing_its_reference_direction_is_reported_by_name() {
        let e = Entity::new(
            "IFCOFFSETCURVE3D",
            vec![
                Value::Ref(EntityId(30)),
                Value::Real(1.0),
                Value::Bool(false),
            ],
        );
        let err = OffsetCurve3D::new(EntityId(1), &e)
            .ref_direction_ref()
            .unwrap_err();
        assert!(err.to_string().contains("RefDirection"), "got: {err}");
    }

    #[test]
    fn pcurve_names_its_surface_and_its_parameter_space_curve() {
        let e = Entity::new(
            "IFCPCURVE",
            vec![Value::Ref(EntityId(50)), Value::Ref(EntityId(51))],
        );
        let view = PCurve::new(EntityId(1), &e);
        assert_eq!(view.basis_surface_ref().unwrap(), EntityId(50));
        assert_eq!(view.reference_curve_ref().unwrap(), EntityId(51));
    }

    /// PCURVE_S1 is element 0 and PCURVE_S2 element 1; getting that backwards
    /// puts an edge on the wrong surface.
    #[test]
    fn master_representation_resolves_to_the_right_geometry() {
        let by_3d = surface_curve("IFCSURFACECURVE", &[60, 61], "CURVE3D");
        assert_eq!(
            SurfaceCurve::new(EntityId(1), &by_3d)
                .master_curve_ref()
                .unwrap(),
            EntityId(40)
        );

        let by_s1 = surface_curve("IFCSURFACECURVE", &[60, 61], "PCURVE_S1");
        assert_eq!(
            SurfaceCurve::new(EntityId(1), &by_s1)
                .master_curve_ref()
                .unwrap(),
            EntityId(60)
        );

        let by_s2 = surface_curve("IFCSURFACECURVE", &[60, 61], "PCURVE_S2");
        assert_eq!(
            SurfaceCurve::new(EntityId(1), &by_s2)
                .master_curve_ref()
                .unwrap(),
            EntityId(61)
        );
    }

    #[test]
    fn naming_a_pcurve_the_file_does_not_have_fails_instead_of_indexing_out_of_bounds() {
        let e = surface_curve("IFCSURFACECURVE", &[60], "PCURVE_S2");
        let err = SurfaceCurve::new(EntityId(3), &e)
            .master_curve_ref()
            .unwrap_err();
        assert!(err.to_string().contains("#3"), "got: {err}");
    }

    #[test]
    fn an_absent_master_representation_defaults_to_the_3d_curve() {
        let e = Entity::new(
            "IFCSURFACECURVE",
            vec![
                Value::Ref(EntityId(40)),
                Value::List(vec![Value::Ref(EntityId(60))]),
            ],
        );
        assert_eq!(
            SurfaceCurve::new(EntityId(1), &e).master_representation(),
            PreferredSurfaceCurveRepresentation::Curve3D
        );
    }

    /// An intersection curve with one p-curve has lost a surface, and a seam
    /// curve with one has lost the far side of the seam.
    #[test]
    fn intersection_and_seam_curves_require_exactly_two_pcurves() {
        for type_name in ["IFCINTERSECTIONCURVE", "IFCSEAMCURVE"] {
            let e = surface_curve(type_name, &[60], "CURVE3D");
            let err = SurfaceCurve::new(EntityId(1), &e)
                .associated_pcurve_refs()
                .unwrap_err();
            assert!(err.to_string().contains("exactly 2"), "{type_name}: {err}");
        }
        // The plain supertype is happy with one.
        let plain = surface_curve("IFCSURFACECURVE", &[60], "CURVE3D");
        assert_eq!(
            SurfaceCurve::new(EntityId(1), &plain)
                .associated_pcurve_refs()
                .unwrap()
                .len(),
            1
        );
    }

    #[test]
    fn surface_curve_subtypes_are_classified_from_the_type_name() {
        for (type_name, expected) in [
            ("IFCSURFACECURVE", SurfaceCurveKind::Plain),
            ("IFCINTERSECTIONCURVE", SurfaceCurveKind::Intersection),
            ("IFCSEAMCURVE", SurfaceCurveKind::Seam),
        ] {
            let e = surface_curve(type_name, &[60, 61], "CURVE3D");
            assert_eq!(SurfaceCurve::new(EntityId(1), &e).kind(), Some(expected));
        }
        let other = surface_curve("IFCPOLYLINE", &[60], "CURVE3D");
        assert_eq!(SurfaceCurve::new(EntityId(1), &other).kind(), None);
    }

    #[test]
    fn preferred_representation_tokens_map_to_list_positions() {
        assert_eq!(
            PreferredSurfaceCurveRepresentation::Curve3D.pcurve_index(),
            None
        );
        assert_eq!(
            PreferredSurfaceCurveRepresentation::PCurveS1.pcurve_index(),
            Some(0)
        );
        assert_eq!(
            PreferredSurfaceCurveRepresentation::PCurveS2.pcurve_index(),
            Some(1)
        );
        assert_eq!(
            PreferredSurfaceCurveRepresentation::from_token("NOPE"),
            None
        );
    }
}
