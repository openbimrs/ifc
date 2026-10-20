//! Copying a product with what it solely owns, and the low-level deep copy
//! of an entity by an explicit policy (#479).
//!
//! # What a product owns
//!
//! Ownership is the inverse of the cascading removal (#478): what
//! `RemoveProduct` would remove with the product, it owns, and the copy
//! copies; what the removal would keep (shared, a related object, a
//! definition), the copy shares. The removal's own reference count
//! answers, so the two operations never disagree on what belongs to a
//! product. Three parts are copied whatever the count says, as the copy
//! needs its own: the `ObjectPlacement`, the representation (by the rules
//! of `CopyRepresentation`, #481) and a type's representation maps. An
//! entity a `UNIQUE` clause binds (an `IfcPropertyEnumeration`) is shared
//! whatever the count says: a copy would break the clause.
//!
//! # Relationships
//!
//! Each relationship of a copied entity is decided by how many of its
//! attributes hold something copied or owned:
//!
//! - two or more (the property set and its object, the opening and the
//!   element it voids, the material usage and its element): the
//!   relationship is copied, between the copies, and the owned ends it
//!   reaches are copied first: an opening when
//!   [`CopyProductOptions::openings`], a port, a property set;
//! - one, a list (the objects a type, a shared property set, a
//!   classification or a container relates): the copy joins the list of
//!   the existing relationship, unless the options keep it out of a
//!   container or a whole;
//! - one, a single reference (a filling's opening, a connection, a space
//!   boundary, the whole of an aggregation, a type's occurrences): the
//!   copy would bind an object that is not copied, so it is skipped,
//!   except an opening's filling, copied with
//!   [`CopyProductOptions::fillings`].
//!
//! The whole copy is one step of the batch, committed with it or not at
//! all.

use std::collections::{BTreeMap, BTreeSet, VecDeque};

use ifc_model::{Entity, EntityId, Value};

use super::kinds::{references_any, Kind, Kinds};
use super::plan::Planner;
use super::report::{CopiedEntity, CopyReport};
use super::represent::{refs, remap};
use super::{AuthoringFailure as F, CopyPlacement, CopyProductOptions};

/// The state of one `CopyProduct`.
struct Copying {
    options: CopyProductOptions,
    /// What the copied objects own, by the cascade's count.
    owned: BTreeSet<EntityId>,
    /// Originals and their copies: objects, content, placements, product
    /// shapes, representation maps, relationships.
    copies: BTreeMap<EntityId, EntityId>,
    /// Further originals and copies, made by a copy of their own (the
    /// items of one placement or one representation), for the report.
    extra: Vec<(EntityId, EntityId)>,
    /// `CopyPlacement::At`: the original placement and the one given.
    alias: Option<(EntityId, EntityId)>,
    /// Copied originals whose relationships are still to decide.
    queue: VecDeque<EntityId>,
    /// Relationships decided.
    seen: BTreeSet<EntityId>,
    /// Relationships the copies join, with the attribute they join.
    joins: BTreeMap<EntityId, usize>,
    skipped: BTreeSet<EntityId>,
    layers: BTreeSet<EntityId>,
    /// Content being copied, to refuse a cycle.
    busy: BTreeSet<EntityId>,
}

impl Copying {
    fn new(options: CopyProductOptions) -> Self {
        Self {
            options,
            owned: BTreeSet::new(),
            copies: BTreeMap::new(),
            extra: Vec::new(),
            alias: None,
            queue: VecDeque::new(),
            seen: BTreeSet::new(),
            joins: BTreeMap::new(),
            skipped: BTreeSet::new(),
            layers: BTreeSet::new(),
            busy: BTreeSet::new(),
        }
    }

    fn moved(&self) -> bool {
        !matches!(self.options.placement, CopyPlacement::Same)
    }

    fn copyable(&self, id: EntityId) -> bool {
        self.owned.contains(&id) || self.copies.contains_key(&id)
    }
}

impl Planner<'_, '_> {
    /// `AuthorOp::CopyProduct`.
    pub(super) fn copy_product(
        &mut self,
        product: EntityId,
        options: CopyProductOptions,
    ) -> Result<(EntityId, CopyReport), F> {
        let type_name = self
            .view
            .get(product)
            .ok_or(F::MissingEntity(product))?
            .type_name
            .clone();
        if !self.schema.is_a(&type_name, "IfcProduct")
            && !self.schema.is_a(&type_name, "IfcTypeObject")
        {
            return Err(F::NotA {
                type_name: type_name.to_string(),
                expected: "IfcProduct or IfcTypeObject".to_owned(),
            });
        }
        let kinds = Kinds::new(self.schema);
        let mut state = Copying::new(options);
        let copy = self.copy_object(&kinds, &mut state, product, true)?;
        while let Some(original) = state.queue.pop_front() {
            self.decide_relationships(&kinds, &mut state, original)?;
        }
        self.apply_joins(&mut state)?;
        Ok((copy, self.copy_report(state)))
    }

    /// Copy an object (the product, an opening, a filling, a port): its
    /// placement, its representation, the content it owns, then it.
    fn copy_object(
        &mut self,
        kinds: &Kinds<'_>,
        state: &mut Copying,
        object: EntityId,
        top: bool,
    ) -> Result<EntityId, F> {
        if let Some(copy) = state.copies.get(&object) {
            return Ok(*copy);
        }
        let owned = self.owned_by(object);
        state.owned.extend(owned);
        let mut entity = self
            .view
            .get(object)
            .cloned()
            .ok_or(F::MissingEntity(object))?;
        let t = entity.type_name.clone();
        let product = self.schema.is_a(&t, "IfcProduct");
        let placement_slot = product
            .then(|| kinds.slot(&t, "ObjectPlacement"))
            .flatten();
        let shape_slot = product.then(|| kinds.slot(&t, "Representation")).flatten();
        let maps_slot = self
            .schema
            .is_a(&t, "IfcTypeProduct")
            .then(|| kinds.slot(&t, "RepresentationMaps"))
            .flatten();
        for (slot, value) in entity.attributes.iter_mut().enumerate() {
            if Some(slot) == placement_slot {
                if let Value::Ref(placement) = value {
                    *placement = self.copy_placement(kinds, state, object, *placement, top)?;
                }
            } else if Some(slot) == shape_slot {
                if let Value::Ref(shape) = value {
                    *shape = self.copy_shape(kinds, state, *shape)?;
                }
            } else if Some(slot) == maps_slot {
                let mut maps = Vec::new();
                value.for_each_ref(&mut |map| maps.push(map));
                let mut copied = Vec::with_capacity(maps.len());
                for map in maps {
                    copied.push(self.copy_map(kinds, state, map)?);
                }
                if !copied.is_empty() {
                    *value = refs(&copied);
                }
            } else {
                self.copy_owned_refs(kinds, state, value)?;
            }
        }
        let copy = self.insert_copy(entity)?;
        state.copies.insert(object, copy);
        state.queue.push_back(object);
        Ok(copy)
    }

    /// Replace each reference in `value` to something copied by its copy,
    /// copying the owned content it names first; the rest is shared.
    fn copy_owned_refs(
        &mut self,
        kinds: &Kinds<'_>,
        state: &mut Copying,
        value: &mut Value,
    ) -> Result<(), F> {
        match value {
            Value::Ref(id) => {
                if let Some(copy) = state.copies.get(id) {
                    *id = *copy;
                } else if self.owned_content(kinds, state, *id) {
                    *id = self.copy_content(kinds, state, *id)?;
                }
                Ok(())
            }
            Value::List(items) => items
                .iter_mut()
                .try_for_each(|item| self.copy_owned_refs(kinds, state, item)),
            Value::Typed { value, .. } => self.copy_owned_refs(kinds, state, value),
            _ => Ok(()),
        }
    }

    /// Whether `id` is content the copied objects own: a property set or a
    /// resource entity (properties, a material usage, a grid axis).
    fn owned_content(&self, kinds: &Kinds<'_>, state: &Copying, id: EntityId) -> bool {
        state.owned.contains(&id)
            && self.view.get(id).is_some_and(|entity| {
                matches!(
                    kinds.of(&entity.type_name),
                    Kind::PropertySet | Kind::Resource
                ) && !kinds.unique(&entity.type_name)
            })
    }

    /// Copy owned content and the owned content it references.
    fn copy_content(
        &mut self,
        kinds: &Kinds<'_>,
        state: &mut Copying,
        original: EntityId,
    ) -> Result<EntityId, F> {
        if let Some(copy) = state.copies.get(&original) {
            return Ok(*copy);
        }
        if !state.busy.insert(original) {
            return Err(F::UnsupportedCopy {
                entity: original,
                detail: "its references form a cycle".to_owned(),
            });
        }
        let mut entity = self
            .view
            .get(original)
            .cloned()
            .ok_or(F::MissingEntity(original))?;
        for value in &mut entity.attributes {
            self.copy_owned_refs(kinds, state, value)?;
        }
        let copy = self.insert_copy(entity)?;
        state.busy.remove(&original);
        state.copies.insert(original, copy);
        state.queue.push_back(original);
        Ok(copy)
    }

    /// A copy of `placement`, the `ObjectPlacement` of `object`: relative
    /// to the copy of its parent when that is copied, else to the same
    /// parent; its relative placement copied; moved by the option for the
    /// top object.
    fn copy_placement(
        &mut self,
        kinds: &Kinds<'_>,
        state: &mut Copying,
        object: EntityId,
        placement: EntityId,
        top: bool,
    ) -> Result<EntityId, F> {
        if top {
            if let CopyPlacement::At(given) = state.options.placement {
                state.copies.insert(placement, given);
                state.alias = Some((placement, given));
                return Ok(given);
            }
        }
        if let Some(copy) = state.copies.get(&placement) {
            return Ok(*copy);
        }
        let mut entity = self
            .view
            .get(placement)
            .cloned()
            .ok_or(F::MissingEntity(placement))?;
        let t = entity.type_name.clone();
        let parent_slot = kinds.slot(&t, "PlacementRelTo");
        for (slot, value) in entity.attributes.iter_mut().enumerate() {
            if Some(slot) == parent_slot {
                if let Value::Ref(parent) = value {
                    if let Some(copy) = state.copies.get(parent) {
                        *parent = *copy;
                    } else if !top && state.moved() {
                        return Err(F::UnsupportedCopy {
                            entity: object,
                            detail: format!(
                                "its placement {placement} is not relative to a copied \
                                 placement, so it would stay behind the moved copy"
                            ),
                        });
                    }
                }
            } else {
                self.copy_items_in(state, value)?;
            }
        }
        if top {
            if let CopyPlacement::Offset(offset) = state.options.placement {
                self.offset_location(kinds, &entity, placement, offset)?;
            }
        }
        let copy = self.insert_copy(entity)?;
        state.copies.insert(placement, copy);
        Ok(copy)
    }

    /// Replace each representation item `value` references by a copy of
    /// it and of the items it is built from, made for this use alone.
    fn copy_items_in(&mut self, state: &mut Copying, value: &mut Value) -> Result<(), F> {
        match value {
            Value::Ref(id) if self.copied(*id) => {
                let root = *id;
                let order = self.copy_order(root)?;
                let mut copies = BTreeMap::new();
                for original in order {
                    let copy = self.copy_of(original, &copies)?;
                    copies.insert(original, copy);
                }
                *id = copies[&root];
                state.extra.extend(copies);
                Ok(())
            }
            Value::List(items) => items
                .iter_mut()
                .try_for_each(|item| self.copy_items_in(state, item)),
            _ => Ok(()),
        }
    }

    /// Move the copied placement `entity` (of `original`) by `offset`: its
    /// relative placement's `Location`, a point copied for it alone, is
    /// rewritten.
    fn offset_location(
        &mut self,
        kinds: &Kinds<'_>,
        entity: &Entity,
        original: EntityId,
        offset: [f64; 3],
    ) -> Result<(), F> {
        let unsupported = |detail: String| F::UnsupportedCopy {
            entity: original,
            detail,
        };
        if !self.schema.is_a(&entity.type_name, "IfcLocalPlacement") {
            return Err(unsupported(format!(
                "an offset moves an IfcLocalPlacement, and its placement is an {}",
                entity.type_name
            )));
        }
        if offset.iter().any(|v| !v.is_finite()) {
            return Err(F::InvalidPlacement {
                detail: "offset has a non-finite coordinate".to_owned(),
            });
        }
        let point = kinds
            .reference(entity, "RelativePlacement")
            .and_then(|axis| self.view.get(axis))
            .and_then(|axis| kinds.reference(axis, "Location"))
            .ok_or_else(|| unsupported("its RelativePlacement has no Location".to_owned()))?;
        let found = self.view.get(point).ok_or(F::MissingEntity(point))?;
        if !self.schema.is_a(&found.type_name, "IfcCartesianPoint") {
            return Err(unsupported(format!(
                "its Location is an {}, not an IfcCartesianPoint",
                found.type_name
            )));
        }
        let coordinates: Option<Vec<f64>> = kinds
            .slot(&found.type_name, "Coordinates")
            .and_then(|slot| found.attributes.get(slot))
            .and_then(Value::as_list)
            .and_then(|items| items.iter().map(|v| v.unwrap_typed().as_f64()).collect());
        let mut coordinates =
            coordinates.ok_or_else(|| unsupported("its Location has no coordinates".to_owned()))?;
        if coordinates.len() < 3 && offset[2] != 0.0 {
            return Err(unsupported(
                "its Location is 2D, and the offset leaves the plane".to_owned(),
            ));
        }
        for (coordinate, delta) in coordinates.iter_mut().zip(offset) {
            *coordinate += delta;
        }
        self.edit(
            point,
            vec![(
                "Coordinates".to_owned(),
                Value::List(coordinates.into_iter().map(Value::Real).collect()),
            )],
        )
        .map(drop)
    }

    /// A copy of a product's `IfcProductDefinitionShape`: each
    /// representation copied by the rules of `CopyRepresentation`, and its
    /// shape aspects with it.
    fn copy_shape(
        &mut self,
        kinds: &Kinds<'_>,
        state: &mut Copying,
        shape: EntityId,
    ) -> Result<EntityId, F> {
        if let Some(copy) = state.copies.get(&shape) {
            return Ok(*copy);
        }
        let mut entity = self
            .view
            .get(shape)
            .cloned()
            .ok_or(F::MissingEntity(shape))?;
        if let Some(slot) = kinds.slot(&entity.type_name, "Representations") {
            if let Some(value) = entity.attributes.get_mut(slot) {
                self.copy_representations_in(state, value)?;
            }
        }
        let copy = self.insert_copy(entity)?;
        state.copies.insert(shape, copy);
        self.copy_aspects(kinds, state, shape, copy)?;
        Ok(copy)
    }

    /// A copy of a type's `IfcRepresentationMap`: its origin and its
    /// representation copied, and its shape aspects with it.
    fn copy_map(
        &mut self,
        kinds: &Kinds<'_>,
        state: &mut Copying,
        map: EntityId,
    ) -> Result<EntityId, F> {
        if let Some(copy) = state.copies.get(&map) {
            return Ok(*copy);
        }
        let mut entity = self.view.get(map).cloned().ok_or(F::MissingEntity(map))?;
        let t = entity.type_name.clone();
        let origin = kinds.slot(&t, "MappingOrigin");
        let mapped = kinds.slot(&t, "MappedRepresentation");
        for (slot, value) in entity.attributes.iter_mut().enumerate() {
            if Some(slot) == origin {
                self.copy_items_in(state, value)?;
            } else if Some(slot) == mapped {
                self.copy_representations_in(state, value)?;
            }
        }
        let copy = self.insert_copy(entity)?;
        state.copies.insert(map, copy);
        self.copy_aspects(kinds, state, map, copy)?;
        Ok(copy)
    }

    fn copy_representations_in(&mut self, state: &mut Copying, value: &mut Value) -> Result<(), F> {
        match value {
            Value::Ref(id) => {
                let copied = self.copy_representation_mapped(*id)?;
                *id = copied.copy;
                state.extra.extend(copied.copies);
                state.layers.extend(copied.layers);
                Ok(())
            }
            Value::List(items) => items
                .iter_mut()
                .try_for_each(|item| self.copy_representations_in(state, item)),
            _ => Ok(()),
        }
    }

    /// Copy the `IfcShapeAspect`s of `holder` onto `copy`.
    fn copy_aspects(
        &mut self,
        kinds: &Kinds<'_>,
        state: &mut Copying,
        holder: EntityId,
        copy: EntityId,
    ) -> Result<(), F> {
        for referrer in self.view.referrers(holder) {
            let Some(entity) = self.view.get(referrer) else {
                continue;
            };
            if !self.schema.is_a(&entity.type_name, "IfcShapeAspect")
                || kinds.reference(entity, "PartOfProductDefinitionShape") != Some(holder)
            {
                continue;
            }
            let mut aspect = entity.clone();
            let t = aspect.type_name.clone();
            let representations = kinds.slot(&t, "ShapeRepresentations");
            let part_of = kinds.slot(&t, "PartOfProductDefinitionShape");
            for (slot, value) in aspect.attributes.iter_mut().enumerate() {
                if Some(slot) == representations {
                    self.copy_representations_in(state, value)?;
                } else if Some(slot) == part_of {
                    *value = Value::Ref(copy);
                }
            }
            let copied = self.insert_copy(aspect)?;
            state.copies.insert(referrer, copied);
        }
        Ok(())
    }

    /// Decide every relationship of the copied `original` (see the module
    /// docs).
    fn decide_relationships(
        &mut self,
        kinds: &Kinds<'_>,
        state: &mut Copying,
        original: EntityId,
    ) -> Result<(), F> {
        for relationship in self.view.referrers(original) {
            if state.seen.contains(&relationship) {
                continue;
            }
            let Some(entity) = self.view.get(relationship) else {
                continue;
            };
            if kinds.of(&entity.type_name) != Kind::Relationship
                || state.copies.values().any(|copy| *copy == relationship)
            {
                continue;
            }
            let entity = entity.clone();
            state.seen.insert(relationship);
            self.decide_relationship(kinds, state, relationship, entity)?;
        }
        Ok(())
    }

    fn decide_relationship(
        &mut self,
        kinds: &Kinds<'_>,
        state: &mut Copying,
        relationship: EntityId,
        entity: Entity,
    ) -> Result<(), F> {
        let t = entity.type_name.clone();
        let slots: Vec<usize> = entity
            .attributes
            .iter()
            .enumerate()
            .filter(|(_, value)| references_any(value, &|id| state.copyable(id)))
            .map(|(slot, _)| slot)
            .collect();
        match slots.as_slice() {
            [] => {}
            [slot] if matches!(entity.attributes[*slot], Value::List(_)) => {
                let containment = self
                    .schema
                    .is_a(&t, "IfcRelContainedInSpatialStructure");
                let decomposition = (self.schema.is_a(&t, "IfcRelAggregates")
                    || self.schema.is_a(&t, "IfcRelNests"))
                    && kinds.slot(&t, "RelatedObjects") == Some(*slot);
                if (containment && !state.options.container)
                    || (decomposition && !state.options.aggregate)
                {
                    state.skipped.insert(relationship);
                } else {
                    state.joins.insert(relationship, *slot);
                }
            }
            [slot] => {
                let filling = (self.schema.is_a(&t, "IfcRelFillsElement")
                    && kinds.slot(&t, "RelatingOpeningElement") == Some(*slot))
                .then(|| kinds.reference(&entity, "RelatedBuildingElement"))
                .flatten();
                match filling {
                    Some(filling) if state.options.fillings => {
                        self.copy_object(kinds, state, filling, false)?;
                        self.copy_relationship(state, relationship, entity)?;
                    }
                    _ => {
                        state.skipped.insert(relationship);
                    }
                }
            }
            _ => {
                let mut ends = Vec::new();
                for value in &entity.attributes {
                    value.for_each_ref(&mut |id| ends.push(id));
                }
                for end in ends {
                    if !state.owned.contains(&end) || state.copies.contains_key(&end) {
                        continue;
                    }
                    let kind = self
                        .view
                        .get(end)
                        .map(|found| kinds.of(&found.type_name));
                    match kind {
                        Some(Kind::Opening) if !state.options.openings => {
                            state.skipped.insert(relationship);
                            return Ok(());
                        }
                        Some(Kind::Opening | Kind::Port | Kind::Root) => {
                            self.copy_object(kinds, state, end, false)?;
                        }
                        Some(Kind::PropertySet | Kind::Resource) => {
                            self.copy_content(kinds, state, end)?;
                        }
                        _ => {}
                    }
                }
                self.copy_relationship(state, relationship, entity)?;
            }
        }
        Ok(())
    }

    /// A copy of `entity` between the copies: a list holding something
    /// copied keeps only the copies; every other end is shared.
    fn copy_relationship(
        &mut self,
        state: &mut Copying,
        relationship: EntityId,
        mut entity: Entity,
    ) -> Result<EntityId, F> {
        for value in &mut entity.attributes {
            match value {
                Value::List(items)
                    if items
                        .iter()
                        .any(|item| references_any(item, &|id| state.copyable(id))) =>
                {
                    items.retain(|item| matches!(item, Value::Ref(id) if state.copies.contains_key(id)));
                    remap(value, &state.copies);
                }
                other => remap(other, &state.copies),
            }
        }
        let copy = self.insert_copy(entity)?;
        state.copies.insert(relationship, copy);
        Ok(copy)
    }

    /// Add the copies to the relationships they join.
    fn apply_joins(&mut self, state: &mut Copying) -> Result<(), F> {
        for (relationship, slot) in &state.joins {
            let Some(entity) = self.view.get(*relationship) else {
                continue;
            };
            let Some(Value::List(items)) = entity.attributes.get(*slot) else {
                continue;
            };
            let mut members = items.clone();
            for item in items {
                if let Value::Ref(original) = item {
                    let added = state.copies.get(original).map(|copy| Value::Ref(*copy));
                    if let Some(added) = added.filter(|added| !members.contains(added)) {
                        members.push(added);
                    }
                }
            }
            let name = self
                .schema
                .attribute_at(&entity.type_name, *slot)
                .map(|attribute| attribute.name.clone())
                .ok_or(F::MissingEntity(*relationship))?;
            self.edit(*relationship, vec![(name, Value::List(members))])?;
        }
        Ok(())
    }

    fn copy_report(&self, state: Copying) -> CopyReport {
        let mut pairs: Vec<(EntityId, EntityId)> = state
            .copies
            .into_iter()
            .filter(|pair| Some(*pair) != state.alias)
            .chain(state.extra)
            .collect();
        pairs.sort_by_key(|(original, copy)| (*copy, *original));
        pairs.dedup();
        let mut joined: BTreeSet<EntityId> = state.joins.into_keys().collect();
        joined.extend(state.layers);
        CopyReport {
            copied: self.copied_entities(pairs),
            joined: joined.into_iter().collect(),
            skipped: state.skipped.into_iter().collect(),
            ..CopyReport::default()
        }
    }

    pub(super) fn copied_entities(&self, pairs: Vec<(EntityId, EntityId)>) -> Vec<CopiedEntity> {
        pairs
            .into_iter()
            .map(|(original, copy)| CopiedEntity {
                original,
                copy,
                type_name: self
                    .view
                    .get(copy)
                    .map(|entity| entity.type_name.to_string())
                    .unwrap_or_default(),
            })
            .collect()
    }
}
