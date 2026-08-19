//! Typed graph identity.

use core::fmt;

/// Stable index into one immutable [`crate::GeometryGraph`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct NodeId(u32);

impl NodeId {
    pub(crate) fn from_index(index: usize) -> Self {
        Self(u32::try_from(index).expect("geometry graph exceeds u32 capacity"))
    }

    /// Zero-based graph index.
    pub const fn index(self) -> usize {
        self.0 as usize
    }
}

impl fmt::Display for NodeId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "geometry#{}", self.0)
    }
}
