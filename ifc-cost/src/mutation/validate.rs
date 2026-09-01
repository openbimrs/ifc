use std::collections::{HashMap, HashSet};

use ifc_model::{guid::Guid, Edit, EntityId, Model, Transaction, Value};

use super::{CostAuthoringError, CostAuthoringResult};

pub(crate) fn guid(
    tx: &Transaction,
    model: &Model,
    entity: &'static str,
    value: &str,
) -> CostAuthoringResult<()> {
    if Guid::parse(value).is_none() {
        return Err(invalid(
            entity,
            "GlobalId",
            "expected a compressed IFC GUID",
        ));
    }
    let mut projected: HashMap<EntityId, String> = model
        .iter()
        .filter_map(|(id, record)| record.text(0).map(|global_id| (id, global_id.to_owned())))
        .collect();
    for edit in tx.edits() {
        match edit {
            Edit::Create { id, entity } => {
                if let Some(global_id) = entity.text(0) {
                    projected.insert(*id, global_id.to_owned());
                } else {
                    projected.remove(id);
                }
            }
            Edit::SetAttribute { id, slot: 0, value } if projected.contains_key(id) => {
                if let Some(global_id) = value.unwrap_typed().as_text() {
                    projected.insert(*id, global_id.to_owned());
                } else {
                    projected.remove(id);
                }
            }
            Edit::Remove { id } => {
                projected.remove(id);
            }
            Edit::SetAttribute { .. } | Edit::Retype { .. } => {}
        }
    }
    if projected.values().any(|global_id| global_id == value) {
        return Err(invalid(entity, "GlobalId", "duplicate GlobalId"));
    }
    Ok(())
}

pub(crate) fn reference_type(
    tx: &Transaction,
    model: &Model,
    owner: &'static str,
    attribute: &'static str,
    target: EntityId,
    expected: &'static str,
) -> CostAuthoringResult<()> {
    let Some(actual) = projected_type(tx, model, target) else {
        return Err(CostAuthoringError::MissingReference {
            entity: owner,
            attribute,
            target,
        });
    };
    if !actual.eq_ignore_ascii_case(expected) {
        return Err(CostAuthoringError::WrongReferenceType {
            entity: owner,
            attribute,
            target,
            actual,
            expected,
        });
    }
    Ok(())
}

pub(crate) fn non_empty_unique(
    entity: &'static str,
    attribute: &'static str,
    ids: &[EntityId],
) -> CostAuthoringResult<()> {
    if ids.is_empty() {
        return Err(invalid(entity, attribute, "expected at least one member"));
    }
    let mut seen = HashSet::new();
    if ids.iter().any(|id| !seen.insert(*id)) {
        return Err(invalid(
            entity,
            attribute,
            "duplicate members are not allowed",
        ));
    }
    Ok(())
}

pub(crate) fn validate_nesting(
    tx: &Transaction,
    model: &Model,
    parent: EntityId,
    children: &[EntityId],
) -> CostAuthoringResult<()> {
    let edges = nesting_edges(tx, model);
    let mut parents = HashMap::new();
    let mut next: HashMap<EntityId, Vec<EntityId>> = HashMap::new();
    for (from, to) in edges {
        parents.entry(to).or_insert(from);
        next.entry(from).or_default().push(to);
    }
    for child in children {
        if *child == parent {
            return Err(CostAuthoringError::NestingCycle { item: *child });
        }
        if let Some(existing_parent) = parents.get(child) {
            return Err(CostAuthoringError::MultipleParents {
                child: *child,
                existing_parent: *existing_parent,
            });
        }
        if reachable(&next, *child, parent) {
            return Err(CostAuthoringError::NestingCycle { item: *child });
        }
    }
    Ok(())
}

fn projected_type(tx: &Transaction, model: &Model, target: EntityId) -> Option<String> {
    for edit in tx.edits().iter().rev() {
        match edit {
            Edit::Remove { id } if *id == target => return None,
            Edit::Retype { id, type_name } if *id == target => return Some(type_name.to_string()),
            Edit::Create { id, entity } if *id == target => {
                return Some(entity.type_name.to_string())
            }
            _ => {}
        }
    }
    model.get(target).map(|entity| entity.type_name.to_string())
}

fn nesting_edges(tx: &Transaction, model: &Model) -> Vec<(EntityId, EntityId)> {
    let mut projected: HashMap<EntityId, _> = model
        .iter()
        .map(|(id, entity)| (id, entity.clone()))
        .collect();
    for edit in tx.edits() {
        match edit {
            Edit::Create { id, entity } => {
                projected.insert(*id, entity.clone());
            }
            Edit::SetAttribute { id, slot, value } => {
                if let Some(entity) = projected.get_mut(id) {
                    if *slot >= entity.attributes.len() {
                        entity.attributes.resize(*slot + 1, Value::Null);
                    }
                    entity.attributes[*slot] = value.clone();
                }
            }
            Edit::Retype { id, type_name } => {
                if let Some(entity) = projected.get_mut(id) {
                    entity.type_name = type_name.clone();
                }
            }
            Edit::Remove { id } => {
                projected.remove(id);
            }
        }
    }
    let mut out = Vec::new();
    for entity in projected
        .values()
        .filter(|entity| entity.is_type("IFCRELNESTS"))
    {
        append_edges(entity.attribute(4), entity.attribute(5), &mut out);
    }
    out
}

fn append_edges(
    parent: Option<&Value>,
    children: Option<&Value>,
    out: &mut Vec<(EntityId, EntityId)>,
) {
    let Some(Value::Ref(parent)) = parent else {
        return;
    };
    if let Some(children) = children {
        children.for_each_ref(&mut |child| out.push((*parent, child)));
    }
}

fn reachable(next: &HashMap<EntityId, Vec<EntityId>>, start: EntityId, goal: EntityId) -> bool {
    let mut stack = vec![start];
    let mut seen = HashSet::new();
    while let Some(node) = stack.pop() {
        if node == goal {
            return true;
        }
        if seen.insert(node) {
            stack.extend(next.get(&node).into_iter().flatten().copied());
        }
    }
    false
}

pub(crate) fn invalid(
    entity: &'static str,
    attribute: &'static str,
    reason: impl Into<String>,
) -> CostAuthoringError {
    CostAuthoringError::InvalidValue {
        entity,
        attribute,
        reason: reason.into(),
    }
}
