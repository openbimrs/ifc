//! Copying entities from another model into this one (#479): IfcOpenShell's
//! `project.append_asset` and `file.add`, as one checked transaction.
//!
//! # What comes along
//!
//! Each entity named, everything it references (its placement chain, its
//! representation down to the points, its owner history, its type's
//! property sets), and what belongs to an object without being referenced
//! by it:
//!
//! - its property and quantity sets, its type, its material, its
//!   classification, document and library references, each with the
//!   relationship that assigns it, related to the copies only;
//! - the openings of an element, the filling of an opening, and the ports
//!   of an element, each with its relationship;
//! - what attaches to a copied resource: styled items, shape aspects,
//!   colour and texture maps, a material's properties and representation,
//!   the presentation layers (holding only the copies).
//!
//! Any other relationship (a spatial containment, an aggregation, a
//! connection) comes along only when every end it relates is copied too,
//! a list end keeping only its copied members; otherwise it is reported as
//! skipped. The caller relates the copies in the target afterwards
//! (`Contain`, `Aggregate`).
//!
//! # Reuse, releases, units and identity
//!
//! A unit, representation context, owner history, material or style
//! equal to one the target holds is not copied: the copies reference the
//! target's ([`ReuseOptions`]). Equal is the same type and attribute
//! values, references compared alike, all the way down.
//!
//! Every copy is written in the target's release. Between releases each
//! attribute moves by name: a type the target does not declare, an
//! attribute it lacks with a value set, a required one the source does not
//! carry, or an enumeration constant or typed value it does not know is
//! refused with [`IncompatibleRelease`](super::AuthoringFailure::IncompatibleRelease),
//! never dropped.
//!
//! The models must measure lengths and plane angles alike: differing
//! units are refused with [`UnitMismatch`](super::AuthoringFailure::UnitMismatch)
//! before anything is copied. A model that declares no such unit counts as
//! its SI base unit without prefix (metre, radian).
//!
//! `GlobalId`s are kept, refused when the target holds one already, or
//! regenerated ([`GlobalIdMode`]).

use std::collections::{BTreeMap, BTreeSet};

use ifc_model::{Entity, EntityId, Model, Value};
use ifc_schema::Schema;

use super::check::{recheck, relations};
use super::kinds::{Kind, Kinds};
use super::overlay::Overlay;
use super::plan::Planner;
use super::report::CopyReport;
use super::{release, AppendOptions, AuthoringFailure as F, GlobalIdMode};

/// The source side of an append.
pub(super) struct Source<'a> {
    pub(super) view: Overlay<'a>,
    pub(super) schema: &'a Schema,
    pub(super) kinds: Kinds<'a>,
}

impl<'a> Source<'a> {
    pub(super) fn new(model: &'a Model, schema: &'a Schema) -> Self {
        Self {
            view: Overlay::new(model),
            schema,
            kinds: Kinds::new(schema),
        }
    }

    pub(super) fn get(&self, id: EntityId) -> Option<&Entity> {
        self.view.get(id)
    }

    fn is_a(&self, id: EntityId, ancestor: &str) -> bool {
        self.get(id)
            .is_some_and(|entity| self.schema.is_a(&entity.type_name, ancestor))
    }
}

/// The closure of an append: what is copied, what is reused.
#[derive(Default)]
struct Closure {
    take: BTreeSet<EntityId>,
    reused: BTreeMap<EntityId, EntityId>,
    queue: Vec<EntityId>,
    /// Relationships (and layers) taken whose list ends keep only what is
    /// copied, and are not followed.
    filtered: BTreeSet<EntityId>,
    /// Relationships the target holds already (`GlobalIdMode::Merge`):
    /// the copies join them.
    joined: BTreeMap<EntityId, EntityId>,
    skipped: BTreeSet<EntityId>,
}

impl Planner<'_, '_> {
    /// Copy `roots` of `source` into the model, by the rules of the module
    /// docs.
    pub(super) fn append(
        &mut self,
        source: &Source<'_>,
        roots: &[EntityId],
        options: &AppendOptions,
    ) -> Result<CopyReport, F> {
        self.check_units(source)?;
        let mut closure = Closure::default();
        for root in roots {
            if source.get(*root).is_none() {
                return Err(F::MissingEntity(*root));
            }
            self.reach(source, options, &mut closure, *root);
        }
        loop {
            self.drain_closure(source, options, &mut closure);
            if !self.carry_relationships(source, options, &mut closure) {
                break;
            }
        }
        let order = append_order(source, &closure.take)?;
        let mut copies: BTreeMap<EntityId, EntityId> = BTreeMap::new();
        let mut regenerated = Vec::new();
        let mut created = Vec::new();
        for original in order {
            let entity = source.get(original).ok_or(F::MissingEntity(original))?;
            let mut entity = entity.clone();
            let filtered = closure.filtered.contains(&original);
            for value in &mut entity.attributes {
                map_value(value, &copies, &closure.reused, filtered);
            }
            let (copy, fresh) = self.write_appended(source, options, original, entity)?;
            if fresh {
                regenerated.push(copy);
            }
            copies.insert(original, copy);
            created.push((original, copy));
        }
        let joined = self.join_appended(source, &closure, &copies)?;
        Ok(CopyReport {
            copied: self.copied_entities(created),
            reused: self.copied_entities(closure.reused.into_iter().collect()),
            joined,
            skipped: closure.skipped.into_iter().collect(),
            regenerated,
        })
    }

    /// The target entity that is source entity `id` by identity: with
    /// [`GlobalIdMode::Merge`], an `IfcRoot` of the same type holding the
    /// same `GlobalId`.
    fn identity(
        &mut self,
        source: &Source<'_>,
        options: &AppendOptions,
        id: EntityId,
    ) -> Option<EntityId> {
        if options.global_ids != GlobalIdMode::Merge {
            return None;
        }
        let entity = source.get(id)?;
        if !source.schema.is_a(&entity.type_name, "IfcRoot") {
            return None;
        }
        let global_id = entity.attributes.first()?.as_text()?.to_owned();
        let holder = self.global_id_holder(&global_id)?;
        self.view
            .get(holder)
            .is_some_and(|found| found.type_name.eq_ignore_ascii_case(&entity.type_name))
            .then_some(holder)
    }

    /// Take `id`, or reuse the target entity that is it or equals it.
    fn reach(
        &mut self,
        source: &Source<'_>,
        options: &AppendOptions,
        closure: &mut Closure,
        id: EntityId,
    ) {
        if closure.take.contains(&id) || closure.reused.contains_key(&id) {
            return;
        }
        let Some(entity) = source.get(id) else {
            return;
        };
        if let Some(holder) = self.identity(source, options, id) {
            closure.reused.insert(id, holder);
            return;
        }
        // A copy would break the clause: the holder of the key is it.
        if source.kinds.unique(&entity.type_name) {
            if let Some(holder) = self.find_by_key(source, &closure.reused, id) {
                closure.reused.insert(id, holder);
                return;
            }
        }
        if self.reusable(source, options, &entity.type_name) {
            if let Some(equal) = self.find_equal(source, &closure.reused, id) {
                closure.reused.insert(id, equal);
                return;
            }
        }
        closure.take.insert(id);
        closure.queue.push(id);
    }

    /// Take relationship `id`, its lists keeping only what is copied; with
    /// [`GlobalIdMode::Merge`], join the target's own instead when it holds
    /// it.
    fn take_relationship(
        &mut self,
        source: &Source<'_>,
        options: &AppendOptions,
        closure: &mut Closure,
        id: EntityId,
    ) -> bool {
        if let Some(holder) = self.identity(source, options, id) {
            return closure.joined.insert(id, holder).is_none();
        }
        if !closure.take.insert(id) {
            return false;
        }
        closure.filtered.insert(id);
        if let Some(owner) = source
            .get(id)
            .and_then(|by| source.kinds.reference(by, "OwnerHistory"))
        {
            self.reach(source, options, closure, owner);
        }
        true
    }

    /// Follow every queued entity: what it references, and what belongs
    /// to it (see the module docs).
    fn drain_closure(
        &mut self,
        source: &Source<'_>,
        options: &AppendOptions,
        closure: &mut Closure,
    ) {
        while let Some(id) = closure.queue.pop() {
            let Some(entity) = source.get(id) else {
                continue;
            };
            let mut targets = Vec::new();
            if !closure.filtered.contains(&id) {
                for value in &entity.attributes {
                    value.for_each_ref(&mut |target| targets.push(target));
                }
            }
            for target in targets {
                self.reach(source, options, closure, target);
            }
            for referrer in source.view.referrers(id) {
                if closure.take.contains(&referrer)
                    || closure.joined.contains_key(&referrer)
                    || closure.skipped.contains(&referrer)
                {
                    continue;
                }
                let Some(by) = source.get(referrer) else {
                    continue;
                };
                let t = &by.type_name;
                if source.schema.is_a(t, "IfcRelationship") {
                    if let Some(ends) = pulled_ends(source, by, id) {
                        self.take_relationship(source, options, closure, referrer);
                        for end in ends {
                            self.reach(source, options, closure, end);
                        }
                    }
                } else if source.schema.is_a(t, "IfcPresentationLayerAssignment") {
                    closure.take.insert(referrer);
                    closure.filtered.insert(referrer);
                    let items = source.kinds.slot(t, "AssignedItems");
                    let mut targets = Vec::new();
                    for (slot, value) in by.attributes.iter().enumerate() {
                        if Some(slot) != items {
                            value.for_each_ref(&mut |target| targets.push(target));
                        }
                    }
                    for target in targets {
                        self.reach(source, options, closure, target);
                    }
                } else if source.kinds.annotates(t) && source.kinds.of(t) != Kind::Relationship {
                    self.reach(source, options, closure, referrer);
                }
            }
        }
    }

    /// Take the relationships every end of which is copied or reused (a
    /// list end: some of its members); report the others as skipped.
    /// Whether anything new was taken.
    fn carry_relationships(
        &mut self,
        source: &Source<'_>,
        options: &AppendOptions,
        closure: &mut Closure,
    ) -> bool {
        let mut candidates = BTreeSet::new();
        for id in &closure.take {
            if closure.filtered.contains(id) {
                continue;
            }
            for referrer in source.view.referrers(*id) {
                let relationship = source
                    .get(referrer)
                    .is_some_and(|by| source.kinds.of(&by.type_name) == Kind::Relationship);
                if relationship
                    && !closure.take.contains(&referrer)
                    && !closure.joined.contains_key(&referrer)
                {
                    candidates.insert(referrer);
                }
            }
        }
        // A taken relationship whose ends are not all there is dropped too.
        candidates.extend(closure.filtered.iter().copied().filter(|id| {
            source
                .get(*id)
                .is_some_and(|by| source.kinds.of(&by.type_name) == Kind::Relationship)
        }));
        let mut grew = false;
        for relationship in candidates {
            let Some(entity) = source.get(relationship) else {
                continue;
            };
            let owner = source.kinds.slot(&entity.type_name, "OwnerHistory");
            let mut complete = true;
            for (slot, value) in entity.attributes.iter().enumerate() {
                if Some(slot) == owner {
                    continue;
                }
                let mut ends = Vec::new();
                value.for_each_ref(&mut |id| ends.push(id));
                let mut mapped = 0;
                for end in &ends {
                    if closure.take.contains(end) || closure.reused.contains_key(end) {
                        mapped += 1;
                    } else if let Some(holder) = self.identity(source, options, *end) {
                        closure.reused.insert(*end, holder);
                        mapped += 1;
                    }
                }
                let list = matches!(value, Value::List(_));
                if (list && !ends.is_empty() && mapped == 0) || (!list && mapped < ends.len()) {
                    complete = false;
                    break;
                }
            }
            if !complete {
                closure.take.remove(&relationship);
                closure.filtered.remove(&relationship);
                closure.skipped.insert(relationship);
            } else if !closure.take.contains(&relationship) {
                grew |= self.take_relationship(source, options, closure, relationship);
            }
        }
        grew || !closure.queue.is_empty()
    }

    /// Add the copies to the target relationships the append joins: each
    /// list the source relationship holds, by attribute name.
    fn join_appended(
        &mut self,
        source: &Source<'_>,
        closure: &Closure,
        copies: &BTreeMap<EntityId, EntityId>,
    ) -> Result<Vec<EntityId>, F> {
        let mut joined = Vec::new();
        for (original, holder) in &closure.joined {
            let Some(entity) = source.get(*original) else {
                continue;
            };
            let declared = source.schema.attributes(&entity.type_name);
            let mut edits = Vec::new();
            for (attribute, value) in declared.iter().zip(&entity.attributes) {
                let Value::List(items) = value else {
                    continue;
                };
                let added: Vec<Value> = items
                    .iter()
                    .filter_map(|item| item.as_ref_id())
                    .filter_map(|id| copies.get(&id).or_else(|| closure.reused.get(&id)))
                    .map(|id| Value::Ref(*id))
                    .collect();
                let Some(target) = self.view.get(*holder) else {
                    continue;
                };
                let Some(slot) = self
                    .schema
                    .attributes(&target.type_name)
                    .iter()
                    .position(|a| a.name.eq_ignore_ascii_case(&attribute.name))
                else {
                    continue;
                };
                let Some(Value::List(present)) = target.attributes.get(slot) else {
                    continue;
                };
                let mut members = present.clone();
                for value in added {
                    if !members.contains(&value) {
                        members.push(value);
                    }
                }
                if members.len() > present.len() {
                    edits.push((attribute.name.clone(), Value::List(members)));
                }
            }
            if !edits.is_empty() {
                self.edit(*holder, edits)?;
                joined.push(*holder);
            }
        }
        joined.sort_unstable();
        Ok(joined)
    }

    /// Write one appended entity in the target's release, with its
    /// `GlobalId` by the mode; whether that was regenerated.
    fn write_appended(
        &mut self,
        source: &Source<'_>,
        options: &AppendOptions,
        original: EntityId,
        entity: Entity,
    ) -> Result<(EntityId, bool), F> {
        let type_name = entity.type_name.to_string();
        let (from, into) = (
            source.schema.name().to_owned(),
            self.schema.name().to_owned(),
        );
        let incompatible = move |detail: String| F::IncompatibleRelease {
            entity: original,
            type_name: type_name.clone(),
            source: from.clone(),
            target: into.clone(),
            detail,
        };
        let cross = !source
            .schema
            .name()
            .eq_ignore_ascii_case(self.schema.name());
        let mut entity = if cross {
            release::migrate(source.schema, self.schema, &entity).map_err(&incompatible)?
        } else {
            if self.schema.entity(&entity.type_name).is_none() {
                return Err(incompatible(format!(
                    "{} does not declare {}",
                    self.schema.name(),
                    entity.type_name
                )));
            }
            entity
        };
        let id = self.view.allocate();
        let root = self.schema.is_a(&entity.type_name, "IfcRoot");
        let mut fresh = false;
        if root {
            let kept = entity
                .attributes
                .first()
                .and_then(Value::as_text)
                .map(str::to_owned);
            fresh = match options.global_ids {
                GlobalIdMode::Keep | GlobalIdMode::Merge => false,
                GlobalIdMode::Regenerate => true,
                GlobalIdMode::RegenerateColliding => {
                    kept.is_none_or(|kept| self.global_id_holder(&kept).is_some())
                }
            };
            if fresh {
                let global_id = self.fresh_global_id(id, &entity.type_name)?;
                if let Some(slot) = entity.attributes.first_mut() {
                    *slot = Value::Text(global_id.into());
                }
            }
        }
        let checked = recheck(self.schema, &entity)
            .and_then(|()| relations(self.schema, &self.view, &entity, None));
        match checked {
            Err(failure) if cross => return Err(incompatible(failure.to_string())),
            other => other?,
        }
        if root {
            self.claim_global_id(id, &entity)?;
        }
        self.view.create(id, entity);
        Ok((id, fresh))
    }
}

/// The ends a relationship brings along when it is reached from `id`: an
/// object's property sets, type and associations, an element's openings
/// and ports, an opening's filling. `None` for a relationship that does
/// not belong to `id` that way.
fn pulled_ends(source: &Source<'_>, relationship: &Entity, id: EntityId) -> Option<Vec<EntityId>> {
    let t = &relationship.type_name;
    let kinds = &source.kinds;
    let is = |class: &str| source.schema.is_a(t, class);
    let related = || {
        kinds
            .references(relationship, "RelatedObjects")
            .contains(&id)
    };
    let single = |name: &str| kinds.reference(relationship, name) == Some(id);
    if is("IfcRelDefinesByProperties") && related() {
        return Some(kinds.references(relationship, "RelatingPropertyDefinition"));
    }
    if is("IfcRelDefinesByType") && related() {
        return Some(kinds.references(relationship, "RelatingType"));
    }
    if is("IfcRelAssociates") && related() {
        let owner = kinds.slot(t, "OwnerHistory");
        let objects = kinds.slot(t, "RelatedObjects");
        let mut ends = Vec::new();
        for (slot, value) in relationship.attributes.iter().enumerate() {
            if Some(slot) != owner && Some(slot) != objects {
                value.for_each_ref(&mut |end| ends.push(end));
            }
        }
        return Some(ends);
    }
    if is("IfcRelVoidsElement") && single("RelatingBuildingElement") {
        return Some(kinds.references(relationship, "RelatedOpeningElement"));
    }
    if is("IfcRelFillsElement") && single("RelatingOpeningElement") {
        return Some(kinds.references(relationship, "RelatedBuildingElement"));
    }
    if is("IfcRelNests") && single("RelatingObject") {
        let ports: Vec<EntityId> = kinds
            .references(relationship, "RelatedObjects")
            .into_iter()
            .filter(|part| source.is_a(*part, "IfcPort"))
            .collect();
        return (!ports.is_empty()).then_some(ports);
    }
    if is("IfcRelConnectsPortToElement") && single("RelatedElement") {
        return Some(kinds.references(relationship, "RelatingPort"));
    }
    None
}

/// `value` with its references mapped to their copies or reused target
/// entities; for a filtered relationship, a list keeps only what maps.
fn map_value(
    value: &mut Value,
    copies: &BTreeMap<EntityId, EntityId>,
    reused: &BTreeMap<EntityId, EntityId>,
    filtered: bool,
) {
    let target = |id: &EntityId| copies.get(id).or_else(|| reused.get(id)).copied();
    match value {
        Value::Ref(id) => {
            if let Some(mapped) = target(id) {
                *id = mapped;
            }
        }
        Value::List(items) => {
            if filtered {
                items.retain(|item| !matches!(item, Value::Ref(id) if target(id).is_none()));
            }
            for item in items {
                map_value(item, copies, reused, filtered);
            }
        }
        Value::Typed { value, .. } => map_value(value, copies, reused, filtered),
        _ => {}
    }
}

/// The taken entities, each after what it references among them.
fn append_order(source: &Source<'_>, take: &BTreeSet<EntityId>) -> Result<Vec<EntityId>, F> {
    let mut order = Vec::with_capacity(take.len());
    let mut done = BTreeSet::new();
    let mut path = BTreeSet::new();
    for root in take {
        let mut stack = vec![(*root, false)];
        while let Some((id, expanded)) = stack.pop() {
            if expanded {
                path.remove(&id);
                if done.insert(id) {
                    order.push(id);
                }
                continue;
            }
            if done.contains(&id) {
                continue;
            }
            if !path.insert(id) {
                return Err(F::UnsupportedCopy {
                    entity: id,
                    detail: "its references form a cycle".to_owned(),
                });
            }
            stack.push((id, true));
            let Some(entity) = source.get(id) else {
                continue;
            };
            let mut targets = Vec::new();
            for value in &entity.attributes {
                value.for_each_ref(&mut |target| targets.push(target));
            }
            for target in targets.into_iter().rev() {
                if take.contains(&target) && !done.contains(&target) {
                    stack.push((target, false));
                }
            }
        }
    }
    Ok(order)
}
