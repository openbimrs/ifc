//! One primitive model write, stored as the exact write that undoes it.
//!
//! A [`Step`] records enough to put the model back byte for byte: not only
//! the prior value but the position in file order and in the type-index
//! bucket, which a plain "create again" could never recover. Applying a step
//! returns the step that undoes the application, so undo and redo are the
//! same operation.
//!
//! Steps are crate-private. They are produced by the `Model` primitives while
//! a recording is open (see `record.rs`) and consumed by `Inverse::apply`.
//! `apply` does not bump the revision; the `Inverse` bumps it once for the
//! whole application.

use std::sync::Arc;

use crate::entity::Entity;
use crate::header::Header;
use crate::index::Footprint;
use crate::lazy::Slot;
use crate::model::Model;
use crate::value::{EntityId, Value};

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Step {
    /// Undoes an insert into a free id.
    Remove { id: EntityId },
    /// Undoes a removal: puts the entity back at its file position and
    /// type-bucket position.
    Insert {
        id: EntityId,
        entity: Entity,
        order: usize,
        bucket: usize,
    },
    /// Undoes an insert over an occupant, keeping its file position.
    Replace {
        id: EntityId,
        entity: Entity,
        bucket: usize,
    },
    /// Restores one slot; `None` means the slot was past the end. `len` is
    /// the attribute count to restore.
    Slot {
        id: EntityId,
        slot: usize,
        value: Option<Value>,
        len: usize,
    },
    /// Restores a type name and the id's position in its old bucket.
    Retype {
        id: EntityId,
        type_name: Arc<str>,
        bucket: usize,
    },
    /// Restores the header.
    Header(Box<Header>),
}

impl Step {
    /// Applies the step and returns the step that undoes it.
    ///
    /// A step applied to a state it was not recorded against is an invariant
    /// violation and panics. `Inverse::apply` checks the revision first, and
    /// revisions are process-unique, so a model other than the recorded one
    /// (or a diverged clone) is refused before any step runs; that cannot
    /// happen through the public API.
    pub(crate) fn apply(self, model: &mut Model) -> Step {
        match self {
            Step::Remove { id } => {
                let order = model.order_position(id);
                model.order_mut().remove(order);
                let built = model.derived_is_built();
                let entity = model
                    .take_entity(id)
                    .expect("a Remove step names a stored entity");
                model.note_write(id, built.then(|| Footprint::of(&entity)), None);
                let key = entity.type_name.to_ascii_uppercase();
                let bucket = model.bucket_position(&key, id);
                remove_from_bucket(model, &key, bucket);
                Step::Insert {
                    id,
                    entity,
                    order,
                    bucket,
                }
            }
            Step::Insert {
                id,
                entity,
                order,
                bucket,
            } => {
                let key = entity.type_name.to_ascii_uppercase();
                let after = model.derived_is_built().then(|| Footprint::of(&entity));
                model.note_write(id, None, after);
                model.entities_mut().insert(id, Slot::decoded(entity));
                model.order_mut().insert(order, id);
                insert_into_bucket(model, key, bucket, id);
                let max_id = model.max_id().max(id.0);
                model.set_max_id(max_id);
                Step::Remove { id }
            }
            Step::Replace { id, entity, bucket } => {
                let new_key = entity.type_name.to_ascii_uppercase();
                let current = model
                    .take_entity(id)
                    .expect("a Replace step names a stored entity");
                let current_key = current.type_name.to_ascii_uppercase();
                let current_bucket = model.bucket_position(&current_key, id);
                if model.derived_is_built() {
                    let (before, after) = (Footprint::of(&current), Footprint::of(&entity));
                    model.note_write(id, Some(before), Some(after));
                }
                model.entities_mut().insert(id, Slot::decoded(entity));
                if current_key != new_key {
                    remove_from_bucket(model, &current_key, current_bucket);
                    insert_into_bucket(model, new_key, bucket, id);
                }
                Step::Replace {
                    id,
                    entity: current,
                    bucket: current_bucket,
                }
            }
            Step::Slot {
                id,
                slot,
                value,
                len,
            } => {
                let built = model.derived_is_built();
                let entity = model
                    .entity_mut(id)
                    .expect("a Slot step names a stored entity");
                let old_len = entity.attributes.len();
                // The slot itself and any slots the restored length drops.
                // The `len..old_len` part is defensive: LIFO undo through
                // `Inverse`/`History` never restores a shorter list.
                let slots: Vec<usize> = if built {
                    std::iter::once(slot).chain(len..old_len).collect()
                } else {
                    Vec::new()
                };
                let before = built.then(|| Footprint::of_slots(entity, &slots));
                let old = (slot < old_len)
                    .then(|| std::mem::replace(&mut entity.attributes[slot], Value::Null));
                if let Some(value) = value {
                    if slot >= entity.attributes.len() {
                        entity.attributes.resize(slot + 1, Value::Null);
                    }
                    entity.attributes[slot] = value;
                }
                entity.attributes.resize(len, Value::Null);
                let after = built.then(|| Footprint::of_slots(entity, &slots));
                model.note_write(id, before, after);
                Step::Slot {
                    id,
                    slot,
                    value: old,
                    len: old_len,
                }
            }
            Step::Retype {
                id,
                type_name,
                bucket,
            } => {
                let entity = model
                    .entity_mut(id)
                    .expect("a Retype step names a stored entity");
                let current = std::mem::replace(&mut entity.type_name, type_name.clone());
                let current_key = current.to_ascii_uppercase();
                let new_key = type_name.to_ascii_uppercase();
                let current_bucket = if current_key == new_key {
                    0
                } else {
                    let position = model.bucket_position(&current_key, id);
                    remove_from_bucket(model, &current_key, position);
                    insert_into_bucket(model, new_key, bucket, id);
                    position
                };
                Step::Retype {
                    id,
                    type_name: current,
                    bucket: current_bucket,
                }
            }
            Step::Header(header) => {
                let current = std::mem::replace(model.header_raw(), *header);
                Step::Header(Box::new(current))
            }
        }
    }
}

/// Drops the entry at `position` from bucket `key`, and the bucket once it is
/// empty (as `Model::unindex` does).
fn remove_from_bucket(model: &mut Model, key: &str, position: usize) {
    let buckets = model.by_type_mut();
    if let Some(ids) = buckets.get_mut(key) {
        ids.remove(position);
        if ids.is_empty() {
            buckets.remove(key);
        }
    }
}

fn insert_into_bucket(model: &mut Model, key: String, position: usize, id: EntityId) {
    model
        .by_type_mut()
        .entry(key)
        .or_default()
        .insert(position, id);
}
