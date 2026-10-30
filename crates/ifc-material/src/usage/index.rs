//! [`MaterialIndex`] and the lookup plumbing it shares with [`MaterialView`].
//!
//! One code path answers both, parameterised by [`Lookup`], so the two agree
//! by construction.

use std::borrow::Cow;
use std::collections::BTreeMap;
use std::fmt;

use ifc_model::{Entity, EntityId, ReverseIndex};

use crate::view::{required_refs, MaterialView};
use crate::{MaterialAssignment, MaterialError, MaterialResult, ResolvedAssignment};

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

/// The material of many objects, resolved in one pass over the model.
///
/// [`MaterialView::assigned_material`] decodes the whole `RelatedObjects` of
/// every relationship naming the object on each call, so resolving every
/// product of a model whose relationships share long lists costs the square
/// of the list length. `MaterialIndex::new` reads the
/// `IfcRelAssociatesMaterial` and `IfcRelDefinesByType` records once and
/// checks each one's `RelatedObjects` once; a lookup then costs the
/// relationships that name the object.
///
/// Every answer, refusals included, equals the view's: both run one shared
/// code path, and a relationship whose `RelatedObjects` is malformed refuses
/// every object it names, exactly as the view does. The index borrows the
/// model, so it cannot go stale: the model cannot change while it lives.
/// When something built the model's reverse index the index reads through
/// it; otherwise it builds its own over those two record types only. It
/// never leaves an index on the model.
///
/// ```
/// # use ifc_material::{MaterialIndex, MaterialResult, MaterialView};
/// # use ifc_model::{EntityId, Model};
/// # fn demo(model: &Model, products: &[EntityId]) -> MaterialResult<()> {
/// let index = MaterialIndex::new(MaterialView::new(model));
/// for &p in products {
///     index.assigned_material(p)?;
/// }
/// # Ok(())
/// # }
/// ```
#[derive(Clone)]
pub struct MaterialIndex<'m> {
    view: MaterialView<'m>,
    referrers: Cow<'m, ReverseIndex>,
    refused: BTreeMap<EntityId, MaterialError>,
}

impl fmt::Debug for MaterialIndex<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("MaterialIndex")
            .field("schema", &self.view.schema())
            .field(
                "borrows_model_index",
                &matches!(self.referrers, Cow::Borrowed(_)),
            )
            .field("refused", &self.refused.len())
            .finish_non_exhaustive()
    }
}

impl<'m> MaterialIndex<'m> {
    /// Indexes the material and typing relationships of `view`'s model.
    ///
    /// Infallible: a malformed relationship is recorded and refuses the
    /// objects it names at lookup, as the view would.
    #[must_use]
    pub fn new(view: MaterialView<'m>) -> Self {
        let model = view.model();
        let ids = |entity: &'static str| model.ids_of_type(entity).iter().copied();
        let referrers = model.reverse_index_if_built().map_or_else(
            || {
                Cow::Owned(ReverseIndex::build_from(
                    model,
                    [ASSOCIATES_MATERIAL, DEFINES_BY_TYPE]
                        .into_iter()
                        .flat_map(ids),
                ))
            },
            Cow::Borrowed,
        );
        let mut refused = BTreeMap::new();
        for entity in [ASSOCIATES_MATERIAL, DEFINES_BY_TYPE] {
            for id in ids(entity) {
                let Some(record) = model.get(id) else {
                    continue;
                };
                if let Err(error) = view.related_objects(entity, id, record) {
                    refused.insert(id, error);
                }
            }
        }
        Self {
            view,
            referrers,
            refused,
        }
    }

    /// The view this index answers for.
    #[must_use]
    pub fn view(&self) -> MaterialView<'m> {
        self.view
    }

    fn lookup(&self) -> Lookup<'_> {
        Lookup {
            referrers: Some(&self.referrers),
            refused: Some(&self.refused),
        }
    }

    /// [`MaterialView::assignments_for`], answered from the index.
    ///
    /// # Errors
    ///
    /// As [`MaterialView::assignments_for`].
    pub fn assignments_for(&self, object: EntityId) -> MaterialResult<Vec<MaterialAssignment<'m>>> {
        self.view.assignments_in(self.lookup(), object)
    }

    /// [`MaterialView::assigned_material`], answered from the index.
    ///
    /// # Errors
    ///
    /// As [`MaterialView::assigned_material`].
    pub fn assigned_material(
        &self,
        object: EntityId,
    ) -> MaterialResult<Option<ResolvedAssignment<'m>>> {
        self.view.assigned_material_in(self.lookup(), object)
    }
}
