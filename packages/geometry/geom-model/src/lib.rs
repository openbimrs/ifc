//! `geom-model` - the format-neutral geometry item tree.
//!
//! # Why this crate exists (the strongest evidence in the reference)
//!
//! IfcOpenShell's geometry engine does not hand IFC entities to its kernels. It
//! first lowers them into `taxonomy` - a format-neutral tree of ~30 item kinds
//! (`matrix4`, `point3`, `line`, `circle`, `plane`, `cylinder`, `edge`, `loop`,
//! `face`, `shell`, `solid`, `extrusion`, `revolve`, `sweep_along_curve`,
//! `boolean_result`, `collection`, ...). Every kernel - OpenCascade, CGAL,
//! Manifold, passthrough - consumes *that*, never IFC.
//!
//! That indirection is why they can ship four interchangeable kernels at all,
//! and it is the piece our stack was missing: we had data types (`geom-core`,
//! `geom-mesh`) and algorithms (`geom-sweep`, `geom-kernel`), but no neutral
//! *description of a shape yet to be built*.
//!
//! # What it buys us
//!
//! ```text
//!   ifc-geometry ─┐
//!   step-cad     ─┼─→ geom-model (Item tree) ─→ any kernel
//!   citygml      ─┘
//! ```
//!
//! A second format costs an `Item` builder, not a second kernel integration.
//! A second kernel costs an `Item` consumer, not a second format reader.
//!
//! # Why an owned tree and not traits
//!
//! An `Item` is inert data: no evaluation, no tessellation, no kernel calls.
//! That makes the whole tree cheap to construct, trivially `Send`, snapshot-
//! testable as a value, and - critically - inspectable in a debugger when a
//! shape comes out wrong. Trait objects would hide exactly the state you need
//! to see at 2am.
//!
//! # Deliberate omissions
//!
//! **No colour, material, or style.** Those are presentation, and a geometry
//! item tree that carries them cannot be refactored without touching a
//! renderer. IfcOpenShell's taxonomy *does* include `colour` and `style` kinds;
//! we consider that a mistake to avoid rather than a pattern to copy.
//!
//! Not yet implemented - see `docs/ROADMAP.md`.

use geom_core::{Mat4, Vec3};

/// A node in the format-neutral geometry tree.
///
/// # Closed enum, by design
///
/// Matching must be exhaustive: when a new kind is added, every kernel fails to
/// compile until it decides what to do. An open trait would let a kernel
/// silently ignore a shape kind and emit nothing - which in a BIM pipeline
/// means a wall that quietly vanishes from a clash report.
#[derive(Debug, Clone, PartialEq)]
pub enum Item {
    /// A point in space.
    Point(Vec3),

    /// A direction. Not normalised on construction - the producer may not know
    /// the magnitude is meaningless, and silently normalising hides bugs in the
    /// reader.
    Direction(Vec3),

    /// A curve, by reference into the curve vocabulary.
    ///
    /// The variant is a placeholder until `geom-curve` defines its curve enum;
    /// the point here is that a curve is an *item*, so a tree can hold one.
    Curve(CurveRef),

    /// A surface, by reference into the surface vocabulary.
    Surface(SurfaceRef),

    /// A closed volume already reduced to triangles.
    ///
    /// This is the terminal representation: tessellated IFC face sets arrive
    /// here directly, and every other kind eventually becomes one.
    Mesh(MeshRef),

    /// A profile swept linearly along a direction.
    ///
    /// The single most common solid in real building models - 25 occurrences of
    /// `IfcExtrudedAreaSolid` across our fixture corpus, versus zero NURBS.
    Extrusion {
        /// The 2D cross-section being swept.
        profile: ProfileRef,
        /// Sweep direction in the profile's local frame.
        direction: Vec3,
        /// Distance swept along `direction`.
        depth: f64,
    },

    /// A profile revolved about an axis.
    Revolve {
        /// The 2D cross-section being revolved.
        profile: ProfileRef,
        /// A point on the axis of revolution.
        axis_origin: Vec3,
        /// Direction of the axis of revolution.
        axis_direction: Vec3,
        /// Sweep angle in radians. A full revolution is `2*PI`.
        angle: f64,
    },

    /// A boolean combination of two shapes.
    ///
    /// Recursive: `IfcBooleanClippingResult` nests, and a wall with eleven
    /// openings is a chain eleven deep.
    Boolean {
        /// Which set operation to apply.
        op: BooleanKind,
        /// The shape being operated on.
        lhs: Box<Item>,
        /// The tool shape.
        rhs: Box<Item>,
    },

    /// A transformed child.
    ///
    /// Placement composition lowers to nested `Transform` nodes, so a kernel
    /// never has to know how a format spells its placement chain.
    Transform {
        /// Affine transform applied to `child`.
        matrix: Mat4,
        /// The shape being placed.
        child: Box<Item>,
    },

    /// Several items treated as one shape.
    ///
    /// A representation with multiple items, or an instanced assembly, becomes
    /// a collection rather than forcing the consumer to handle a list at every
    /// call site.
    Collection(Vec<Item>),
}

/// Which set operation a [`Item::Boolean`] performs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BooleanKind {
    /// Everything in either operand.
    Union,
    /// Only what lies in both operands.
    Intersection,
    /// The left operand with the right removed - the opening cut.
    Difference,
}

/// Placeholder handle for a curve defined in `geom-curve`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CurveRef(pub u32);

/// Placeholder handle for a surface defined in `geom-surface`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SurfaceRef(pub u32);

/// Placeholder handle for a profile defined in `geom-profile`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ProfileRef(pub u32);

/// Placeholder handle for a mesh defined in `geom-mesh`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MeshRef(pub u32);

impl Item {
    /// Number of nodes in this subtree, counting itself.
    ///
    /// Cheap structural metric, useful for deciding whether a shape is worth
    /// dispatching to a GPU backend and for catching runaway nesting in
    /// malformed files before it becomes a stack overflow.
    pub fn node_count(&self) -> usize {
        match self {
            Item::Boolean { lhs, rhs, .. } => 1 + lhs.node_count() + rhs.node_count(),
            Item::Transform { child, .. } => 1 + child.node_count(),
            Item::Collection(items) => 1 + items.iter().map(Item::node_count).sum::<usize>(),
            _ => 1,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn node_count_walks_nested_booleans() {
        // A wall minus two openings: the shape of nearly every real facade.
        let wall = Item::Mesh(MeshRef(0));
        let cut_once = Item::Boolean {
            op: BooleanKind::Difference,
            lhs: Box::new(wall),
            rhs: Box::new(Item::Mesh(MeshRef(1))),
        };
        let cut_twice = Item::Boolean {
            op: BooleanKind::Difference,
            lhs: Box::new(cut_once),
            rhs: Box::new(Item::Mesh(MeshRef(2))),
        };
        assert_eq!(cut_twice.node_count(), 5);
    }

    #[test]
    fn transform_and_collection_contribute_to_the_count() {
        let inner = Item::Collection(vec![Item::Point(Vec3::ZERO), Item::Point(Vec3::ONE)]);
        let placed = Item::Transform {
            matrix: Mat4::IDENTITY,
            child: Box::new(inner),
        };
        assert_eq!(placed.node_count(), 4);
    }
}
