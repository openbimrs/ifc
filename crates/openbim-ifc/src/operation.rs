//! What door and window operation share (#148, #170): the hinge [`Side`],
//! the swept [`Sector`], a rigid placement frame, and exact attribute
//! reading from the declared release.
//!
//! Both joins read an occurrence's placement (`ifc-geometry`) and its
//! predefined panel sets (`ifc-properties`), so this module is compiled
//! under the same features as they are: `geometry-select` and `properties`.

#![cfg(all(feature = "geometry-select", feature = "properties"))]

mod frame;
mod read;

pub(crate) use frame::{FrameError, RigidFrame};
pub(crate) use read::{
    enumeration, governing, positive_length, type_object, world_transform, ReadError,
};

/// Left or right, as seen looking in a leaf's opening direction from above.
///
/// For a door leaf and a window panel alike this is the plan side: the
/// viewer looks along the local +y axis the leaf opens towards, with world
/// +z up.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Side {
    /// The left-hand side.
    Left,
    /// The right-hand side.
    Right,
}

/// A circular sector swept by a leaf about its hinge, in world metres.
///
/// Its boundary direction at angle `t ∈ [0, sweep]` is
/// `cos t · start + sin t · (axis × start)`.
///
/// A door leaf's sector lies in the floor plane of the door's placement. A
/// window panel's lies in the plane through one end of its hinge and
/// perpendicular to it; the panel sweeps it along the whole hinge (see
/// [`WindowPanel`](crate::WindowPanel)).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Sector {
    /// The hinge point: on the door's x axis for a door leaf, and one end of
    /// the hinge line for a window panel.
    pub center: [f64; 3],
    /// The leaf's extent from its hinge to its free edge: a door leaf's
    /// width; a window panel's width about a side hinge and its height about
    /// a top or bottom hinge.
    pub radius: f64,
    /// Unit direction of the first boundary ray: the closed leaf (hinge to
    /// free edge) for a single swing, the leaf fully open to local -y for a
    /// double-acting one.
    pub start: [f64; 3],
    /// Unit rotation axis, oriented so the sweep is positive about it.
    pub axis: [f64; 3],
    /// Swept angle in radians: π/2, or π for a double-acting leaf.
    pub sweep: f64,
}

impl Sector {
    /// Unit direction of the boundary ray at `angle` radians from `start`.
    #[must_use]
    pub fn direction_at(&self, angle: f64) -> [f64; 3] {
        frame::rotate(self.start, self.axis, angle)
    }

    /// Unit direction of the last boundary ray: the leaf fully open to
    /// local +y.
    #[must_use]
    pub fn end(&self) -> [f64; 3] {
        self.direction_at(self.sweep)
    }
}
