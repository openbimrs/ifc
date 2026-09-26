//! Reading a native rule's operands, and saying so when they cannot be read.
//!
//! # Three outcomes, never two
//!
//! A rule reading one attribute of one instance gets one of:
//!
//! - a value of the shape the rule reasons about -- it is evaluated;
//! - no value (`$`, `*`, or a slot the record does not have) -- the rule is
//!   not evaluated, because presence is `structure::required`'s concern and
//!   the IFC rules implemented here guard their optional operands themselves
//!   (`NOT(EXISTS(Priority)) OR ...`);
//! - anything else -- the rule **could not be decided**, and that is reported
//!   as [`Severity::EvaluationError`](crate::Severity::EvaluationError).
//!
//! The third case used to be a silent `continue`. Collapsing it into the
//! second made a rule that could not read its input indistinguishable from a
//! rule that passed.

use ifc_model::{Entity, EntityId, Model, Value};
use ifc_schema::Schema;

use crate::report::{Finding, Path, Report};
use crate::type_check::describe_value;

/// One rule applied to one instance.
pub(super) struct Site<'a> {
    /// The rule id every finding from this site carries.
    pub rule: &'a str,
    /// The instance being checked.
    pub id: EntityId,
    /// Its record.
    pub entity: &'a Entity,
    /// The tables the rule's attribute names are resolved against.
    pub schema: &'a Schema,
}

impl<'a> Site<'a> {
    /// The slot of `attribute`, or an evaluation error when the schema
    /// tables do not declare it for this entity.
    ///
    /// Every native rule is written against attribute names the bundled
    /// schemas declare, so a miss means the rule and the tables disagree --
    /// a validator defect, which must not read as a pass.
    pub fn slot(&self, attribute: &str, report: &mut Report) -> Option<usize> {
        let index = self.lookup(attribute);
        if index.is_none() {
            self.undeclared(attribute, report);
        }
        index
    }

    /// The slot of `attribute`, without reporting a miss.
    ///
    /// For a rule that may still be decided without this operand -- a
    /// disjunction another operand already satisfies -- and so reports the
    /// miss itself through [`Self::undeclared`] only when it matters.
    pub fn lookup(&self, attribute: &str) -> Option<usize> {
        self.schema
            .attribute_names(&self.entity.type_name)
            .iter()
            .position(|name| name.eq_ignore_ascii_case(attribute))
    }

    /// Reports that the tables do not declare an operand the rule needs.
    pub fn undeclared(&self, attribute: &str, report: &mut Report) {
        report.push(Finding::evaluation_error(
            self.rule,
            Path::Entity(self.id),
            format!(
                "schema {} declares no attribute {attribute} on {}, so the rule \
                 cannot be evaluated",
                self.schema.name(),
                self.entity.type_name
            ),
        ));
    }

    /// The value in `index`, or `None` when it is unset or absent.
    pub fn value(&self, index: usize) -> Option<&'a Value> {
        self.entity
            .attribute(index)
            .filter(|value| !matches!(value, Value::Null | Value::Derived))
    }

    /// The path of one of this instance's slots.
    pub fn path(&self, index: usize, attribute: &str) -> Path {
        Path::Attribute {
            entity: self.id,
            index,
            name: Some(attribute.into()),
        }
    }

    /// Reports an operand whose written shape the rule cannot reason about.
    pub fn unreadable(
        &self,
        index: usize,
        attribute: &str,
        expected: &str,
        found: &Value,
        report: &mut Report,
    ) {
        report.push(Finding::evaluation_error(
            self.rule,
            self.path(index, attribute),
            format!(
                "{attribute} must be {expected} for the rule to be evaluated; the file wrote {}",
                describe_value(found)
            ),
        ));
    }

    /// The reference in `index`, reporting any other written shape.
    ///
    /// `None` without a finding when the slot is unset.
    pub fn reference(
        &self,
        index: usize,
        attribute: &str,
        report: &mut Report,
    ) -> Option<EntityId> {
        let value = self.value(index)?;
        let reference = value.unwrap_typed().as_ref_id();
        if reference.is_none() {
            self.unreadable(index, attribute, "an entity reference", value, report);
        }
        reference
    }

    /// The record `target` names, reporting a target the file lacks.
    ///
    /// The dangling reference itself is `structure`'s finding; this one says
    /// that the rule, which needed the target's type, was not decided.
    pub fn target(
        &self,
        model: &'a Model,
        target: EntityId,
        index: usize,
        attribute: &str,
        report: &mut Report,
    ) -> Option<&'a Entity> {
        let entity = model.get(target);
        if entity.is_none() {
            report.push(Finding::evaluation_error(
                self.rule,
                self.path(index, attribute),
                format!(
                    "{attribute} names {target}, which the file does not contain, so its \
                     type cannot be tested"
                ),
            ));
        }
        entity
    }
}
