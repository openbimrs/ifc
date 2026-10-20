//! INVERSE attribute cardinality (#461).
//!
//! An INVERSE clause states how many instances of the `FOR` entity may
//! reference an instance through the `FOR` attribute: `SET [1:?]` at least
//! one, `SET [0:1]` at most one, and a single entity (no aggregate) exactly
//! one. `IfcOpeningElement.VoidsElements` is such a single inverse: an
//! opening that voids no element, or two, breaks it.
//!
//! The clauses come from the declared release's own tables, inherited ones
//! included; a subtype redeclaring an inverse replaces it. Referrers are
//! counted once each through the reverse index, as the facade's INVERSE
//! reads do (#462): an instance of the `FOR` entity or a subtype, holding
//! the target in the `FOR` attribute's slot. An upper bound written as an
//! expression is not evaluated.

use std::collections::HashMap;

use ifc_model::{EntityId, Model, ReverseIndex};
use ifc_schema::{Aggregation, Schema};

use crate::report::{Finding, Path, Report};

/// One INVERSE clause of a type, resolved for counting.
struct Clause<'s> {
    name: &'s str,
    entity: &'s str,
    slot: usize,
    lower: u64,
    upper: Option<u64>,
    declared: String,
}

/// Reports every instance referenced by more or fewer instances than one
/// of its INVERSE clauses allows.
pub fn inverse_cardinality(model: &Model, schema: &Schema, report: &mut Report) {
    let mut types: Vec<(&str, usize)> = model.type_histogram();
    types.sort_unstable_by_key(|(name, _)| *name);
    let clauses: Vec<(&str, Vec<Clause<'_>>)> = types
        .iter()
        .map(|(name, _)| (*name, clauses(schema, name)))
        .filter(|(_, clauses)| !clauses.is_empty())
        .collect();
    if clauses.is_empty() {
        return;
    }
    let reverse = ReverseIndex::build(model);
    let mut is_a: HashMap<(&str, &str), bool> = HashMap::new();
    let mut findings: Vec<(EntityId, Finding)> = Vec::new();
    for (type_name, clauses) in &clauses {
        for &id in model.ids_of_type(type_name) {
            for clause in clauses {
                let mut count = 0u64;
                let mut last = None;
                for referrer in reverse.referrers(id) {
                    if referrer.slot != clause.slot || last == Some(referrer.from) {
                        continue;
                    }
                    let Some(source) = model.get(referrer.from) else {
                        continue;
                    };
                    let source_type: &str = &source.type_name;
                    let fits = *is_a
                        .entry((source_type, clause.entity))
                        .or_insert_with(|| schema.is_a(source_type, clause.entity));
                    if fits {
                        count += 1;
                        last = Some(referrer.from);
                    }
                }
                let too_many = clause.upper.is_some_and(|upper| count > upper);
                if count < clause.lower || too_many {
                    findings.push((
                        id,
                        Finding::error(
                            "structure.inverse.cardinality",
                            Path::Entity(id),
                            format!(
                                "{} is {}, but {count} {} reference it",
                                clause.name,
                                clause.declared,
                                if count == 1 {
                                    "instance does"
                                } else {
                                    "instances do"
                                }
                            ),
                        ),
                    ));
                }
            }
        }
    }
    findings.sort_by_key(|(id, _)| *id);
    for (_, finding) in findings {
        report.push(finding);
    }
}

/// Every INVERSE clause of `type_name`, inherited first, redeclarations
/// replacing what they redeclare.
fn clauses<'s>(schema: &'s Schema, type_name: &str) -> Vec<Clause<'s>> {
    let Some(entity) = schema.entity(type_name) else {
        return Vec::new();
    };
    let mut chain: Vec<&str> = schema.supertypes(&entity.name);
    chain.reverse();
    chain.push(&entity.name);
    let mut out: Vec<Clause<'s>> = Vec::new();
    for owner in chain {
        let Some(owner) = schema.entity(owner) else {
            continue;
        };
        for inverse in &owner.inverses {
            let Some(slot) = schema
                .attribute_names(&inverse.entity)
                .iter()
                .position(|name| name.eq_ignore_ascii_case(&inverse.for_attribute))
            else {
                continue;
            };
            let (lower, upper, declared) = bounds(inverse.aggregation.as_ref(), &inverse.entity);
            let clause = Clause {
                name: &inverse.name,
                entity: &inverse.entity,
                slot,
                lower,
                upper,
                declared,
            };
            match out
                .iter_mut()
                .find(|existing| existing.name.eq_ignore_ascii_case(&inverse.name))
            {
                Some(existing) => *existing = clause,
                None => out.push(clause),
            }
        }
    }
    out
}

/// The bounds of a clause and how it reads: a single inverse is `[1:1]`.
fn bounds(aggregation: Option<&Aggregation>, entity: &str) -> (u64, Option<u64>, String) {
    let Some(aggregation) = aggregation else {
        return (1, Some(1), format!("exactly one {entity}"));
    };
    let lower = aggregation.lower.as_integer().unwrap_or(0);
    let upper = aggregation.upper.as_integer();
    let upper_text = upper.map_or_else(|| "?".to_owned(), |u| u.to_string());
    let kind = format!("{:?}", aggregation.kind).to_ascii_uppercase();
    (
        lower,
        upper,
        format!("{kind} [{lower}:{upper_text}] OF {entity}"),
    )
}

#[cfg(all(test, feature = "ifc4"))]
mod tests {
    use super::*;
    use ifc_model::{Entity, Value};

    fn opening(voids: usize) -> Report {
        let schema = ifc_schema::ifc4();
        let mut model = Model::new();
        let width = schema.attribute_count("IfcOpeningElement");
        let opening = model.push(Entity::new("IFCOPENINGELEMENT", vec![Value::Null; width]));
        let names = schema.attribute_names("IfcRelVoidsElement");
        let slot = names
            .iter()
            .position(|n| *n == "RelatedOpeningElement")
            .expect("declared");
        for _ in 0..voids {
            let mut attributes = vec![Value::Null; names.len()];
            attributes[slot] = Value::Ref(opening);
            model.push(Entity::new("IFCRELVOIDSELEMENT", attributes));
        }
        let mut report = Report::new();
        inverse_cardinality(&model, schema, &mut report);
        report
    }

    /// `VoidsElements` is a single inverse: exactly one relation.
    #[test]
    fn a_single_inverse_needs_exactly_one_referrer() {
        assert_eq!(opening(1).findings().len(), 0);
        for count in [0, 2] {
            let report = opening(count);
            assert_eq!(report.findings().len(), 1, "{count}");
            let finding = &report.findings()[0];
            assert_eq!(finding.rule, "structure.inverse.cardinality");
            assert!(
                finding.message.contains("VoidsElements"),
                "{}",
                finding.message
            );
        }
    }
}
