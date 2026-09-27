//! Property templates: `IfcSimplePropertyTemplate` and
//! `IfcComplexPropertyTemplate`.
//!
//! The two concrete subtypes of `IfcPropertyTemplate` share only their
//! `IfcRoot` attributes, so each record is read by attribute name through
//! [`Layout`] and an attribute the entity does not declare stays unset.
//!
//! `IfcComplexPropertyTemplate.HasPropertyTemplates` nests further
//! templates. The schema forbids only a direct self-member
//! (`NoSelfReference`), and one template may be shared by several complex
//! templates (`PartOfComplexTemplate` is `SET [0:?]`), so nested templates
//! are read through [`Nesting`] exactly like nested complex properties: the
//! path from the root cuts a cycle of any length, and a depth bound and a
//! member budget cap the work, each cut reported as a [`PropertyAnomaly`].

use std::collections::BTreeMap;
use std::sync::Arc;

use ifc_model::{EntityId, Model, Value};

use super::layout::Layout;
use crate::error::{PropertyAnomaly, TemplateError};
use crate::nesting::Nesting;

/// Which concrete `IfcPropertyTemplate` a template is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum PropertyTemplateKind {
    /// `IfcSimplePropertyTemplate`: one simple property or simple quantity.
    Simple,
    /// `IfcComplexPropertyTemplate`: a complex property or complex quantity
    /// whose members are the nested [`PropertyTemplate::templates`].
    Complex,
}

/// A property template: what one property should look like.
///
/// Every attribute of both template entities has a field; the ones the
/// entity does not declare are `None` (or empty). Enumeration constants are
/// as written in the file, without their dots.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct PropertyTemplate {
    /// The entity.
    pub id: EntityId,
    /// `Name`: the property name this template governs.
    pub name: Option<Arc<str>>,
    /// `Description`.
    pub description: Option<Arc<str>>,
    /// Simple or complex.
    pub kind: PropertyTemplateKind,
    /// `TemplateType`: an `IfcSimplePropertyTemplateTypeEnum` constant such
    /// as `P_SINGLEVALUE` for a simple template, an
    /// `IfcComplexPropertyTemplateTypeEnum` constant (`P_COMPLEX`,
    /// `Q_COMPLEX`) for a complex one.
    ///
    /// This states which `IfcProperty` or `IfcPhysicalQuantity` subtype an
    /// instance should use, so it is the link between a template and the
    /// property families in `pset`.
    pub template_type: Option<Arc<str>>,
    /// `PrimaryMeasureType`, e.g. `IfcLengthMeasure` (simple only).
    pub primary_measure: Option<Arc<str>>,
    /// `SecondaryMeasureType`, used by bounded and table values (simple
    /// only).
    pub secondary_measure: Option<Arc<str>>,
    /// `Enumerators`: the `IfcPropertyEnumeration` an enumerated value
    /// selects from (simple only).
    pub enumerators: Option<EntityId>,
    /// `PrimaryUnit` (simple only).
    pub primary_unit: Option<EntityId>,
    /// `SecondaryUnit`, the defined unit of a table value (simple only).
    pub secondary_unit: Option<EntityId>,
    /// `Expression`: a table's correlation or a quantity's formula (simple
    /// only).
    pub expression: Option<Arc<str>>,
    /// `AccessState`, an `IfcStateEnum` constant such as `READONLY` (simple
    /// only).
    pub access_state: Option<Arc<str>>,
    /// `UsageName` (complex only).
    pub usage_name: Option<Arc<str>>,
    /// `HasPropertyTemplates`, in file order (complex only; empty when `$`).
    ///
    /// Members the traversal refuses (cycle, depth, budget, absent entity,
    /// non-template) are left out; the checked readers report each one.
    pub templates: Vec<PropertyTemplate>,
}

impl PropertyTemplate {
    /// Look up a nested template of a complex template by name.
    pub fn template(&self, name: &str) -> Option<&PropertyTemplate> {
        self.templates
            .iter()
            .find(|t| t.name.as_deref() == Some(name))
    }
}

/// Read one property template by id.
///
/// `None` when the entity is absent or is neither an
/// `IfcSimplePropertyTemplate` nor an `IfcComplexPropertyTemplate` in the
/// release the header declares (the IFC4 ADD2 TC1 table when it declares
/// none it bundles). Until #108 every entity was read with the simple
/// template's layout, so a complex template reported its `UsageName` as its
/// `TemplateType` and any other entity read as a template.
///
/// Nested templates the traversal refuses are left out without saying so;
/// use [`property_template_checked`] to have each one reported.
pub fn property_template(model: &Model, id: EntityId) -> Option<PropertyTemplate> {
    let mut anomalies = Vec::new();
    let mut nesting = Nesting::new(&mut anomalies);
    read_template(model, Layout::permissive(model), id, &mut nesting)
}

/// Read one property template bound to the release the model declares,
/// reporting every malformed fact met on the way.
///
/// The same value as [`property_template`], with a [`PropertyAnomaly`] for
/// each nested member left out ([`PropertyAnomaly::ComplexCycle`],
/// [`PropertyAnomaly::ComplexTooDeep`],
/// [`PropertyAnomaly::ComplexBudgetExceeded`],
/// [`PropertyAnomaly::MissingMember`], [`PropertyAnomaly::MemberNotReference`],
/// [`PropertyAnomaly::DuplicateMember`], [`PropertyAnomaly::NotATemplate`]),
/// each repeated template name ([`PropertyAnomaly::DuplicatePropertyName`]),
/// each record of the wrong arity ([`PropertyAnomaly::SlotCountMismatch`])
/// and each attribute its declared type does not admit
/// ([`PropertyAnomaly::MalformedAttribute`]).
///
/// # Errors
///
/// [`TemplateError::Release`] or [`TemplateError::NoTemplates`] when the
/// model binds to no release that has templates (IFC2X3 has none);
/// [`TemplateError::MissingEntity`] or [`TemplateError::NotATemplate`] for
/// the entity itself.
pub fn property_template_checked(
    model: &Model,
    id: EntityId,
) -> Result<(PropertyTemplate, Vec<PropertyAnomaly>), TemplateError> {
    let layout = Layout::declared(model)?;
    let entity = model.get(id).ok_or(TemplateError::MissingEntity { id })?;
    let mut anomalies = Vec::new();
    let mut nesting = Nesting::new(&mut anomalies);
    let template =
        read_template(model, layout, id, &mut nesting).ok_or(TemplateError::NotATemplate {
            id,
            type_name: entity.type_name.to_string(),
        })?;
    Ok((template, anomalies))
}

/// Read the template `id` with `layout`, or `None` when it is not one.
pub(crate) fn read_template(
    model: &Model,
    layout: Layout,
    id: EntityId,
    nesting: &mut Nesting<'_>,
) -> Option<PropertyTemplate> {
    let entity = model.get(id)?;
    let kind = if layout.is_a(&entity.type_name, "IFCSIMPLEPROPERTYTEMPLATE") {
        PropertyTemplateKind::Simple
    } else if layout.is_a(&entity.type_name, "IFCCOMPLEXPROPERTYTEMPLATE") {
        PropertyTemplateKind::Complex
    } else {
        return None;
    };
    let mut found = Vec::new();
    layout.check_arity(id, entity, &mut found);
    let out = &mut found;
    let mut template = PropertyTemplate {
        id,
        name: layout.text(id, entity, "Name", out),
        description: layout.text(id, entity, "Description", out),
        kind,
        template_type: layout.enumeration(id, entity, "TemplateType", out),
        primary_measure: layout.text(id, entity, "PrimaryMeasureType", out),
        secondary_measure: layout.text(id, entity, "SecondaryMeasureType", out),
        enumerators: layout.reference(model, id, entity, "Enumerators", out),
        primary_unit: layout.reference(model, id, entity, "PrimaryUnit", out),
        secondary_unit: layout.reference(model, id, entity, "SecondaryUnit", out),
        expression: layout.text(id, entity, "Expression", out),
        access_state: layout.enumeration(id, entity, "AccessState", out),
        usage_name: layout.text(id, entity, "UsageName", out),
        templates: Vec::new(),
    };
    for anomaly in found {
        nesting.report(anomaly);
    }
    if kind == PropertyTemplateKind::Complex {
        let members = layout.get(entity, "HasPropertyTemplates");
        template.templates = nested_templates(model, layout, id, members, nesting);
    }
    Some(template)
}

/// Resolve the `HasPropertyTemplates` of `container` in file order.
///
/// `container` is a complex template, entered on the nesting path, or a
/// property set template, whose own members cost no budget.
pub(crate) fn member_templates(
    model: &Model,
    layout: Layout,
    container: EntityId,
    members: Option<&Value>,
    nesting: &mut Nesting<'_>,
) -> Vec<PropertyTemplate> {
    let mut templates = Vec::new();
    for member in nesting.members(container, "HasPropertyTemplates", members) {
        if !nesting.admit(model, container, member) {
            continue;
        }
        match read_template(model, layout, member, nesting) {
            Some(template) => templates.push(template),
            None => nesting.report(PropertyAnomaly::NotATemplate {
                container,
                member,
                type_name: model
                    .get(member)
                    .map(|entity| entity.type_name.to_string())
                    .unwrap_or_default(),
            }),
        }
    }
    duplicate_names(container, &templates, nesting);
    templates
}

fn nested_templates(
    model: &Model,
    layout: Layout,
    complex: EntityId,
    members: Option<&Value>,
    nesting: &mut Nesting<'_>,
) -> Vec<PropertyTemplate> {
    if !nesting.enter(complex) {
        return Vec::new();
    }
    let templates = member_templates(model, layout, complex, members, nesting);
    nesting.leave();
    templates
}

/// `UniquePropertyNames`: report each template whose name an earlier one
/// in the same container already has.
fn duplicate_names(container: EntityId, templates: &[PropertyTemplate], nesting: &mut Nesting<'_>) {
    let mut first: BTreeMap<&str, EntityId> = BTreeMap::new();
    for template in templates {
        let Some(name) = template.name.as_deref() else {
            continue;
        };
        match first.get(name) {
            Some(&kept) => nesting.report(PropertyAnomaly::DuplicatePropertyName {
                set: container,
                kept,
                rejected: template.id,
            }),
            None => {
                first.insert(name, template.id);
            }
        }
    }
}
