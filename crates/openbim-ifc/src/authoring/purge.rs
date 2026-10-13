//! Purging unreferenced entities (#478).
//!
//! Without classes, the purge keeps what the model's objects use: every
//! `IfcRoot`, everything an `IfcRoot` reaches through references, and,
//! to a fixed point, every annotation of something kept (a styled item,
//! a presentation layer assignment, a shape aspect, a material's
//! representation or properties, a resource-level relationship, a
//! coordinate operation, a colour or texture map), with what it reaches.
//! The owner history with its actors and the representation contexts are
//! kept too unless asked. Everything else goes: geometry no product
//! shows, property values no set holds, styles no item wears.
//!
//! With classes, it removes the entities of those classes, subtypes
//! included, that nothing references once the rest of the purge is gone;
//! an `IfcRoot` among them only with `include_roots`, and the owner
//! history chain only with `include_owner_history`.
//!
//! Nothing the purge removes is referenced by anything that stays, so it
//! detaches nothing and refuses nothing but an unknown class.

use std::collections::BTreeSet;

use ifc_model::EntityId;

use super::kinds::{Kind, Kinds};
use super::plan::Planner;
use super::report::{KeepReason, KeptEntity, RemovalReport, RemovedEntity};
use super::{AuthoringFailure as F, PurgeOptions};

impl Planner<'_, '_> {
    pub(super) fn purge(&mut self, options: &PurgeOptions) -> Result<RemovalReport, F> {
        let kinds = Kinds::new(self.schema);
        let mut classes = Vec::new();
        for class in &options.classes {
            let definition = self.schema.entity(class).ok_or_else(|| F::UnknownEntity {
                schema: self.schema.name().to_owned(),
                type_name: class.clone(),
            })?;
            classes.push(definition.name.clone());
        }
        let ids = self.view.ids();
        let (purged, protected) = if classes.is_empty() {
            self.unreachable(&kinds, &ids, options)
        } else {
            self.unreferenced(&kinds, &ids, &classes, options)
        };
        let mut report = RemovalReport::default();
        for id in protected {
            if let Some(entity) = self.view.get(id) {
                report.kept.push(KeptEntity {
                    id,
                    type_name: entity.type_name.to_string(),
                    reason: KeepReason::Protected,
                    users: Vec::new(),
                });
            }
        }
        for id in &purged {
            if let Some(entity) = self.view.get(*id) {
                report.removed.push(RemovedEntity {
                    id: *id,
                    type_name: entity.type_name.to_string(),
                });
            }
        }
        for id in purged {
            self.view.remove(id);
        }
        Ok(report)
    }

    /// The default purge: what nothing kept reaches, and the protected
    /// entities it would otherwise have taken.
    fn unreachable(
        &self,
        kinds: &Kinds<'_>,
        ids: &[EntityId],
        options: &PurgeOptions,
    ) -> (BTreeSet<EntityId>, Vec<EntityId>) {
        let mut kept = BTreeSet::new();
        let mut stack = Vec::new();
        let flood = |kept: &mut BTreeSet<EntityId>, stack: &mut Vec<EntityId>| {
            while let Some(id) = stack.pop() {
                if let Some(entity) = self.view.get(id) {
                    for value in &entity.attributes {
                        value.for_each_ref(&mut |target| {
                            if kept.insert(target) {
                                stack.push(target);
                            }
                        });
                    }
                }
            }
        };
        for id in ids {
            if self
                .view
                .get(*id)
                .is_some_and(|entity| kinds.is_root(&entity.type_name))
                && kept.insert(*id)
            {
                stack.push(*id);
            }
        }
        flood(&mut kept, &mut stack);
        let mut protected = Vec::new();
        if !options.include_owner_history {
            for id in ids {
                let protects = self.view.get(*id).is_some_and(|entity| {
                    matches!(
                        kinds.of(&entity.type_name),
                        Kind::OwnerChain | Kind::Context
                    )
                });
                if protects && kept.insert(*id) {
                    protected.push(*id);
                    stack.push(*id);
                }
            }
            flood(&mut kept, &mut stack);
        }
        let annotators: Vec<EntityId> = ids
            .iter()
            .copied()
            .filter(|id| {
                !kept.contains(id)
                    && self
                        .view
                        .get(*id)
                        .is_some_and(|entity| kinds.annotates(&entity.type_name))
            })
            .collect();
        loop {
            let mut progress = false;
            for id in &annotators {
                if kept.contains(id) {
                    continue;
                }
                let Some(entity) = self.view.get(*id) else {
                    continue;
                };
                let mut annotates = false;
                for value in &entity.attributes {
                    value.for_each_ref(&mut |target| annotates |= kept.contains(&target));
                }
                if annotates {
                    kept.insert(*id);
                    stack.push(*id);
                    progress = true;
                }
            }
            flood(&mut kept, &mut stack);
            if !progress {
                break;
            }
        }
        let purged = ids
            .iter()
            .copied()
            .filter(|id| !kept.contains(id))
            .collect();
        (purged, protected)
    }

    /// The purge by class: entities of `classes` nothing references once
    /// the rest of the purge is gone, and the protected ones it skipped.
    fn unreferenced(
        &self,
        kinds: &Kinds<'_>,
        ids: &[EntityId],
        classes: &[String],
        options: &PurgeOptions,
    ) -> (BTreeSet<EntityId>, Vec<EntityId>) {
        let names_project = classes
            .iter()
            .any(|class| class.eq_ignore_ascii_case("IfcProject"));
        let mut candidates = BTreeSet::new();
        let mut skipped = Vec::new();
        for id in ids {
            let Some(entity) = self.view.get(*id) else {
                continue;
            };
            let t = &entity.type_name;
            if !classes.iter().any(|class| kinds.is_a(t, class)) {
                continue;
            }
            let allowed = if kinds.is_root(t) {
                options.include_roots && (names_project || !kinds.is_a(t, "IfcProject"))
            } else {
                options.include_owner_history || kinds.of(t) != Kind::OwnerChain
            };
            if allowed {
                candidates.insert(*id);
            } else {
                skipped.push(*id);
            }
        }
        let mut purged = BTreeSet::new();
        let mut queue: Vec<EntityId> = candidates.iter().copied().collect();
        while let Some(id) = queue.pop() {
            if purged.contains(&id) {
                continue;
            }
            if self
                .view
                .referrers(id)
                .iter()
                .any(|by| !purged.contains(by))
            {
                continue;
            }
            purged.insert(id);
            if let Some(entity) = self.view.get(id) {
                for value in &entity.attributes {
                    value.for_each_ref(&mut |target| {
                        if candidates.contains(&target) && !purged.contains(&target) {
                            queue.push(target);
                        }
                    });
                }
            }
        }
        let protected = skipped
            .into_iter()
            .filter(|id| {
                self.view
                    .referrers(*id)
                    .iter()
                    .all(|by| purged.contains(by))
            })
            .collect();
        (purged, protected)
    }
}
