//! `INVERSE` attributes by name and the entities referencing one (#462).
//!
//! [`IfcModel::attribute_by_name`] resolves an `INVERSE` name
//! (`IsDefinedBy`) through the facade's reverse index, with the `FOR`
//! clause of the release the header declares: IFC2X3's `IsDefinedBy` holds
//! type and property relationships, IFC4's only property ones beside a
//! separate `IsTypedBy`. The value has the declared shape:
//!
//! - a `SET` or `BAG` inverse, `SET [0:1]` included (`ContainedInStructure`,
//!   `Decomposes`), is a `list` of `ref`s in ascending id order, empty when
//!   nothing references the entity: an optional single inverse is a list of
//!   zero or one, never unwrapped;
//! - a non-aggregate inverse (`VoidsElements`, IFC2X3's
//!   `IfcGroup.IsGroupedBy`) is one `ref`, or `null` when nothing
//!   references the entity; two or more are refused with `invalid-model`.
//!
//! [`IfcModel::inverse_attribute_names`] lists the declarations,
//! [`IfcModel::inverse_references`] every entity referencing one with the
//! attribute index it references through (IfcOpenShell's `get_inverse`
//! with indices), and [`IfcModel::total_inverses`] counts the distinct
//! referrers (`get_total_inverses`).
//!
//! The index is built on the first such read and again only after the
//! model changes, so a host reading many inverses pays one pass. An
//! `INVERSE` name holds no slot: a write to one is refused with
//! `inverse-attribute`.

use std::sync::{Arc, Mutex};

use ifc::{AttributeValue, EntityId, InverseValue, NamedAttributeError, ReverseIndex};

use crate::attribute::refused;
use crate::record::{Field, Record, ToRecord};
use crate::value::Tagged;
use crate::{BindingError, IfcModel};

/// One `INVERSE` attribute of an entity, as its declared release defines
/// it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InverseAttributeInfo {
    /// The declared name, in the schema's spelling (`IsDefinedBy`).
    pub name: String,
    /// The referencing entity type named before `FOR`; its subtypes count.
    pub entity: String,
    /// The attribute of `entity` named after `FOR` (`RelatedObjects`).
    pub for_attribute: String,
    /// Whether the inverse is a `SET` or `BAG` (read as a `list`) rather
    /// than a single entity (read as a `ref` or `null`).
    pub aggregate: bool,
    /// The declared lower bound; `1` for a non-aggregate inverse.
    pub lower: u64,
    /// The declared upper bound, `None` for `?`; `Some(1)` for a
    /// non-aggregate inverse.
    pub upper: Option<u64>,
    /// The entity whose declaration introduces the inverse.
    pub declared_by: String,
}

impl ToRecord for InverseAttributeInfo {
    fn to_record(&self) -> Record {
        Record::new(
            "InverseAttributeInfo",
            vec![
                ("name", Field::Text(self.name.clone())),
                ("entity", Field::Text(self.entity.clone())),
                ("for_attribute", Field::Text(self.for_attribute.clone())),
                ("aggregate", Field::Bool(self.aggregate)),
                ("lower", Field::Count(count(self.lower))),
                (
                    "upper",
                    self.upper
                        .map_or(Field::Null, |upper| Field::Count(count(upper))),
                ),
                ("declared_by", Field::Text(self.declared_by.clone())),
            ],
        )
    }
}

/// One reference to an entity: who holds it and in which attribute.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InverseReference {
    /// The referencing entity.
    pub id: u64,
    /// The zero-based attribute index (STEP slot) of `id` holding the
    /// reference, directly or inside an aggregate or typed value.
    pub index: usize,
}

impl ToRecord for InverseReference {
    fn to_record(&self) -> Record {
        Record::new(
            "InverseReference",
            vec![
                ("id", Field::Id(self.id)),
                ("index", Field::Count(self.index)),
            ],
        )
    }
}

/// The facade's reverse index, and the model revision it was built from.
#[derive(Debug, Default)]
pub(crate) struct ReverseCache(Mutex<Option<(u64, Arc<ReverseIndex>)>>);

impl IfcModel {
    /// Every `INVERSE` attribute of entity `id`, inherited ones first,
    /// resolved against the declared release.
    pub fn inverse_attribute_names(
        &self,
        id: u64,
    ) -> Result<Vec<InverseAttributeInfo>, BindingError> {
        let schema = self.declared_schema()?;
        let type_name = &self.entity(id)?.type_name;
        Ok(ifc::inverse_slots(schema, type_name)
            .map_err(refused)?
            .into_iter()
            .map(|slot| InverseAttributeInfo {
                name: slot.name.to_owned(),
                entity: slot.entity.to_owned(),
                for_attribute: slot.for_attribute.to_owned(),
                aggregate: slot.is_aggregate(),
                lower: slot.lower(),
                upper: slot.upper(),
                declared_by: slot.declared_by.to_owned(),
            })
            .collect())
    }

    /// Every reference to entity `id`, in ascending `(id, index)` order:
    /// one per referencing entity and attribute, whatever the attribute
    /// (IfcOpenShell's `get_inverse(.., with_attribute_indices=True)`).
    /// Needs no schema.
    pub fn inverse_references(&self, id: u64) -> Result<Vec<InverseReference>, BindingError> {
        self.entity(id)?;
        let reverse = self.reverse_index();
        Ok(reverse
            .referrers(EntityId(id))
            .iter()
            .map(|referrer| InverseReference {
                id: referrer.from.0,
                index: referrer.slot,
            })
            .collect())
    }

    /// How many distinct entities reference entity `id`, through any
    /// attribute (IfcOpenShell's `get_total_inverses`). Needs no schema.
    pub fn total_inverses(&self, id: u64) -> Result<usize, BindingError> {
        self.entity(id)?;
        Ok(ifc::total_inverses(&self.reverse_index(), EntityId(id)))
    }

    /// The `INVERSE` attribute `name` of `id`, after the slot lookup named
    /// it one.
    pub(crate) fn inverse_by_name(&self, id: u64, name: &str) -> Result<Tagged, BindingError> {
        let schema = self.declared_schema()?;
        let reverse = self.reverse_index();
        match ifc::attribute_value_by_name(&self.inner, schema, &reverse, EntityId(id), name)
            .map_err(inverse_refused)?
        {
            AttributeValue::Inverse(value) => Ok(to_tagged(&value)),
            AttributeValue::Explicit(value) => Ok(value.map_or(Tagged::Null, Tagged::from_value)),
            // `AttributeValue` is non-exhaustive.
            other => Err(BindingError::InvalidModel(format!(
                "unexpected read {other:?}"
            ))),
        }
    }

    /// The reverse index of the model as it is now, rebuilt first when an
    /// edit changed it since the last build.
    fn reverse_index(&self) -> Arc<ReverseIndex> {
        let revision = self.inner.revision();
        // A poisoned lock only means another thread panicked mid-build; the
        // slot is rebuilt below whatever it holds.
        let mut slot = self
            .reverse_index
            .0
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        match slot.as_ref() {
            Some((built, index)) if *built == revision => Arc::clone(index),
            _ => {
                let index = Arc::new(ReverseIndex::build(&self.inner));
                *slot = Some((revision, Arc::clone(&index)));
                index
            }
        }
    }
}

/// A single-valued inverse more than one entity references is a model
/// contradicting its schema; every other refusal is the slot lookup's.
fn inverse_refused(error: NamedAttributeError) -> BindingError {
    match error {
        NamedAttributeError::InverseCardinality { .. } => {
            BindingError::InvalidModel(error.to_string())
        }
        other => refused(other),
    }
}

fn to_tagged(value: &InverseValue) -> Tagged {
    match value {
        InverseValue::Single(found) => found.map_or(Tagged::Null, |EntityId(id)| Tagged::Ref(id)),
        InverseValue::Aggregate(ids) => {
            Tagged::List(ids.iter().map(|EntityId(id)| Tagged::Ref(*id)).collect())
        }
        // `InverseValue` is non-exhaustive: every shape lists its ids.
        other => Tagged::List(
            other
                .ids()
                .iter()
                .map(|EntityId(id)| Tagged::Ref(*id))
                .collect(),
        ),
    }
}

fn count(bound: u64) -> usize {
    usize::try_from(bound).unwrap_or(usize::MAX)
}
