//! `IfcPropertySetTemplate` and the sets it governs.
//!
//! ```text
//! IfcPropertySetTemplate   TemplateType ApplicableEntity HasPropertyTemplates
//! IfcRelDefinesByTemplate  RelatedPropertySets RelatingTemplate
//! ```
//!
//! Every attribute is read by name from the bound release's table
//! ([`Layout`]); both releases that define templates (IFC4 ADD2 TC1, IFC4X3
//! ADD2) place them after the four `IfcRoot` attributes.
//!
//! A template DESCRIBES what a property set should contain; it does not carry
//! values. Reading it as a property set yields nothing useful, which is why
//! it lives here rather than in `pset`.

use std::collections::BTreeMap;
use std::sync::Arc;

use ifc_model::{EntityId, Model, Value};

use super::layout::Layout;
use super::property::{member_templates, PropertyTemplate};
use crate::error::{PropertyAnomaly, TemplateError};
use crate::nesting::Nesting;

/// An `IfcPropertySetTemplate` with its property templates.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct PropertySetTemplate {
    /// The entity.
    pub id: EntityId,
    /// `Name`, required by `ExistsName`.
    pub name: Option<Arc<str>>,
    /// `Description`.
    pub description: Option<Arc<str>>,
    /// `TemplateType`, e.g. `PSET_TYPEDRIVENOVERRIDE`.
    pub template_type: Option<Arc<str>>,
    /// `ApplicableEntity`: which IFC entity the set applies to.
    ///
    /// A free-text identifier such as `IfcWall`, not a validated reference.
    pub applicable_entity: Option<Arc<str>>,
    /// Property templates, in file order, simple and complex, complex ones
    /// with their nested templates.
    pub properties: Vec<PropertyTemplate>,
}

impl PropertySetTemplate {
    /// Look up a property template by name.
    pub fn property(&self, name: &str) -> Option<&PropertyTemplate> {
        self.properties
            .iter()
            .find(|p| p.name.as_deref() == Some(name))
    }
}

/// Read one `IfcPropertySetTemplate` by id.
///
/// `None` when the entity is absent, is not a set template, or the release
/// the header declares defines none (IFC2X3). Members the traversal refuses
/// are left out without saying so; use [`property_set_template_checked`] to
/// have each one reported.
pub fn property_set_template(model: &Model, id: EntityId) -> Option<PropertySetTemplate> {
    let mut anomalies = Vec::new();
    read_set_template(model, Layout::permissive(model), id, &mut anomalies)
}

/// Read one `IfcPropertySetTemplate` bound to the release the model
/// declares, reporting every malformed fact met on the way.
///
/// The anomalies are those described on
/// [`property_template_checked`](crate::property_template_checked), for the
/// set template and every template below it. Only nested members cost the
/// traversal budget, as for property sets.
///
/// # Errors
///
/// [`TemplateError::Release`] or [`TemplateError::NoTemplates`] when the
/// model binds to no release that has templates;
/// [`TemplateError::MissingEntity`] or [`TemplateError::NotATemplate`] for
/// the entity itself.
pub fn property_set_template_checked(
    model: &Model,
    id: EntityId,
) -> Result<(PropertySetTemplate, Vec<PropertyAnomaly>), TemplateError> {
    let layout = Layout::declared(model)?;
    let entity = model.get(id).ok_or(TemplateError::MissingEntity { id })?;
    let mut anomalies = Vec::new();
    let template = read_set_template(model, layout, id, &mut anomalies).ok_or(
        TemplateError::NotATemplate {
            id,
            type_name: entity.type_name.to_string(),
        },
    )?;
    Ok((template, anomalies))
}

pub(crate) fn read_set_template(
    model: &Model,
    layout: Layout,
    id: EntityId,
    anomalies: &mut Vec<PropertyAnomaly>,
) -> Option<PropertySetTemplate> {
    let entity = model.get(id)?;
    if !layout.is_a(&entity.type_name, "IFCPROPERTYSETTEMPLATE") {
        return None;
    }
    layout.check_arity(id, entity, anomalies);
    let mut template = PropertySetTemplate {
        id,
        name: layout.text(id, entity, "Name", anomalies),
        description: layout.text(id, entity, "Description", anomalies),
        template_type: layout.enumeration(id, entity, "TemplateType", anomalies),
        applicable_entity: layout.text(id, entity, "ApplicableEntity", anomalies),
        properties: Vec::new(),
    };
    let mut nesting = Nesting::new(anomalies);
    let members = layout.get(entity, "HasPropertyTemplates");
    template.properties = member_templates(model, layout, id, members, &mut nesting);
    Some(template)
}

/// Every template in the file, ascending by id.
pub fn property_set_templates(model: &Model) -> Vec<PropertySetTemplate> {
    let mut ids: Vec<_> = model.ids_of_type("IFCPROPERTYSETTEMPLATE").to_vec();
    ids.sort_unstable();
    ids.into_iter()
        .filter_map(|id| property_set_template(model, id))
        .collect()
}

/// Which templates define each property set, via `IfcRelDefinesByTemplate`.
///
/// A set with no entry is untemplated, which is normal: templates describe
/// custom property sets and standard Psets rely on the published catalogue
/// instead.
///
/// Each set maps to EVERY template defining it, ascending by id and without
/// repeats. `IfcPropertySetDefinition.IsDefinedBy` is `SET [0:?] OF
/// IfcRelDefinesByTemplate`, so several templates are legal. Before 0.4 this
/// returned one template per set, and silently dropped the rest (#60).
pub fn template_of_set(model: &Model) -> BTreeMap<EntityId, Vec<EntityId>> {
    template_links(model, Layout::permissive(model))
        .into_iter()
        .map(|(set, links)| {
            let mut templates: Vec<EntityId> = links.into_iter().map(|(_, t)| t).collect();
            templates.sort_unstable();
            templates.dedup();
            (set, templates)
        })
        .collect()
}

/// Every `(relationship, template)` pair naming each set, in relationship
/// id order.
pub(crate) fn template_links(
    model: &Model,
    layout: Layout,
) -> BTreeMap<EntityId, Vec<(EntityId, EntityId)>> {
    let mut out: BTreeMap<EntityId, Vec<(EntityId, EntityId)>> = BTreeMap::new();
    let mut relationships = model.ids_of_type("IFCRELDEFINESBYTEMPLATE").to_vec();
    relationships.sort_unstable();
    for id in relationships {
        let Some(rel) = model.get(id) else { continue };
        let Some(template) = layout.get(rel, "RelatingTemplate").and_then(one_ref) else {
            continue;
        };
        let sets = match layout.get(rel, "RelatedPropertySets") {
            Some(Value::List(items)) => items.iter().filter_map(one_ref).collect(),
            _ => Vec::new(),
        };
        for set in sets {
            out.entry(set).or_default().push((id, template));
        }
    }
    out
}

fn one_ref(value: &Value) -> Option<EntityId> {
    match value.unwrap_typed() {
        Value::Ref(id) => Some(*id),
        _ => None,
    }
}
