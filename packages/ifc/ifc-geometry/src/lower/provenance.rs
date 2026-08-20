//! IFC source attribution for format-neutral geometry nodes.
//!
//! Provenance is deliberately a side table. `axiolid-model` stays IFC-agnostic,
//! while diagnostics and consumers of a lowered result can still trace a node
//! back to the entity that emitted it.

use axiolid_model::NodeId;
use ifc_model::EntityId;
use std::collections::BTreeMap;

/// Source IFC entities for nodes in one lowered geometry graph.
///
/// The map is partial: caller-synthesized nodes created outside an active IFC
/// entity scope have no source instead of receiving a fabricated entity id.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ProvenanceMap {
    sources: BTreeMap<NodeId, EntityId>,
}

impl ProvenanceMap {
    /// Return the IFC entity that emitted `node`, if it has an IFC source.
    pub fn source(&self, node: NodeId) -> Option<EntityId> {
        self.sources.get(&node).copied()
    }

    /// Number of attributed graph nodes.
    pub fn len(&self) -> usize {
        self.sources.len()
    }

    /// Whether no graph nodes carry IFC attribution.
    pub fn is_empty(&self) -> bool {
        self.sources.is_empty()
    }

    /// Iterate deterministically over `(node, source entity)` pairs.
    pub fn iter(&self) -> impl Iterator<Item = (NodeId, EntityId)> + '_ {
        self.sources.iter().map(|(&node, &source)| (node, source))
    }

    pub(crate) fn record(&mut self, node: NodeId, source: EntityId) {
        if let Some(previous) = self.sources.insert(node, source) {
            debug_assert_eq!(
                previous, source,
                "one graph node cannot have two IFC sources"
            );
        }
    }
}
