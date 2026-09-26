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

use std::ops::Range;
use std::sync::Arc;

use crate::diagnostic::Diagnostic;
use crate::entity::Entity;
use crate::header::Header;
use crate::lazy::{EntitySource, Slot};
use crate::value::EntityId;
use ahash::AHashMap;

/// A parsed IFC file: header, entities, and indices over them.
#[derive(Debug, Clone, Default)]
pub struct Model {
    header: Header,
    /// Entities keyed by their in-file id, so `#42` survives a round-trip.
    /// A slot holds its entity decoded, or the span `source` decodes it from
    /// on first access (see [`EntitySource`]).
    entities: AHashMap<EntityId, Slot>,
    /// Insertion order, so a re-export preserves the original file order
    /// instead of hash order. Diffing two exports is otherwise unreadable.
    order: Vec<EntityId>,
    /// Type name to entity ids. Built during insertion because "every
    /// IfcWall" is the most common query in any consumer.
    by_type: AHashMap<String, Vec<EntityId>>,
    max_id: u64,
    /// Bumped by every structural change. A transaction opened against one
    /// revision refuses to commit against another, so an editor working from
    /// a stale view is told rather than silently overwriting.
    ///
    /// Not a content hash: two different edit sequences can reach the same
    /// bytes and still get different revisions. It answers "did anything
    /// change", which is the question optimistic concurrency asks.
    revision: u64,
    /// Non-fatal findings from the read that produced this model. Empty
    /// unless a codec recovered from damaged input.
    diagnostics: Vec<Diagnostic>,
    /// Decodes lazily registered entities; `None` for a model built only
    /// from decoded entities. Shared by clones.
    source: Option<Arc<dyn EntitySource>>,
}

impl Model {
    /// An empty model.
    pub fn new() -> Self {
        Self::default()
    }

    /// An empty model whose entities a codec registers with
    /// [`Model::insert_lazy`] and `source` decodes on first access.
    pub fn with_source(source: Arc<dyn EntitySource>) -> Self {
        Self {
            source: Some(source),
            ..Self::default()
        }
    }

    /// The file header (schema declaration, description, author).
    pub fn header(&self) -> &Header {
        &self.header
    }

    /// Mutable access to the header, for writers and editors.
    pub fn header_mut(&mut self) -> &mut Header {
        &mut self.header
    }

    /// Non-fatal problems reported by the codec that read this model.
    ///
    /// Empty for a clean file. A non-empty slice means the model is
    /// incomplete relative to its source: the codec recovered from damage and
    /// each entry says exactly what was dropped, so a consumer can surface
    /// "loaded, 1 record skipped" instead of pretending the read was lossless.
    pub fn diagnostics(&self) -> &[Diagnostic] {
        &self.diagnostics
    }

    /// Whether the read that produced this model dropped anything.
    pub fn is_complete(&self) -> bool {
        self.diagnostics.is_empty()
    }

    /// Attaches a codec diagnostic. Called by codecs during a recovered read.
    pub fn push_diagnostic(&mut self, diagnostic: Diagnostic) {
        self.diagnostics.push(diagnostic);
    }

    /// Insert an entity under a specific id, replacing any previous occupant.
    ///
    /// Codecs use this to preserve file ids exactly.
    pub fn insert(&mut self, id: EntityId, entity: Entity) {
        #[cfg(feature = "authored-dump")]
        crate::authored_dump::record(&entity.type_name, "insert");
        let type_name = std::sync::Arc::clone(&entity.type_name);
        self.place(id, &type_name, Slot::decoded(entity));
    }

    /// Register the entity stored at `span` of this model's source under
    /// `id`, without decoding it; it is decoded on first access. Replaces
    /// any previous occupant exactly as [`Model::insert`] does.
    ///
    /// `type_name` must be the type the span decodes to, which lets
    /// [`Model::ids_of_type`] answer without decoding. Only a codec that
    /// validated the span may register it (see [`EntitySource`]).
    ///
    /// # Panics
    ///
    /// When the model has no source ([`Model::with_source`]).
    pub fn insert_lazy(&mut self, id: EntityId, type_name: &str, span: Range<usize>) {
        assert!(
            self.source.is_some(),
            "insert_lazy needs a model built with Model::with_source"
        );
        #[cfg(feature = "authored-dump")]
        crate::authored_dump::record(type_name, "insert");
        self.place(id, type_name, Slot::lazy(span));
    }

    /// Reserve room for `additional` more entities; a codec that knows the
    /// record count up front avoids regrowing the storage while it loads.
    pub fn reserve(&mut self, additional: usize) {
        self.entities.reserve(additional);
        self.order.reserve(additional);
    }

    /// The shared tail of [`Model::insert`] and [`Model::insert_lazy`].
    fn place(&mut self, id: EntityId, type_name: &str, slot: Slot) {
        // Type names are ASCII upper case in practice, so the key is
        // usually `type_name` itself and nothing is allocated per entity.
        let upper;
        let key = if type_name.bytes().any(|byte| byte.is_ascii_lowercase()) {
            upper = type_name.to_ascii_uppercase();
            upper.as_str()
        } else {
            type_name
        };
        match self.entities.insert(id, slot) {
            None => self.order.push(id),
            // Replacing an occupant: drop its old type-index entry, otherwise
            // `ids_of_type` reports the id twice for the same type, or keeps
            // reporting it under a type the entity no longer has. Both make an
            // edit layer silently wrong.
            Some(previous) => {
                let previous = previous.into_entity(self.source.as_deref());
                let previous_key = previous.type_name.to_ascii_uppercase();
                if previous_key != key {
                    if let Some(ids) = self.by_type.get_mut(&previous_key) {
                        ids.retain(|existing| *existing != id);
                    }
                } else {
                    // Same type: the entry is still correct, so re-adding it
                    // below would duplicate it.
                    self.max_id = self.max_id.max(id.0);
                    self.revision += 1;
                    return;
                }
            }
        }
        match self.by_type.get_mut(key) {
            Some(ids) => ids.push(id),
            None => {
                self.by_type.insert(key.to_owned(), vec![id]);
            }
        }
        self.max_id = self.max_id.max(id.0);
        self.revision += 1;
    }

    /// Append an entity, allocating the next free id.
    pub fn push(&mut self, entity: Entity) -> EntityId {
        let id = EntityId(self.max_id + 1);
        self.insert(id, entity);
        id
    }

    /// How many structural changes this model has seen.
    ///
    /// Starts at zero and increases; the absolute value carries no meaning
    /// beyond comparison. See [`Transaction`](crate::Transaction).
    pub fn revision(&self) -> u64 {
        self.revision
    }

    /// Record a structural change made through a sibling mutation module.
    pub(crate) fn bump_revision(&mut self) {
        self.revision += 1;
    }

    /// The id [`Model::push`] would allocate next.
    ///
    /// A transaction reserves ids from here so several creates in one batch
    /// cannot collide with each other or with existing entities.
    pub fn next_id(&self) -> EntityId {
        EntityId(self.max_id + 1)
    }

    /// Look up one entity, decoding it first if it was loaded lazily.
    pub fn get(&self, id: EntityId) -> Option<&Entity> {
        self.entities
            .get(&id)
            .map(|slot| slot.get(self.source.as_deref()))
    }

    /// Whether `id` names an entity, without decoding it.
    pub fn contains(&self, id: EntityId) -> bool {
        self.entities.contains_key(&id)
    }

    /// How many entities have been decoded. Equals [`Model::len`] for a
    /// model built from decoded entities; for a lazily loaded one it counts
    /// the entities accessed so far.
    pub fn decoded_len(&self) -> usize {
        self.entities
            .values()
            .filter(|slot| slot.is_decoded())
            .count()
    }

    /// Decode every entity now, on up to `threads` threads.
    ///
    /// A lazily loaded model decodes on first access, one entity at a time.
    /// A consumer about to touch most of the model -- a writer, a full
    /// validation, a geometry pass -- can decode everything up front in
    /// parallel instead. Idempotent, and a no-op on a decoded model.
    pub fn decode_all(&self, threads: usize) {
        let pending: Vec<&Slot> = self
            .entities
            .values()
            .filter(|slot| !slot.is_decoded())
            .collect();
        if pending.is_empty() {
            return;
        }
        let source = self.source.as_deref();
        let threads = threads.clamp(1, pending.len());
        if threads == 1 {
            for slot in pending {
                slot.get(source);
            }
            return;
        }
        let chunk = pending.len().div_ceil(threads);
        std::thread::scope(|scope| {
            for part in pending.chunks(chunk) {
                scope.spawn(move || {
                    for slot in part {
                        slot.get(source);
                    }
                });
            }
        });
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
            .filter_map(move |id| self.get(*id).map(|e| (*id, e)))
    }

    /// Ids of every entity with this exact type name, case-insensitive.
    ///
    /// This is an exact-type query and does **not** include subtypes: asking
    /// for `IfcElement` will not return walls. Subtype queries need the schema,
    /// which this crate does not depend on. The facade's
    /// `ifc::ids_of_type_including_subtypes` (feature `schema`) joins the two.
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
            .filter_map(move |id| self.get(*id).map(|e| (*id, e)))
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

    // --- crate-internal seams for `mutation::edit` -----------------------
    //
    // Kept private-to-crate rather than `pub`: an edit that changes
    // `type_name` must also fix up `by_type`, or `ids_of_type` silently goes
    // stale. `mutation::edit` is the only module trusted to touch these
    // fields directly, and it exists precisely to keep that invariant in one
    // place instead of copied into every editor.

    /// The entity under `id` for editing, decoded first when needed.
    pub(crate) fn entity_mut(&mut self, id: EntityId) -> Option<&mut Entity> {
        let source = self.source.clone();
        Some(self.entities.get_mut(&id)?.get_mut(source.as_deref()))
    }

    /// Takes the entity under `id` out of storage, decoded; the caller
    /// fixes up `by_type` and `order`.
    pub(crate) fn take_entity(&mut self, id: EntityId) -> Option<Entity> {
        let slot = self.entities.remove(&id)?;
        Some(slot.into_entity(self.source.as_deref()))
    }

    pub(crate) fn by_type_mut(&mut self) -> &mut AHashMap<String, Vec<EntityId>> {
        &mut self.by_type
    }

    pub(crate) fn order_mut(&mut self) -> &mut Vec<EntityId> {
        &mut self.order
    }
}
