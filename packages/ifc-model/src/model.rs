//! The entity graph — storage, lookup, and nothing else.
//!
//! # What this type deliberately does NOT do
//!
//! `Model` has no idea what a cost item, a task, a wall, or a material is. It
//! stores entities and answers structural questions about them. Every domain
//! meaning lives in a separate crate that borrows a `&Model` and interprets it.
//!
//! That is not a stylistic preference, it is what makes two things possible:
//!
//! 1. **Thin builds.** An app that only reads geometry compiles no cost,
//!    schedule, or structural code, because those crates are optional features
//!    rather than parts of the model.
//! 2. **Lossless round-trips of data we do not understand.** Since the model
//!    stores entities structurally, a cost entity survives parse and re-export
//!    byte-for-byte in content even when `ifc-cost` is not compiled in. If the
//!    model instead held a `CostItem` struct, dropping the feature would drop
//!    the data.
//!
//! The rule to preserve: **no `if type_name == "IFCWALL"` in this crate.**

use crate::entity::Entity;
use crate::header::Header;
use crate::value::EntityId;
use ahash::AHashMap;

/// A parsed IFC file: header, entities, and indices over them.
#[derive(Debug, Clone, Default)]
pub struct Model {
    header: Header,
    /// Entities keyed by their in-file id, so `#42` survives a round-trip.
    entities: AHashMap<EntityId, Entity>,
    /// Insertion order, so a re-export preserves the original file order
    /// instead of hash order. Diffing two exports is otherwise unreadable.
    order: Vec<EntityId>,
    /// Type name to entity ids. Built during insertion because "every
    /// IfcWall" is the most common query in any consumer.
    by_type: AHashMap<String, Vec<EntityId>>,
    max_id: u64,
}

impl Model {
    /// An empty model.
    pub fn new() -> Self {
        Self::default()
    }

    /// The file header (schema declaration, description, author).
    pub fn header(&self) -> &Header {
        &self.header
    }

    /// Mutable access to the header, for writers and editors.
    pub fn header_mut(&mut self) -> &mut Header {
        &mut self.header
    }

    /// Insert an entity under a specific id, replacing any previous occupant.
    ///
    /// Codecs use this to preserve file ids exactly.
    pub fn insert(&mut self, id: EntityId, entity: Entity) {
        let key = entity.type_name.to_ascii_uppercase();
        if self.entities.insert(id, entity).is_none() {
            self.order.push(id);
        }
        self.by_type.entry(key).or_default().push(id);
        self.max_id = self.max_id.max(id.0);
    }

    /// Append an entity, allocating the next free id.
    pub fn push(&mut self, entity: Entity) -> EntityId {
        let id = EntityId(self.max_id + 1);
        self.insert(id, entity);
        id
    }

    /// Look up one entity.
    pub fn get(&self, id: EntityId) -> Option<&Entity> {
        self.entities.get(&id)
    }

    /// Number of entities.
    pub fn len(&self) -> usize {
        self.entities.len()
    }

    /// Whether the model holds no entities.
    pub fn is_empty(&self) -> bool {
        self.entities.is_empty()
    }

    /// Entity ids in original file order.
    pub fn ids(&self) -> impl Iterator<Item = EntityId> + '_ {
        self.order.iter().copied()
    }

    /// Entities in original file order.
    pub fn iter(&self) -> impl Iterator<Item = (EntityId, &Entity)> + '_ {
        self.order
            .iter()
            .filter_map(move |id| self.entities.get(id).map(|e| (*id, e)))
    }

    /// Ids of every entity with this exact type name, case-insensitive.
    ///
    /// This is an exact-type query and does **not** include subtypes: asking
    /// for `IfcElement` will not return walls. Subtype queries need the schema,
    /// which this crate does not depend on; `ifc-schema` provides that on top.
    pub fn ids_of_type(&self, type_name: &str) -> &[EntityId] {
        self.by_type
            .get(&type_name.to_ascii_uppercase())
            .map(|v| v.as_slice())
            .unwrap_or(&[])
    }

    /// Entities with this exact type name.
    pub fn of_type<'a>(&'a self, type_name: &str) -> impl Iterator<Item = (EntityId, &'a Entity)> {
        self.ids_of_type(type_name)
            .iter()
            .filter_map(move |id| self.entities.get(id).map(|e| (*id, e)))
            .collect::<Vec<_>>()
            .into_iter()
    }

    /// Every distinct type name present, with its instance count.
    ///
    /// Useful as a cheap file summary and as the basis for a coverage report
    /// of what a given build can and cannot interpret.
    pub fn type_histogram(&self) -> Vec<(&str, usize)> {
        let mut v: Vec<_> = self
            .by_type
            .iter()
            .map(|(k, ids)| (k.as_str(), ids.len()))
            .collect();
        v.sort_unstable_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(b.0)));
        v
    }

    /// Ids that are referenced by some entity but do not exist.
    ///
    /// A dangling reference is the most common corruption in real files, and
    /// it is a structural question, so it belongs here rather than in a
    /// validation crate.
    pub fn dangling_references(&self) -> Vec<(EntityId, EntityId)> {
        let mut out = Vec::new();
        for (id, entity) in self.iter() {
            for target in entity.references() {
                if !self.entities.contains_key(&target) {
                    out.push((id, target));
                }
            }
        }
        out
    }
}
