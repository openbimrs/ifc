//! Assembling the containment tree from relationship entities.
//!
//! # Tolerating real files
//!
//! The canonical hierarchy is project → site → building → storey → element, and
//! plenty of real exports do not follow it: sites are omitted, elements hang
//! directly off the building, storeys are duplicated, and occasionally a
//! relationship points at an entity that is not in the file. The tree records
//! what the file says rather than asserting the ideal shape, and reports the
//! anomalies separately so a caller can decide whether to care.

use std::collections::BTreeMap;

use ifc_model::{EntityId, Model};

use super::kind::SpatialKind;
use crate::relation::{Relationship, RelationshipKind};

/// One entity's place in the containment tree.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpatialNode {
    /// The entity this node describes.
    pub id: EntityId,
    /// Its spatial role.
    pub kind: SpatialKind,
    /// Its container, if any relationship names one.
    pub parent: Option<EntityId>,
    /// Sub-containers, in file order.
    pub children: Vec<EntityId>,
    /// Physical elements placed directly in this container, in file order.
    pub elements: Vec<EntityId>,
}

/// The containment tree of one model.
#[derive(Debug, Clone, Default)]
pub struct SpatialTree {
    nodes: BTreeMap<EntityId, SpatialNode>,
    /// Element to its direct container, so `container_of` is a lookup.
    container_of_element: BTreeMap<EntityId, EntityId>,
    roots: Vec<EntityId>,
    orphans: Vec<EntityId>,
    dangling: Vec<(EntityId, EntityId)>,
}

impl SpatialTree {
    /// Build the containment tree of `model`.
    ///
    /// One pass over the aggregation and containment relationships. Cost is
    /// linear in the number of relationship entities, not in model size,
    /// because relationships are found through the type index.
    #[must_use]
    pub fn build(model: &Model) -> Self {
        let mut tree = Self::default();

        // Ensure every spatial entity has a node, even one no relationship
        // mentions -- a lone IfcBuildingStorey is still part of the file.
        for id in model.ids() {
            let Some(entity) = model.get(id) else {
                continue;
            };
            let kind = SpatialKind::classify(&entity.type_name);
            if kind.is_container() {
                tree.nodes.insert(
                    id,
                    SpatialNode {
                        id,
                        kind,
                        parent: None,
                        children: Vec::new(),
                        elements: Vec::new(),
                    },
                );
            }
        }

        for relationship in crate::relation::all(model) {
            tree.apply(model, &relationship);
        }

        // A container with no parent is a root. Sorted for determinism, then
        // ordered so the project (if any) leads.
        tree.roots = tree
            .nodes
            .values()
            .filter(|node| node.parent.is_none())
            .map(|node| node.id)
            .collect();
        tree.roots.sort_by_key(|id| {
            let kind = tree.nodes[id].kind;
            (kind, *id)
        });

        // Every root beyond the most-general one is a detached branch: the file
        // declares a container that nothing aggregates. Reported rather than
        // hidden, because a viewer would otherwise silently lose it.
        if tree.roots.len() > 1 {
            tree.orphans = tree.roots[1..].to_vec();
        }
        tree
    }

    /// Record one relationship's effect on the tree.
    fn apply(&mut self, model: &Model, relationship: &Relationship) {
        let Some(parent) = relationship.relating else {
            return;
        };
        // A relationship naming an entity the file does not contain is a
        // defect worth reporting, not a reason to abandon the tree.
        if model.get(parent).is_none() {
            self.dangling.push((relationship.id, parent));
            return;
        }
        if !self.nodes.contains_key(&parent) {
            // The relating end is not a container -- an element aggregating
            // its parts. Valid IFC, but not spatial containment.
            return;
        }

        for &child in &relationship.related {
            let Some(child_entity) = model.get(child) else {
                self.dangling.push((relationship.id, child));
                continue;
            };
            let child_kind = SpatialKind::classify(&child_entity.type_name);

            if child_kind.is_container() {
                // Containment relationships name elements, not containers;
                // a container arriving here means an aggregation edge.
                if relationship.kind == RelationshipKind::ContainedIn {
                    continue;
                }
                if let Some(node) = self.nodes.get_mut(&child) {
                    // A second parent is a malformed file. Keep the first so
                    // the tree stays a tree, and record nothing further --
                    // `orphans` and `dangling` cover the reportable defects.
                    if node.parent.is_none() {
                        node.parent = Some(parent);
                        if let Some(parent_node) = self.nodes.get_mut(&parent) {
                            parent_node.children.push(child);
                        }
                    }
                }
            } else if let Some(parent_node) = self.nodes.get_mut(&parent) {
                if !parent_node.elements.contains(&child) {
                    parent_node.elements.push(child);
                    // First container wins: an element named by two containment
                    // relationships is malformed, and picking the first keeps
                    // the answer stable across runs.
                    self.container_of_element.entry(child).or_insert(parent);
                }
            }
        }
    }
}

impl SpatialTree {
    /// The node describing `id`, if it is a spatial container.
    #[must_use]
    pub fn node(&self, id: EntityId) -> Option<&SpatialNode> {
        self.nodes.get(&id)
    }

    /// Containers with no parent, most-general kind first.
    ///
    /// A conformant file yields exactly one root, the `IfcProject`. More than
    /// one means the file omits an aggregation relationship somewhere.
    #[must_use]
    pub fn roots(&self) -> &[EntityId] {
        &self.roots
    }

    /// Every spatial container in the model, ordered by entity id.
    pub fn containers(&self) -> impl Iterator<Item = &SpatialNode> {
        self.nodes.values()
    }

    /// Containers of a given kind, ordered by entity id.
    pub fn of_kind(&self, kind: SpatialKind) -> impl Iterator<Item = &SpatialNode> + '_ {
        self.nodes.values().filter(move |node| node.kind == kind)
    }

    /// Elements placed directly in `container`.
    ///
    /// Direct only: an element in a space inside a storey is not returned for
    /// the storey. Use [`elements_recursive`](Self::elements_recursive) for the
    /// transitive set.
    #[must_use]
    pub fn elements_of(&self, container: EntityId) -> &[EntityId] {
        self.nodes
            .get(&container)
            .map_or(&[], |node| node.elements.as_slice())
    }

    /// Every element in `container` or any container beneath it.
    ///
    /// Breadth-first, so a container's own elements precede those of its
    /// children. Bounded by the tree's depth, which is finite because each node
    /// holds at most one parent.
    #[must_use]
    pub fn elements_recursive(&self, container: EntityId) -> Vec<EntityId> {
        let mut out = Vec::new();
        let mut queue = std::collections::VecDeque::from([container]);
        let mut seen = std::collections::BTreeSet::new();
        while let Some(current) = queue.pop_front() {
            if !seen.insert(current) {
                continue;
            }
            let Some(node) = self.nodes.get(&current) else {
                continue;
            };
            out.extend_from_slice(&node.elements);
            queue.extend(node.children.iter().copied());
        }
        out
    }

    /// The chain of containers above `id`, nearest first.
    ///
    /// Empty for a root. Terminates even if the file contains a containment
    /// cycle, because the walk stops at the first repeated entity.
    #[must_use]
    pub fn ancestors(&self, id: EntityId) -> Vec<EntityId> {
        let mut out = Vec::new();
        let mut seen = std::collections::BTreeSet::new();
        let mut current = self.nodes.get(&id).and_then(|node| node.parent);
        while let Some(parent) = current {
            if !seen.insert(parent) {
                break;
            }
            out.push(parent);
            current = self.nodes.get(&parent).and_then(|node| node.parent);
        }
        out
    }

    /// The container holding `element`, if any relationship places it.
    ///
    /// Answers the question a drawing or take-off actually asks: which storey
    /// is this wall on? Backed by a map built during assembly, so this is a
    /// lookup rather than a scan over every container.
    #[must_use]
    pub fn container_of(&self, element: EntityId) -> Option<EntityId> {
        self.container_of_element.get(&element).copied()
    }

    /// Relationship/target pairs naming an entity the model does not contain.
    #[must_use]
    pub fn dangling(&self) -> &[(EntityId, EntityId)] {
        &self.dangling
    }

    /// Containers that no relationship places under a parent, excluding the
    /// most-general root.
    ///
    /// A conformant file has none: every site is aggregated into the project,
    /// every building into a site. Entries here mean the file omits an
    /// aggregation relationship, which a viewer would show as a detached
    /// branch.
    #[must_use]
    pub fn orphans(&self) -> &[EntityId] {
        &self.orphans
    }
}
