//! Property sets compared with the `IfcPropertySetTemplate` that
//! `IfcRelDefinesByTemplate` links them to (#109).
//!
//! The check reads the model and its templates only, bound to the release
//! the header declares; the shipped Pset catalogue is not consulted (that is
//! `ifc-template-catalog`, which does not depend on the model). Each pair of
//! a set and one of its templates is compared for:
//!
//! - the set's entity against the template's `TemplateType` (`form.rs`);
//! - its members against the property templates, by `Name`: missing,
//!   unexpected, wrong form, wrong measure type, recursively into complex
//!   properties and complex templates (`members.rs`);
//! - every object carrying the set against the template's `TemplateType`
//!   and `ApplicableEntity` (`applicability.rs`).
//!
//! What the release's documentation leaves open is reported as
//! [`TemplateFinding::Undecided`] rather than guessed, and malformed facts
//! met on the way as [`PropertyAnomaly`].
//!
//! ## Internal split
//!
//! - `finding.rs`: the public report types.
//! - `form.rs`: what each template type prescribes, per release.
//! - `members.rs`: members read and compared.
//! - `applicability.rs`: carriers, attachment side and `ApplicableEntity`.

mod applicability;
mod finding;
mod form;
mod members;

use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use ifc_model::{Entity, EntityId, Model};

pub use finding::{MeasureRole, TemplateFinding, TemplateReport, UndecidedReason};

use super::layout::Layout;
use super::property_set::{read_set_template, template_links, PropertySetTemplate};
use crate::error::{PropertyAnomaly, TemplateError};
use crate::nesting::Nesting;
use applicability::{applicable, carriers, Verdict};
use form::{attachment, set_entity};
use members::{read_members, Comparison, Member};

/// Compare every templated property set in the model with its templates.
///
/// A set is templated when an `IfcRelDefinesByTemplate` names it; a set
/// with several templates is compared with each. Untemplated sets are not
/// looked at. See [`TemplateFinding`] for what is reported and
/// [`TemplateReport`] for the order.
///
/// # Errors
///
/// [`TemplateError::Release`] when the model binds to no single supported
/// release (as [`exact_schema`](crate::exact_schema) fails), and
/// [`TemplateError::NoTemplates`] for a release without templates (IFC2X3).
pub fn template_deviations(model: &Model) -> Result<TemplateReport, TemplateError> {
    let layout = Layout::declared(model)?;
    let mut report = TemplateReport::default();
    let links = template_links(model, layout);
    for &id in model.ids_of_type("IFCRELDEFINESBYTEMPLATE") {
        if let Some(rel) = model.get(id) {
            layout.check_arity(id, rel, &mut report.anomalies);
        }
    }
    let templates = read_templates(model, layout, &links, &mut report.anomalies);
    let templated: BTreeSet<EntityId> = links.keys().copied().collect();
    let carried = carriers(model, layout, &templated, &mut report.anomalies);
    let no_carriers = BTreeSet::new();
    for (&set_id, pairs) in &links {
        let relationship = pairs[0].0;
        let Some(set) = model.get(set_id) else {
            report.anomalies.push(PropertyAnomaly::MissingDefinition {
                relationship,
                definition: set_id,
            });
            continue;
        };
        if !layout.is_a(&set.type_name, "IFCPROPERTYSETDEFINITION") {
            report.anomalies.push(PropertyAnomaly::MalformedAttribute {
                entity: relationship,
                attribute: "RelatedPropertySets",
                found: format!("#{} {}", set_id.0, set.type_name),
            });
            continue;
        }
        layout.check_arity(set_id, set, &mut report.anomalies);
        let members = set_members(model, layout, set_id, set, &mut report.anomalies);
        let mut ids: Vec<EntityId> = pairs.iter().map(|(_, template)| *template).collect();
        ids.sort_unstable();
        ids.dedup();
        for template in ids.iter().filter_map(|id| templates.get(id)) {
            let pair = Pair {
                set_id,
                set,
                members: members.as_deref(),
                template,
                objects: carried.get(&set_id).unwrap_or(&no_carriers),
            };
            pair.check(model, layout, &mut report.findings);
        }
    }
    Ok(report)
}

/// Read each linked template once, reporting links that name none.
fn read_templates(
    model: &Model,
    layout: Layout,
    links: &BTreeMap<EntityId, Vec<(EntityId, EntityId)>>,
    anomalies: &mut Vec<PropertyAnomaly>,
) -> BTreeMap<EntityId, PropertySetTemplate> {
    // Each template once, with the first relationship naming it.
    let mut first: BTreeMap<EntityId, EntityId> = BTreeMap::new();
    for &(relationship, template) in links.values().flatten() {
        first
            .entry(template)
            .and_modify(|seen| *seen = (*seen).min(relationship))
            .or_insert(relationship);
    }
    let mut read = BTreeMap::new();
    for (id, relationship) in first {
        match model.get(id) {
            None => anomalies.push(PropertyAnomaly::MissingDefinition {
                relationship,
                definition: id,
            }),
            Some(entity) => match read_set_template(model, layout, id, anomalies) {
                Some(template) => {
                    read.insert(id, template);
                }
                None => anomalies.push(PropertyAnomaly::NotATemplate {
                    container: relationship,
                    member: id,
                    type_name: entity.type_name.to_string(),
                }),
            },
        }
    }
    read
}

/// The members of a property set or element quantity; `None` for any other
/// property set definition, whose members are not named properties.
fn set_members<'m>(
    model: &'m Model,
    layout: Layout,
    id: EntityId,
    set: &'m Entity,
    anomalies: &mut Vec<PropertyAnomaly>,
) -> Option<Vec<Member<'m>>> {
    let attribute = if layout.is_a(&set.type_name, "IFCPROPERTYSET") {
        "HasProperties"
    } else if layout.is_a(&set.type_name, "IFCELEMENTQUANTITY") {
        "Quantities"
    } else {
        return None;
    };
    let mut nesting = Nesting::new(anomalies);
    let list = layout.get(set, attribute);
    Some(read_members(
        model,
        layout,
        id,
        list,
        attribute,
        &mut nesting,
    ))
}

/// One set and one of its templates.
struct Pair<'a, 'm> {
    set_id: EntityId,
    set: &'m Entity,
    members: Option<&'a [Member<'m>]>,
    template: &'a PropertySetTemplate,
    objects: &'a BTreeSet<EntityId>,
}

impl Pair<'_, '_> {
    fn check(&self, model: &Model, layout: Layout, findings: &mut Vec<TemplateFinding>) {
        let set = self.set_id;
        let template = self.template.id;
        let template_type = self.template.template_type.as_ref();
        let undecided = |subject, reason| TemplateFinding::Undecided {
            set,
            template,
            subject,
            reason,
        };
        let rule = match template_type {
            None => Some(form::Attachment::Any),
            Some(token) => {
                let rule = attachment(layout, token);
                if rule.is_none() {
                    let value = token.clone();
                    findings.push(undecided(
                        template,
                        UndecidedReason::UnknownTemplateType { value },
                    ));
                }
                rule
            }
        };
        if let (Some(_), Some(token)) = (rule, template_type) {
            if let Some(expected) = set_entity(token) {
                if !layout.is_a(&self.set.type_name, expected) {
                    findings.push(TemplateFinding::WrongSetKind {
                        set,
                        template,
                        template_type: token.clone(),
                        expected,
                        found: self.set.type_name.clone(),
                    });
                }
            }
        }
        match self.members {
            Some(members) => Comparison {
                model,
                layout,
                set,
                template,
                findings: &mut *findings,
            }
            .members(set, members, &self.template.properties),
            None => findings.push(undecided(
                set,
                UndecidedReason::PredefinedSet {
                    found: self.set.type_name.clone(),
                },
            )),
        }
        for &object in self.objects {
            let entity = model.get(object).expect("carriers are in the model");
            if let (Some(rule), Some(token)) = (rule, template_type) {
                if !rule.admits(layout, &entity.type_name) {
                    findings.push(TemplateFinding::WrongAttachment {
                        set,
                        template,
                        object,
                        template_type: token.clone(),
                        found: entity.type_name.clone(),
                    });
                }
            }
            let Some(applicable_entity) = &self.template.applicable_entity else {
                continue;
            };
            match applicable(layout, entity, applicable_entity) {
                Verdict::Admits => {}
                Verdict::Excludes => findings.push(TemplateFinding::OutsideApplicableEntity {
                    set,
                    template,
                    object,
                    found: entity.type_name.clone(),
                    applicable_entity: Arc::clone(applicable_entity),
                }),
                Verdict::Undecided(reason) => findings.push(undecided(object, reason)),
            }
        }
    }
}
