//! Which elements each container lists, and in what order.
//!
//! Everything here reads what `ifc-spatial` already resolved: containment
//! and references from [`SpatialTree`], element aggregation from
//! [`ifc_spatial::relation::all`]. No relationship slot is read here.

use std::collections::{BTreeMap, BTreeSet};

use ifc_model::{EntityId, Model, Value};
use ifc_spatial::relation::{self, RelationshipKind};
use ifc_spatial::SpatialTree;

use super::{ContainerName, ElementMember, SpatialContainer, SpatialMembership};

/// `IfcRoot.Name`: GlobalId, OwnerHistory, Name, Description in IFC2X3 TC1,
/// IFC4 ADD2 TC1 and IFC4X3 ADD2 alike.
const ROOT_NAME: usize = 2;

/// One container with its members, in listing order.
#[derive(Debug, Clone)]
pub(super) struct Slot<'m> {
    pub(super) container: SpatialContainer<'m>,
    pub(super) members: Vec<ElementMember<'m>>,
}

/// Every container in spatial tree order with its members, and the
/// `(relationship, part)` pairs naming parts the model lacks.
pub(super) fn collect<'m>(
    model: &'m Model,
    tree: &SpatialTree,
) -> (Vec<Slot<'m>>, Vec<(EntityId, EntityId)>) {
    let (parts, dangling) = element_parts(model, tree);
    let slots = tree_order(tree)
        .into_iter()
        .filter_map(|id| {
            let container = container(model, tree, id)?;
            let members = members_of(model, tree, &parts, id);
            Some(Slot { container, members })
        })
        .collect();
    (slots, dangling)
}

/// Depth first from the roots, children in the tree's order; then any
/// container no root reaches, by id, so a cycle hides nothing.
fn tree_order(tree: &SpatialTree) -> Vec<EntityId> {
    let mut order = Vec::new();
    let mut seen = BTreeSet::new();
    let mut stack: Vec<EntityId> = tree.roots().iter().rev().copied().collect();
    while let Some(id) = stack.pop() {
        if !seen.insert(id) {
            continue;
        }
        order.push(id);
        if let Some(node) = tree.node(id) {
            stack.extend(node.children.iter().rev().copied());
        }
    }
    for node in tree.containers() {
        if seen.insert(node.id) {
            order.push(node.id);
        }
    }
    order
}

fn container<'m>(
    model: &'m Model,
    tree: &SpatialTree,
    id: EntityId,
) -> Option<SpatialContainer<'m>> {
    let node = tree.node(id)?;
    let entity = model.get(id)?;
    let name = match entity.attribute(ROOT_NAME) {
        None | Some(Value::Null) => ContainerName::Unset,
        Some(Value::Text(text)) => ContainerName::Text(text),
        Some(_) => ContainerName::Malformed,
    };
    Some(SpatialContainer {
        id,
        kind: node.kind,
        type_name: &entity.type_name,
        name,
        parent: node.parent,
    })
}

/// Composite element to its `IfcRelAggregates` parts, in file order.
type Parts = BTreeMap<EntityId, Vec<EntityId>>;

/// Element aggregation: every `IfcRelAggregates` whose whole is not a
/// spatial container (container aggregation is the tree itself).
fn element_parts(model: &Model, tree: &SpatialTree) -> (Parts, Vec<(EntityId, EntityId)>) {
    let mut parts = Parts::new();
    let mut dangling = Vec::new();
    for rel in relation::all(model) {
        if rel.kind != RelationshipKind::Aggregates {
            continue;
        }
        let Some(whole) = rel.relating else { continue };
        if tree.node(whole).is_some() || model.get(whole).is_none() {
            continue;
        }
        for part in rel.related {
            if model.get(part).is_none() {
                dangling.push((rel.id, part));
            } else {
                parts.entry(whole).or_default().push(part);
            }
        }
    }
    (parts, dangling)
}

/// Contained elements with their parts, then referenced elements, sorted by
/// element id and membership.
fn members_of<'m>(
    model: &'m Model,
    tree: &SpatialTree,
    parts: &Parts,
    container: EntityId,
) -> Vec<ElementMember<'m>> {
    let mut listed: Vec<(EntityId, SpatialMembership)> = Vec::new();
    let mut seen_parts = BTreeSet::new();
    for &element in tree.elements_of(container) {
        listed.push((element, SpatialMembership::Contained));
        // Iterative walk; `seen_parts` ends an aggregation cycle.
        let mut stack = vec![element];
        while let Some(whole) = stack.pop() {
            for &part in parts.get(&whole).map_or(&[][..], Vec::as_slice) {
                // A part the file contains itself is listed there, once.
                if tree.container_of(part).is_some() || !seen_parts.insert(part) {
                    continue;
                }
                listed.push((part, SpatialMembership::Part { whole }));
                stack.push(part);
            }
        }
    }
    for &element in tree.referenced_elements(container) {
        listed.push((element, SpatialMembership::Referenced));
    }
    listed.sort_unstable();
    listed
        .into_iter()
        .filter_map(|(element, membership)| {
            Some(ElementMember {
                element,
                type_name: &model.get(element)?.type_name,
                membership,
            })
        })
        .collect()
}
