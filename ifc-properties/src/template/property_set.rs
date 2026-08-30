//! `IfcPropertySetTemplate` and the sets it governs.
//!
//! # Slots, verified against the IFC4 EXPRESS schema
//!
//! ```text
//! IfcPropertySetTemplate  4 = TemplateType  5 = ApplicableEntity
//!                         6 = HasPropertyTemplates
//! IfcSimplePropertyTemplate
//!                         4 = TemplateType        5 = PrimaryMeasureType
//!                         6 = SecondaryMeasureType 7 = Enumerators
//!                         8 = PrimaryUnit          9 = SecondaryUnit
//!                        10 = Expression
//! IfcRelDefinesByTemplate 4 = RelatedPropertySets 5 = RelatingTemplate
//! ```
//!
//! A template DESCRIBES what a property set should contain; it does not carry
//! values. Reading it as a property set yields nothing useful, which is why
//! it lives here rather than in `pset`.

use std::collections::BTreeMap;
use std::sync::Arc;

use ifc_model::{EntityId, Model, Value};

const ROOT_NAME: usize = 2;
const ROOT_DESCRIPTION: usize = 3;
const SET_TEMPLATE_TYPE: usize = 4;
const SET_APPLICABLE_ENTITY: usize = 5;
const SET_HAS_TEMPLATES: usize = 6;
const PROP_TEMPLATE_TYPE: usize = 4;
const PROP_PRIMARY_MEASURE: usize = 5;
const PROP_SECONDARY_MEASURE: usize = 6;
const PROP_PRIMARY_UNIT: usize = 8;
const REL_RELATED_SETS: usize = 4;
const REL_RELATING_TEMPLATE: usize = 5;

/// A property template: what one property should look like.
#[derive(Debug, Clone, PartialEq)]
pub struct PropertyTemplate {
    /// The entity.
    pub id: EntityId,
    /// `Name`: the property name this template governs.
    pub name: Option<Arc<str>>,
    /// `Description`.
    pub description: Option<Arc<str>>,
    /// `TemplateType`, e.g. `P_SINGLEVALUE`.
    ///
    /// This states which `IfcProperty` subtype an instance should use, so it
    /// is the link between a template and the property families in `pset`.
    pub template_type: Option<Arc<str>>,
    /// `PrimaryMeasureType`, e.g. `IfcLengthMeasure`.
    pub primary_measure: Option<Arc<str>>,
    /// `SecondaryMeasureType`, used by bounded and table values.
    pub secondary_measure: Option<Arc<str>>,
    /// `PrimaryUnit`.
    pub primary_unit: Option<EntityId>,
}

/// An `IfcPropertySetTemplate` with its property templates.
#[derive(Debug, Clone, PartialEq)]
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
    /// Property templates, in file order.
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
pub fn property_set_template(model: &Model, id: EntityId) -> Option<PropertySetTemplate> {
    let entity = model.get(id)?;
    if !entity
        .type_name
        .eq_ignore_ascii_case("IFCPROPERTYSETTEMPLATE")
    {
        return None;
    }
    let properties = entity
        .attributes
        .get(SET_HAS_TEMPLATES)
        .and_then(refs)
        .unwrap_or_default()
        .into_iter()
        .filter_map(|t| property_template(model, t))
        .collect();
    Some(PropertySetTemplate {
        id,
        name: entity.attributes.get(ROOT_NAME).and_then(text),
        description: entity.attributes.get(ROOT_DESCRIPTION).and_then(text),
        template_type: entity.attributes.get(SET_TEMPLATE_TYPE).and_then(enum_text),
        applicable_entity: entity.attributes.get(SET_APPLICABLE_ENTITY).and_then(text),
        properties,
    })
}

/// Read one property template by id.
pub fn property_template(model: &Model, id: EntityId) -> Option<PropertyTemplate> {
    let entity = model.get(id)?;
    Some(PropertyTemplate {
        id,
        name: entity.attributes.get(ROOT_NAME).and_then(text),
        description: entity.attributes.get(ROOT_DESCRIPTION).and_then(text),
        template_type: entity
            .attributes
            .get(PROP_TEMPLATE_TYPE)
            .and_then(enum_text),
        primary_measure: entity.attributes.get(PROP_PRIMARY_MEASURE).and_then(text),
        secondary_measure: entity.attributes.get(PROP_SECONDARY_MEASURE).and_then(text),
        primary_unit: entity.attributes.get(PROP_PRIMARY_UNIT).and_then(one_ref),
    })
}

/// Every template in the file, ascending by id.
pub fn property_set_templates(model: &Model) -> Vec<PropertySetTemplate> {
    let mut ids: Vec<_> = model.ids_of_type("IFCPROPERTYSETTEMPLATE").to_vec();
    ids.sort_unstable();
    ids.into_iter()
        .filter_map(|id| property_set_template(model, id))
        .collect()
}

/// Which template governs each property set, via `IfcRelDefinesByTemplate`.
///
/// A set with no entry is untemplated, which is normal: templates describe
/// custom property sets and standard Psets rely on the published catalogue
/// instead.
pub fn template_of_set(model: &Model) -> BTreeMap<EntityId, EntityId> {
    let mut out = BTreeMap::new();
    for &id in model.ids_of_type("IFCRELDEFINESBYTEMPLATE") {
        let Some(rel) = model.get(id) else { continue };
        let Some(template) = rel.attributes.get(REL_RELATING_TEMPLATE).and_then(one_ref) else {
            continue;
        };
        for set in rel
            .attributes
            .get(REL_RELATED_SETS)
            .and_then(refs)
            .unwrap_or_default()
        {
            out.insert(set, template);
        }
    }
    out
}

fn text(value: &Value) -> Option<Arc<str>> {
    match value.unwrap_typed() {
        Value::Text(t) => Some(t.clone()),
        _ => None,
    }
}

fn enum_text(value: &Value) -> Option<Arc<str>> {
    match value.unwrap_typed() {
        Value::Enum(t) | Value::Text(t) => Some(t.clone()),
        _ => None,
    }
}

fn one_ref(value: &Value) -> Option<EntityId> {
    match value.unwrap_typed() {
        Value::Ref(id) => Some(*id),
        _ => None,
    }
}

fn refs(value: &Value) -> Option<Vec<EntityId>> {
    match value {
        Value::List(items) => Some(items.iter().filter_map(one_ref).collect()),
        _ => None,
    }
}
