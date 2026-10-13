//! Entities by `GlobalId` (#463).
//!
//! [`GlobalIdIndex::build`] reads every `IfcRoot`'s `GlobalId` once, in
//! O(n), into a map; each [`GlobalIdIndex::get`] is then a hash lookup.
//! [`by_guid`] answers one lookup with a single scan and no map, for a
//! caller that asks once.
//!
//! # Which entities, and which slot
//!
//! Only an `IfcRoot` has a `GlobalId`, and whether a type is one is a
//! question for the schema, which ADR 0003 keeps out of `ifc-model`, so the
//! caller passes the release's table, as for
//! [`ids_of_type_including_subtypes`](crate::ids_of_type_including_subtypes).
//! The slot is looked up by attribute name in that table, never assumed.
//! Entities of other types are not decoded.
//!
//! # Duplicates are reported, not overwritten
//!
//! The schema makes a `GlobalId` unique, but files break that. The index
//! keeps every holder: [`GlobalIdIndex::get`] refuses a duplicated
//! `GlobalId` with [`DuplicateGlobalId`], naming every entity that carries
//! it, rather than answering with whichever came first or last, and
//! [`GlobalIdIndex::duplicates`] lists them all. An `IfcRoot` whose
//! `GlobalId` is unset or is not 22 characters of IFC's alphabet is listed
//! by [`GlobalIdIndex::invalid`], not indexed.
//!
//! ```
//! # #[cfg(all(feature = "step", feature = "ifc4"))] {
//! use ifc::{Codec, EntityId, GlobalIdIndex, Guid, StepCodec};
//!
//! let model = StepCodec.read_bytes(b"ISO-10303-21;HEADER;FILE_DESCRIPTION((''),'2;1');\
//!     FILE_NAME('','',(''),(''),'','','');FILE_SCHEMA(('IFC4'));ENDSEC;DATA;\
//!     #1=IFCWALL('2O2Fr$t4X7Zf8NOew3FLOH',$,'Wall',$,$,$,$,$,$);\
//!     #2=IFCSLAB('1kTvXnbbzCWw8lcMd1dR4o',$,'Slab',$,$,$,$,$,$);\
//!     #3=IFCBEAM('1kTvXnbbzCWw8lcMd1dR4o',$,'Beam',$,$,$,$,$,$);\
//!     ENDSEC;END-ISO-10303-21;").unwrap();
//!
//! let index = GlobalIdIndex::build(&model, ifc::schema::ifc4());
//! let wall = Guid::parse("2O2Fr$t4X7Zf8NOew3FLOH").unwrap();
//! assert_eq!(index.get(&wall), Ok(Some(EntityId(1))));
//!
//! let shared = Guid::parse("1kTvXnbbzCWw8lcMd1dR4o").unwrap();
//! let duplicate = index.get(&shared).unwrap_err();
//! assert_eq!(duplicate.entities, [EntityId(2), EntityId(3)]);
//! assert_eq!(index.duplicates().count(), 1);
//! # }
//! ```
#![cfg(feature = "schema-api")]

use std::collections::{BTreeMap, HashMap};

use ifc_model::guid::Guid;
use ifc_model::{EntityId, Model, Value};
use ifc_schema::Schema;

/// Several entities carry one `GlobalId`, which the schema makes unique.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct DuplicateGlobalId {
    /// The shared `GlobalId`.
    pub global_id: Guid,
    /// Every entity that carries it, in file order; at least two.
    pub entities: Vec<EntityId>,
}

impl std::fmt::Display for DuplicateGlobalId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "GlobalId {} is carried by", self.global_id)?;
        for (index, EntityId(id)) in self.entities.iter().enumerate() {
            let separator = if index == 0 { " " } else { ", " };
            write!(f, "{separator}#{id}")?;
        }
        Ok(())
    }
}

impl std::error::Error for DuplicateGlobalId {}

/// Every `IfcRoot` of a model by `GlobalId`, built once.
///
/// Owns its map and borrows the model only while it is built, so it can be
/// kept beside the model; it describes the model as it was then, and a
/// later edit is not reflected until it is built again.
#[derive(Debug, Clone, Default)]
pub struct GlobalIdIndex {
    unique: HashMap<Guid, EntityId>,
    duplicates: BTreeMap<Guid, Vec<EntityId>>,
    invalid: Vec<EntityId>,
}

impl GlobalIdIndex {
    /// Index every `IfcRoot` of `model` by `GlobalId`, reading `schema`'s
    /// table for which types are `IfcRoot`s and where their `GlobalId`
    /// sits. One pass; only `IfcRoot` entities are decoded.
    #[must_use]
    pub fn build(model: &Model, schema: &Schema) -> Self {
        let mut index = Self::default();
        for_each_global_id(model, schema, |id, global_id| match global_id {
            Some(guid) => index.insert(guid, id),
            None => index.invalid.push(id),
        });
        index
    }

    fn insert(&mut self, guid: Guid, id: EntityId) {
        if let Some(holders) = self.duplicates.get_mut(&guid) {
            holders.push(id);
        } else if let Some(first) = self.unique.remove(&guid) {
            self.duplicates.insert(guid, vec![first, id]);
        } else {
            self.unique.insert(guid, id);
        }
    }

    /// The entity carrying `guid`, or `None` when none does.
    ///
    /// # Errors
    ///
    /// [`DuplicateGlobalId`] when several entities carry it.
    pub fn get(&self, guid: &Guid) -> Result<Option<EntityId>, DuplicateGlobalId> {
        if let Some(entities) = self.duplicates.get(guid) {
            return Err(DuplicateGlobalId {
                global_id: *guid,
                entities: entities.clone(),
            });
        }
        Ok(self.unique.get(guid).copied())
    }

    /// Every `GlobalId` several entities carry, in `GlobalId` order, each
    /// with its holders in file order.
    pub fn duplicates(&self) -> impl Iterator<Item = (&Guid, &[EntityId])> + '_ {
        self.duplicates
            .iter()
            .map(|(guid, entities)| (guid, entities.as_slice()))
    }

    /// `IfcRoot` entities whose `GlobalId` is unset or is not a valid
    /// 22-character `GlobalId`, in file order. They are not indexed.
    #[must_use]
    pub fn invalid(&self) -> &[EntityId] {
        &self.invalid
    }

    /// How many distinct `GlobalId`s are indexed, duplicated ones included.
    #[must_use]
    pub fn len(&self) -> usize {
        self.unique.len() + self.duplicates.len()
    }

    /// Whether no `GlobalId` is indexed.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

/// The entity carrying `guid`, found by one scan without building an index.
///
/// For one lookup; build a [`GlobalIdIndex`] for several.
///
/// # Errors
///
/// [`DuplicateGlobalId`] when several entities carry it.
pub fn by_guid(
    model: &Model,
    schema: &Schema,
    guid: &Guid,
) -> Result<Option<EntityId>, DuplicateGlobalId> {
    let mut holders = Vec::new();
    for_each_global_id(model, schema, |id, global_id| {
        if global_id.as_ref() == Some(guid) {
            holders.push(id);
        }
    });
    match holders.as_slice() {
        [] => Ok(None),
        [one] => Ok(Some(*one)),
        _ => Err(DuplicateGlobalId {
            global_id: *guid,
            entities: holders,
        }),
    }
}

/// Call `visit` with every `IfcRoot` of `model` in file order and its
/// parsed `GlobalId` (`None` when unset or invalid).
fn for_each_global_id(
    model: &Model,
    schema: &Schema,
    mut visit: impl FnMut(EntityId, Option<Guid>),
) {
    // The slot of `GlobalId` per type, from the table, once per type.
    let mut slots: HashMap<String, Option<usize>> = HashMap::new();
    for id in crate::ids_of_type_including_subtypes(model, schema, "IfcRoot") {
        let Some(entity) = model.get(id) else {
            continue;
        };
        let slot = *slots
            .entry(entity.type_name.to_string())
            .or_insert_with(|| {
                schema
                    .attribute_names(&entity.type_name)
                    .iter()
                    .position(|name| name.eq_ignore_ascii_case("GlobalId"))
            });
        let guid = slot
            .and_then(|slot| entity.attributes.get(slot))
            .and_then(|value| match value.unwrap_typed() {
                Value::Text(text) => Guid::parse(text),
                _ => None,
            });
        visit(id, guid);
    }
}

#[cfg(test)]
mod tests;
