//! Evaluating a [`Selector`]: [`SelectorIndex::select`].
//!
//! Each facet list works on one set, facet by facet, left to right. A
//! class or `GlobalId` facet adds its entities to the set, and with `!`
//! removes them; every other facet keeps only the entities it holds for.
//! A list whose first facet is a filter or a `!` facet starts from the
//! default set: every `IfcProduct` and `IfcTypeProduct`, or the base set
//! the caller passes. The lists' sets are then united.

use std::collections::BTreeSet;
use std::ops::Range;

use ifc_model::EntityId;

use super::ast::{Comparison, Facet, FacetKind, Key, Keyword, Literal, Selector};
use super::error::SelectorError;
use super::index::{Rel, SelectorIndex};
use super::value::{compare, SelectedValue};

impl<'m> SelectorIndex<'m> {
    /// The entities `selector` selects from the whole model, in ascending
    /// id order.
    ///
    /// # Errors
    ///
    /// [`SelectorError::UnknownClass`] and
    /// [`SelectorError::UnknownAttribute`] for a name the release does not
    /// declare; a duplicated `GlobalId` an instance facet names; a
    /// relationship or property refusal met on the way. Nothing is
    /// skipped silently.
    pub fn select(&self, selector: &Selector) -> Result<Vec<EntityId>, SelectorError> {
        self.run(selector, None)
    }

    /// The entities `selector` selects from `base` alone, in ascending id
    /// order: class and `GlobalId` facets add only entities of `base`, and
    /// filters start from it.
    ///
    /// # Errors
    ///
    /// As [`Self::select`], and [`SelectorError::MissingEntity`] for an id
    /// of `base` the model lacks.
    pub fn select_from(
        &self,
        selector: &Selector,
        base: &[EntityId],
    ) -> Result<Vec<EntityId>, SelectorError> {
        for &id in base {
            self.entity(id)?;
        }
        let base: BTreeSet<EntityId> = base.iter().copied().collect();
        self.run(selector, Some(&base))
    }

    fn run(
        &self,
        selector: &Selector,
        base: Option<&BTreeSet<EntityId>>,
    ) -> Result<Vec<EntityId>, SelectorError> {
        self.check_names(selector)?;
        let mut selected = BTreeSet::new();
        for list in &selector.lists {
            let mut working: Option<BTreeSet<EntityId>> = None;
            for facet in &list.facets {
                working = Some(self.apply(facet, working, base)?);
            }
            selected.extend(working.unwrap_or_default());
        }
        Ok(selected.into_iter().collect())
    }

    /// Refuse a class or attribute the release does not declare, and a
    /// property facet a build cannot evaluate, before any entity is read:
    /// a misspelled name is an error even where no entity reaches its
    /// facet.
    fn check_names(&self, selector: &Selector) -> Result<(), SelectorError> {
        for facet in selector.lists.iter().flat_map(|list| &list.facets) {
            match &facet.kind {
                FacetKind::Class { name, .. } => {
                    self.class(name, &facet.span)?;
                }
                FacetKind::Attribute { name, .. } if name != "PredefinedType" => {
                    let declared = self.schema.entities().any(|entity| {
                        entity
                            .attributes
                            .iter()
                            .any(|attribute| attribute.name.eq_ignore_ascii_case(name))
                    });
                    if !declared {
                        return Err(SelectorError::UnknownAttribute {
                            name: name.clone(),
                            span: facet.span.clone(),
                        });
                    }
                }
                #[cfg(not(feature = "properties"))]
                FacetKind::Property { .. } => {
                    return Err(SelectorError::FeatureDisabled("properties"));
                }
                _ => {}
            }
        }
        Ok(())
    }

    fn default_set(&self, base: Option<&BTreeSet<EntityId>>) -> BTreeSet<EntityId> {
        match base {
            Some(base) => base.clone(),
            None => self
                .of_class("IfcProduct")
                .chain(self.of_class("IfcTypeProduct"))
                .collect(),
        }
    }

    fn apply(
        &self,
        facet: &Facet,
        working: Option<BTreeSet<EntityId>>,
        base: Option<&BTreeSet<EntityId>>,
    ) -> Result<BTreeSet<EntityId>, SelectorError> {
        let additive = match &facet.kind {
            FacetKind::Instance { negated, global_id } => {
                let found = self
                    .guids()
                    .get(global_id)
                    .map_err(SelectorError::DuplicateGlobalId)?;
                let found: BTreeSet<EntityId> = found
                    .into_iter()
                    .filter(|id| base.is_none_or(|base| base.contains(id)))
                    .collect();
                Some((*negated, found))
            }
            FacetKind::Class { negated, name } => {
                let class = self.class(name, &facet.span)?;
                let found: BTreeSet<EntityId> = match base {
                    Some(base) => base
                        .iter()
                        .copied()
                        .filter(|id| self.is_a(*id, class))
                        .collect(),
                    None => self.of_class(class).collect(),
                };
                Some((*negated, found))
            }
            _ => None,
        };
        if let Some((negated, found)) = additive {
            return Ok(if negated {
                let mut set = working.unwrap_or_else(|| self.default_set(base));
                set.retain(|id| !found.contains(id));
                set
            } else {
                let mut set = working.unwrap_or_default();
                set.extend(found);
                set
            });
        }
        let set = working.unwrap_or_else(|| self.default_set(base));
        let mut kept = BTreeSet::new();
        for id in set {
            if self.holds(facet, id)? {
                kept.insert(id);
            }
        }
        Ok(kept)
    }

    /// The release's spelling of class `name`, refused when it declares
    /// none.
    fn class<'n>(&self, name: &'n str, span: &Range<usize>) -> Result<&'n str, SelectorError> {
        if self.schema.entity(name).is_some() {
            Ok(name)
        } else {
            Err(SelectorError::UnknownClass {
                name: name.to_owned(),
                span: span.clone(),
            })
        }
    }

    /// Whether filter `facet` holds for `id`.
    fn holds(&self, facet: &Facet, id: EntityId) -> Result<bool, SelectorError> {
        let (value, comparison, literal) = match &facet.kind {
            FacetKind::Attribute {
                name,
                comparison,
                value,
            } => (self.attribute_value(id, name)?, comparison, value),
            FacetKind::Property {
                set,
                property,
                comparison,
                value,
            } => (self.property_value(id, set, property)?, comparison, value),
            FacetKind::Keyword {
                keyword,
                comparison,
                value,
            } => (self.keyword_value(id, *keyword)?, comparison, value),
            FacetKind::Query {
                path,
                comparison,
                value,
            } => (self.value(id, path)?, comparison, value),
            // Additive facets are handled by the caller.
            _ => return Ok(true),
        };
        Ok(matches(&value, *comparison, literal))
    }

    /// An attribute's value; its name was checked by [`Self::check_names`].
    fn attribute_value(&self, id: EntityId, name: &str) -> Result<SelectedValue, SelectorError> {
        if name == "PredefinedType" {
            return self.predefined_type(id);
        }
        Ok(self
            .attribute(id, name)
            .map_or(SelectedValue::Null, SelectedValue::from_model))
    }

    #[cfg(feature = "properties")]
    fn property_value(
        &self,
        id: EntityId,
        set: &Key,
        property: &Key,
    ) -> Result<SelectedValue, SelectorError> {
        use ifc_properties::{ExactPropertyError, ExactResolution};

        let refused = |error| SelectorError::Property {
            entity: Some(id),
            error,
        };
        let index = self.properties();
        if let (Key::Text(set), Key::Text(property)) = (set, property) {
            return match index.exact_property(id, Some(set), property) {
                Ok(ExactResolution::Present(found)) => super::exact::selected(id, &found.value),
                Ok(_) | Err(ExactPropertyError::InvalidQueryObject { .. }) => {
                    Ok(SelectedValue::Null)
                }
                Err(error) => Err(refused(error)),
            };
        }
        let entries = match index.exact_properties_where(
            id,
            |name| set.matches(name),
            |name| property.matches(name),
        ) {
            Ok(entries) => entries,
            Err(ExactPropertyError::InvalidQueryObject { .. }) => Vec::new(),
            Err(error) => return Err(refused(error)),
        };
        if entries.is_empty() {
            return Ok(SelectedValue::Null);
        }
        entries
            .iter()
            .map(|entry| super::exact::selected(id, &entry.property.value))
            .collect::<Result<_, _>>()
            .map(SelectedValue::List)
    }

    #[cfg(not(feature = "properties"))]
    #[allow(clippy::unused_self)]
    fn property_value(
        &self,
        _id: EntityId,
        _set: &Key,
        _property: &Key,
    ) -> Result<SelectedValue, SelectorError> {
        Err(SelectorError::FeatureDisabled("properties"))
    }

    /// The names and identifiers a keyword facet compares, as a list;
    /// `NULL` when the entity has none of the relation.
    fn keyword_value(
        &self,
        id: EntityId,
        keyword: Keyword,
    ) -> Result<SelectedValue, SelectorError> {
        let mut texts: Vec<&'m str> = Vec::new();
        let mut push = |found: Option<&'m str>| {
            if let Some(text) = found {
                texts.push(text);
            }
        };
        match keyword {
            Keyword::Type => {
                if let Some(type_object) = self.type_object(id)? {
                    push(self.text(type_object, "Name"));
                    push(self.text(type_object, "GlobalId"));
                }
            }
            Keyword::Material => {
                for definition in self.material_definitions(id)? {
                    for material in self.materials_of(definition)? {
                        push(self.text(material, "Name"));
                        push(self.text(material, "Category"));
                    }
                }
            }
            Keyword::Classification => {
                for reference in self.classifications(id)? {
                    push(self.text(reference, "Identification"));
                    push(self.text(reference, "ItemReference"));
                    push(self.text(reference, "Name"));
                    // An IFC2X3 notation: the value of each facet.
                    if let Some(ifc_model::Value::List(facets)) =
                        self.attribute(reference, "NotationFacets")
                    {
                        for facet in facets.iter().filter_map(ifc_model::Value::as_ref_id) {
                            push(self.text(facet, "NotationValue"));
                        }
                    }
                }
            }
            Keyword::Location => {
                for ancestor in self.ancestors(id)? {
                    push(self.text(ancestor, "Name"));
                    push(self.text(ancestor, "GlobalId"));
                }
            }
            Keyword::Group => {
                for &group in self.links(Rel::Group)?.forward(id) {
                    push(self.text(group, "Name"));
                    push(self.text(group, "GlobalId"));
                }
            }
        }
        Ok(if texts.is_empty() {
            SelectedValue::Null
        } else {
            SelectedValue::List(
                texts
                    .into_iter()
                    .map(|text| SelectedValue::Text(text.to_owned()))
                    .collect(),
            )
        })
    }
}

fn matches(value: &SelectedValue, comparison: Comparison, literal: &Literal) -> bool {
    compare(value, comparison, literal)
}
