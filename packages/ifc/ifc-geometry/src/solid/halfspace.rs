//! Half spaces: **infinite** solids, valid only as boolean operands.
//!
//! # Read this before using anything in this module
//!
//! An `IfcHalfSpaceSolid` is everything on one side of a surface. It has
//! **infinite volume**. It cannot be tessellated, cannot be given a bounding
//! box, and has no meaningful surface area. Any pipeline that treats it as an
//! ordinary solid either hangs, allocates until it dies, or emits a mesh the
//! size of the coordinate space.
//!
//! It is legal in exactly one place: as an operand of an `IfcBooleanResult`
//! (in practice as the `SecondOperand` of an `IfcBooleanClippingResult`, where
//! the schema requires it). The intended reading is "cut this solid with a
//! plane", and the infinite half space is how IFC spells the cutting tool.
//!
//! # `BaseSurface` and `AgreementFlag`
//!
//! `BaseSurface` is an `IfcSurface`, and in real files essentially always an
//! `IfcPlane`. It divides space in two. `AgreementFlag` picks which of the two
//! halves is the material:
//!
//! - `.T.` -- the solid is on the side the plane's **normal points away from**
//!   (that is, the side of decreasing surface parameter / below the plane in
//!   its own coordinate system).
//! - `.F.` -- the solid is on the other side.
//!
//! Getting this backwards does not fail: it cuts away the part that should have
//! been kept, producing a wall with the wrong end missing. There is no
//! geometric check that catches it, so the flag must be transcribed exactly.
//!
//! # The two bounded subtypes
//!
//! Both bound the half space, but in different senses, and the difference
//! matters:
//!
//! - [`BoxedHalfSpace`] carries an `Enclosure` bounding box. It is a
//!   **declaration of the region of interest**, letting a consumer clip the
//!   infinite solid to something finite before the boolean.
//! - [`PolygonalBoundedHalfSpace`] carries a 2D `PolygonalBoundary` in the XY
//!   plane of its own `Position` and is bounded by the **prism** obtained by
//!   extruding that boundary along +Z of that same `Position`. The subtraction
//!   body is the intersection of the half space with that prism.

use crate::error::GeometryResult;
use crate::slots::Slots;
use ifc_model::{Entity, EntityId};

/// Half space attribute slots.
///
/// EXPRESS (IFC4 ADD2 TC1): `IfcHalfSpaceSolid` subtypes
/// `IfcGeometricRepresentationItem`, which declares no explicit attributes, so
/// `BaseSurface` and `AgreementFlag` are absolute slots 0 and 1 -- and the
/// subtypes' own attributes therefore start at slot 2.
mod slot {
    /// `BaseSurface : IfcSurface`, on `IfcHalfSpaceSolid`.
    pub const BASE_SURFACE: usize = 0;
    /// `AgreementFlag : IfcBoolean`, on `IfcHalfSpaceSolid`.
    pub const AGREEMENT_FLAG: usize = 1;
    /// `Enclosure : IfcBoundingBox` on `IfcBoxedHalfSpace`, absolute slot 2.
    pub const ENCLOSURE: usize = 2;
    /// `Position : IfcAxis2Placement3D` on `IfcPolygonalBoundedHalfSpace`.
    pub const POSITION: usize = 2;
    /// `PolygonalBoundary : IfcBoundedCurve` on the polygonal subtype.
    pub const POLYGONAL_BOUNDARY: usize = 3;
}

/// `IfcHalfSpaceSolid`: an **infinite** solid on one side of a surface.
///
/// See the module documentation. This type is not tessellatable and callers
/// must route it through a boolean; [`Self::is_infinite`] exists so that a
/// pipeline can assert the invariant rather than discovering it at render time.
#[derive(Debug, Clone, Copy)]
pub struct HalfSpaceSolid<'m> {
    slots: Slots<'m>,
}

impl<'m> HalfSpaceSolid<'m> {
    /// Wrap an entity assumed to be an `IfcHalfSpaceSolid` or a subtype.
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

    /// The `IfcSurface` reference dividing space. Normally an `IfcPlane`.
    ///
    /// TODO: resolve through the surface module once it exists; this crate
    /// deliberately does not define a competing surface view.
    pub fn base_surface(&self) -> GeometryResult<EntityId> {
        self.slots.req_ref(slot::BASE_SURFACE, "BaseSurface")
    }

    /// Which side of `BaseSurface` is material.
    ///
    /// `true` means the solid lies on the side the surface normal points away
    /// from. Inverting this silently cuts the wrong half away.
    pub fn agreement_flag(&self) -> GeometryResult<bool> {
        self.slots.req_bool(slot::AGREEMENT_FLAG, "AgreementFlag")
    }

    /// Is the solid unbounded in every direction?
    ///
    /// `true` for a plain `IfcHalfSpaceSolid`. `false` for the two bounded
    /// subtypes, whose extra attributes give a finite region to work in.
    ///
    /// Note that even a `false` here does not make the entity meaningful on its
    /// own: it is still only valid as a boolean operand. What changes is that a
    /// consumer can build a finite body for it.
    pub fn is_infinite(&self) -> bool {
        !self.is_bounded()
    }

    /// Does the concrete type carry a bounding attribute?
    pub fn is_bounded(&self) -> bool {
        let name = self.type_name();
        name.eq_ignore_ascii_case("IFCBOXEDHALFSPACE")
            || name.eq_ignore_ascii_case("IFCPOLYGONALBOUNDEDHALFSPACE")
    }

    /// Reject use of this half space anywhere other than a boolean operand.
    ///
    /// Exists so the "cannot be tessellated" rule is a single call rather than
    /// a comment every consumer is expected to have read. The error is
    /// [`crate::GeometryError::Unsupported`], because the file is perfectly
    /// valid -- it is the requested operation that is not.
    pub fn reject_standalone_use(&self) -> crate::GeometryError {
        self.slots
            .unsupported("IfcHalfSpaceSolid is infinite and can only be used as a boolean operand")
    }
}

/// `IfcBoxedHalfSpace`: a half space with a declared bounding box.
///
/// `Enclosure` is an `IfcBoundingBox` that bounds the half space, turning an
/// infinite operand into a finite one a kernel can actually build. The schema
/// additionally forbids `BaseSurface` from being an `IfcCurveBoundedPlane`
/// here, so the base surface really is unbounded and the box is doing all the
/// bounding.
///
/// The box is expressed in the coordinate system of the containing
/// representation, not relative to `BaseSurface`.
#[derive(Debug, Clone, Copy)]
pub struct BoxedHalfSpace<'m> {
    slots: Slots<'m>,
}

impl<'m> BoxedHalfSpace<'m> {
    /// Wrap an entity assumed to be an `IfcBoxedHalfSpace`.
    pub fn new(id: EntityId, entity: &'m Entity) -> Self {
        Self {
            slots: Slots::new(id, entity),
        }
    }

    /// The entity id.
    pub fn id(&self) -> EntityId {
        self.slots.id()
    }

    /// The inherited `IfcHalfSpaceSolid` attributes.
    pub fn base(&self) -> HalfSpaceSolid<'m> {
        HalfSpaceSolid { slots: self.slots }
    }

    /// The `IfcBoundingBox` reference that bounds the half space.
    ///
    /// See [`crate::solid::bbox::BoundingBox`] for reading it.
    pub fn enclosure(&self) -> GeometryResult<EntityId> {
        self.slots.req_ref(slot::ENCLOSURE, "Enclosure")
    }
}

/// `IfcPolygonalBoundedHalfSpace`: a half space clipped by an extruded polygon.
///
/// # How the bounding actually works
///
/// This is the single most misread entity in IFC clipping, so, precisely:
///
/// 1. `Position` is an `IfcAxis2Placement3D` establishing a local coordinate
///    system. It is **independent of `BaseSurface`**; the two need not share an
///    origin or an orientation.
/// 2. `PolygonalBoundary` is a **2D** bounded curve (an `IfcPolyline` or an
///    `IfcCompositeCurve`, per the schema's `BoundaryType` rule) lying in the
///    **XY plane of `Position`**. Its coordinates are 2D, so it is not a curve
///    in world space and cannot be used without applying `Position`.
/// 3. The bounding body is that polygon extruded **along +Z of `Position`**,
///    unbounded in that direction.
/// 4. The final solid is the intersection of that prism with the infinite half
///    space defined by `BaseSurface` and `AgreementFlag`.
///
/// The consequence is that the polygon does not lie on the base surface and
/// should never be projected onto it. Doing so produces a clip of roughly the
/// right shape in roughly the wrong place, which is why bad wall openings look
/// almost correct.
#[derive(Debug, Clone, Copy)]
pub struct PolygonalBoundedHalfSpace<'m> {
    slots: Slots<'m>,
}

impl<'m> PolygonalBoundedHalfSpace<'m> {
    /// Wrap an entity assumed to be an `IfcPolygonalBoundedHalfSpace`.
    pub fn new(id: EntityId, entity: &'m Entity) -> Self {
        Self {
            slots: Slots::new(id, entity),
        }
    }

    /// The entity id.
    pub fn id(&self) -> EntityId {
        self.slots.id()
    }

    /// The inherited `IfcHalfSpaceSolid` attributes.
    pub fn base(&self) -> HalfSpaceSolid<'m> {
        HalfSpaceSolid { slots: self.slots }
    }

    /// The `IfcAxis2Placement3D` whose XY plane holds the boundary.
    ///
    /// Required here, unlike the optional `Position` on `IfcSweptAreaSolid`:
    /// without it the 2D boundary cannot be placed at all.
    pub fn position(&self) -> GeometryResult<EntityId> {
        self.slots.req_ref(slot::POSITION, "Position")
    }

    /// The 2D `IfcBoundedCurve` reference bounding the half space.
    ///
    /// Its coordinates are in the XY plane of [`Self::position`]; the clipping
    /// body is this curve extruded along that placement's +Z.
    ///
    /// TODO: resolve through the curve module once it exists; this crate
    /// deliberately does not define a competing curve view.
    pub fn polygonal_boundary(&self) -> GeometryResult<EntityId> {
        self.slots
            .req_ref(slot::POLYGONAL_BOUNDARY, "PolygonalBoundary")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::solid::testkit::{entity, r};
    use ifc_model::Value;

    #[test]
    fn base_surface_and_agreement_flag_are_slots_zero_and_one() {
        let e = entity("IFCHALFSPACESOLID", vec![r(10), Value::Bool(true)]);
        let view = HalfSpaceSolid::new(EntityId(1), &e);
        assert_eq!(view.base_surface().unwrap(), EntityId(10));
        assert!(view.agreement_flag().unwrap());
    }

    /// The flag has no geometric fallback: reading it wrong keeps the wrong
    /// half, so both states must round-trip exactly.
    #[test]
    fn agreement_flag_preserves_both_states_without_defaulting() {
        for expected in [true, false] {
            let e = entity("IFCHALFSPACESOLID", vec![r(10), Value::Bool(expected)]);
            assert_eq!(
                HalfSpaceSolid::new(EntityId(1), &e)
                    .agreement_flag()
                    .unwrap(),
                expected
            );
        }
    }

    /// A logical unknown is not a false; it must surface as an error rather
    /// than silently selecting a side.
    #[test]
    fn a_logical_unknown_agreement_flag_is_an_error_not_a_false() {
        let e = entity("IFCHALFSPACESOLID", vec![r(10), Value::LogicalUnknown]);
        assert!(HalfSpaceSolid::new(EntityId(1), &e)
            .agreement_flag()
            .is_err());
    }

    /// The defining property of this whole module: an unbounded half space has
    /// no finite body and must never reach a tessellator.
    #[test]
    fn a_plain_half_space_is_infinite_and_refuses_standalone_use() {
        let e = entity("IFCHALFSPACESOLID", vec![r(10), Value::Bool(true)]);
        let view = HalfSpaceSolid::new(EntityId(77), &e);
        assert!(view.is_infinite());
        assert!(!view.is_bounded());

        let err = view.reject_standalone_use();
        assert!(err.is_unsupported());
        assert_eq!(err.entity(), Some(EntityId(77)));
        assert!(err.to_string().contains("boolean operand"));
    }

    #[test]
    fn the_bounded_subtypes_are_not_reported_as_infinite() {
        for name in ["IFCBOXEDHALFSPACE", "IFCPOLYGONALBOUNDEDHALFSPACE"] {
            let e = entity(name, vec![r(10), Value::Bool(true), r(20), r(30)]);
            let view = HalfSpaceSolid::new(EntityId(1), &e);
            assert!(view.is_bounded(), "{name}");
            assert!(!view.is_infinite(), "{name}");
        }
    }

    #[test]
    fn boxed_half_space_enclosure_follows_the_inherited_pair() {
        let e = entity("IFCBOXEDHALFSPACE", vec![r(10), Value::Bool(false), r(20)]);
        let view = BoxedHalfSpace::new(EntityId(1), &e);
        assert_eq!(view.base().base_surface().unwrap(), EntityId(10));
        assert!(!view.base().agreement_flag().unwrap());
        assert_eq!(view.enclosure().unwrap(), EntityId(20));
    }

    /// The verified layout from the schema: BaseSurface, AgreementFlag,
    /// Position, PolygonalBoundary. Reading Position from slot 0 is the
    /// local-index mistake that silently swaps a surface for a placement.
    #[test]
    fn polygonal_bounded_half_space_reads_position_at_two_and_boundary_at_three() {
        let e = entity(
            "IFCPOLYGONALBOUNDEDHALFSPACE",
            vec![r(10), Value::Bool(true), r(20), r(30)],
        );
        let view = PolygonalBoundedHalfSpace::new(EntityId(1), &e);
        assert_eq!(view.base().base_surface().unwrap(), EntityId(10));
        assert!(view.base().agreement_flag().unwrap());
        assert_eq!(view.position().unwrap(), EntityId(20));
        assert_eq!(view.polygonal_boundary().unwrap(), EntityId(30));
    }

    /// The boundary is placed by Position, not by BaseSurface, so the two must
    /// never collapse into one reference.
    #[test]
    fn boundary_placement_is_independent_of_the_base_surface() {
        let e = entity(
            "IFCPOLYGONALBOUNDEDHALFSPACE",
            vec![r(10), Value::Bool(true), r(20), r(30)],
        );
        let view = PolygonalBoundedHalfSpace::new(EntityId(1), &e);
        assert_ne!(
            view.position().unwrap(),
            view.base().base_surface().unwrap(),
            "Position places the 2D boundary; BaseSurface divides space"
        );
    }

    #[test]
    fn a_polygonal_bounded_half_space_without_a_position_is_an_error() {
        let e = entity(
            "IFCPOLYGONALBOUNDEDHALFSPACE",
            vec![r(10), Value::Bool(true)],
        );
        let view = PolygonalBoundedHalfSpace::new(EntityId(6), &e);
        assert!(view.position().is_err());
        assert!(view.polygonal_boundary().is_err());
    }
}
