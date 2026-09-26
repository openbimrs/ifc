//! Lazily decoded entities: a slot per id, filled on first access.
//!
//! # Why
//!
//! Decoding every attribute of every record into owned `Value`s is most of
//! the cost of reading a file: on real IFC exports the allocator alone was
//! 41% of the read and `Model::insert` another 12%. A viewer that opens a
//! file to show a type census, or the properties of a few elements, pays for
//! millions of values it never looks at.
//!
//! A codec may instead register each entity as a byte span of its source
//! plus its type name, and hand the model an [`EntitySource`] that decodes a
//! span on demand. [`Model::get`](crate::Model::get) then decodes an entity
//! the first time anyone asks for it and keeps the result, so every later
//! access is a plain lookup and the returned reference is stable.
//!
//! # Contract
//!
//! The codec validates every record when it builds the model -- syntax and
//! every conversion that could fail -- so decoding a registered span cannot
//! fail while the source is unchanged. A source that changes underneath the
//! model (a memory-mapped file edited by another process) breaks that
//! promise; [`EntitySource::decode`] then panics with a message saying so
//! rather than returning a different entity.

use std::ops::Range;
use std::sync::OnceLock;

use crate::entity::Entity;

/// Decodes the entities of a lazily loaded [`Model`](crate::Model).
///
/// Implemented by codecs; the model never knows what format a span is in.
pub trait EntitySource: Send + Sync + std::fmt::Debug {
    /// Decodes the entity whose record occupies `span` of the source.
    ///
    /// # Panics
    ///
    /// When the record no longer decodes, which means the source changed
    /// after the codec validated it. A codec must never register a span it
    /// has not validated.
    fn decode(&self, span: Range<usize>) -> Entity;
}

/// One entity: decoded already, or a span still to decode.
#[derive(Debug, Clone)]
pub(crate) struct Slot {
    entity: OnceLock<Entity>,
    /// The record's bytes in the model's source; empty for an entity that
    /// was inserted decoded.
    span: Range<usize>,
}

impl Slot {
    pub(crate) fn decoded(entity: Entity) -> Self {
        Self {
            entity: OnceLock::from(entity),
            span: 0..0,
        }
    }

    pub(crate) fn lazy(span: Range<usize>) -> Self {
        Self {
            entity: OnceLock::new(),
            span,
        }
    }

    /// The entity, decoding it on first access.
    pub(crate) fn get(&self, source: Option<&dyn EntitySource>) -> &Entity {
        self.entity.get_or_init(|| {
            source
                .expect("a lazy slot exists only in a model with a source")
                .decode(self.span.clone())
        })
    }

    /// The entity for editing, decoding it first when needed.
    pub(crate) fn get_mut(&mut self, source: Option<&dyn EntitySource>) -> &mut Entity {
        self.get(source);
        self.entity
            .get_mut()
            .expect("`get` just initialised the slot")
    }

    /// The entity by value, decoding it first when needed.
    pub(crate) fn into_entity(self, source: Option<&dyn EntitySource>) -> Entity {
        self.get(source);
        self.entity
            .into_inner()
            .expect("`get` just initialised the slot")
    }

    /// Whether the entity has been decoded.
    pub(crate) fn is_decoded(&self) -> bool {
        self.entity.get().is_some()
    }
}
