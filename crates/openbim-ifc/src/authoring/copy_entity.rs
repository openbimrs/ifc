//! The low-level deep copy of an entity by an explicit policy, and what
//! every copy shares: the order a subgraph is copied in and the checked
//! write of a copy (#479).

use std::collections::{BTreeMap, BTreeSet};

use ifc_model::{Entity, EntityId, Value};

use super::check::{recheck, relations};
use super::kinds::{Kind, Kinds};
use super::plan::Planner;
use super::report::CopyReport;
use super::represent::remap;
use super::{AuthoringFailure as F, CopyPolicy};

impl Planner<'_, '_> {
    /// `AuthorOp::CopyEntity`.
    pub(super) fn copy_entity(
        &mut self,
        root: EntityId,
        policy: &CopyPolicy,
    ) -> Result<(EntityId, CopyReport), F> {
        if self.view.get(root).is_none() {
            return Err(F::MissingEntity(root));
        }
        for class in policy.copy.iter().chain(&policy.share) {
            if self.schema.entity(class).is_none() {
                return Err(F::UnknownEntity {
                    schema: self.schema.name().to_owned(),
                    type_name: class.clone(),
                });
            }
        }
        let kinds = Kinds::new(self.schema);
        let copied = |id: EntityId| {
            let Some(entity) = self.view.get(id) else {
                return false;
            };
            let t = &entity.type_name;
            if policy.copy.iter().any(|class| self.schema.is_a(t, class)) {
                return true;
            }
            if policy.share.iter().any(|class| self.schema.is_a(t, class)) || kinds.unique(t) {
                return false;
            }
            match kinds.of(t) {
                Kind::Definition | Kind::OwnerChain | Kind::Context => policy.copy_definitions,
                _ if kinds.is_root(t) => policy.copy_roots,
                _ => true,
            }
        };
        let order = self
            .subgraph_order(root, &copied)
            .map_err(|entity| F::UnsupportedCopy {
                entity,
                detail: "its references form a cycle".to_owned(),
            })?;
        let mut copies = BTreeMap::new();
        for original in order {
            let mut entity = self
                .view
                .get(original)
                .cloned()
                .ok_or(F::MissingEntity(original))?;
            for value in &mut entity.attributes {
                remap(value, &copies);
            }
            let copy = self.insert_copy(entity)?;
            copies.insert(original, copy);
        }
        let copy = copies[&root];
        let mut pairs: Vec<(EntityId, EntityId)> = copies.into_iter().collect();
        pairs.sort_by_key(|(_, copy)| *copy);
        Ok((
            copy,
            CopyReport {
                copied: self.copied_entities(pairs),
                ..CopyReport::default()
            },
        ))
    }

    /// `root` and every entity it reaches through references `follow`
    /// admits, each before what uses it; the entity closing a cycle as the
    /// error.
    pub(super) fn subgraph_order(
        &self,
        root: EntityId,
        follow: &dyn Fn(EntityId) -> bool,
    ) -> Result<Vec<EntityId>, EntityId> {
        let mut order = Vec::new();
        let mut done = BTreeSet::new();
        let mut path = BTreeSet::new();
        // Depth first, iteratively: (entity, whether its references are
        // queued already).
        let mut stack = vec![(root, false)];
        while let Some((id, expanded)) = stack.pop() {
            if expanded {
                path.remove(&id);
                if done.insert(id) {
                    order.push(id);
                }
                continue;
            }
            if done.contains(&id) {
                continue;
            }
            if !path.insert(id) {
                return Err(id);
            }
            stack.push((id, true));
            let Some(entity) = self.view.get(id) else {
                return Err(id);
            };
            let mut targets = Vec::new();
            for value in &entity.attributes {
                value.for_each_ref(&mut |target| targets.push(target));
            }
            for target in targets.into_iter().rev() {
                if !done.contains(&target) && follow(target) {
                    stack.push((target, false));
                }
            }
        }
        Ok(order)
    }

    /// Write a copy: an `IfcRoot` gets a new `GlobalId`; the record is
    /// checked as an edit checks one.
    pub(super) fn insert_copy(&mut self, mut entity: Entity) -> Result<EntityId, F> {
        let id = self.view.allocate();
        let root = self.schema.is_a(&entity.type_name, "IfcRoot");
        if root {
            let global_id = self.fresh_global_id(id, &entity.type_name)?;
            if let Some(slot) = entity.attributes.first_mut() {
                *slot = Value::Text(global_id.into());
            }
        }
        recheck(self.schema, &entity)?;
        relations(self.schema, &self.view, &entity, None)?;
        if root {
            self.claim_global_id(id, &entity)?;
        }
        self.view.create(id, entity);
        Ok(id)
    }
}
