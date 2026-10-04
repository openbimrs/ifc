//! A whole-model scan of property assignments, shared by many queries
//! (#352).
//!
//! Every exact query validates every `IfcRelDefinesByProperties` and
//! `IfcRelDefinesByType` of the file, because a malformed one could hide an
//! assignment. Asked once per object, that makes "every property of every
//! object" quadratic: objects times relationships. [`PropertyIndex`] runs
//! the validation once and records which objects each relationship relates,
//! so each query afterwards costs only its own object's sets.

use std::fmt;

use ifc_model::{EntityId, Model};
use ifc_schema::SchemaVersion;

use super::assignment::{Relations, Scope};
use super::enumerate::{properties_where_in, property_sets_where_in};
use super::predefined::predefined_sets_in;
use super::release::{validate_model, Release};
use super::{
    property_in, ExactPredefinedSet, ExactPropertyEntry, ExactPropertyError, ExactPropertySetEntry,
    ExactResolution,
};

/// The exact resolver's view of one model, scanned once for many queries.
///
/// Answers exactly what the free functions answer, results and errors
/// alike: [`Self::exact_property`] is [`exact_property`](super::exact_property),
/// [`Self::exact_properties`] is [`exact_properties`](super::exact_properties),
/// and so on, for every object of the model. The difference is cost. A free
/// function validates every property relationship in the file on each
/// call; the index validates them once, in [`Self::build`], and records the
/// sets each object is assigned. A loop resolving every object is then
/// linear in objects plus relationships instead of their product.
///
/// ```
/// # use ifc_model::{EntityId, Model};
/// # use ifc_properties::{exact_property, PropertyIndex};
/// # fn every_object(model: &Model, objects: &[EntityId]) {
/// let index = PropertyIndex::build(model);
/// for &object in objects {
///     assert_eq!(
///         index.exact_property(object, Some("Pset_WallCommon"), "IsExternal"),
///         exact_property(model, object, Some("Pset_WallCommon"), "IsExternal"),
///     );
/// }
/// # }
/// ```
///
/// # It cannot go stale
///
/// The index is a borrowed view (ADR 0003): it holds `&Model` for its
/// whole life, so the model cannot be mutated, by a
/// [`Transaction`](ifc_model::Transaction) or otherwise, while the index
/// exists. To read after an edit, drop the index, commit, and build a new
/// one from the edited model.
///
/// # Refusals
///
/// A model the exact resolver refuses as a whole (diagnostics, no or
/// several schemas, an unsupported release) still builds; every query then
/// returns that refusal, as every free function would. A malformed
/// relationship is likewise reported by each query it would have refused.
#[derive(Clone)]
pub struct PropertyIndex<'m> {
    model: &'m Model,
    bound: Result<(Release, Relations), ExactPropertyError>,
}

impl fmt::Debug for PropertyIndex<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut out = f.debug_struct("PropertyIndex");
        match &self.bound {
            Ok((release, relations)) => out
                .field("schema", &release.version)
                .field("objects", &relations.objects()),
            Err(error) => out.field("refused", error),
        };
        out.finish_non_exhaustive()
    }
}

impl<'m> PropertyIndex<'m> {
    /// Bind `model` to its declared release and validate every property
    /// relationship once.
    ///
    /// Linear in the model's `IfcRelDefinesByProperties` and
    /// `IfcRelDefinesByType` and the objects they relate. Never fails: a
    /// refusal is kept and returned by every query.
    #[must_use]
    pub fn build(model: &'m Model) -> Self {
        let bound = validate_model(model)
            .map(|release| (release, Relations::scan(model, release, Scope::All)));
        Self { model, bound }
    }

    /// The model this index was built from.
    #[must_use]
    pub fn model(&self) -> &'m Model {
        self.model
    }

    /// The release every query resolves against, as
    /// [`exact_schema`](super::exact_schema) answers.
    ///
    /// # Errors
    ///
    /// The model-level refusal [`exact_schema`](super::exact_schema) returns.
    pub fn schema(&self) -> Result<SchemaVersion, ExactPropertyError> {
        self.bound
            .as_ref()
            .map(|(release, _)| release.version)
            .map_err(Clone::clone)
    }

    fn bound(&self) -> Result<(Release, &Relations), ExactPropertyError> {
        self.bound
            .as_ref()
            .map(|(release, relations)| (*release, relations))
            .map_err(Clone::clone)
    }

    /// [`exact_property`](super::exact_property) of `object`.
    ///
    /// # Errors
    ///
    /// As for [`exact_property`](super::exact_property).
    pub fn exact_property(
        &self,
        object: EntityId,
        set_name: Option<&str>,
        property_name: &str,
    ) -> Result<ExactResolution, ExactPropertyError> {
        let (release, relations) = self.bound()?;
        property_in(
            self.model,
            release,
            Some(relations),
            object,
            set_name,
            property_name,
        )
    }

    /// [`exact_properties`](super::exact_properties) of `object`.
    ///
    /// # Errors
    ///
    /// As for [`exact_properties`](super::exact_properties).
    pub fn exact_properties(
        &self,
        object: EntityId,
    ) -> Result<Vec<ExactPropertyEntry>, ExactPropertyError> {
        self.exact_properties_where(object, |_| true, |_| true)
    }

    /// [`exact_properties_where`](super::exact_properties_where) of
    /// `object`.
    ///
    /// # Errors
    ///
    /// As for [`exact_properties_where`](super::exact_properties_where).
    pub fn exact_properties_where<S, P>(
        &self,
        object: EntityId,
        mut select_set: S,
        mut select_property: P,
    ) -> Result<Vec<ExactPropertyEntry>, ExactPropertyError>
    where
        S: FnMut(&str) -> bool,
        P: FnMut(&str) -> bool,
    {
        let (release, relations) = self.bound()?;
        properties_where_in(
            self.model,
            release,
            Some(relations),
            object,
            &mut select_set,
            &mut select_property,
        )
    }

    /// [`exact_property_sets_where`](super::exact_property_sets_where) of
    /// `object`.
    ///
    /// # Errors
    ///
    /// As for [`exact_property_sets_where`](super::exact_property_sets_where).
    pub fn exact_property_sets_where<S>(
        &self,
        object: EntityId,
        mut select_set: S,
    ) -> Result<Vec<ExactPropertySetEntry>, ExactPropertyError>
    where
        S: FnMut(&str) -> bool,
    {
        let (release, relations) = self.bound()?;
        property_sets_where_in(
            self.model,
            release,
            Some(relations),
            object,
            &mut select_set,
        )
    }

    /// [`exact_predefined_sets`](super::exact_predefined_sets) of `object`.
    ///
    /// # Errors
    ///
    /// As for [`exact_predefined_sets`](super::exact_predefined_sets).
    pub fn exact_predefined_sets(
        &self,
        object: EntityId,
        entity: &str,
    ) -> Result<Vec<ExactPredefinedSet>, ExactPropertyError> {
        let (release, relations) = self.bound()?;
        predefined_sets_in(self.model, release, Some(relations), object, entity)
    }
}
