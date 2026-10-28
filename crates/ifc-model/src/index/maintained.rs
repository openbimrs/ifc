//! Derived indexes a model keeps current once they are built (#587).
//!
//! The reverse index and a GlobalId index are built on the first query, one
//! scan each, and every primitive write keeps them current after that: a
//! write computes the [`Footprint`] of what it changes before and after, and
//! [`Derived::update`] moves only the difference. A model nobody asks never
//! builds them, so a codec read costs what it always did (ADR 0023).
//!
//! The GlobalId index is schema-free: it lists every entity whose first
//! attribute is text that [`Guid::parse`] accepts, which is where IFC puts
//! the `GlobalId` of every `IfcRoot`. Callers that know the schema filter
//! the candidates by type.

use std::fmt;
use std::sync::OnceLock;

use ahash::AHashMap;

use super::reverse::{Referrer, ReverseIndex};
use crate::entity::Entity;
use crate::guid::Guid;
use crate::model::Model;
use crate::value::{EntityId, Value};

/// The indexes of one model, each built on first request.
#[derive(Clone, Default)]
pub(crate) struct Derived {
    reverse: OnceLock<ReverseIndex>,
    global_ids: OnceLock<GlobalIds>,
}

impl fmt::Debug for Derived {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let state = |built: bool| if built { "built" } else { "unbuilt" };
        f.debug_struct("Derived")
            .field(
                "reverse",
                &format_args!("{}", state(self.reverse.get().is_some())),
            )
            .field(
                "global_ids",
                &format_args!("{}", state(self.global_ids.get().is_some())),
            )
            .finish()
    }
}

/// Entities by the GlobalId text in their first attribute.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct GlobalIds {
    unique: AHashMap<Guid, EntityId>,
    /// Ascending, at least two ids.
    shared: AHashMap<Guid, Vec<EntityId>>,
}

impl GlobalIds {
    pub(crate) fn build(model: &Model) -> Self {
        let mut index = Self::default();
        for (id, entity) in model.iter() {
            if let Some(guid) = global_id_of(entity) {
                index.add(guid, id);
            }
        }
        index
    }

    pub(crate) fn candidates(&self, guid: &Guid) -> &[EntityId] {
        if let Some(id) = self.unique.get(guid) {
            std::slice::from_ref(id)
        } else {
            self.shared.get(guid).map_or(&[], Vec::as_slice)
        }
    }

    fn add(&mut self, guid: Guid, id: EntityId) {
        if let Some(ids) = self.shared.get_mut(&guid) {
            if let Err(at) = ids.binary_search(&id) {
                ids.insert(at, id);
            }
        } else if let Some(&holder) = self.unique.get(&guid) {
            if holder != id {
                self.unique.remove(&guid);
                self.shared
                    .insert(guid, vec![holder.min(id), holder.max(id)]);
            }
        } else {
            self.unique.insert(guid, id);
        }
    }

    fn remove(&mut self, guid: Guid, id: EntityId) {
        if self.unique.get(&guid) == Some(&id) {
            self.unique.remove(&guid);
        } else if let Some(ids) = self.shared.get_mut(&guid) {
            ids.retain(|held| *held != id);
            if ids.len() == 1 {
                let last = ids[0];
                self.shared.remove(&guid);
                self.unique.insert(guid, last);
            }
        }
    }
}

fn global_id_of(entity: &Entity) -> Option<Guid> {
    match entity.attributes.first() {
        Some(Value::Text(text)) => Guid::parse(text),
        _ => None,
    }
}

/// What an entity contributes to the indexes: its references by slot and
/// its GlobalId text.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct Footprint {
    /// Sorted and deduplicated, as the reverse index stores them.
    refs: Vec<(usize, EntityId)>,
    global_id: Option<Guid>,
}

impl Footprint {
    /// Everything `entity` contributes.
    pub(crate) fn of(entity: &Entity) -> Self {
        let mut refs = Vec::new();
        for (slot, attribute) in entity.attributes.iter().enumerate() {
            attribute.for_each_ref(&mut |target| refs.push((slot, target)));
        }
        refs.sort_unstable();
        refs.dedup();
        Self {
            refs,
            global_id: global_id_of(entity),
        }
    }

    /// What `slots` of `entity` contribute; a slot past the end contributes
    /// nothing, and the GlobalId only counts when slot 0 is among them.
    pub(crate) fn of_slots(entity: &Entity, slots: &[usize]) -> Self {
        let mut refs = Vec::new();
        for &slot in slots {
            if let Some(attribute) = entity.attributes.get(slot) {
                attribute.for_each_ref(&mut |target| refs.push((slot, target)));
            }
        }
        refs.sort_unstable();
        refs.dedup();
        Self {
            refs,
            global_id: if slots.contains(&0) {
                global_id_of(entity)
            } else {
                None
            },
        }
    }
}

impl Derived {
    pub(crate) fn is_built(&self) -> bool {
        self.reverse.get().is_some() || self.global_ids.get().is_some()
    }

    /// Frees both indexes.
    pub(crate) fn clear(&mut self) {
        *self = Self::default();
    }

    /// Moves every built index from the `before` footprint of `id` to the
    /// `after` one; `None` is an entity that was absent (or is removed).
    pub(crate) fn update(
        &mut self,
        id: EntityId,
        before: Option<&Footprint>,
        after: Option<&Footprint>,
    ) {
        let none = Footprint::default();
        let (before, after) = (before.unwrap_or(&none), after.unwrap_or(&none));
        if let Some(reverse) = self.reverse.get_mut() {
            for &(slot, target) in &before.refs {
                if after.refs.binary_search(&(slot, target)).is_err() {
                    reverse.unlink(target, Referrer { from: id, slot });
                }
            }
            for &(slot, target) in &after.refs {
                if before.refs.binary_search(&(slot, target)).is_err() {
                    reverse.link(target, Referrer { from: id, slot });
                }
            }
        }
        if let Some(global_ids) = self.global_ids.get_mut() {
            if before.global_id != after.global_id {
                if let Some(guid) = before.global_id {
                    global_ids.remove(guid, id);
                }
                if let Some(guid) = after.global_id {
                    global_ids.add(guid, id);
                }
            }
        }
    }
}

impl Model {
    /// The reverse index of this model, built on the first call (one scan,
    /// decoding every entity) and kept current by every write after that.
    ///
    /// Equal to [`ReverseIndex::build`] at every moment. Cloning the model
    /// clones the index; [`Model::drop_derived_indexes`] frees it.
    pub fn reverse_index(&self) -> &ReverseIndex {
        self.derived()
            .reverse
            .get_or_init(|| ReverseIndex::build(self))
    }

    /// Entities whose first attribute is the text of `global_id`, ascending
    /// by id; built on the first call and kept current like
    /// [`Model::reverse_index`].
    ///
    /// Schema-free: it lists any entity with that text first, so a caller
    /// that knows the schema filters by type (`IfcRoot`).
    pub fn global_id_candidates(&self, global_id: &Guid) -> &[EntityId] {
        self.derived()
            .global_ids
            .get_or_init(|| GlobalIds::build(self))
            .candidates(global_id)
    }

    /// Frees the reverse and GlobalId indexes; the next query rebuilds them.
    pub fn drop_derived_indexes(&mut self) {
        self.derived_mut().clear();
    }

    /// The reverse index if something already built it.
    pub(crate) fn maintained_reverse_index(&self) -> Option<&ReverseIndex> {
        self.derived().reverse.get()
    }

    /// Whether a write must report its footprints.
    pub(crate) fn derived_is_built(&self) -> bool {
        self.derived().is_built()
    }

    /// Tells the built indexes that `id` went from `before` to `after`.
    pub(crate) fn note_write(
        &mut self,
        id: EntityId,
        before: Option<Footprint>,
        after: Option<Footprint>,
    ) {
        self.derived_mut()
            .update(id, before.as_ref(), after.as_ref());
    }
}
