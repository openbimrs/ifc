//! Entities by `GlobalId`, new `GlobalId`s, and an object's type, a type's
//! occurrences and the openings and fillings of elements (#463).
//!
//! Every operation reads against the release the header declares and is
//! refused with `unsupported-schema` when it names no bundled release.
//!
//! [`IfcModel::by_guid`] keeps the facade's `GlobalIdIndex` beside the
//! model, built on the first lookup and again only after the model or its
//! header changes, so a host looking up many `GlobalId`s pays one pass. A
//! `GlobalId` several entities carry is refused with `invalid-model`
//! naming all of them, never answered with one;
//! [`IfcModel::duplicate_global_ids`] lists every such `GlobalId`.
//!
//! The relation queries refuse an id the model lacks (`missing-entity`),
//! an entity of a type the query does not accept (`wrong-entity-type`), a
//! malformed relationship or several answers where the schema allows one
//! (`invalid-model`), and an answer the model lacks (`missing-reference`).

use std::sync::Mutex;

use ifc::{EntityId, GlobalIdIndex, Guid, GuidError, ObjectRelationError};

use crate::value::Tagged;
use crate::{BindingError, IfcModel};

/// One `GlobalId` several entities carry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DuplicateGlobalId {
    /// The shared `GlobalId`.
    pub global_id: String,
    /// Every entity that carries it, in file order.
    pub ids: Vec<u64>,
}

/// The facade's index, and the model state it was built from.
#[derive(Debug)]
pub(crate) struct CachedIndex {
    revision: u64,
    schema: Option<String>,
    index: GlobalIdIndex,
}

/// The cache slot [`IfcModel`] carries.
#[derive(Debug, Default)]
pub(crate) struct GuidCache(Mutex<Option<CachedIndex>>);

impl IfcModel {
    /// The entity whose `GlobalId` is `global_id`, or `None` when none.
    ///
    /// Refused with `invalid-value` when `global_id` is not 22 characters
    /// of IFC's alphabet, and with `invalid-model` when several entities
    /// carry it.
    pub fn by_guid(&self, global_id: &str) -> Result<Option<u64>, BindingError> {
        let guid = parse_guid(global_id)?;
        self.with_index(|index| {
            index
                .get(&guid)
                .map(|found| found.map(|EntityId(id)| id))
                .map_err(|duplicate| BindingError::InvalidModel(duplicate.to_string()))
        })
    }

    /// Every `GlobalId` several entities carry, in `GlobalId` order, each
    /// with its holders in file order. Empty for a conformant file.
    pub fn duplicate_global_ids(&self) -> Result<Vec<DuplicateGlobalId>, BindingError> {
        self.with_index(|index| {
            Ok(index
                .duplicates()
                .map(|(guid, ids)| DuplicateGlobalId {
                    global_id: guid.to_string(),
                    ids: ids.iter().map(|EntityId(id)| *id).collect(),
                })
                .collect())
        })
    }

    /// The type object of `occurrence` (`IfcRelDefinesByType`), or `None`.
    pub fn type_object(&self, occurrence: u64) -> Result<Option<u64>, BindingError> {
        let schema = self.declared_schema()?;
        ifc::type_of(&self.inner, schema, EntityId(occurrence))
            .map(|found| found.map(|EntityId(id)| id))
            .map_err(relation_error)
    }

    /// Every object `type_object` types, in file order.
    pub fn occurrences_of(&self, type_object: u64) -> Result<Vec<u64>, BindingError> {
        let schema = self.declared_schema()?;
        ifc::occurrences_of(&self.inner, schema, EntityId(type_object))
            .map(raw)
            .map_err(relation_error)
    }

    /// Every opening voiding `element` (`IfcRelVoidsElement`), in file
    /// order.
    pub fn openings_of(&self, element: u64) -> Result<Vec<u64>, BindingError> {
        let schema = self.declared_schema()?;
        ifc::openings_of(&self.inner, schema, EntityId(element))
            .map(raw)
            .map_err(relation_error)
    }

    /// Every element filling `opening` (`IfcRelFillsElement`), in file
    /// order.
    pub fn fillings_of(&self, opening: u64) -> Result<Vec<u64>, BindingError> {
        let schema = self.declared_schema()?;
        ifc::fillings_of(&self.inner, schema, EntityId(opening))
            .map(raw)
            .map_err(relation_error)
    }

    /// The element `opening` voids (`IfcRelVoidsElement`), or `None`.
    pub fn voided_element_of(&self, opening: u64) -> Result<Option<u64>, BindingError> {
        let schema = self.declared_schema()?;
        ifc::voided_element_of(&self.inner, schema, EntityId(opening))
            .map(|found| found.map(|EntityId(id)| id))
            .map_err(relation_error)
    }

    /// Run `answer` against the cached index, rebuilding it first when the
    /// model or its declared release changed since it was built.
    fn with_index<T>(
        &self,
        answer: impl FnOnce(&GlobalIdIndex) -> Result<T, BindingError>,
    ) -> Result<T, BindingError> {
        let schema = self.declared_schema()?;
        let revision = self.inner.revision();
        let token = self.schema().map(str::to_owned);
        // A poisoned lock only means another thread panicked mid-build; the
        // slot is rebuilt below whatever it holds.
        let mut slot = self
            .guid_index
            .0
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let current = slot
            .as_ref()
            .is_some_and(|cached| cached.revision == revision && cached.schema == token);
        if !current {
            *slot = Some(CachedIndex {
                revision,
                schema: token,
                index: GlobalIdIndex::build(&self.inner, schema),
            });
        }
        answer(&slot.as_ref().expect("built above").index)
    }
}

/// A new random `GlobalId` (RFC 9562 version 4) in IFC's 22-character
/// form.
///
/// Refused with `unsupported` when the build or host has no random source:
/// a WebAssembly build without the facade's `js-random` feature, or a
/// JavaScript host without `crypto.getRandomValues` (Node 18).
pub fn new_guid() -> Result<String, BindingError> {
    ifc::new_guid()
        .map(|guid| guid.to_string())
        .map_err(|error: GuidError| BindingError::Unsupported(error.to_string()))
}

/// A fresh seed for one unseeded batch.
///
/// Refused with `unsupported` when the build or host has no random source,
/// as [`new_guid`] is.
pub(crate) fn fresh_seed() -> Result<u128, BindingError> {
    ifc::fresh_seed().map_err(|error: GuidError| BindingError::Unsupported(error.to_string()))
}

/// The `GlobalId` named by `parts` (RFC 9562 version 8): the same parts
/// give the same `GlobalId` in every host and release.
pub fn name_based_guid<S: AsRef<str>>(parts: &[S]) -> String {
    let parts: Vec<&str> = parts.iter().map(AsRef::as_ref).collect();
    ifc::name_based_guid(&parts).to_string()
}

/// The duplicates as one tagged `list`, each a `list` of the `GlobalId`
/// (text) and its holders (`list` of `ref`). The C binding's tape form.
pub fn duplicates_to_tagged(duplicates: &[DuplicateGlobalId]) -> Tagged {
    Tagged::List(
        duplicates
            .iter()
            .map(|duplicate| {
                Tagged::List(vec![
                    Tagged::Text(duplicate.global_id.clone()),
                    Tagged::List(duplicate.ids.iter().copied().map(Tagged::Ref).collect()),
                ])
            })
            .collect(),
    )
}

fn parse_guid(text: &str) -> Result<Guid, BindingError> {
    Guid::parse(text).ok_or_else(|| {
        BindingError::InvalidValue(format!(
            "{text:?} is not a GlobalId: 22 characters of 0-9, A-Z, a-z, _ and $, the first 0-3"
        ))
    })
}

fn raw(ids: Vec<EntityId>) -> Vec<u64> {
    ids.into_iter().map(|EntityId(id)| id).collect()
}

fn relation_error(error: ObjectRelationError) -> BindingError {
    match error {
        ObjectRelationError::MissingEntity(EntityId(id)) => BindingError::MissingEntity(id),
        ObjectRelationError::WrongEntityType { .. } => {
            BindingError::WrongEntityType(error.to_string())
        }
        ObjectRelationError::MissingReference { .. } => {
            BindingError::MissingReference(error.to_string())
        }
        // Malformed, ambiguous, and any refusal added after this binding.
        _ => BindingError::InvalidModel(error.to_string()),
    }
}
