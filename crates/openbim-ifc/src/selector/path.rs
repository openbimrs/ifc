//! Reading a value by path: [`SelectorIndex::value`].
//!
//! Each key applies to the value the keys before it produced. On an
//! entity, a key is first a relation keyword (lower case, exactly as
//! listed), then an explicit attribute of the entity's type (any case),
//! then a property or quantity set name. On a property set it names a
//! property; on a list it applies to every item, or picks one by index.
//! `NULL` short-circuits: the rest of the path reads `NULL` too.
//!
//! | Key | On an entity |
//! | --- | --- |
//! | `type` | its type object (a type object itself) |
//! | `material`, `mat` | its material definition, a usage read as its set |
//! | `materials`, `mats` | every `IfcMaterial` it is made of |
//! | `container` | its spatial container |
//! | `parent` | its container, else the whole it is a part of |
//! | `space`, `storey`, `building`, `site` | the nearest such structure above it |
//! | `types`, `occurrences` | the objects a type object types |
//! | `classification` | its classification references, its type's included |
//! | `group` | the groups it is assigned to |
//! | `class` | its class, as the release spells it |
//! | `id` | its entity id |
//! | `predefined_type` | the predefined type facet `PredefinedType` compares |
//! | `count` | on any value: a list's length, else 1 |

use ifc_model::EntityId;

use super::ast::{Key, ValuePath};
use super::error::SelectorError;
use super::index::{Rel, SelectorIndex};
use super::value::SelectedValue;

/// Keys a path reads as relations or computed values, not as attribute or
/// property names.
pub(super) const KEYWORDS: &[&str] = &[
    "type",
    "material",
    "mat",
    "materials",
    "mats",
    "container",
    "parent",
    "space",
    "storey",
    "building",
    "site",
    "types",
    "occurrences",
    "classification",
    "group",
    "class",
    "id",
    "predefined_type",
    "count",
];

impl SelectorIndex<'_> {
    /// The value `path` reads from `entity`.
    ///
    /// # Errors
    ///
    /// [`SelectorError::MissingEntity`] for an id the model lacks; a
    /// relationship or property refusal met on the way.
    pub fn value(
        &self,
        entity: EntityId,
        path: &ValuePath,
    ) -> Result<SelectedValue, SelectorError> {
        self.entity(entity)?;
        let mut current = SelectedValue::Entity(entity);
        for key in &path.keys {
            if current == SelectedValue::Null {
                break;
            }
            current = self.step(&current, &key.key)?;
        }
        Ok(current)
    }

    pub(super) fn step(
        &self,
        value: &SelectedValue,
        key: &Key,
    ) -> Result<SelectedValue, SelectorError> {
        if matches!(key, Key::Text(text) if text == "count") {
            return Ok(SelectedValue::Integer(match value {
                SelectedValue::List(items) => i64::try_from(items.len()).unwrap_or(i64::MAX),
                _ => 1,
            }));
        }
        match value {
            SelectedValue::Entity(id) => self.entity_step(*id, key),
            SelectedValue::Properties(entries) => Ok(property_step(entries, key)),
            SelectedValue::List(items) => {
                if let Key::Text(text) = key {
                    if let Ok(index) = text.parse::<usize>() {
                        return Ok(items.get(index).cloned().unwrap_or(SelectedValue::Null));
                    }
                }
                let mut out = Vec::new();
                for item in items {
                    match self.step(item, key)? {
                        SelectedValue::List(nested) => out.extend(nested),
                        other => out.push(other),
                    }
                }
                Ok(SelectedValue::List(out))
            }
            _ => Ok(SelectedValue::Null),
        }
    }

    fn entity_step(&self, id: EntityId, key: &Key) -> Result<SelectedValue, SelectorError> {
        let entity =
            |found: Option<EntityId>| found.map_or(SelectedValue::Null, SelectedValue::Entity);
        let entities = |ids: Vec<EntityId>| {
            SelectedValue::List(ids.into_iter().map(SelectedValue::Entity).collect())
        };
        if let Key::Text(text) = key {
            match text.as_str() {
                "type" => return Ok(entity(self.type_object(id)?)),
                "material" | "mat" => {
                    let definitions = self
                        .material_definitions(id)?
                        .into_iter()
                        .map(|definition| self.skip_usage(definition))
                        .collect::<Vec<_>>();
                    return Ok(match definitions.as_slice() {
                        [] => SelectedValue::Null,
                        [one] => SelectedValue::Entity(*one),
                        _ => entities(definitions),
                    });
                }
                "materials" | "mats" => {
                    let mut out = Vec::new();
                    for definition in self.material_definitions(id)? {
                        for material in self.materials_of(definition)? {
                            if !out.contains(&material) {
                                out.push(material);
                            }
                        }
                    }
                    return Ok(entities(out));
                }
                "container" => return Ok(entity(self.single(Rel::Container, id)?)),
                "parent" => return Ok(entity(self.parent(id)?)),
                "space" | "storey" | "building" | "site" => {
                    let class = match text.as_str() {
                        "space" => "IfcSpace",
                        "storey" => "IfcBuildingStorey",
                        "building" => "IfcBuilding",
                        _ => "IfcSite",
                    };
                    let found = self
                        .ancestors(id)?
                        .into_iter()
                        .find(|ancestor| self.is_a(*ancestor, class));
                    return Ok(entity(found));
                }
                "types" | "occurrences" => {
                    return Ok(entities(self.links(Rel::Type)?.backward(id).to_vec()))
                }
                "classification" => return Ok(entities(self.classifications(id)?)),
                "group" => return Ok(entities(self.links(Rel::Group)?.forward(id).to_vec())),
                "class" => {
                    let type_name = &self.entity(id)?.type_name;
                    let spelled = self
                        .schema
                        .entity(type_name)
                        .map_or(type_name.to_string(), |definition| definition.name.clone());
                    return Ok(SelectedValue::Text(spelled));
                }
                "id" => {
                    return Ok(SelectedValue::Integer(
                        i64::try_from(id.0).unwrap_or(i64::MAX),
                    ))
                }
                "predefined_type" => return self.predefined_type(id),
                _ => {}
            }
            // IFC names a layer set's name `LayerSetName`; `Name` reads it,
            // as IfcOpenShell does.
            let name = if text == "Name" && self.is_a(id, "IfcMaterialLayerSet") {
                "LayerSetName"
            } else {
                text.as_str()
            };
            if let Some(value) = self.attribute(id, name) {
                return Ok(SelectedValue::from_model(value));
            }
        }
        self.property_sets(id, key)
    }

    /// A material usage read as the set it uses.
    fn skip_usage(&self, definition: EntityId) -> EntityId {
        let set = if self.is_a(definition, "IfcMaterialLayerSetUsage") {
            "ForLayerSet"
        } else if self.is_a(definition, "IfcMaterialProfileSetUsage") {
            "ForProfileSet"
        } else {
            return definition;
        };
        self.attribute(definition, set)
            .and_then(ifc_model::Value::as_ref_id)
            .unwrap_or(definition)
    }

    /// The predefined type an entity states, as IfcOpenShell reads it: its
    /// type object's when that states one other than `NOTDEFINED`, else
    /// its own; `USERDEFINED` reads as the user-defined name
    /// (`ObjectType`, `ElementType`, `ProcessType` or `ResourceType`) when
    /// one is stated.
    pub(super) fn predefined_type(&self, id: EntityId) -> Result<SelectedValue, SelectorError> {
        if let Some(type_object) = self.type_object(id)? {
            if type_object != id {
                let stated = self.attribute(type_object, "PredefinedType");
                let defined = stated
                    .and_then(|value| match value.unwrap_typed() {
                        ifc_model::Value::Enum(item) => Some(item),
                        _ => None,
                    })
                    .is_some_and(|item| !item.eq_ignore_ascii_case("NOTDEFINED"));
                if defined {
                    return Ok(self.user_defined(type_object));
                }
            }
        }
        Ok(self.user_defined(id))
    }

    fn user_defined(&self, id: EntityId) -> SelectedValue {
        let Some(value) = self.attribute(id, "PredefinedType") else {
            return SelectedValue::Null;
        };
        let value = SelectedValue::from_model(value);
        if !matches!(&value, SelectedValue::Enum(item) if item.eq_ignore_ascii_case("USERDEFINED"))
        {
            return value;
        }
        ["ObjectType", "ElementType", "ProcessType", "ResourceType"]
            .iter()
            .find_map(|name| self.text(id, name))
            .map_or(value, |name| SelectedValue::Text(name.to_owned()))
    }

    /// The property or quantity sets of `id` that `key` names: one set's
    /// properties, a list of sets for a pattern naming several, or `NULL`.
    #[cfg(feature = "properties")]
    fn property_sets(&self, id: EntityId, key: &Key) -> Result<SelectedValue, SelectorError> {
        let entries =
            match self
                .properties()
                .exact_properties_where(id, |set| key.matches(set), |_| true)
            {
                Ok(entries) => entries,
                Err(ifc_properties::ExactPropertyError::InvalidQueryObject { .. }) => Vec::new(),
                Err(error) => {
                    return Err(SelectorError::Property {
                        entity: Some(id),
                        error,
                    })
                }
            };
        let mut sets: Vec<(String, Vec<(String, SelectedValue)>)> = Vec::new();
        for entry in entries {
            let value = super::exact::selected(id, &entry.property.value)?;
            let set = entry.property.property_set.to_string();
            match sets.iter_mut().find(|(name, _)| *name == set) {
                Some((_, properties)) => properties.push((entry.name.to_string(), value)),
                None => sets.push((set, vec![(entry.name.to_string(), value)])),
            }
        }
        let mut sets: Vec<SelectedValue> = sets
            .into_iter()
            .map(|(_, properties)| SelectedValue::Properties(properties))
            .collect();
        Ok(match sets.len() {
            0 => SelectedValue::Null,
            1 => sets.remove(0),
            _ => SelectedValue::List(sets),
        })
    }

    #[cfg(not(feature = "properties"))]
    #[allow(clippy::unused_self)]
    fn property_sets(&self, _id: EntityId, _key: &Key) -> Result<SelectedValue, SelectorError> {
        Err(SelectorError::FeatureDisabled("properties"))
    }
}

/// A key applied to a property set: the property it names, or for a
/// pattern every property it matches.
fn property_step(entries: &[(String, SelectedValue)], key: &Key) -> SelectedValue {
    match key {
        Key::Text(name) => entries
            .iter()
            .find(|(property, _)| property == name)
            .map_or(SelectedValue::Null, |(_, value)| value.clone()),
        Key::Regex(pattern) => {
            let mut out = Vec::new();
            for (_, value) in entries.iter().filter(|(name, _)| pattern.is_match(name)) {
                match value {
                    SelectedValue::List(items) => out.extend(items.iter().cloned()),
                    other => out.push(other.clone()),
                }
            }
            match out.len() {
                0 => SelectedValue::Null,
                1 => out.remove(0),
                _ => SelectedValue::List(out),
            }
        }
    }
}
