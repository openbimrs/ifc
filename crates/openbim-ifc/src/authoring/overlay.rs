//! The model as the batch so far would leave it.
//!
//! A [`Transaction`] cannot be read back, and later operations of a batch
//! must see what earlier ones created, edited and removed: a product is
//! contained in the storey the batch just made. The overlay keeps those
//! changes beside the untouched base model, answers reads through them,
//! and only at the end emits one transaction holding all of them.
//!
//! # Cost
//!
//! Reads of the base model go through the indexes it keeps (its reverse
//! index, ADR 0023). The overlay keeps two of its own over the batch's
//! records: who references a target, and which created entities have a
//! type. Every lookup then costs the answer plus the batch's own records
//! that match, never the size of the model or of the batch (#587).

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::sync::Arc;

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
    /// Per target, the created and changed records that reference it.
    batch_referrers: HashMap<EntityId, BTreeSet<EntityId>>,
    /// Per exact stored type name, the entities the batch created.
    created_by_type: HashMap<Arc<str>, BTreeSet<EntityId>>,
}

impl<'m> Overlay<'m> {
    pub(super) fn new(base: &'m Model) -> Self {
        Self {
            base,
            next_id: base.next_id().0,
            created: BTreeMap::new(),
            changed: BTreeMap::new(),
            removed: BTreeSet::new(),
            batch_referrers: HashMap::new(),
            created_by_type: HashMap::new(),
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
        if let Some(old) = self.created.remove(&id) {
            self.unlink(id, &old);
            self.unbucket(id, &old);
        }
        self.link(id, &entity);
        self.created_by_type
            .entry(Arc::clone(&entity.type_name))
            .or_default()
            .insert(id);
        self.created.insert(id, entity);
    }

    /// Replace an entity the batch created or the base holds.
    pub(super) fn replace(&mut self, id: EntityId, entity: Entity) {
        if self.created.contains_key(&id) {
            let old = self.created.remove(&id).expect("checked");
            self.unlink(id, &old);
            self.link(id, &entity);
            if old.type_name != entity.type_name {
                self.unbucket(id, &old);
                self.created_by_type
                    .entry(Arc::clone(&entity.type_name))
                    .or_default()
                    .insert(id);
            }
            self.created.insert(id, entity);
        } else {
            if let Some(old) = self.changed.remove(&id) {
                self.unlink(id, &old);
            }
            self.link(id, &entity);
            self.changed.insert(id, entity);
        }
    }

    pub(super) fn remove(&mut self, id: EntityId) {
        if let Some(old) = self.created.remove(&id) {
            self.unlink(id, &old);
            self.unbucket(id, &old);
        } else {
            if let Some(old) = self.changed.remove(&id) {
                self.unlink(id, &old);
            }
            self.removed.insert(id);
        }
    }

    /// Record that the batch's record `id` references what `entity` does.
    fn link(&mut self, id: EntityId, entity: &Entity) {
        for attribute in &entity.attributes {
            attribute.for_each_ref(&mut |target| {
                self.batch_referrers.entry(target).or_default().insert(id);
            });
        }
    }

    fn unlink(&mut self, id: EntityId, entity: &Entity) {
        for attribute in &entity.attributes {
            attribute.for_each_ref(&mut |target| {
                if let Some(holders) = self.batch_referrers.get_mut(&target) {
                    holders.remove(&id);
                    if holders.is_empty() {
                        self.batch_referrers.remove(&target);
                    }
                }
            });
        }
    }

    fn unbucket(&mut self, id: EntityId, entity: &Entity) {
        if let Some(ids) = self.created_by_type.get_mut(&entity.type_name) {
            ids.remove(&id);
            if ids.is_empty() {
                self.created_by_type.remove(&entity.type_name);
            }
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
                self.created_by_type
                    .get(type_name)
                    .into_iter()
                    .flatten()
                    .copied(),
            )
            .collect()
    }

    /// Every entity holding a reference to `target`, in id order: the base
    /// model's referrers from its reverse index, minus those the batch
    /// removed or rewrote, plus the batch's own records that hold it.
    pub(super) fn referrers(&self, target: EntityId) -> Vec<EntityId> {
        let mut found: BTreeSet<EntityId> = self
            .base
            .reverse_index()
            .referrers(target)
            .iter()
            .map(|referrer| referrer.from)
            .filter(|id| !self.removed.contains(id) && !self.changed.contains_key(id))
            .collect();
        found.extend(self.batch_referrers.get(&target).into_iter().flatten());
        found.remove(&target);
        found.into_iter().collect()
    }

    /// Every entity of the model as the batch so far leaves it, in id
    /// order.
    pub(super) fn ids(&self) -> Vec<EntityId> {
        let mut ids: BTreeSet<EntityId> = self
            .base
            .ids()
            .filter(|id| !self.removed.contains(id))
            .collect();
        ids.extend(self.created.keys().copied());
        ids.into_iter().collect()
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
