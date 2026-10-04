//! The model as the batch so far would leave it.
//!
//! A [`Transaction`] cannot be read back, and later operations of a batch
//! must see what earlier ones created, edited and removed: a product is
//! contained in the storey the batch just made. The overlay keeps those
//! changes beside the untouched base model, answers reads through them,
//! and only at the end emits one transaction holding all of them.

use std::collections::{BTreeMap, BTreeSet};

use ifc_model::{Edit, Entity, EntityId, Model, Transaction, Value};

/// The base model plus the batch's staged changes.
pub(super) struct Overlay<'m> {
    base: &'m Model,
    next_id: u64,
    /// Entities the batch creates, by id; ids are allocated in ascending
    /// order, so this is creation order too.
    created: BTreeMap<EntityId, Entity>,
    /// Base entities the batch edits, as they will stand.
    changed: BTreeMap<EntityId, Entity>,
    /// Base entities the batch removes.
    removed: BTreeSet<EntityId>,
}

impl<'m> Overlay<'m> {
    pub(super) fn new(base: &'m Model) -> Self {
        Self {
            base,
            next_id: base.next_id().0,
            created: BTreeMap::new(),
            changed: BTreeMap::new(),
            removed: BTreeSet::new(),
        }
    }

    pub(super) const fn base(&self) -> &'m Model {
        self.base
    }

    /// The entity as the batch so far leaves it.
    pub(super) fn get(&self, id: EntityId) -> Option<&Entity> {
        if self.removed.contains(&id) {
            return None;
        }
        self.created
            .get(&id)
            .or_else(|| self.changed.get(&id))
            .or_else(|| self.base.get(id))
    }

    /// Reserve the id of the next entity.
    pub(super) fn allocate(&mut self) -> EntityId {
        let id = EntityId(self.next_id);
        self.next_id += 1;
        id
    }

    pub(super) fn create(&mut self, id: EntityId, entity: Entity) {
        self.created.insert(id, entity);
    }

    /// Replace an entity the batch created or the base holds.
    pub(super) fn replace(&mut self, id: EntityId, entity: Entity) {
        if let Some(slot) = self.created.get_mut(&id) {
            *slot = entity;
        } else {
            self.changed.insert(id, entity);
        }
    }

    pub(super) fn remove(&mut self, id: EntityId) {
        if self.created.remove(&id).is_none() {
            self.changed.remove(&id);
            self.removed.insert(id);
        }
    }

    /// Ids of every entity of exactly `type_name` (upper case).
    pub(super) fn ids_of_type(&self, type_name: &str) -> Vec<EntityId> {
        self.base
            .ids_of_type(type_name)
            .iter()
            .copied()
            .filter(|id| !self.removed.contains(id))
            .chain(
                self.created
                    .iter()
                    .filter(|(_, entity)| &*entity.type_name == type_name)
                    .map(|(id, _)| *id),
            )
            .collect()
    }

    /// Every entity holding a reference to `target`, in id order.
    pub(super) fn referrers(&self, target: EntityId) -> Vec<EntityId> {
        let mut found: BTreeSet<EntityId> = self
            .base
            .iter()
            .filter(|(id, _)| !self.removed.contains(id) && !self.changed.contains_key(id))
            .filter(|(_, entity)| holds(&entity.attributes, target))
            .map(|(id, _)| id)
            .collect();
        found.extend(
            self.created
                .iter()
                .chain(&self.changed)
                .filter(|(_, entity)| holds(&entity.attributes, target))
                .map(|(id, _)| *id),
        );
        found.remove(&target);
        found.into_iter().collect()
    }

    /// One transaction holding every staged change.
    pub(super) fn into_transaction(self) -> Transaction {
        let mut tx = Transaction::new(self.base);
        for (id, entity) in self.created {
            tx.stage(Edit::Create { id, entity });
        }
        for (id, entity) in self.changed {
            let before = self.base.get(id).map(|e| e.attributes.as_slice());
            for (slot, value) in entity.attributes.into_iter().enumerate() {
                if before.and_then(|b| b.get(slot)) != Some(&value) {
                    tx.set_attribute(id, slot, value);
                }
            }
        }
        for id in self.removed {
            tx.remove(id);
        }
        tx
    }
}

/// Whether `values` reference `target`, at any depth.
pub(super) fn holds(values: &[Value], target: EntityId) -> bool {
    values.iter().any(|value| match value {
        Value::Ref(id) => *id == target,
        Value::List(items) => holds(items, target),
        Value::Typed { value, .. } => holds(std::slice::from_ref(value), target),
        _ => false,
    })
}
