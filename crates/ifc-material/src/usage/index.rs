//! Lookup plumbing shared by [`MaterialView`] and `MaterialIndex`.
//!
//! One code path answers both, parameterised by [`Lookup`], so the two agree
//! by construction.

use std::collections::BTreeMap;

use ifc_model::{Entity, EntityId, ReverseIndex};

use crate::view::{required_refs, MaterialView};
use crate::{MaterialError, MaterialResult};

pub(crate) const ASSOCIATES_MATERIAL: &str = "IFCRELASSOCIATESMATERIAL";
pub(crate) const DEFINES_BY_TYPE: &str = "IFCRELDEFINESBYTYPE";

/// Where a lookup finds the relationships naming an object, and whether the
/// refusal of each relationship's `RelatedObjects` is already known.
#[derive(Clone, Copy)]
pub(crate) struct Lookup<'a> {
    /// `None`: scan the record type.
    pub(crate) referrers: Option<&'a ReverseIndex>,
    /// `None`: decode `RelatedObjects` per lookup.
    pub(crate) refused: Option<&'a BTreeMap<EntityId, MaterialError>>,
}

impl<'m> MaterialView<'m> {
    /// The per-call lookup: the model's reverse index when something built
    /// it, and a decode of every candidate's `RelatedObjects`.
    pub(crate) fn lookup(self) -> Lookup<'m> {
        Lookup {
            referrers: self.model().reverse_index_if_built(),
            refused: None,
        }
    }

    /// The strictly decoded `RelatedObjects` of a relationship record.
    pub(crate) fn related_objects(
        self,
        entity: &'static str,
        id: EntityId,
        record: &Entity,
    ) -> MaterialResult<Vec<EntityId>> {
        let slot = self.release().slot(entity, id, "RelatedObjects")?;
        required_refs(entity, id, record, slot, "RelatedObjects", 1)
    }

    /// Whether the relationship's `RelatedObjects` names `object`; a
    /// malformed list refuses.
    pub(crate) fn names(
        self,
        lookup: Lookup<'_>,
        entity: &'static str,
        id: EntityId,
        record: &Entity,
        object: EntityId,
    ) -> MaterialResult<bool> {
        match lookup.refused {
            Some(refused) => refused.get(&id).map_or(Ok(true), |e| Err(e.clone())),
            None => Ok(self.related_objects(entity, id, record)?.contains(&object)),
        }
    }
}
