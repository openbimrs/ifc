//! A templated set's members, compared with its property templates.
//!
//! IFC4 ADD2 TC1 `IfcPropertySetTemplate`: "Between IfcProperty's within the
//! HasProperties set of IfcPropertySet having the same Name attribute value
//! as the IfcPropertyTemplate's within the HasPropertyTemplates set of
//! IfcPropertySetTemplate an implicit definition relationship is
//! established". Members and templates are therefore matched by `Name`
//! alone, exactly (the same rule holds one level down for a complex
//! property and its complex template), and each matched pair is checked
//! for form and measure type (`form.rs`).
//!
//! The member tree is read once per set through [`Nesting`], so a complex
//! property that cycles, nests too deep or exhausts the budget is cut and
//! reported exactly as the property readers report it; the template tree
//! was cut the same way when it was read.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use ifc_model::{Entity, EntityId, Model, Value};

use super::super::layout::Layout;
use super::finding::{MeasureRole, TemplateFinding, UndecidedReason};
use super::form::{measured, prescribed, Prescribed};
use crate::error::PropertyAnomaly;
use crate::nesting::Nesting;
use crate::template::{PropertyTemplate, PropertyTemplateKind};

/// Complex entities and the attribute holding their members.
const NESTED: [(&str, &str); 2] = [
    ("IFCCOMPLEXPROPERTY", "HasProperties"),
    ("IFCPHYSICALCOMPLEXQUANTITY", "HasQuantities"),
];

/// A property or quantity of a templated set, with its nested members.
pub(super) struct Member<'m> {
    id: EntityId,
    entity: &'m Entity,
    name: Option<Arc<str>>,
    children: Vec<Member<'m>>,
}

/// Read the members listed in `set`'s `attribute`, in file order.
pub(super) fn read_members<'m>(
    model: &'m Model,
    layout: Layout,
    set: EntityId,
    list: Option<&Value>,
    attribute: &'static str,
    nesting: &mut Nesting<'_>,
) -> Vec<Member<'m>> {
    let mut members = Vec::new();
    for member in nesting.members(set, attribute, list) {
        if nesting.admit(model, set, member) {
            members.push(read_member(model, layout, member, nesting));
        }
    }
    members
}

fn read_member<'m>(
    model: &'m Model,
    layout: Layout,
    id: EntityId,
    nesting: &mut Nesting<'_>,
) -> Member<'m> {
    let entity = model.get(id).expect("admitted members are in the model");
    let mut found = Vec::new();
    layout.check_arity(id, entity, &mut found);
    let name = layout.text(id, entity, "Name", &mut found);
    let unset = layout.get(entity, "Name").map(Value::unwrap_typed);
    if matches!(unset, None | Some(Value::Null)) {
        // `Name` is required on every `IfcProperty` and `IfcPhysicalQuantity`.
        found.push(PropertyAnomaly::MalformedAttribute {
            entity: id,
            attribute: "Name",
            found: "Null".to_owned(),
        });
    }
    for anomaly in found {
        nesting.report(anomaly);
    }
    let mut children = Vec::new();
    if let Some((_, attribute)) = NESTED
        .iter()
        .find(|(complex, _)| layout.is_a(&entity.type_name, complex))
    {
        if nesting.enter(id) {
            let list = layout.get(entity, attribute);
            children = read_members(model, layout, id, list, attribute, nesting);
            nesting.leave();
        }
    }
    Member {
        id,
        entity,
        name,
        children,
    }
}

/// One set compared with one set template.
pub(super) struct Comparison<'a> {
    pub(super) model: &'a Model,
    pub(super) layout: Layout,
    pub(super) set: EntityId,
    pub(super) template: EntityId,
    pub(super) findings: &'a mut Vec<TemplateFinding>,
}

impl Comparison<'_> {
    fn undecided(&mut self, subject: EntityId, reason: UndecidedReason) {
        self.findings.push(TemplateFinding::Undecided {
            set: self.set,
            template: self.template,
            subject,
            reason,
        });
    }

    /// Compare the members of `container` with `templates`.
    pub(super) fn members(
        &mut self,
        container: EntityId,
        members: &[Member<'_>],
        templates: &[PropertyTemplate],
    ) {
        // The first template of each name governs it; a repeated name was
        // reported as an anomaly when the template was read.
        let mut governing: BTreeMap<&str, &PropertyTemplate> = BTreeMap::new();
        for template in templates {
            match template.name.as_deref() {
                Some(name) => {
                    governing.entry(name).or_insert(template);
                }
                None => self.undecided(template.id, UndecidedReason::UnnamedTemplate),
            }
        }
        let mut present = BTreeSet::new();
        for member in members {
            // A nameless member was reported as an anomaly when read.
            let Some(name) = member.name.as_deref() else {
                continue;
            };
            present.insert(name);
            match governing.get(name) {
                Some(template) => self.member(member, template),
                None => self.findings.push(TemplateFinding::UnexpectedProperty {
                    set: self.set,
                    template: self.template,
                    container,
                    property: member.id,
                    name: name.into(),
                }),
            }
        }
        for template in templates {
            let Some(name) = template.name.as_deref() else {
                continue;
            };
            let governs = governing.get(name).is_some_and(|t| t.id == template.id);
            if governs && !present.contains(name) {
                self.findings.push(TemplateFinding::MissingProperty {
                    set: self.set,
                    template: self.template,
                    container,
                    property_template: template.id,
                    name: name.into(),
                });
            }
        }
    }

    /// Check one member against the template of its name.
    fn member(&mut self, member: &Member<'_>, template: &PropertyTemplate) {
        let template_type = template.template_type.as_deref();
        let expected = match prescribed(self.layout, template.kind, template_type) {
            Prescribed::Entities(expected) => expected,
            other => {
                let value: Arc<str> = template_type.unwrap_or_default().into();
                let reason = if other == Prescribed::Unknown {
                    UndecidedReason::UnknownTemplateType { value }
                } else {
                    UndecidedReason::UndocumentedTemplateType { value }
                };
                self.undecided(template.id, reason);
                return;
            }
        };
        let found = &member.entity.type_name;
        if !expected
            .iter()
            .any(|entity| self.layout.is_a(found, entity))
        {
            self.findings.push(TemplateFinding::WrongForm {
                set: self.set,
                template: self.template,
                property: member.id,
                property_template: template.id,
                template_type: template.template_type.clone(),
                expected,
                found: found.clone(),
            });
            return;
        }
        if let Some(template_type) = template_type {
            self.measure(member, template, template_type, MeasureRole::Primary);
            self.measure(member, template, template_type, MeasureRole::Secondary);
        }
        if template.kind == PropertyTemplateKind::Complex && !template.templates.is_empty() {
            self.members(member.id, &member.children, &template.templates);
        }
    }

    /// Check the values a measure type of `template` determines.
    fn measure(
        &mut self,
        member: &Member<'_>,
        template: &PropertyTemplate,
        template_type: &str,
        measure: MeasureRole,
    ) {
        let label = match measure {
            MeasureRole::Primary => &template.primary_measure,
            MeasureRole::Secondary => &template.secondary_measure,
        };
        let Some(expected) = label else {
            return;
        };
        let attributes = measured(template_type, measure);
        if attributes.is_empty() {
            return;
        }
        let schema = self.layout.schema();
        if schema.type_def(expected).is_none() && schema.entity(expected).is_none() {
            let value = expected.clone();
            self.undecided(
                template.id,
                UndecidedReason::UnknownMeasureType { measure, value },
            );
            return;
        }
        for &attribute in attributes {
            let Some(value) = self.layout.get(member.entity, attribute) else {
                continue;
            };
            match self.nonconforming(expected, value) {
                Ok(None) => {}
                Ok(Some(found)) => self.findings.push(TemplateFinding::WrongMeasureType {
                    set: self.set,
                    template: self.template,
                    property: member.id,
                    property_template: template.id,
                    measure,
                    attribute,
                    expected: expected.clone(),
                    found,
                }),
                Err(()) => self.undecided(member.id, UndecidedReason::UntypedValue { attribute }),
            }
        }
    }

    /// The type of the first value in `value` that `expected` does not
    /// admit; `Err` when a value's type cannot be read.
    fn nonconforming(&self, expected: &str, value: &Value) -> Result<Option<Arc<str>>, ()> {
        let items = match value {
            Value::List(items) => items.as_slice(),
            single => std::slice::from_ref(single),
        };
        let schema = self.layout.schema();
        for item in items {
            let found = match item {
                Value::Null => continue,
                Value::Typed { type_name, .. } => type_name.clone(),
                Value::Ref(id) => self.model.get(*id).ok_or(())?.type_name.clone(),
                _ => return Err(()),
            };
            if !schema.accepts_type(expected, &found) {
                return Ok(Some(found));
            }
        }
        Ok(None)
    }
}
