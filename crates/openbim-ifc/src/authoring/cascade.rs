//! Removing a product with everything it solely owns (#478).
//!
//! # Ownership
//!
//! "Solely owned" is a reference count over the batch's view of the model
//! (the base model's reverse index, corrected for what the batch already
//! changed), taken against the set of entities the cascade has decided to
//! remove so far. An entity reached from a removed one goes too when every
//! entity that references it and stays
//!
//! - is a relationship that keeps an end without it (it only loses a
//!   list member), or
//! - is a presentation layer assignment (it loses the item), or
//! - is attached to it: the `IfcStyledItem` styling it, the
//!   `IfcShapeAspect` of the product shape (those go with it).
//!
//! Anything else that stays and references it makes it shared, and it is
//! kept. The set grows to a fixed point: removing an entity can leave a
//! relationship without an end (it goes, and its other ends are reached),
//! and can free what it referenced.
//!
//! Which reached entities may go at all:
//!
//! - resource entities (representations, items, profiles, points,
//!   properties, material usages, styles): when solely owned;
//! - property and quantity sets: when solely owned;
//! - openings voiding a removed element, and ports nested in or connected
//!   to one: always, each with the same cascade;
//! - the product's `ObjectPlacement`: when no other placement that stays is
//!   relative to it; otherwise refused, or, by option, each such placement
//!   is re-based onto the removed placement's parent with the composed
//!   transform;
//! - other `IfcRoot`s (types, containers, fillings, groups) and
//!   definitions (materials, classification references, units, contexts,
//!   the owner history): never; they are reported as kept.
//!
//! The whole cascade is one step of the batch, so it commits with the
//! batch's one transaction and its preflight, or not at all.

use std::collections::{BTreeMap, BTreeSet};

use ifc_model::{EntityId, Value};

use super::kinds::{detach, loses_an_end, Kind, Kinds};
use super::plan::Planner;
use super::rebase::Transform;
use super::report::{KeepReason, KeptEntity, RemovalReport, RemovedEntity};
use super::{AuthoringFailure as F, RemoveProductOptions};

/// The cascade's state.
#[derive(Default)]
struct Cascade {
    /// Entities decided to go.
    removed: BTreeSet<EntityId>,
    /// Removed entities whose references are not followed yet.
    queue: Vec<EntityId>,
    /// Entities reached and not removed, with the reason they stay.
    reached: BTreeMap<EntityId, KeepReason>,
    /// `ObjectPlacement`s of removed products, decided last.
    placements: BTreeSet<EntityId>,
    /// Placements re-based onto a kept ancestor.
    rebased: Vec<EntityId>,
    /// Entities that stay whatever the counts say, unreported: the
    /// representation an unassignment takes off a removed map or shape.
    keep: BTreeSet<EntityId>,
}

impl Cascade {
    fn add(&mut self, id: EntityId) {
        if self.removed.insert(id) {
            self.queue.push(id);
            self.reached.remove(&id);
        }
    }

    fn reach(&mut self, id: EntityId, reason: KeepReason) {
        if !self.removed.contains(&id) {
            self.reached.entry(id).or_insert(reason);
        }
    }
}

impl Planner<'_, '_> {
    pub(super) fn remove_product(
        &mut self,
        target: EntityId,
        options: RemoveProductOptions,
    ) -> Result<RemovalReport, F> {
        let kinds = Kinds::new(self.schema);
        let type_name = self
            .view
            .get(target)
            .ok_or(F::MissingEntity(target))?
            .type_name
            .clone();
        let is_type = self.schema.is_a(&type_name, "IfcTypeObject");
        if !is_type && !self.schema.is_a(&type_name, "IfcProduct") {
            return Err(F::NotA {
                type_name: type_name.to_string(),
                expected: "IfcProduct or IfcTypeObject".to_owned(),
            });
        }
        if is_type && !options.untype_occurrences {
            let occurrences = self.occurrences(&kinds, target);
            if !occurrences.is_empty() {
                return Err(F::TypeHasOccurrences {
                    type_object: target,
                    occurrences,
                });
            }
        }
        self.cascade(&[target], BTreeSet::new(), options)
    }

    /// Remove `roots` with everything they solely own, never `keep`, and
    /// report it: the cascade of [`Self::remove_product`] from any start.
    pub(super) fn cascade(
        &mut self,
        roots: &[EntityId],
        keep: BTreeSet<EntityId>,
        options: RemoveProductOptions,
    ) -> Result<RemovalReport, F> {
        let kinds = Kinds::new(self.schema);
        let mut cascade = Cascade {
            keep,
            ..Cascade::default()
        };
        for root in roots {
            cascade.add(*root);
        }
        self.drain(&kinds, &mut cascade);
        self.settle_placements(&kinds, &mut cascade, options)?;
        self.apply_cascade(&kinds, cascade)
    }

    /// Remove each of `roots` that nothing that stays still needs, with
    /// what it solely owns, by the cascade's rules; a root still in use
    /// stays. For the representations and usages a type propagated, once
    /// the object no longer references them (#480).
    pub(super) fn remove_owned(&mut self, roots: &[EntityId]) -> Result<(), F> {
        let kinds = Kinds::new(self.schema);
        let mut cascade = Cascade::default();
        for root in roots {
            self.consider(&kinds, &mut cascade, *root);
        }
        self.drain(&kinds, &mut cascade);
        self.apply_cascade(&kinds, cascade).map(drop)
    }

    /// The objects `type_object` types, through `IfcRelDefinesByType`.
    fn occurrences(&self, kinds: &Kinds<'_>, type_object: EntityId) -> Vec<EntityId> {
        let mut found = BTreeSet::new();
        for relationship in self.view.referrers(type_object) {
            let Some(entity) = self.view.get(relationship) else {
                continue;
            };
            if self.schema.is_a(&entity.type_name, "IfcRelDefinesByType")
                && kinds.reference(entity, "RelatingType") == Some(type_object)
            {
                found.extend(kinds.references(entity, "RelatedObjects"));
            }
        }
        found.into_iter().collect()
    }

    /// Follow every queued entity: the relationships it leaves without an
    /// end and its attachments go; what it references is reached.
    fn drain(&self, kinds: &Kinds<'_>, cascade: &mut Cascade) {
        while let Some(id) = cascade.queue.pop() {
            let Some(entity) = self.view.get(id) else {
                continue;
            };
            let kind = kinds.of(&entity.type_name);
            for referrer in self.view.referrers(id) {
                if cascade.removed.contains(&referrer) {
                    continue;
                }
                let Some(by) = self.view.get(referrer) else {
                    continue;
                };
                let goes = match kinds.of(&by.type_name) {
                    Kind::Relationship => {
                        loses_an_end(by, &|other| cascade.removed.contains(&other))
                    }
                    Kind::StyledItem => kinds.reference(by, "Item") == Some(id),
                    Kind::ShapeAspect => {
                        kinds.reference(by, "PartOfProductDefinitionShape") == Some(id)
                    }
                    _ => false,
                };
                if goes {
                    cascade.add(referrer);
                } else if kinds.of(&by.type_name) == Kind::Relationship {
                    // It stays, relating the rest: its single ends, such as
                    // the property set it gives or the type it assigns,
                    // are reported, never cascaded into.
                    for value in &by.attributes {
                        if let Value::Ref(end) = value {
                            if *end != id {
                                self.note(kinds, cascade, *end);
                            }
                        }
                    }
                }
            }
            if kind == Kind::Relationship {
                self.reach_from_relationship(kinds, cascade, id);
                continue;
            }
            let placement = (self.schema.is_a(&entity.type_name, "IfcProduct"))
                .then(|| kinds.reference(entity, "ObjectPlacement"))
                .flatten();
            let mut targets = Vec::new();
            for value in &entity.attributes {
                value.for_each_ref(&mut |target| targets.push(target));
            }
            for target in targets {
                if Some(target) == placement {
                    if !cascade.removed.contains(&target) {
                        cascade.placements.insert(target);
                    }
                } else {
                    self.consider(kinds, cascade, target);
                }
            }
        }
    }

    /// A removed relationship's ends: the openings of a removed element
    /// and the ports of one go; the rest is reached.
    fn reach_from_relationship(&self, kinds: &Kinds<'_>, cascade: &mut Cascade, id: EntityId) {
        let Some(entity) = self.view.get(id) else {
            return;
        };
        let t = &entity.type_name;
        let gone = |cascade: &Cascade, end: Option<EntityId>| {
            end.is_some_and(|end| cascade.removed.contains(&end))
        };
        let mut owned = Vec::new();
        if self.schema.is_a(t, "IfcRelVoidsElement")
            && gone(cascade, kinds.reference(entity, "RelatingBuildingElement"))
        {
            owned.extend(kinds.reference(entity, "RelatedOpeningElement"));
        }
        if self.schema.is_a(t, "IfcRelNests")
            && gone(cascade, kinds.reference(entity, "RelatingObject"))
        {
            owned.extend(
                kinds
                    .references(entity, "RelatedObjects")
                    .into_iter()
                    .filter(|part| self.is_kind(kinds, *part, Kind::Port)),
            );
        }
        if self.schema.is_a(t, "IfcRelConnectsPortToElement")
            && gone(cascade, kinds.reference(entity, "RelatedElement"))
        {
            owned.extend(kinds.reference(entity, "RelatingPort"));
        }
        for part in owned {
            if self.view.get(part).is_some() {
                cascade.add(part);
            }
        }
        let mut targets = Vec::new();
        for value in &entity.attributes {
            value.for_each_ref(&mut |target| targets.push(target));
        }
        for target in targets {
            self.consider(kinds, cascade, target);
        }
    }

    fn is_kind(&self, kinds: &Kinds<'_>, id: EntityId, kind: Kind) -> bool {
        self.view
            .get(id)
            .is_some_and(|entity| kinds.of(&entity.type_name) == kind)
    }

    /// An entity a removed one references: removed when solely owned and
    /// of a kind the cascade removes, reached otherwise.
    fn consider(&self, kinds: &Kinds<'_>, cascade: &mut Cascade, id: EntityId) {
        if cascade.removed.contains(&id) || cascade.keep.contains(&id) {
            return;
        }
        let Some(entity) = self.view.get(id) else {
            return;
        };
        match kinds.of(&entity.type_name) {
            Kind::Relationship => {}
            Kind::Root | Kind::Opening | Kind::Port => {
                cascade.reach(id, KeepReason::RelatedObject);
            }
            Kind::Definition | Kind::OwnerChain | Kind::Context => {
                cascade.reach(id, KeepReason::Definition);
            }
            Kind::PropertySet
            | Kind::Resource
            | Kind::LayerAssignment
            | Kind::StyledItem
            | Kind::ShapeAspect => {
                if self.users(kinds, &cascade.removed, id).is_empty() {
                    cascade.add(id);
                } else {
                    cascade.reach(id, KeepReason::Shared);
                }
            }
        }
    }

    /// Report an end of a relationship that stays, without removing it.
    fn note(&self, kinds: &Kinds<'_>, cascade: &mut Cascade, id: EntityId) {
        let Some(entity) = self.view.get(id) else {
            return;
        };
        let reason = match kinds.of(&entity.type_name) {
            Kind::Relationship => return,
            Kind::Root | Kind::Opening | Kind::Port => KeepReason::RelatedObject,
            Kind::Definition | Kind::OwnerChain | Kind::Context => KeepReason::Definition,
            _ => KeepReason::Shared,
        };
        cascade.reach(id, reason);
    }

    /// The entities that stay and still need `id` once `removed` goes.
    fn users(
        &self,
        kinds: &Kinds<'_>,
        removed: &BTreeSet<EntityId>,
        id: EntityId,
    ) -> Vec<EntityId> {
        let mut users = Vec::new();
        for referrer in self.view.referrers(id) {
            if removed.contains(&referrer) {
                continue;
            }
            let Some(by) = self.view.get(referrer) else {
                continue;
            };
            let needs = match kinds.of(&by.type_name) {
                Kind::LayerAssignment => false,
                Kind::StyledItem => kinds.reference(by, "Item") != Some(id),
                Kind::ShapeAspect => {
                    kinds.reference(by, "PartOfProductDefinitionShape") != Some(id)
                }
                Kind::Relationship => {
                    loses_an_end(by, &|other| other == id || removed.contains(&other))
                }
                _ => true,
            };
            if needs {
                users.push(referrer);
            }
        }
        users
    }

    /// The placements relative to `placement` that stay, and the other
    /// entities that stay and use it.
    fn placement_users(
        &self,
        kinds: &Kinds<'_>,
        removed: &BTreeSet<EntityId>,
        placement: EntityId,
    ) -> (Vec<EntityId>, Vec<EntityId>) {
        let (mut users, mut dependents) = (Vec::new(), Vec::new());
        for user in self.users(kinds, removed, placement) {
            let relative = self.view.get(user).is_some_and(|entity| {
                self.schema.is_a(&entity.type_name, "IfcObjectPlacement")
                    && kinds.reference(entity, "PlacementRelTo") == Some(placement)
            });
            if relative {
                dependents.push(user);
            } else {
                users.push(user);
            }
        }
        (users, dependents)
    }

    /// Decide the removed products' placements, last, when every product
    /// and opening the cascade removes is known.
    fn settle_placements(
        &mut self,
        kinds: &Kinds<'_>,
        cascade: &mut Cascade,
        options: RemoveProductOptions,
    ) -> Result<(), F> {
        loop {
            let free: Vec<EntityId> = cascade
                .placements
                .iter()
                .copied()
                .filter(|placement| !cascade.removed.contains(placement))
                .filter(|placement| {
                    let (users, dependents) =
                        self.placement_users(kinds, &cascade.removed, *placement);
                    users.is_empty() && dependents.is_empty()
                })
                .collect();
            if free.is_empty() {
                break;
            }
            for placement in free {
                cascade.add(placement);
            }
            self.drain(kinds, cascade);
        }
        // What is left is shared with an object that stays, or has
        // placements relative to it that stay.
        let mut doomed = BTreeSet::new();
        let mut dependents_of = BTreeMap::new();
        for placement in cascade.placements.clone() {
            if cascade.removed.contains(&placement) {
                continue;
            }
            let (users, dependents) = self.placement_users(kinds, &cascade.removed, placement);
            if users.is_empty() {
                doomed.insert(placement);
                dependents_of.insert(placement, dependents);
            } else {
                cascade.reach(placement, KeepReason::Shared);
            }
        }
        let mut surviving = BTreeSet::new();
        for (placement, dependents) in &dependents_of {
            let staying: Vec<EntityId> = dependents
                .iter()
                .copied()
                .filter(|dependent| !doomed.contains(dependent))
                .collect();
            if !staying.is_empty() && !options.rebase_placements {
                return Err(F::PlacementHasDependents {
                    placement: *placement,
                    dependents: staying,
                });
            }
            surviving.extend(staying);
        }
        let mut freed = Vec::new();
        for dependent in surviving {
            freed.extend(self.rebase(kinds, dependent, &doomed)?);
            cascade.rebased.push(dependent);
        }
        for placement in doomed {
            cascade.add(placement);
        }
        self.drain(kinds, cascade);
        for previous in freed {
            self.consider(kinds, cascade, previous);
        }
        self.drain(kinds, cascade);
        Ok(())
    }

    /// Re-base `placement` onto its nearest ancestor outside `doomed`,
    /// composing the doomed placements' transforms into its own. Returns
    /// the relative placement it no longer uses.
    fn rebase(
        &mut self,
        kinds: &Kinds<'_>,
        placement: EntityId,
        doomed: &BTreeSet<EntityId>,
    ) -> Result<Option<EntityId>, F> {
        let (mut transform, previous) = self.local_transform(kinds, placement)?;
        let mut parent = self.parent_placement(kinds, placement);
        let mut seen = BTreeSet::from([placement]);
        while let Some(ancestor) = parent.filter(|ancestor| doomed.contains(ancestor)) {
            if !seen.insert(ancestor) {
                return Err(F::UnsupportedRebase {
                    placement: ancestor,
                    detail: "its PlacementRelTo chain is a cycle".to_owned(),
                });
            }
            transform = self.local_transform(kinds, ancestor)?.0.then(&transform);
            parent = self.parent_placement(kinds, ancestor);
        }
        let axes = (!transform.is_unrotated()).then_some((transform.z, transform.x));
        let relative = self.axis_placement(transform.origin, axes)?;
        self.edit(
            placement,
            vec![
                (
                    "PlacementRelTo".to_owned(),
                    parent.map_or(Value::Null, Value::Ref),
                ),
                ("RelativePlacement".to_owned(), Value::Ref(relative)),
            ],
        )?;
        Ok(previous)
    }

    fn parent_placement(&self, kinds: &Kinds<'_>, placement: EntityId) -> Option<EntityId> {
        self.view
            .get(placement)
            .and_then(|entity| kinds.reference(entity, "PlacementRelTo"))
    }

    /// The transform an `IfcLocalPlacement` applies relative to its
    /// parent, with the `IfcAxis2Placement3D` it reads it from.
    fn local_transform(
        &self,
        kinds: &Kinds<'_>,
        placement: EntityId,
    ) -> Result<(Transform, Option<EntityId>), F> {
        let unsupported = |id: EntityId, detail: String| F::UnsupportedRebase {
            placement: id,
            detail,
        };
        let entity = self
            .view
            .get(placement)
            .ok_or(F::MissingEntity(placement))?;
        if !self.schema.is_a(&entity.type_name, "IfcLocalPlacement") {
            return Err(unsupported(
                placement,
                format!("an {}, not an IfcLocalPlacement", entity.type_name),
            ));
        }
        let relative = kinds
            .reference(entity, "RelativePlacement")
            .ok_or_else(|| unsupported(placement, "no RelativePlacement".to_owned()))?;
        let axis = self.view.get(relative).ok_or(F::MissingEntity(relative))?;
        if !self.schema.is_a(&axis.type_name, "IfcAxis2Placement3D") {
            return Err(unsupported(
                placement,
                format!("its RelativePlacement is an {}", axis.type_name),
            ));
        }
        let read = |name: &str, kind: &str| -> Result<Option<[f64; 3]>, F> {
            let Some(id) = kinds.reference(axis, name) else {
                return Ok(None);
            };
            let entity = self.view.get(id).ok_or(F::MissingEntity(id))?;
            let attribute = if self.schema.is_a(&entity.type_name, kind) {
                if kind == "IfcCartesianPoint" {
                    "Coordinates"
                } else {
                    "DirectionRatios"
                }
            } else {
                return Err(unsupported(
                    placement,
                    format!("its {name} is an {}", entity.type_name),
                ));
            };
            let values: Option<Vec<f64>> = kinds
                .slot(&entity.type_name, attribute)
                .and_then(|slot| entity.attributes.get(slot))
                .and_then(Value::as_list)
                .map(|items| {
                    items
                        .iter()
                        .map(|item| item.unwrap_typed().as_f64())
                        .collect()
                })
                .and_then(|values: Option<Vec<f64>>| values);
            match values.as_deref() {
                Some([x, y, z]) if [x, y, z].iter().all(|v| v.is_finite()) => {
                    Ok(Some([*x, *y, *z]))
                }
                _ => Err(unsupported(
                    placement,
                    format!("its {name} is not three finite numbers"),
                )),
            }
        };
        let location = read("Location", "IfcCartesianPoint")?
            .ok_or_else(|| unsupported(placement, "no Location".to_owned()))?;
        let z = read("Axis", "IfcDirection")?;
        let x = read("RefDirection", "IfcDirection")?;
        let transform = Transform::from_axes(location, z, x).ok_or_else(|| {
            unsupported(
                placement,
                "its Axis and RefDirection are degenerate".to_owned(),
            )
        })?;
        Ok((transform, Some(relative)))
    }

    /// Detach what stays from what goes, refuse what cannot be detached,
    /// remove the cascade and report it.
    fn apply_cascade(&mut self, kinds: &Kinds<'_>, cascade: Cascade) -> Result<RemovalReport, F> {
        let mut removed = cascade.removed;
        let mut detached = BTreeSet::new();
        for id in &removed {
            for referrer in self.view.referrers(*id) {
                if removed.contains(&referrer) {
                    continue;
                }
                let kind = self
                    .view
                    .get(referrer)
                    .map(|entity| kinds.of(&entity.type_name));
                match kind {
                    Some(Kind::Relationship | Kind::LayerAssignment) => {
                        detached.insert(referrer);
                    }
                    _ => {
                        return Err(F::StillReferenced {
                            id: *id,
                            by: referrer,
                        })
                    }
                }
            }
        }
        let mut emptied = BTreeSet::new();
        for id in &detached {
            let Some(mut entity) = self.view.get(*id).cloned() else {
                continue;
            };
            if detach(&mut entity, &|other| removed.contains(&other)) {
                emptied.insert(*id);
            } else {
                self.view.replace(*id, entity);
            }
        }
        for id in &emptied {
            if let Some(by) = self
                .view
                .referrers(*id)
                .into_iter()
                .find(|by| !removed.contains(by))
            {
                return Err(F::StillReferenced { id: *id, by });
            }
            detached.remove(id);
        }
        removed.extend(emptied);
        let mut report = RemovalReport {
            detached: detached.into_iter().collect(),
            rebased: cascade.rebased,
            ..RemovalReport::default()
        };
        for (id, reason) in cascade.reached {
            if removed.contains(&id) {
                continue;
            }
            let Some(entity) = self.view.get(id) else {
                continue;
            };
            let users = if reason == KeepReason::Shared {
                self.users(kinds, &removed, id)
            } else {
                Vec::new()
            };
            report.kept.push(KeptEntity {
                id,
                type_name: entity.type_name.to_string(),
                reason,
                users,
            });
        }
        for id in &removed {
            if let Some(entity) = self.view.get(*id) {
                report.removed.push(RemovedEntity {
                    id: *id,
                    type_name: entity.type_name.to_string(),
                });
            }
        }
        for id in removed {
            self.view.remove(id);
        }
        Ok(report)
    }
}
