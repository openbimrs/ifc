//! Validate a batch of edits, then apply it as a unit.
//!
//! # Why the direct edit methods are not enough
//!
//! [`Model::set_attribute`], [`Model::retype`] and [`Model::remove`] each do
//! one thing correctly and immediately. That is right for a codec and wrong
//! for an author: `remove` is documented to leave every reference to the
//! removed entity dangling, so deleting a storey that walls still reference
//! produces a file that parses and is wrong.
//!
//! A transaction closes that gap without changing the primitives. It collects
//! edits, checks them against a PROJECTED view of the model -- what would
//! exist if the whole batch applied -- and only then writes.
//!
//! # Atomicity without an undo log
//!
//! Every failure mode is decided during preflight. Once preflight returns
//! clean, apply consists of map inserts, slot writes and map removals against
//! entities already proven to exist. None of those can fail, so there is
//! nothing to roll back and no half-applied state to observe.
//!
//! That is a stronger guarantee than "we rolled back on error", because a
//! rollback path is itself code that can be wrong and is exercised only on
//! failure. Here the failure path never touches the model at all.
//!
//! # Projection is what makes co-dependent edits expressible
//!
//! Checking each edit against the CURRENT model would reject a transaction
//! that creates an entity and references it in the same batch, and would
//! accept one that removes an entity another edit still points at. Both
//! answers are wrong. Validating against the projected end state gets both
//! right, and is the reason "delete a storey and re-parent its walls" is a
//! single atomic edit rather than a sequence with a broken middle.
//!
//! ```
//! use ifc_model::{Entity, Model, Transaction, Value};
//!
//! let mut model = Model::new();
//! let storey = model.push(Entity::new("IFCBUILDINGSTOREY", vec![]));
//!
//! // A wall that references the storey.
//! let mut tx = Transaction::new(&model);
//! let wall = tx.create(Entity::new("IFCWALL", vec![Value::Ref(storey)]));
//! tx.commit(&mut model).expect("the storey exists");
//!
//! // Removing the storey alone is refused: the wall still points at it.
//! let mut tx = Transaction::new(&model);
//! tx.remove(storey);
//! assert!(tx.commit(&mut model).is_err());
//!
//! // Removing both together is fine -- nothing survives to dangle.
//! let mut tx = Transaction::new(&model);
//! tx.remove(storey);
//! tx.remove(wall);
//! assert!(tx.commit(&mut model).is_ok());
//! ```

use ahash::{AHashMap, AHashSet};

use crate::entity::Entity;
use crate::index::ReverseIndex;
use crate::model::Model;
use crate::mutation::conflict::Conflict;
use crate::value::{EntityId, Value};

/// One staged change.
///
/// Deliberately structural: there is no `set_name` or `set_material` here.
/// Domain-shaped authoring belongs in the crate that owns the domain, built
/// on top of these operations -- otherwise every schema concept leaks into
/// the model layer, which is the boundary this crate exists to hold.
#[derive(Debug, Clone, PartialEq)]
pub enum Edit {
    /// Create an entity under a reserved id.
    Create {
        /// The id reserved for it by [`Transaction::create`].
        id: EntityId,
        /// The entity to store.
        entity: Entity,
    },
    /// Replace one positional attribute.
    SetAttribute {
        /// The entity to edit.
        id: EntityId,
        /// Attribute slot.
        slot: usize,
        /// New value.
        value: Value,
    },
    /// Change an entity's type name, keeping its id and attributes.
    Retype {
        /// The entity to retype.
        id: EntityId,
        /// The new type name.
        type_name: std::sync::Arc<str>,
    },
    /// Delete an entity.
    Remove {
        /// The entity to delete.
        id: EntityId,
    },
}

/// A batch of edits, validated together and applied as a unit.
///
/// Opened against a model snapshot and carrying that snapshot's revision, so
/// a commit against a model that has since changed is refused rather than
/// applied to state the caller never saw.
#[derive(Debug, Clone)]
pub struct Transaction {
    revision: u64,
    next_id: u64,
    edits: Vec<Edit>,
}

impl Transaction {
    /// Open a transaction against a model.
    #[must_use]
    pub fn new(model: &Model) -> Self {
        Self {
            revision: model.revision(),
            next_id: model.next_id().0,
            edits: Vec::new(),
        }
    }

    /// The model revision this transaction was opened against.
    #[must_use]
    pub fn revision(&self) -> u64 {
        self.revision
    }

    /// The staged edits, in the order they were added.
    #[must_use]
    pub fn edits(&self) -> &[Edit] {
        &self.edits
    }

    /// Whether anything is staged.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.edits.is_empty()
    }

    /// Number of staged edits.
    #[must_use]
    pub fn len(&self) -> usize {
        self.edits.len()
    }

    /// Stage a new entity, returning the id reserved for it.
    ///
    /// The id is allocated from the transaction, not the model, so several
    /// creates in one batch cannot collide and the caller can reference a
    /// newly created entity from another edit in the same transaction before
    /// anything is written.
    pub fn create(&mut self, entity: Entity) -> EntityId {
        let id = EntityId(self.next_id);
        self.next_id += 1;
        self.edits.push(Edit::Create { id, entity });
        id
    }

    /// Stage an already-constructed edit.
    ///
    /// [`Transaction::create`] allocates ids and is what an author should
    /// use. This exists for replaying a batch that was built elsewhere --
    /// deserialized, or produced by another process -- where the edit list
    /// is data rather than a sequence of calls. Preflight checks it exactly
    /// the same way, so a replayed batch cannot bypass validation.
    pub fn stage(&mut self, edit: Edit) -> &mut Self {
        if let Edit::Create { id, .. } = &edit {
            // Keep the allocator ahead of any id staged this way, so a later
            // `create` cannot hand out an id this batch already occupies.
            self.next_id = self.next_id.max(id.0 + 1);
        }
        self.edits.push(edit);
        self
    }

    /// Stage an attribute write.
    pub fn set_attribute(&mut self, id: EntityId, slot: usize, value: Value) -> &mut Self {
        self.edits.push(Edit::SetAttribute { id, slot, value });
        self
    }

    /// Stage a type change.
    pub fn retype(&mut self, id: EntityId, type_name: impl Into<std::sync::Arc<str>>) -> &mut Self {
        self.edits.push(Edit::Retype {
            id,
            type_name: type_name.into(),
        });
        self
    }

    /// Stage a removal.
    pub fn remove(&mut self, id: EntityId) -> &mut Self {
        self.edits.push(Edit::Remove { id });
        self
    }

    /// Check every edit without touching the model.
    ///
    /// Returns every conflict found rather than the first: an author fixing a
    /// batch wants the whole list, and stopping at the first turns one review
    /// into N round trips.
    ///
    /// Conflicts are ordered by the edit that produced them, so the report
    /// reads in the order the caller wrote the batch.
    #[must_use]
    pub fn preflight(&self, model: &Model) -> Vec<Conflict> {
        let mut conflicts = Vec::new();

        if model.revision() != self.revision {
            conflicts.push(Conflict::StaleRevision {
                expected: self.revision,
                found: model.revision(),
            });
            // Every other check would be computed against a model the caller
            // never saw, so the results would be noise. Report the one fact
            // that matters and stop.
            return conflicts;
        }

        // --- project the end state -------------------------------------
        let mut created: AHashMap<EntityId, &Entity> = AHashMap::new();
        let mut removed: AHashSet<EntityId> = AHashSet::new();
        // Slot writes, keyed by entity then slot: a later edit to the same
        // slot wins, matching apply order.
        let mut writes: AHashMap<EntityId, AHashMap<usize, &Value>> = AHashMap::new();

        for edit in &self.edits {
            match edit {
                Edit::Create { id, entity } => {
                    created.insert(*id, entity);
                    removed.remove(id);
                }
                Edit::Remove { id } => {
                    removed.insert(*id);
                }
                Edit::SetAttribute { id, slot, value } => {
                    writes.entry(*id).or_default().insert(*slot, value);
                }
                Edit::Retype { .. } => {}
            }
        }

        let exists = |id: EntityId| -> bool {
            !removed.contains(&id) && (created.contains_key(&id) || model.get(id).is_some())
        };

        // --- per-edit checks -------------------------------------------
        for (index, edit) in self.edits.iter().enumerate() {
            match edit {
                Edit::Create { id, entity } => {
                    if model.get(*id).is_some() {
                        conflicts.push(Conflict::IdAlreadyExists {
                            edit: index,
                            id: *id,
                        });
                    }
                    for (slot, attribute) in entity.attributes.iter().enumerate() {
                        // A slot overwritten later in the same batch is not
                        // what will be stored, so checking it would reject a
                        // batch that is actually coherent.
                        if writes.get(id).is_some_and(|w| w.contains_key(&slot)) {
                            continue;
                        }
                        check_refs(attribute, index, *id, slot, &exists, &mut conflicts);
                    }
                }
                Edit::SetAttribute { id, slot, value } => {
                    if !exists(*id) {
                        conflicts.push(Conflict::MissingTarget {
                            edit: index,
                            id: *id,
                        });
                        continue;
                    }
                    // Only the winning write for a slot is checked; an earlier
                    // superseded one never reaches the model.
                    if writes
                        .get(id)
                        .and_then(|w| w.get(slot))
                        .is_some_and(|winner| !std::ptr::eq(*winner, value))
                    {
                        continue;
                    }
                    check_refs(value, index, *id, *slot, &exists, &mut conflicts);
                }
                Edit::Retype { id, .. } => {
                    if !exists(*id) {
                        conflicts.push(Conflict::MissingTarget {
                            edit: index,
                            id: *id,
                        });
                    }
                }
                Edit::Remove { id } => {
                    if model.get(*id).is_none() && !created.contains_key(id) {
                        conflicts.push(Conflict::MissingTarget {
                            edit: index,
                            id: *id,
                        });
                    }
                }
            }
        }

        // --- removals must not orphan surviving references --------------
        //
        // Built once for the whole batch rather than per removal: the index is
        // a single scan, and a batch deleting a hundred entities would
        // otherwise rescan the model a hundred times.
        if self.edits.iter().any(|e| matches!(e, Edit::Remove { .. })) {
            let index = ReverseIndex::build(model);
            // Staging the same removal twice is one problem, not two: report
            // it against the first occurrence only.
            let mut reported: AHashSet<EntityId> = AHashSet::new();
            for (position, edit) in self.edits.iter().enumerate() {
                let Edit::Remove { id } = edit else { continue };
                if !reported.insert(*id) {
                    continue;
                }
                for referrer in index.referrers(*id) {
                    // A referrer that is itself going away cannot dangle.
                    if removed.contains(&referrer.from) {
                        continue;
                    }
                    // A slot being rewritten in this batch is governed by the
                    // new value, which the per-edit check already validated.
                    if writes
                        .get(&referrer.from)
                        .is_some_and(|w| w.contains_key(&referrer.slot))
                    {
                        continue;
                    }
                    conflicts.push(Conflict::RemovalWouldDangle {
                        edit: position,
                        removed: *id,
                        referrer: referrer.from,
                        slot: referrer.slot,
                    });
                }
            }
        }

        conflicts
    }

    /// Validate and apply, or report every conflict and change nothing.
    ///
    /// On `Err` the model is untouched: preflight runs to completion before
    /// the first write, so a rejected transaction cannot leave partial state.
    pub fn commit(self, model: &mut Model) -> Result<Applied, Vec<Conflict>> {
        let conflicts = self.preflight(model);
        if !conflicts.is_empty() {
            return Err(conflicts);
        }

        let mut applied = Applied {
            created: Vec::new(),
            removed: Vec::new(),
            revision: 0,
        };

        for edit in self.edits {
            match edit {
                Edit::Create { id, entity } => {
                    model.insert(id, entity);
                    applied.created.push(id);
                }
                Edit::SetAttribute { id, slot, value } => {
                    // Preflight proved the entity exists.
                    model.set_attribute(id, slot, value);
                }
                Edit::Retype { id, type_name } => {
                    model.retype(id, type_name);
                }
                Edit::Remove { id } => {
                    if let Some(entity) = model.remove(id) {
                        applied.removed.push((id, entity));
                    }
                }
            }
        }

        applied.revision = model.revision();
        Ok(applied)
    }
}

/// What a committed transaction did.
#[derive(Debug, Clone, PartialEq)]
pub struct Applied {
    /// Ids created, in commit order.
    pub created: Vec<EntityId>,
    /// Entities removed, with their contents.
    ///
    /// Returned rather than dropped so a caller can undo, log, or re-file
    /// them. A delete that silently discards the payload makes an editor's
    /// undo stack impossible to build.
    pub removed: Vec<(EntityId, Entity)>,
    /// The model revision after the commit.
    pub revision: u64,
}

/// Record a conflict for every reference in `value` that will not resolve.
fn check_refs(
    value: &Value,
    edit: usize,
    from: EntityId,
    slot: usize,
    exists: &impl Fn(EntityId) -> bool,
    conflicts: &mut Vec<Conflict>,
) {
    value.for_each_ref(&mut |target| {
        if !exists(target) {
            conflicts.push(Conflict::DanglingReference {
                edit,
                from,
                slot,
                target,
            });
        }
    });
}
