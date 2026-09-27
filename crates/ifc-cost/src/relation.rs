//! Cost item nesting and control assignment.
//!
//! # Two different relationships, two different meanings
//!
//! ```text
//! IfcRelNests             cost item -> sub-items      (the breakdown tree)
//!   4 RelatingObject      the parent
//!   5 RelatedObjects      LIST of children, order significant
//!
//! IfcRelAssignsToControl  cost item -> products       (what it prices)
//!   4 RelatedObjects      SET of assigned objects
//!   5 RelatedObjectsType
//!   6 RelatingControl     the cost item
//! ```
//!
//! `RelatingControl` is slot **6**, not 5: `IfcRelAssigns` contributes both
//! `RelatedObjects` and `RelatedObjectsType` before the subtype's own field.
//!
//! # Why nesting needs a cycle budget
//!
//! `IfcRelNests` has a `NoSelfReference` WHERE rule, so a file cannot state
//! that an item nests itself directly. It says nothing about longer cycles:
//! A nests B nests A satisfies every stated rule and is what a naive recursive
//! total does not survive. Depth is therefore bounded and a cycle is reported
//! as data, not discovered as a stack overflow.

use std::collections::{HashMap, HashSet};

use ifc_model::{EntityId, Model, Value};

/// `IfcRelNests` slots.
mod nests {
    /// `RelatingObject`, the parent.
    pub const RELATING: usize = 4;
    /// `RelatedObjects`, the children.
    pub const RELATED: usize = 5;
}

/// `IfcRelAssignsToControl` slots.
mod assigns {
    /// `RelatedObjects`, what the control applies to.
    pub const RELATED: usize = 4;
    /// `RelatingControl`, the controlling entity.
    pub const RELATING: usize = 6;
}

/// The maximum nesting depth walked before reporting a runaway tree.
///
/// Cost breakdowns are authored by humans and are shallow in practice; a tree
/// deeper than this is a malformed or adversarial file, not a real estimate.
pub const MAX_NESTING_DEPTH: usize = 64;

/// A refusal to resolve a cost relationship.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum CostRelationError {
    /// A nesting cycle was found: the item is its own ancestor.
    NestingCycle {
        /// The item the walk returned to.
        repeated: EntityId,
        /// The path taken, from the root to the repeat.
        path: Vec<EntityId>,
    },
    /// The tree exceeded [`MAX_NESTING_DEPTH`] without terminating.
    DepthExceeded {
        /// Where the walk stopped.
        at: EntityId,
        /// The budget that was exhausted.
        limit: usize,
    },
}

/// A nesting the file states but the schema forbids, and how it was resolved.
///
/// IFC4 `IfcObjectDefinition.Nests` and IFC2X3 `Decomposes` are both
/// `SET [0:1]`: a cost item has at most one parent. When a file names two,
/// the first `IfcRelNests` in file order wins everywhere ([`parent_of`],
/// [`children_of`], [`descendants_of`], the rollups), so an item is never
/// counted under two parents. The losing claim is reported here.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum CostAnomaly {
    /// A cost item nested under two different parents.
    NestedTwice {
        /// The item with two parents.
        item: EntityId,
        /// The parent kept.
        kept: EntityId,
        /// The parent rejected; it does not list the item.
        rejected: EntityId,
        /// The `IfcRelNests` that was rejected.
        relation: EntityId,
    },
}

/// Each nested child's kept parent: the first `IfcRelNests` naming it.
fn kept_parents(model: &Model) -> HashMap<EntityId, EntityId> {
    let mut kept = HashMap::new();
    for (_, entity) in model.of_type("IFCRELNESTS") {
        let Some(Value::Ref(parent)) = entity.attribute(nests::RELATING) else {
            continue;
        };
        if let Some(v) = entity.attribute(nests::RELATED) {
            v.for_each_ref(&mut |child| {
                kept.entry(child).or_insert(*parent);
            });
        }
    }
    kept
}

/// Second parents the file states for nested items, in file order.
///
/// Empty for a conformant file. Restating the same parent in another
/// relationship is redundant, not contradictory, and is not reported.
#[must_use]
pub fn nesting_anomalies(model: &Model) -> Vec<CostAnomaly> {
    let kept = kept_parents(model);
    let mut out = Vec::new();
    for (relation, entity) in model.of_type("IFCRELNESTS") {
        let Some(Value::Ref(parent)) = entity.attribute(nests::RELATING) else {
            continue;
        };
        if let Some(v) = entity.attribute(nests::RELATED) {
            v.for_each_ref(&mut |item| {
                if let Some(&first) = kept.get(&item) {
                    if first != *parent {
                        out.push(CostAnomaly::NestedTwice {
                            item,
                            kept: first,
                            rejected: *parent,
                            relation,
                        });
                    }
                }
            });
        }
    }
    out
}

/// Direct children of a cost item, in authored order.
///
/// Order is preserved: `RelatedObjects` is a LIST in `IfcRelNests`, so a file
/// stating `[Labour, Plant, Material]` means that sequence, and a caller
/// rendering a breakdown should not reorder it.
///
/// Only children whose kept parent is `parent` are listed, each once, so this
/// agrees with [`parent_of`]. A child a malformed file also nests elsewhere
/// is listed under its first parent only; see [`nesting_anomalies`].
#[must_use]
pub fn children_of(model: &Model, parent: EntityId) -> Vec<EntityId> {
    let kept = kept_parents(model);
    let mut out = Vec::new();
    for (_, entity) in model.of_type("IFCRELNESTS") {
        let relating = match entity.attribute(nests::RELATING) {
            Some(Value::Ref(id)) => *id,
            _ => continue,
        };
        if relating != parent {
            continue;
        }
        if let Some(v) = entity.attribute(nests::RELATED) {
            v.for_each_ref(&mut |id| {
                if kept.get(&id) == Some(&parent) && !out.contains(&id) {
                    out.push(id);
                }
            });
        }
    }
    out
}

/// The parent of a cost item, if it is nested.
///
/// `IfcObjectDefinition.Nests` is `SET [0:1]`, so an item has at most one
/// parent. A file stating two is malformed; the first in file order is
/// returned, every other view agrees with it, and [`nesting_anomalies`]
/// reports the rejected claim. [`parents_of`] still lists every claimant.
#[must_use]
pub fn parent_of(model: &Model, child: EntityId) -> Option<EntityId> {
    parents_of(model, child).into_iter().next()
}

/// Every parent claiming this item, for detecting malformed multi-parenting.
///
/// Returns more than one element only for a file that violates the `[0:1]`
/// cardinality of `Nests`.
#[must_use]
pub fn parents_of(model: &Model, child: EntityId) -> Vec<EntityId> {
    let mut out = Vec::new();
    for (_, entity) in model.of_type("IFCRELNESTS") {
        let mut names_child = false;
        if let Some(v) = entity.attribute(nests::RELATED) {
            v.for_each_ref(&mut |id| {
                if id == child {
                    names_child = true;
                }
            });
        }
        if !names_child {
            continue;
        }
        if let Some(Value::Ref(parent)) = entity.attribute(nests::RELATING) {
            if !out.contains(parent) {
                out.push(*parent);
            }
        }
    }
    out
}

/// Every descendant of a cost item, depth-first in authored order.
///
/// Excludes the root itself. Returns an error rather than looping if the file
/// states a cycle or an unreasonably deep tree.
///
/// # Errors
///
/// [`CostRelationError::NestingCycle`] if an item is its own ancestor, or
/// [`CostRelationError::DepthExceeded`] past [`MAX_NESTING_DEPTH`].
pub fn descendants_of(model: &Model, root: EntityId) -> Result<Vec<EntityId>, CostRelationError> {
    let mut out = Vec::new();
    let mut path = Vec::new();
    let mut on_path = HashSet::new();
    walk(model, root, &mut out, &mut path, &mut on_path)?;
    Ok(out)
}

fn walk(
    model: &Model,
    node: EntityId,
    out: &mut Vec<EntityId>,
    path: &mut Vec<EntityId>,
    on_path: &mut HashSet<EntityId>,
) -> Result<(), CostRelationError> {
    if path.len() >= MAX_NESTING_DEPTH {
        return Err(CostRelationError::DepthExceeded {
            at: node,
            limit: MAX_NESTING_DEPTH,
        });
    }
    path.push(node);
    on_path.insert(node);

    for child in children_of(model, node) {
        if on_path.contains(&child) {
            let mut cycle = path.clone();
            cycle.push(child);
            return Err(CostRelationError::NestingCycle {
                repeated: child,
                path: cycle,
            });
        }
        out.push(child);
        walk(model, child, out, path, on_path)?;
    }

    path.pop();
    on_path.remove(&node);
    Ok(())
}

/// Objects a cost item is assigned to price, via `IfcRelAssignsToControl`.
///
/// This is how a cost item reaches the products it costs. Deduplicated and
/// returned in file order.
#[must_use]
pub fn controlled_by(model: &Model, control: EntityId) -> Vec<EntityId> {
    let mut out = Vec::new();
    for (_, entity) in model.of_type("IFCRELASSIGNSTOCONTROL") {
        let relating = match entity.attribute(assigns::RELATING) {
            Some(Value::Ref(id)) => *id,
            _ => continue,
        };
        if relating != control {
            continue;
        }
        if let Some(v) = entity.attribute(assigns::RELATED) {
            v.for_each_ref(&mut |id| {
                if !out.contains(&id) {
                    out.push(id);
                }
            });
        }
    }
    out
}

/// The controls assigned to an object: the inverse of [`controlled_by`].
#[must_use]
pub fn controls_of(model: &Model, object: EntityId) -> Vec<EntityId> {
    let mut out = Vec::new();
    for (_, entity) in model.of_type("IFCRELASSIGNSTOCONTROL") {
        let mut names_object = false;
        if let Some(v) = entity.attribute(assigns::RELATED) {
            v.for_each_ref(&mut |id| {
                if id == object {
                    names_object = true;
                }
            });
        }
        if !names_object {
            continue;
        }
        if let Some(Value::Ref(control)) = entity.attribute(assigns::RELATING) {
            if !out.contains(control) {
                out.push(*control);
            }
        }
    }
    out
}
