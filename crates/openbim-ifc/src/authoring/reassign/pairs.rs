//! A type object with its occurrences (#636): which entities a type rule
//! moves along with the one asked for, and how.

use ifc_model::EntityId;

use super::type_rule::{occurrence_class, type_rule};
use crate::authoring::plan::Planner;
use crate::authoring::relate::TYPED;
use crate::authoring::report::{ReassignReport, RelationshipConflict};
use crate::authoring::{AuthoringFailure as F, NamedValues, ReassignClassOptions};

/// An occurrence whose class's type rule its type object's class fails.
struct Unsatisfied {
    occurrence: EntityId,
    relationship: EntityId,
    class: String,
    rule: String,
    required: String,
}

impl Planner<'_, '_> {
    /// Change `id` to the class `type_name`, with the entities a type rule
    /// moves along (rule 7); see the module docs.
    pub(in crate::authoring) fn reassign_class(
        &mut self,
        id: EntityId,
        type_name: &str,
        attributes: NamedValues,
        options: ReassignClassOptions,
    ) -> Result<ReassignReport, F> {
        let is_type = self
            .view
            .get(id)
            .is_some_and(|entity| self.schema.is_a(&entity.type_name, "IfcTypeObject"));
        let mut paired = Vec::new();
        let mut report = if is_type {
            let occurrences = self.related_to(&TYPED, id);
            let mut report = self.reassign_one(id, type_name, attributes, options)?;
            self.settle_occurrences(id, &occurrences, options, &mut report, &mut paired)?;
            report
        } else {
            if options.move_pairs {
                self.move_type_of(id, type_name, options, &mut paired)?;
            }
            self.reassign_one(id, type_name, attributes, options)?
        };
        paired.sort_by_key(|pair| pair.entity);
        report.paired = paired;
        Ok(report)
    }

    /// The occurrences of the reassigned type object `id` that its new class
    /// does not satisfy: refused, taken out of their relationship, or moved.
    fn settle_occurrences(
        &mut self,
        id: EntityId,
        occurrences: &[(EntityId, EntityId)],
        options: ReassignClassOptions,
        report: &mut ReassignReport,
        paired: &mut Vec<ReassignReport>,
    ) -> Result<(), F> {
        let occurrences: Vec<EntityId> = occurrences.iter().map(|(object, _)| *object).collect();
        for unsatisfied in self.unsatisfied(id, &occurrences)? {
            if options.move_pairs {
                let target = self.pairing_class(&unsatisfied, &report.to)?;
                paired.push(self.reassign_pair(unsatisfied.occurrence, &target, options)?);
            } else if options.remove_relationships {
                let reason = RelationshipConflict::TypeRule {
                    type_object: id,
                    to_rule: unsatisfied.rule,
                };
                report.removed_relationships.push(self.leave_relationship(
                    unsatisfied.occurrence,
                    unsatisfied.relationship,
                    reason,
                )?);
            } else {
                return Err(F::TypeNotAccepted {
                    type_object: id,
                    to: report.to.clone(),
                    object: unsatisfied.occurrence,
                    rule: unsatisfied.rule,
                    required: unsatisfied.required,
                });
            }
        }
        report
            .removed_relationships
            .sort_by_key(|entry| entry.relationship);
        Ok(())
    }

    /// Where the type object of the occurrence `id` fails the type rule of
    /// its new class `to`: the type object moves to the class the rule
    /// requires, then so do its other occurrences that class fails.
    fn move_type_of(
        &mut self,
        id: EntityId,
        to: &str,
        options: ReassignClassOptions,
        paired: &mut Vec<ReassignReport>,
    ) -> Result<(), F> {
        let Some(relationship) = self.related_by(&TYPED, id) else {
            return Ok(());
        };
        let Some(type_object) = self.relating_of(&TYPED, relationship) else {
            return Ok(());
        };
        let schema = self.schema;
        let Some(class) = schema.entity(to) else {
            return Ok(());
        };
        let from = self
            .view
            .get(id)
            .map_or_else(String::new, |entity| entity.type_name.to_string());
        let Some(rule) =
            type_rule(schema, &class.name).map_err(|detail| F::UnsupportedReassignment {
                entity: id,
                from,
                to: class.name.clone(),
                detail,
            })?
        else {
            return Ok(());
        };
        if self.satisfies(type_object, rule.type_class) {
            return Ok(());
        }
        paired.push(self.reassign_pair(type_object, rule.type_class, options)?);
        let others: Vec<EntityId> = self
            .related_to(&TYPED, type_object)
            .into_iter()
            .map(|(object, _)| object)
            .filter(|object| *object != id)
            .collect();
        let new_class = paired[paired.len() - 1].to.clone();
        for unsatisfied in self.unsatisfied(type_object, &others)? {
            let target = self.pairing_class(&unsatisfied, &new_class)?;
            paired.push(self.reassign_pair(unsatisfied.occurrence, &target, options)?);
        }
        Ok(())
    }

    /// Whether `type_object`'s class is `required` or a subtype.
    fn satisfies(&self, type_object: EntityId, required: &str) -> bool {
        self.view
            .get(type_object)
            .is_some_and(|entity| self.schema.is_a(&entity.type_name, required))
    }

    /// Of `occurrences`, those whose class has a type rule that
    /// `type_object`'s current class fails, ascending.
    fn unsatisfied(
        &self,
        type_object: EntityId,
        occurrences: &[EntityId],
    ) -> Result<Vec<Unsatisfied>, F> {
        let schema = self.schema;
        let mut found = Vec::new();
        for &occurrence in occurrences {
            let Some(entity) = self.view.get(occurrence) else {
                continue;
            };
            let class = schema
                .entity(&entity.type_name)
                .map_or_else(|| entity.type_name.to_string(), |def| def.name.clone());
            let rule = type_rule(schema, &class).map_err(|detail| F::UnsupportedReassignment {
                entity: occurrence,
                from: class.clone(),
                to: class.clone(),
                detail,
            })?;
            let Some(rule) = rule else { continue };
            if self.satisfies(type_object, rule.type_class) {
                continue;
            }
            let Some(relationship) = self.related_by(&TYPED, occurrence) else {
                continue;
            };
            found.push(Unsatisfied {
                occurrence,
                relationship,
                class,
                rule: rule.name(),
                required: rule.type_class.to_owned(),
            });
        }
        Ok(found)
    }

    /// The class an unsatisfied occurrence moves to for the type class
    /// `type_class`.
    fn pairing_class(&self, unsatisfied: &Unsatisfied, type_class: &str) -> Result<String, F> {
        occurrence_class(self.schema, type_class)
            .map(str::to_owned)
            .ok_or_else(|| F::UnsupportedReassignment {
                entity: unsatisfied.occurrence,
                from: unsatisfied.class.clone(),
                to: type_class.to_owned(),
                detail: format!("no occurrence class pairs with {type_class}"),
            })
    }

    /// Reassign a paired entity with its attributes unchanged.
    fn reassign_pair(
        &mut self,
        entity: EntityId,
        class: &str,
        options: ReassignClassOptions,
    ) -> Result<ReassignReport, F> {
        let options = ReassignClassOptions {
            move_pairs: false,
            ..options
        };
        self.reassign_one(entity, class, Vec::new(), options)
            .map_err(|cause| F::PairNotReassigned {
                entity,
                to: class.to_owned(),
                cause: Box::new(cause),
            })
    }
}
