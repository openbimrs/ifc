//! Schema-agnostic edit operations: update an existing entity's attributes
//! or remove it, without corrupting the derived by-type index.
//!
//! # Why not a raw `&mut Entity`
//!
//! `Model.get_mut` cannot exist as a bare accessor: an entity's `type_name`
//! is also a key into `Model::by_type`, so mutating the type field through a
//! `&mut Entity` would silently desynchronize the index (`ids_of_type` keeps
//! reporting the old type, or both). Every write therefore goes through this
//! module, which knows how to keep the index correct.
//!
//! # What is NOT checked here
//!
//! These operations are schema-agnostic: they trust the caller with slot
//! indices and don't know an entity's declared attribute count. Reference
//! integrity, arity, and declared-type checks are `ifc-author`'s job on
//! construction (`EntityBuilder`) and
//! `ifc-validate`'s job on audit. This is the primitive both build on.

use crate::entity::Entity;
use crate::model::Model;
use crate::value::{EntityId, Value};

impl Model {
    /// Set one positional attribute on an existing entity.
    ///
    /// Returns the value previously at that slot, or `None` if `id` does not
    /// name an entity in this model. Growing past the entity's current
    /// attribute count pads with [`Value::Null`], matching how STEP itself
    /// treats a missing trailing optional attribute.
    ///
    /// Does not touch `type_name`, so the by-type index stays correct without
    /// needing to run the same reindex logic [`Model::insert`] does. Renaming
    /// an entity's type is a structural change, not an attribute edit: use
    /// [`Model::retype`].
    ///
    /// ```
    /// use ifc_model::{Entity, EntityId, Model, Value};
    ///
    /// let mut model = Model::new();
    /// let id = model.push(Entity::new("IFCCARTESIANPOINT", vec![Value::List(vec![
    ///     Value::Real(0.0), Value::Real(0.0),
    /// ])]));
    ///
    /// let previous = model.set_attribute(id, 0, Value::List(vec![
    ///     Value::Real(10.0), Value::Real(20.0),
    /// ]));
    /// assert!(previous.is_some());
    /// let coords = model.get(id).unwrap().attribute(0).unwrap().as_list().unwrap();
    /// assert_eq!(coords[0].as_f64(), Some(10.0));
    /// ```
    pub fn set_attribute(&mut self, id: EntityId, index: usize, value: Value) -> Option<Value> {
        let entity = self.entity_mut(id)?;
        if index >= entity.attributes.len() {
            entity.attributes.resize(index + 1, Value::Null);
        }
        let previous = std::mem::replace(&mut entity.attributes[index], value);
        self.bump_revision();
        Some(previous)
    }

    /// Apply several attribute edits to one entity as a single unit.
    ///
    /// `edits` are applied in order; a later edit to the same slot wins. This
    /// exists because a caller updating several attributes wants to do it in
    /// one lookup rather than repeating [`Model::set_attribute`]'s entity
    /// lookup per field — the practical difference for callers is that this
    /// is one indexmap probe instead of N, not a transactional guarantee
    /// (there is nothing to roll back: slot writes cannot themselves fail).
    ///
    /// Returns the previous values in `edits` order, or `None` if `id` does
    /// not name an entity.
    pub fn set_attributes(
        &mut self,
        id: EntityId,
        edits: impl IntoIterator<Item = (usize, Value)>,
    ) -> Option<Vec<Value>> {
        let entity = self.entity_mut(id)?;
        let mut previous = Vec::new();
        for (index, value) in edits {
            if index >= entity.attributes.len() {
                entity.attributes.resize(index + 1, Value::Null);
            }
            previous.push(std::mem::replace(&mut entity.attributes[index], value));
        }
        self.bump_revision();
        Some(previous)
    }

    /// Change an entity's type in place, keeping its id and attributes.
    ///
    /// Reindexes `by_type` so `ids_of_type` reflects the new type immediately
    /// -- the operation [`Model::insert`] already performs when an id is
    /// reused with a different type, exposed here without requiring the
    /// caller to reconstruct the whole entity.
    ///
    /// Returns the previous type name, or `None` if `id` does not name an
    /// entity.
    pub fn retype(
        &mut self,
        id: EntityId,
        type_name: impl Into<std::sync::Arc<str>>,
    ) -> Option<std::sync::Arc<str>> {
        let entity = self.entity_mut(id)?;
        let previous = entity.type_name.clone();
        let new_name = type_name.into();
        if previous.eq_ignore_ascii_case(&new_name) {
            entity.type_name = new_name;
            self.bump_revision();
            return Some(previous);
        }
        entity.type_name = new_name.clone();

        let old_key = previous.to_ascii_uppercase();
        let new_key = new_name.to_ascii_uppercase();
        if let Some(ids) = self.by_type_mut().get_mut(&old_key) {
            ids.retain(|existing| *existing != id);
        }
        self.by_type_mut().entry(new_key).or_default().push(id);
        self.bump_revision();

        Some(previous)
    }

    /// Remove an entity entirely.
    ///
    /// Returns the removed entity, or `None` if `id` was not present. Leaves
    /// every reference to `id` from other entities dangling -- detect those
    /// with [`Model::dangling_references`] after a batch of removals, the
    /// same way a codec would detect them in a hand-edited file.
    pub fn remove(&mut self, id: EntityId) -> Option<Entity> {
        let entity = self.entities_mut().remove(&id)?;
        let key = entity.type_name.to_ascii_uppercase();
        if let Some(ids) = self.by_type_mut().get_mut(&key) {
            ids.retain(|existing| *existing != id);
        }
        self.order_mut().retain(|existing| *existing != id);
        self.bump_revision();
        Some(entity)
    }

    fn entity_mut(&mut self, id: EntityId) -> Option<&mut Entity> {
        self.entities_mut().get_mut(&id)
    }
}
