//! Bounded construction-resource composition through `IfcRelNests`.

use std::collections::{HashMap, HashSet};

use ifc_model::{Budget, EntityId};

use crate::error::{ResourceError, ResourceResult};
use crate::view::ResourceView;

struct NestingIndex {
    children: HashMap<EntityId, Vec<EntityId>>,
    parent: HashMap<EntityId, EntityId>,
}

impl<'m, 's> ResourceView<'m, 's> {
    fn nesting_index(&self) -> ResourceResult<NestingIndex> {
        let mut children: HashMap<EntityId, Vec<EntityId>> = HashMap::new();
        let mut parent = HashMap::new();

        for relation in self.ids_of_ancestor("IfcRelNests") {
            let record = self.record(relation, "IfcRelNests")?;
            let relating = record.required_ref_select(
                "RelatingObject",
                "IfcObjectDefinition",
                &["IfcObjectDefinition"],
            )?;
            let relating_entity =
                self.model
                    .get(relating)
                    .ok_or(ResourceError::DanglingReference {
                        entity: relation,
                        attribute: "RelatingObject",
                        target: relating,
                    })?;
            if !self
                .schema
                .is_a(&relating_entity.type_name, "IfcConstructionResource")
            {
                continue;
            }

            let related =
                record.refs("RelatedObjects", "IfcConstructionResource", 1, false, false)?;
            for child in related {
                if child == relating {
                    return Err(ResourceError::SemanticViolation {
                        entity: Some(relation),
                        rule: "IfcRelNests must not nest a resource under itself",
                    });
                }
                if let Some(existing) = parent.insert(child, relating) {
                    if existing == relating {
                        return Err(ResourceError::DuplicateReference {
                            entity: relation,
                            attribute: "RelatedObjects",
                            target: child,
                        });
                    }
                    return Err(ResourceError::SemanticViolation {
                        entity: Some(child),
                        rule: "IfcObject.Nests permits at most one resource parent",
                    });
                }
                children.entry(relating).or_default().push(child);
            }
        }
        Ok(NestingIndex { children, parent })
    }

    /// Return the authored construction-resource parent, if any.
    pub fn parent_resource(&self, child: EntityId) -> ResourceResult<Option<EntityId>> {
        self.resource(child)?;
        Ok(self.nesting_index()?.parent.get(&child).copied())
    }

    /// Return direct authored members in relation/LIST order.
    pub fn direct_members(&self, parent: EntityId) -> ResourceResult<Vec<EntityId>> {
        self.resource(parent)?;
        Ok(self
            .nesting_index()?
            .children
            .remove(&parent)
            .unwrap_or_default())
    }

    /// Return depth-first authored descendants within explicit limits.
    pub fn descendants(&self, root: EntityId, budget: Budget) -> ResourceResult<Vec<EntityId>> {
        self.resource(root)?;
        if budget.max_nodes == 0 {
            return Err(ResourceError::BudgetExceeded {
                max_depth: budget.max_depth,
                max_nodes: budget.max_nodes,
            });
        }

        let index = self.nesting_index()?;
        let mut result = Vec::new();
        let mut visited = HashSet::from([root]);
        let mut stack = vec![(root, 0_usize)];
        while let Some((parent, depth)) = stack.pop() {
            let children = index
                .children
                .get(&parent)
                .map(Vec::as_slice)
                .unwrap_or_default();
            if !children.is_empty() && depth >= budget.max_depth {
                return Err(ResourceError::BudgetExceeded {
                    max_depth: budget.max_depth,
                    max_nodes: budget.max_nodes,
                });
            }
            for child in children {
                if !visited.insert(*child) {
                    return Err(ResourceError::Cycle { at: *child });
                }
                if visited.len() > budget.max_nodes {
                    return Err(ResourceError::BudgetExceeded {
                        max_depth: budget.max_depth,
                        max_nodes: budget.max_nodes,
                    });
                }
                result.push(*child);
            }
            stack.extend(children.iter().rev().map(|child| (*child, depth + 1)));
        }
        Ok(result)
    }
}
