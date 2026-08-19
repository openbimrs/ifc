//! Immutable geometry DAG and append-only builder.

use core::fmt;

use crate::{GeometryNode, NodeId};

/// Invalid graph construction.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GraphError {
    /// A node referenced itself or a later/not-yet-inserted node.
    NonPriorReference { node: NodeId, reference: NodeId },
    /// A requested root does not exist.
    UnknownRoot { root: NodeId, node_count: usize },
}

impl fmt::Display for GraphError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NonPriorReference { node, reference } => {
                write!(f, "{node} references non-prior {reference}")
            }
            Self::UnknownRoot { root, node_count } => {
                write!(f, "root {root} exceeds graph size {node_count}")
            }
        }
    }
}

impl std::error::Error for GraphError {}

/// Immutable acyclic geometry graph with one or more roots.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct GeometryGraph {
    nodes: Vec<GeometryNode>,
    roots: Vec<NodeId>,
}

impl GeometryGraph {
    /// Number of nodes.
    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    /// Whether the graph has no nodes.
    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    /// Read a node by typed handle.
    pub fn get(&self, id: NodeId) -> Option<&GeometryNode> {
        self.nodes.get(id.index())
    }

    /// Output roots in caller-specified order.
    pub fn roots(&self) -> &[NodeId] {
        &self.roots
    }

    /// All nodes in stable topological insertion order.
    pub fn iter(&self) -> impl ExactSizeIterator<Item = (NodeId, &GeometryNode)> {
        self.nodes
            .iter()
            .enumerate()
            .map(|(index, node)| (NodeId::from_index(index), node))
    }
}

/// Append-only builder that makes cycles and dangling references unrepresentable.
#[derive(Debug, Default)]
pub struct GeometryGraphBuilder {
    nodes: Vec<GeometryNode>,
}

impl GeometryGraphBuilder {
    /// Create an empty builder.
    pub const fn new() -> Self {
        Self { nodes: Vec::new() }
    }

    /// Insert a node. Every reference must be to an earlier node.
    pub fn push(&mut self, node: GeometryNode) -> Result<NodeId, GraphError> {
        let id = NodeId::from_index(self.nodes.len());
        if let Some(reference) = node
            .references()
            .into_iter()
            .find(|reference| reference.index() >= id.index())
        {
            return Err(GraphError::NonPriorReference {
                node: id,
                reference,
            });
        }
        self.nodes.push(node);
        Ok(id)
    }

    /// Freeze the graph after validating roots.
    pub fn finish(self, roots: Vec<NodeId>) -> Result<GeometryGraph, GraphError> {
        if let Some(&root) = roots.iter().find(|root| root.index() >= self.nodes.len()) {
            return Err(GraphError::UnknownRoot {
                root,
                node_count: self.nodes.len(),
            });
        }
        Ok(GeometryGraph {
            nodes: self.nodes,
            roots,
        })
    }
}

#[cfg(test)]
mod tests {
    use geom_core::Vec3;

    use super::*;
    use crate::Instance;

    #[test]
    fn insertion_order_is_topological_order() {
        let mut builder = GeometryGraphBuilder::new();
        let source = builder.push(GeometryNode::Point3(Vec3::ZERO)).unwrap();
        let instance = builder
            .push(GeometryNode::Instance(Instance {
                source,
                transform: geom_core::Transform3::IDENTITY,
            }))
            .unwrap();
        let graph = builder.finish(vec![instance]).unwrap();
        assert_eq!(graph.len(), 2);
        assert_eq!(graph.roots(), &[instance]);
    }
}
