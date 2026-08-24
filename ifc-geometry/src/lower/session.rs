//! One recursive lowering session over one shared graph builder.
//!
//! # Why a session exists
//!
//! `AGENTS.md` requires that recursive lowering appends to a single
//! session-owned builder and that family lowerers return [`NodeId`] instead of
//! freezing isolated child graphs. That is not a style preference: a [`NodeId`]
//! is owned by the graph that minted it, so handles from two independently
//! finished graphs are mutually foreign. Every composite IFC family needs two
//! children in one graph:
//!
//! - `IfcBooleanResult` references two operands,
//! - `IfcMappedItem` reuses one source under many transforms,
//! - a B-rep face set shares one surface across many faces,
//! - `IfcCsgSolid` nests operations arbitrarily deep.
//!
//! The session also carries the three things every lowerer needed to thread
//! manually before: the model, the resolved unit scale, and the tolerance.
//!
//! # What it guarantees
//!
//! - **One graph.** All nodes land in one builder, so any two lowered results
//!   are composable.
//! - **Memoization.** A shared IFC entity lowered under the same frame yields
//!   the same node instead of a duplicate subtree.
//! - **Bounded recursion.** Cyclic and over-deep chains produce typed errors
//!   rather than a stack overflow.
//! - **Located failures.** Graph construction faults are translated into
//!   [`GeometryError`] values that name the offending IFC entity.

use std::collections::BTreeMap;

use axiolid_model::{GeometryGraphBuilder, GeometryNode, GraphError, NodeId};
use ifc_model::{EntityId, Model};

use crate::error::{GeometryError, GeometryResult};
use crate::lower::{LoweredGeometry, ProvenanceMap, Tolerance};
use crate::slots::Slots;
use crate::transform::Transform;
use crate::units::UnitScale;

/// Recursion budget for chained IFC references.
///
/// IFC places no normative limit on placement or mapped-item nesting, so a
/// budget is the only way to terminate on malformed input that is deep rather
/// than strictly cyclic.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SessionLimits {
    /// Maximum simultaneously active entities in one chain.
    pub max_depth: usize,
}

impl SessionLimits {
    /// Depth budget used when a caller states no preference.
    ///
    /// Real exporter output nests placements a few levels deep; 64 is far
    /// above observed depth while still terminating quickly on bad input.
    pub const DEFAULT_MAX_DEPTH: usize = 64;
}

impl Default for SessionLimits {
    fn default() -> Self {
        Self {
            max_depth: Self::DEFAULT_MAX_DEPTH,
        }
    }
}

/// Identity of one lowering result, used to deduplicate shared entities.
///
/// The frame is part of the key. Two `IfcMappedItem`s reusing one source under
/// different transforms are different results, and collapsing them would place
/// geometry at one location only. Floats are keyed by bit pattern so the key is
/// totally ordered without imposing a tolerance policy: memoization must be an
/// exact-identity optimization, never a geometric approximation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct MemoKey {
    entity: u64,
    family: &'static str,
    basis: [[u64; 3]; 3],
    origin: [u64; 3],
}

impl MemoKey {
    fn new(entity: EntityId, family: &'static str, frame: Transform) -> Self {
        Self {
            entity: entity.0,
            family,
            basis: frame.basis.map(|axis| axis.map(f64::to_bits)),
            origin: frame.origin.map(f64::to_bits),
        }
    }
}

/// A single recursive lowering pass over one shared graph builder.
///
/// Family lowerers take `&mut LoweringSession` and return [`NodeId`]. Only the
/// public entry point calls [`LoweringSession::finish`].
#[derive(Debug)]
pub struct LoweringSession<'a> {
    model: &'a Model,
    units: &'a UnitScale,
    tolerance: Tolerance,
    limits: SessionLimits,
    builder: GeometryGraphBuilder,
    nodes: usize,
    memo: BTreeMap<MemoKey, NodeId>,
    active: Vec<EntityId>,
    provenance: ProvenanceMap,
}

impl<'a> LoweringSession<'a> {
    /// Open a session with the default recursion budget.
    pub fn new(model: &'a Model, units: &'a UnitScale, tolerance: Tolerance) -> Self {
        Self::with_limits(model, units, tolerance, SessionLimits::default())
    }

    /// Open a session with an explicit recursion budget.
    pub fn with_limits(
        model: &'a Model,
        units: &'a UnitScale,
        tolerance: Tolerance,
        limits: SessionLimits,
    ) -> Self {
        Self {
            model,
            units,
            tolerance,
            limits,
            builder: GeometryGraphBuilder::new(),
            nodes: 0,
            memo: BTreeMap::new(),
            active: Vec::new(),
            provenance: ProvenanceMap::default(),
        }
    }

    /// The model being lowered.
    pub fn model(&self) -> &'a Model {
        self.model
    }

    /// The resolved unit scale for this model.
    pub fn units(&self) -> &'a UnitScale {
        self.units
    }

    /// The tolerance policy for this session.
    ///
    /// Returned by value because [`Tolerance`] is `Copy`; handing back a
    /// borrow would force callers that also need `&mut self` to clone it.
    pub fn tolerance(&self) -> Tolerance {
        self.tolerance
    }

    /// Number of nodes appended so far.
    ///
    /// Exposed so tests can assert that a memoized hit appends nothing.
    pub fn node_count(&self) -> usize {
        self.nodes
    }

    /// Append one node, attributing any graph fault to the current entity.
    pub fn node(&mut self, node: GeometryNode) -> GeometryResult<NodeId> {
        let source = self.active.last().copied();
        let id = self
            .builder
            .push(node)
            .map_err(|error| graph_error(source.unwrap_or(EntityId(0)), error))?;
        self.nodes += 1;
        if let Some(source) = source {
            self.provenance.record(id, source);
        }
        Ok(id)
    }

    /// Append one node, attributing any graph fault to `entity`.
    pub fn node_for(&mut self, entity: EntityId, node: GeometryNode) -> GeometryResult<NodeId> {
        let id = self
            .builder
            .push(node)
            .map_err(|error| graph_error(entity, error))?;
        self.nodes += 1;
        self.provenance.record(id, entity);
        Ok(id)
    }

    /// Source attribution accumulated so far.
    pub fn provenance(&self) -> &ProvenanceMap {
        &self.provenance
    }

    /// Resolve an entity or report the dangling reference against `referrer`.
    pub fn entity(
        &self,
        referrer: EntityId,
        id: EntityId,
    ) -> GeometryResult<&'a ifc_model::Entity> {
        self.model.get(id).ok_or(GeometryError::MissingEntity {
            referrer,
            missing: id,
        })
    }

    /// Look up a previously lowered result for `entity` under `frame`.
    pub fn memoized(
        &self,
        entity: EntityId,
        family: &'static str,
        frame: Transform,
    ) -> Option<NodeId> {
        self.memo.get(&MemoKey::new(entity, family, frame)).copied()
    }

    /// Record the lowered result for `entity` under `frame`.
    pub fn memoize(
        &mut self,
        entity: EntityId,
        family: &'static str,
        frame: Transform,
        node: NodeId,
    ) {
        self.memo.insert(MemoKey::new(entity, family, frame), node);
    }

    /// Mark `entity` as active in the current chain.
    ///
    /// Returns [`GeometryError::CyclicChain`] if the entity is already active
    /// and [`GeometryError::ChainTooDeep`] once the depth budget is exhausted.
    /// Every successful call must be paired with [`LoweringSession::exit`].
    pub fn enter(&mut self, entity: EntityId, kind: &'static str) -> GeometryResult<()> {
        if self.active.contains(&entity) {
            return Err(GeometryError::CyclicChain { entity, kind });
        }
        if self.active.len() >= self.limits.max_depth {
            return Err(GeometryError::ChainTooDeep {
                entity,
                kind,
                limit: self.limits.max_depth,
            });
        }
        self.active.push(entity);
        Ok(())
    }

    /// Release `entity` from the active chain.
    ///
    /// Sharing is not recursion: once a subtree is complete the entity must be
    /// reachable again from a sibling branch.
    pub fn exit(&mut self, entity: EntityId) {
        if self.active.last() == Some(&entity) {
            self.active.pop();
            return;
        }
        debug_assert!(false, "lowering scopes must exit in LIFO order");
        if let Some(index) = self.active.iter().rposition(|&active| active == entity) {
            self.active.remove(index);
        }
    }

    /// Borrowed attribute view for `entity`.
    pub fn slots(&self, entity: EntityId) -> GeometryResult<Slots<'a>> {
        let resolved = self.entity(entity, entity)?;
        Ok(Slots::new(entity, resolved))
    }

    /// Upper-cased IFC type name for `entity`.
    ///
    /// Dispatch compares against canonical upper-case names because STEP files
    /// are case-insensitive in practice and exporters disagree.
    pub fn type_name(&self, entity: EntityId) -> GeometryResult<String> {
        Ok(self.entity(entity, entity)?.type_name.to_ascii_uppercase())
    }

    /// Build a typed `Unsupported` error naming the offending entity.
    pub fn unsupported(
        &self,
        entity: EntityId,
        type_name: &str,
        detail: &'static str,
    ) -> GeometryError {
        GeometryError::Unsupported {
            entity,
            type_name: type_name.to_string(),
            detail,
        }
    }

    /// Build a typed `Degenerate` error naming the offending entity.
    ///
    /// Structurally impossible geometry is distinct from an unimplemented
    /// family: the file is understood and the shape does not exist.
    pub fn degenerate(
        &self,
        entity: EntityId,
        type_name: &str,
        detail: impl Into<String>,
    ) -> GeometryError {
        GeometryError::Degenerate {
            entity,
            type_name: type_name.to_string(),
            detail: detail.into(),
        }
    }

    /// Lower a nested operand through the total dispatcher.
    ///
    /// Kept on the session so recursive families do not each re-import the
    /// dispatcher and risk diverging on cycle/limit handling.
    pub fn lower_operand(&mut self, entity: EntityId, frame: Transform) -> GeometryResult<NodeId> {
        crate::lower::dispatch::lower_representation_item(self, entity, frame)
    }

    /// Freeze the graph with `root` as its single output root.
    pub fn finish(self, root: NodeId) -> GeometryResult<LoweredGeometry> {
        let entity = self.current_entity();
        let graph = self
            .builder
            .finish(vec![root])
            .map_err(|error| graph_error(entity, error))?;
        Ok(LoweredGeometry {
            graph,
            root,
            provenance: self.provenance,
        })
    }

    /// Best-effort attribution target for graph faults raised outside a family.
    fn current_entity(&self) -> EntityId {
        self.active.last().copied().unwrap_or(EntityId(0))
    }
}

/// Translate a graph construction fault into a located IFC error.
///
/// A bare [`GraphError`] names a `NodeId`, which is meaningless when debugging
/// a 500k-entity file; the IFC entity is the addressable unit.
pub(crate) fn graph_error(entity: EntityId, error: GraphError) -> GeometryError {
    GeometryError::Degenerate {
        entity,
        type_name: "geometry graph".to_string(),
        detail: error.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_memo_key_separates_entity_family_and_frame() {
        let frame = Transform::identity();
        let mut moved = Transform::identity();
        moved.origin = [1.0, 0.0, 0.0];

        let base = MemoKey::new(EntityId(1), "solid", frame);
        assert_eq!(base, MemoKey::new(EntityId(1), "solid", frame));
        assert_ne!(base, MemoKey::new(EntityId(2), "solid", frame));
        assert_ne!(base, MemoKey::new(EntityId(1), "profile", frame));
        assert_ne!(base, MemoKey::new(EntityId(1), "solid", moved));
    }

    #[test]
    fn signed_zero_does_not_alias_positive_zero_in_the_key() {
        // -0.0 == 0.0 numerically but has a distinct bit pattern. Keying on
        // bits keeps memoization an exact-identity optimization.
        let mut negative = Transform::identity();
        negative.origin = [-0.0, 0.0, 0.0];
        assert_ne!(
            MemoKey::new(EntityId(1), "solid", Transform::identity()),
            MemoKey::new(EntityId(1), "solid", negative)
        );
    }

    #[test]
    fn the_default_depth_budget_is_documented() {
        assert_eq!(
            SessionLimits::default().max_depth,
            SessionLimits::DEFAULT_MAX_DEPTH
        );
    }
}
