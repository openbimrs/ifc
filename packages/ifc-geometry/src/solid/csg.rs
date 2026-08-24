//! CSG: analytic primitives and the solid that roots a CSG tree.
//!
//! # The shape of a CSG solid
//!
//! `IfcCsgSolid` holds a single `TreeRootExpression`, an `IfcCsgSelect` which
//! is either an `IfcBooleanResult` or an `IfcCsgPrimitive3D`. The tree lives in
//! the boolean results, so a CSG solid is a one-attribute wrapper, not a tree
//! node itself.
//!
//! # Where a primitive sits
//!
//! Every `IfcCsgPrimitive3D` has a **required** `Position`
//! (`IfcAxis2Placement3D`) at slot 0. Unlike `IfcSweptAreaSolid.Position` it is
//! not optional, so there is no identity default to fall back on.
//!
//! # Origin conventions differ per primitive
//!
//! This is the trap. The placement origin is the **corner** of a block and of a
//! pyramid's base, but the **centre** of the base circle for a cylinder and a
//! cone, and the **centre** of a sphere. Applying one convention uniformly
//! offsets half the primitives by half their size.

use crate::error::GeometryResult;
use crate::slots::Slots;
use ifc_model::{Entity, EntityId};

/// `IfcCsgSolid` slots.
///
/// EXPRESS (IFC4 ADD2 TC1): subtypes `IfcSolidModel`, which declares no
/// explicit attributes, so `TreeRootExpression` is absolute slot 0.
mod csg_solid_slot {
    /// `TreeRootExpression : IfcCsgSelect`.
    pub const TREE_ROOT_EXPRESSION: usize = 0;
}

/// `IfcCsgPrimitive3D` slots, inherited by every primitive.
///
/// EXPRESS: `Position : IfcAxis2Placement3D` is declared on
/// `IfcCsgPrimitive3D`, whose supertype `IfcGeometricRepresentationItem` has no
/// explicit attributes -- so it is absolute slot 0 and each primitive's own
/// dimensions start at slot 1.
mod primitive_slot {
    /// `Position : IfcAxis2Placement3D`, required.
    pub const POSITION: usize = 0;
    /// First dimension attribute of the concrete primitive.
    pub const DIM_0: usize = 1;
    /// Second dimension attribute, where the primitive has one.
    pub const DIM_1: usize = 2;
    /// Third dimension attribute, where the primitive has one.
    pub const DIM_2: usize = 3;
}

/// `IfcCsgSolid`: a solid defined by a CSG expression tree.
///
/// The root reference is returned unresolved: it may be an `IfcBooleanResult`
/// or an `IfcCsgPrimitive3D`, and deciding which is the caller's dispatch, not
/// this view's.
#[derive(Debug, Clone, Copy)]
pub struct CsgSolid<'m> {
    slots: Slots<'m>,
}

impl<'m> CsgSolid<'m> {
    /// Wrap an entity assumed to be an `IfcCsgSolid`.
    pub fn new(id: EntityId, entity: &'m Entity) -> Self {
        Self {
            slots: Slots::new(id, entity),
        }
    }

    /// The entity id.
    pub fn id(&self) -> EntityId {
        self.slots.id()
    }

    /// The `IfcCsgSelect` root: an `IfcBooleanResult` or `IfcCsgPrimitive3D`.
    pub fn tree_root_expression(&self) -> GeometryResult<EntityId> {
        self.slots
            .req_ref(csg_solid_slot::TREE_ROOT_EXPRESSION, "TreeRootExpression")
    }
}

/// `IfcCsgPrimitive3D`: the abstract primitive, giving access to `Position`.
///
/// Every concrete primitive delegates here for its placement, since it is at
/// the same absolute slot 0 in all five.
#[derive(Debug, Clone, Copy)]
pub struct CsgPrimitive3D<'m> {
    slots: Slots<'m>,
}

impl<'m> CsgPrimitive3D<'m> {
    /// Wrap an entity assumed to be an `IfcCsgPrimitive3D` subtype.
    pub fn new(id: EntityId, entity: &'m Entity) -> Self {
        Self {
            slots: Slots::new(id, entity),
        }
    }

    /// The entity id.
    pub fn id(&self) -> EntityId {
        self.slots.id()
    }

    /// The IFC type name, naming the concrete primitive.
    pub fn type_name(&self) -> &'m str {
        self.slots.type_name()
    }

    /// The `IfcAxis2Placement3D` reference. Required, with no default.
    pub fn position(&self) -> GeometryResult<EntityId> {
        self.slots.req_ref(primitive_slot::POSITION, "Position")
    }
}

/// Emit an accessor-bearing primitive newtype with the shared shape.
///
/// Written as a macro because the five primitives differ only in which of the
/// three dimension slots they use and what they are called; hand-writing them
/// would be 200 lines whose only content is a slot number, and a typo in one of
/// them would be invisible on review.
macro_rules! csg_primitive {
    (
        $(#[$meta:meta])*
        $name:ident { $( $(#[$field_meta:meta])* $method:ident => $slot:expr , $express:literal );+ $(;)? }
    ) => {
        $(#[$meta])*
        #[derive(Debug, Clone, Copy)]
        pub struct $name<'m> {
            slots: Slots<'m>,
        }

        impl<'m> $name<'m> {
            /// Wrap an entity assumed to be of this primitive's type.
            pub fn new(id: EntityId, entity: &'m Entity) -> Self {
                Self { slots: Slots::new(id, entity) }
            }

            /// The entity id.
            pub fn id(&self) -> EntityId {
                self.slots.id()
            }

            /// The inherited `IfcCsgPrimitive3D` attributes.
            pub fn base(&self) -> CsgPrimitive3D<'m> {
                CsgPrimitive3D { slots: self.slots }
            }

            $(
                $(#[$field_meta])*
                pub fn $method(&self) -> GeometryResult<f64> {
                    self.slots.req_f64($slot, $express)
                }
            )+
        }
    };
}

csg_primitive! {
    /// `IfcBlock`: an axis-aligned box.
    ///
    /// The placement origin is the box **corner**, and the box extends along
    /// +X, +Y and +Z of `Position`. It is not centred, so treating it as a
    /// centred box offsets it by half its size in all three axes.
    Block {
        /// `XLength`, extent along the placement's X axis.
        x_length => primitive_slot::DIM_0, "XLength";
        /// `YLength`, extent along the placement's Y axis.
        y_length => primitive_slot::DIM_1, "YLength";
        /// `ZLength`, extent along the placement's Z axis.
        z_length => primitive_slot::DIM_2, "ZLength";
    }
}

csg_primitive! {
    /// `IfcRectangularPyramid`: a pyramid on a rectangular base.
    ///
    /// The base rectangle's **corner** is at the placement origin, extending
    /// along +X and +Y; the apex sits above the base centre at `Height`.
    RectangularPyramid {
        /// `XLength`, base extent along the placement's X axis.
        x_length => primitive_slot::DIM_0, "XLength";
        /// `YLength`, base extent along the placement's Y axis.
        y_length => primitive_slot::DIM_1, "YLength";
        /// `Height`, apex height above the base plane.
        height => primitive_slot::DIM_2, "Height";
    }
}

csg_primitive! {
    /// `IfcRightCircularCone`: a cone standing on the placement XY plane.
    ///
    /// The base circle is **centred** on the placement origin (unlike
    /// [`Block`]), and the apex is at `Height` along +Z. There is no top
    /// radius: this is always a full cone, never a frustum.
    RightCircularCone {
        /// `Height`, apex height above the base plane.
        height => primitive_slot::DIM_0, "Height";
        /// `BottomRadius`, radius of the base circle.
        bottom_radius => primitive_slot::DIM_1, "BottomRadius";
    }
}

csg_primitive! {
    /// `IfcRightCircularCylinder`: a cylinder standing on the placement XY
    /// plane.
    ///
    /// The base circle is **centred** on the placement origin and the axis is
    /// +Z. Note the attribute order is `Height` then `Radius`, the opposite of
    /// how most APIs spell a cylinder.
    RightCircularCylinder {
        /// `Height`, extent along the placement's Z axis.
        height => primitive_slot::DIM_0, "Height";
        /// `Radius`, radius of the circular section.
        radius => primitive_slot::DIM_1, "Radius";
    }
}

csg_primitive! {
    /// `IfcSphere`: a sphere **centred** on the placement origin.
    ///
    /// The only primitive whose placement orientation is geometrically
    /// irrelevant; only the origin matters.
    Sphere {
        /// `Radius`.
        radius => primitive_slot::DIM_0, "Radius";
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::solid::testkit::{entity, n, r};

    #[test]
    fn csg_solid_exposes_its_tree_root_without_resolving_it() {
        let e = entity("IFCCSGSOLID", vec![r(55)]);
        let view = CsgSolid::new(EntityId(1), &e);
        assert_eq!(view.tree_root_expression().unwrap(), EntityId(55));
    }

    #[test]
    fn a_csg_solid_without_a_root_expression_is_an_error_not_an_empty_tree() {
        let e = entity("IFCCSGSOLID", vec![]);
        let err = CsgSolid::new(EntityId(3), &e)
            .tree_root_expression()
            .unwrap_err();
        assert_eq!(err.entity(), Some(EntityId(3)));
    }

    /// Position is slot 0 for every primitive, so each primitive's own
    /// dimensions begin at slot 1.
    #[test]
    fn primitive_position_precedes_the_dimension_attributes() {
        let e = entity("IFCBLOCK", vec![r(9), n(1.0), n(2.0), n(3.0)]);
        let block = Block::new(EntityId(1), &e);
        assert_eq!(block.base().position().unwrap(), EntityId(9));
        assert_eq!(block.x_length().unwrap(), 1.0);
        assert_eq!(block.y_length().unwrap(), 2.0);
        assert_eq!(block.z_length().unwrap(), 3.0);
    }

    /// Position is required on IfcCsgPrimitive3D; unlike a swept solid there
    /// is no identity default.
    #[test]
    fn primitive_position_is_required_and_has_no_identity_default() {
        let e = entity("IFCSPHERE", vec![ifc_model::Value::Null, n(2.0)]);
        let sphere = Sphere::new(EntityId(4), &e);
        assert!(sphere.base().position().is_err());
        assert_eq!(sphere.radius().unwrap(), 2.0);
    }

    #[test]
    fn pyramid_reads_height_where_block_reads_z_length() {
        let attrs = vec![r(9), n(4.0), n(5.0), n(6.0)];
        let pyramid = entity("IFCRECTANGULARPYRAMID", attrs.clone());
        let block = entity("IFCBLOCK", attrs);
        assert_eq!(
            RectangularPyramid::new(EntityId(1), &pyramid)
                .height()
                .unwrap(),
            6.0
        );
        assert_eq!(Block::new(EntityId(1), &block).z_length().unwrap(), 6.0);
    }

    /// The cylinder and cone both spell Height BEFORE their radius, which is
    /// the reverse of the usual API convention.
    #[test]
    fn cylinder_and_cone_declare_height_before_radius() {
        let cyl = entity("IFCRIGHTCIRCULARCYLINDER", vec![r(9), n(10.0), n(0.5)]);
        let view = RightCircularCylinder::new(EntityId(1), &cyl);
        assert_eq!(view.height().unwrap(), 10.0);
        assert_eq!(view.radius().unwrap(), 0.5);

        let cone = entity("IFCRIGHTCIRCULARCONE", vec![r(9), n(10.0), n(0.5)]);
        let view = RightCircularCone::new(EntityId(1), &cone);
        assert_eq!(view.height().unwrap(), 10.0);
        assert_eq!(view.bottom_radius().unwrap(), 0.5);
    }

    #[test]
    fn every_primitive_reports_its_concrete_type_name() {
        for name in [
            "IFCBLOCK",
            "IFCRECTANGULARPYRAMID",
            "IFCRIGHTCIRCULARCONE",
            "IFCRIGHTCIRCULARCYLINDER",
            "IFCSPHERE",
        ] {
            let e = entity(name, vec![r(1), n(1.0), n(1.0), n(1.0)]);
            assert_eq!(CsgPrimitive3D::new(EntityId(1), &e).type_name(), name);
        }
    }
}
