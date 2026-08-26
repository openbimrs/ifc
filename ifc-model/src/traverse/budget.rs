//! Limits that keep a walk over a malformed file finite.
//!
//! A cycle in a reference graph is not hypothetical: an `IfcRelAggregates`
//! whose relating object is also one of its related objects appears in real
//! exports. An unbounded walk hangs the caller with no diagnostic, so every
//! traversal here takes a budget and reports why it stopped.

/// Limits applied to a single walk.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Budget {
    /// Maximum edges followed from the start before stopping.
    pub max_depth: usize,
    /// Maximum distinct entities visited.
    pub max_nodes: usize,
}

impl Budget {
    /// A budget large enough for well-formed building models.
    ///
    /// Depth 64 clears the deepest real spatial nesting by a wide margin;
    /// 1_000_000 nodes bounds memory without truncating a large model.
    pub const DEFAULT: Self = Self {
        max_depth: 64,
        max_nodes: 1_000_000,
    };

    /// A budget with the given depth and the default node ceiling.
    #[must_use]
    pub const fn with_depth(max_depth: usize) -> Self {
        Self {
            max_depth,
            ..Self::DEFAULT
        }
    }
}

impl Default for Budget {
    fn default() -> Self {
        Self::DEFAULT
    }
}

/// Why a walk stopped.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Stop {
    /// Every reachable entity was visited.
    Exhausted,
    /// The depth limit was hit; results are partial.
    DepthLimit,
    /// The node limit was hit; results are partial.
    NodeLimit,
}

impl Stop {
    /// Whether the walk finished rather than being truncated.
    #[must_use]
    pub const fn is_complete(self) -> bool {
        matches!(self, Self::Exhausted)
    }
}

/// The outcome of a bounded walk.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Walk {
    /// Entities visited, in traversal order, each exactly once.
    pub visited: Vec<crate::value::EntityId>,
    /// Why the walk ended.
    pub stop: Stop,
    /// Entities that were reached again after being visited.
    ///
    /// Non-empty means the graph is cyclic along the followed edges. Reported
    /// rather than silently skipped: in a spatial tree it is a defect worth
    /// surfacing to the caller.
    pub revisited: Vec<crate::value::EntityId>,
}
