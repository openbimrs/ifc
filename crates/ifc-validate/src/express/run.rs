//! Running every rule of an [`ExpressRules`] over a model.
//!
//! - **Global rules** run once over the file.
//! - **Entity WHERE rules** run on every instance of the declaring entity
//!   and of its subtypes (rules are inherited), in ascending id order.
//! - **Type WHERE rules** run on every value of the defined type found in
//!   an explicit attribute: directly, inside an aggregate, through a
//!   defined type based on it, or wrapped in a SELECT (`IFCLABEL('x')`),
//!   as IfcOpenShell's `express_rules` mode does.
//!
//! A rule that evaluates FALSE is an error, UNKNOWN or `?` an
//! [`Severity::Indeterminate`](crate::Severity::Indeterminate) finding. A
//! rule the evaluator cannot run is reported once, as unsupported, naming
//! what it needs; data it cannot read and a per-rule budget are evaluation
//! errors on the instance; the run budget stops evaluation and truncates
//! the report.

use std::collections::HashSet;

use ifc_model::{EntityId, Model, Value};

use super::ast::{DomainRule, Logical, TypeExpr};
use super::eval::{logical, Cx};
use super::machine::{Fault, Machine};
use super::rules::ExpressRules;
use super::value::{Inst, Val};
use crate::report::{
    BudgetExceeded, Finding, FindingDetail, NotEvaluated, Path, Report, RuleDetail, RuleOutcome,
    RuleScope,
};
use crate::where_rule::Budget;

/// Evaluates every rule in `rules` against `model`.
pub(crate) fn evaluate(model: &Model, rules: &ExpressRules, budget: Budget, report: &mut Report) {
    let machine = Machine::new(&rules.tables, model, rules.limits);
    let mut run = Run {
        machine: &machine,
        budget,
        report,
        unsupported: HashSet::new(),
        reported: HashSet::new(),
        stopped: false,
    };
    run.globals();
    let mut instances: Vec<(EntityId, usize)> = Vec::new();
    for (name, _) in model.type_histogram() {
        if let Some(entity) = rules.tables.entity_index(name) {
            instances.extend(model.ids_of_type(name).iter().map(|id| (*id, entity)));
        }
    }
    instances.sort_unstable();
    for (id, entity) in instances {
        if run.stopped {
            break;
        }
        run.entity_rules(id, entity);
        run.type_rules(id, entity);
    }
}

struct Run<'r, 'm> {
    machine: &'r Machine<'m>,
    budget: Budget,
    report: &'r mut Report,
    /// Rule ids already reported as unsupported.
    unsupported: HashSet<String>,
    /// `(rule id, path)` pairs already reported, so a value repeated in an
    /// aggregate reports a type rule once per attribute.
    reported: HashSet<(String, Path)>,
    stopped: bool,
}

/// What one rule application is about, for its findings.
struct Site<'a> {
    id: String,
    scope: RuleScope,
    rule: &'a DomainRule,
    path: Path,
    /// The instance's type, for the message.
    subject: String,
}

impl Run<'_, '_> {
    fn release(&self) -> String {
        self.machine.tables.schema.name.to_string()
    }

    fn globals(&mut self) {
        let tables = self.machine.tables;
        for decl in &tables.schema.rules {
            self.machine.reset_steps();
            let mut cx = Cx::new(Val::Indet, decl.frame);
            let prepared = self
                .machine
                .init_locals(&decl.locals, &mut cx)
                .and_then(|()| self.machine.block(&decl.body, &mut cx).map(|_| ()));
            for rule in &decl.rules {
                let site = Site {
                    id: format!("global.{}", decl.spelling),
                    scope: RuleScope::Global {
                        rule: decl.spelling.to_string(),
                    },
                    rule,
                    path: Path::File,
                    subject: "the file".into(),
                };
                let result = match &prepared {
                    Err(fault) => Err(fault.clone()),
                    Ok(()) => self
                        .machine
                        .eval(&rule.expr, &mut cx)
                        .and_then(|value| logical(&value)),
                };
                self.record(&site, result);
            }
        }
    }

    fn entity_rules(&mut self, id: EntityId, entity: usize) {
        let tables = self.machine.tables;
        let subject = tables.schema.entities[entity].spelling.to_string();
        for &owner in &tables.ancestors[entity] {
            let declared = &tables.schema.entities[owner];
            for rule in &declared.rules {
                if self.stopped {
                    return;
                }
                let site = Site {
                    id: format!("{}.{}", declared.spelling, rule.label),
                    scope: RuleScope::Entity {
                        declared_on: declared.spelling.to_string(),
                    },
                    rule,
                    path: Path::Entity(id),
                    subject: subject.clone(),
                };
                if self.unsupported.contains(&site.id) {
                    continue;
                }
                let result = self.apply(rule, Val::Inst(Inst::Model(id)));
                self.record(&site, result);
            }
        }
    }

    fn type_rules(&mut self, id: EntityId, entity: usize) {
        let tables = self.machine.tables;
        let Some(record) = self.machine.model.get(id) else {
            return;
        };
        for (slot, &(owner, idx)) in tables.layout[entity].iter().enumerate() {
            let Some(raw) = record.attribute(slot) else {
                continue;
            };
            let attr = &tables.schema.entities[owner].explicit[idx];
            let path = Path::Attribute {
                entity: id,
                index: slot,
                name: Some(attr.spelling.to_string()),
            };
            self.walk(raw, &attr.ty, &path, 0);
        }
    }

    /// Checks every type rule `ty` imposes on `raw`, and on its parts.
    fn walk(&mut self, raw: &Value, ty: &TypeExpr, path: &Path, depth: usize) {
        if self.stopped || depth > 16 {
            return;
        }
        let tables = self.machine.tables;
        match (raw, ty) {
            (Value::Null | Value::Derived, _) => {}
            (Value::List(items), TypeExpr::Aggregate { elem, .. }) => {
                for item in items {
                    self.walk(item, elem, path, depth + 1);
                }
            }
            (_, TypeExpr::Named(name)) => {
                let Some(&index) = tables.globals.types.get(name) else {
                    return;
                };
                let decl = &tables.schema.types[index];
                match (&decl.underlying, raw) {
                    (TypeExpr::Select(_), Value::Typed { type_name, value }) => {
                        if let Some(member) = tables.type_index(type_name) {
                            let member = TypeExpr::Named(tables.schema.types[member].name.clone());
                            self.walk(value, &member, path, depth + 1);
                        }
                    }
                    (TypeExpr::Select(_), _) => {}
                    (underlying, _) => {
                        for rule in &decl.rules {
                            let site = Site {
                                id: format!("{}.{}", decl.spelling, rule.label),
                                scope: RuleScope::Type {
                                    type_name: decl.spelling.to_string(),
                                },
                                rule,
                                path: path.clone(),
                                subject: format!("{} value", decl.spelling),
                            };
                            if self.unsupported.contains(&site.id)
                                || self.reported.contains(&(site.id.clone(), path.clone()))
                            {
                                continue;
                            }
                            let value = self.machine.typed(raw, ty);
                            let result = value.and_then(|value| self.apply(rule, value));
                            self.record(&site, result);
                        }
                        let inner = match raw {
                            Value::Typed { value, .. } => value,
                            other => other,
                        };
                        self.walk(inner, underlying, path, depth + 1);
                    }
                }
            }
            _ => {}
        }
    }

    fn apply(&self, rule: &DomainRule, this: Val) -> Result<Logical, Fault> {
        self.machine.reset_steps();
        let mut cx = Cx::new(this, rule.frame);
        let value = self.machine.eval(&rule.expr, &mut cx)?;
        logical(&value)
    }

    fn record(&mut self, site: &Site<'_>, result: Result<Logical, Fault>) {
        let outcome = match result {
            Ok(Logical::True) => return,
            Ok(Logical::False) => RuleOutcome::Violated,
            Ok(Logical::Unknown) => RuleOutcome::Indeterminate,
            Err(Fault::Unsupported(construct)) => {
                RuleOutcome::NotEvaluated(NotEvaluated::Unsupported { construct })
            }
            Err(Fault::Budget(limit)) => {
                RuleOutcome::NotEvaluated(NotEvaluated::BudgetExceeded(limit))
            }
            Err(Fault::Data(detail)) => {
                RuleOutcome::NotEvaluated(NotEvaluated::InvalidData { detail })
            }
        };
        if self.report.findings().len() >= self.budget.max_findings {
            self.report.mark_truncated();
            self.stopped = true;
            return;
        }
        let release = self.release();
        let label = &site.rule.label;
        let declared = match &site.scope {
            RuleScope::Entity { declared_on } => declared_on.clone(),
            RuleScope::Type { type_name } => type_name.clone(),
            RuleScope::Global { rule } => rule.clone(),
        };
        let detail = FindingDetail::Rule(RuleDetail {
            release: release.clone(),
            scope: site.scope.clone(),
            label: label.to_string(),
            outcome: outcome.clone(),
        });
        let finding = match &outcome {
            RuleOutcome::Violated => Finding::error(
                site.id.clone(),
                site.path.clone(),
                format!("{} violates {declared}.{label} ({release})", site.subject),
            ),
            RuleOutcome::Indeterminate => Finding::indeterminate(
                site.id.clone(),
                site.path.clone(),
                format!(
                    "{declared}.{label} is UNKNOWN for {} ({release}): the data does not decide it",
                    site.subject
                ),
            ),
            RuleOutcome::NotEvaluated(NotEvaluated::Unsupported { construct }) => {
                self.unsupported.insert(site.id.clone());
                Finding::unsupported(
                    site.id.clone(),
                    Path::File,
                    format!(
                        "{declared}.{label} ({release}) was not evaluated: it needs {construct}"
                    ),
                )
            }
            RuleOutcome::NotEvaluated(NotEvaluated::BudgetExceeded(
                limit @ BudgetExceeded::Total { .. },
            )) => {
                self.stopped = true;
                self.report.mark_truncated();
                Finding::evaluation_error(
                    "where.express.budget",
                    Path::File,
                    format!("EXPRESS rule evaluation stopped at {declared}.{label}: {limit}"),
                )
            }
            RuleOutcome::NotEvaluated(reason) => Finding::evaluation_error(
                site.id.clone(),
                site.path.clone(),
                format!(
                    "{declared}.{label} ({release}) could not be decided for {}: {reason}",
                    site.subject
                ),
            ),
        };
        self.reported.insert((site.id.clone(), site.path.clone()));
        self.report.push(finding.with_detail(detail));
    }
}
