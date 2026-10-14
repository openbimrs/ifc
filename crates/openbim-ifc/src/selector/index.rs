//! [`SelectorIndex`]: the model, its release, and the indexes a selector
//! reads, each built on first use and kept for every later query.
//!
//! Relationships are read forward from the objectified relationship that
//! states them, through the model's type index, with slots looked up by
//! attribute name in the release's table, as
//! [`type_of`](crate::type_of) reads them: `IfcRelDefinesByType`,
//! `IfcRelContainedInSpatialStructure`, `IfcRelAggregates`,
//! `IfcRelAssociatesMaterial`, `IfcRelAssociatesClassification` and
//! `IfcRelAssignsToGroup`, subtypes included. Each kind is scanned once,
//! when a facet or a path first needs it, so a query costs its
//! relationships plus its working set, never a pass per element.

use std::cell::{OnceCell, RefCell};
use std::collections::{HashMap, HashSet};

use ifc_model::{Entity, EntityId, Model, Value};
use ifc_schema::Schema;

use super::error::SelectorError;
use crate::GlobalIdIndex;

/// One relationship kind read both ways: each `from` entity's `to`
/// entities and each `to` entity's `from` entities, in file order, each
/// once.
#[derive(Debug, Default)]
pub(super) struct Links {
    forward: HashMap<EntityId, Vec<EntityId>>,
    backward: HashMap<EntityId, Vec<EntityId>>,
}

impl Links {
    pub(super) fn forward(&self, from: EntityId) -> &[EntityId] {
        self.forward.get(&from).map_or(&[], Vec::as_slice)
    }

    pub(super) fn backward(&self, to: EntityId) -> &[EntityId] {
        self.backward.get(&to).map_or(&[], Vec::as_slice)
    }
}

/// The relationship kinds a selector reads.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Rel {
    /// Object to its type.
    Type,
    /// Element to its spatial structure.
    Container,
    /// Part to its whole.
    Aggregate,
    /// Object to its material definition.
    Material,
    /// Object to its classification reference.
    Classification,
    /// Object to its group.
    Group,
}

impl Rel {
    /// `(relationship type, the end read from, whether it is a list, the
    /// end read to, whether it is a list)`.
    const fn layout(self) -> (&'static str, &'static str, bool, &'static str, bool) {
        match self {
            Self::Type => (
                "IfcRelDefinesByType",
                "RelatedObjects",
                true,
                "RelatingType",
                false,
            ),
            Self::Container => (
                "IfcRelContainedInSpatialStructure",
                "RelatedElements",
                true,
                "RelatingStructure",
                false,
            ),
            Self::Aggregate => (
                "IfcRelAggregates",
                "RelatedObjects",
                true,
                "RelatingObject",
                false,
            ),
            Self::Material => (
                "IfcRelAssociatesMaterial",
                "RelatedObjects",
                true,
                "RelatingMaterial",
                false,
            ),
            Self::Classification => (
                "IfcRelAssociatesClassification",
                "RelatedObjects",
                true,
                "RelatingClassification",
                false,
            ),
            Self::Group => (
                "IfcRelAssignsToGroup",
                "RelatedObjects",
                true,
                "RelatingGroup",
                false,
            ),
        }
    }

    const fn slot(self) -> usize {
        self as usize
    }

    /// What [`SelectorError::Ambiguous`] calls a second answer.
    pub(super) const fn noun(self) -> &'static str {
        match self {
            Self::Type => "type",
            Self::Container => "container",
            Self::Aggregate => "aggregate",
            Self::Material => "material",
            Self::Classification => "classification",
            Self::Group => "group",
        }
    }
}

const RELATIONS: usize = 6;

/// A model, the release it declares, and the indexes selectors read.
///
/// Borrowed (ADR 0003): the model cannot change while the index exists, so
/// nothing it caches goes stale. Build one per model and run every query,
/// path and staged write against it; drop it before committing.
///
/// Not `Sync`: the caches fill on first use behind shared references.
pub struct SelectorIndex<'m> {
    pub(super) model: &'m Model,
    pub(super) schema: &'m Schema,
    relations: [OnceCell<Result<Links, SelectorError>>; RELATIONS],
    guids: OnceCell<GlobalIdIndex>,
    #[cfg(feature = "properties")]
    properties: OnceCell<ifc_properties::PropertyIndex<'m>>,
    /// `(upper-case type, attribute)` to its slot, `None` when the type
    /// declares no such attribute.
    slots: RefCell<HashMap<(String, String), Option<usize>>>,
}

impl std::fmt::Debug for SelectorIndex<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SelectorIndex")
            .field("schema", &self.schema.name())
            .field("entities", &self.model.len())
            .finish_non_exhaustive()
    }
}

impl<'m> SelectorIndex<'m> {
    /// An index over `model`, read against `schema`, the release its header
    /// declares (as for [`ids_of_type_including_subtypes`](crate::ids_of_type_including_subtypes)).
    /// Nothing is scanned until a query needs it.
    #[must_use]
    pub fn new(model: &'m Model, schema: &'m Schema) -> Self {
        Self {
            model,
            schema,
            relations: Default::default(),
            guids: OnceCell::new(),
            #[cfg(feature = "properties")]
            properties: OnceCell::new(),
            slots: RefCell::new(HashMap::new()),
        }
    }

    /// The model.
    #[must_use]
    pub fn model(&self) -> &'m Model {
        self.model
    }

    /// The release the model is read against.
    #[must_use]
    pub fn schema(&self) -> &'m Schema {
        self.schema
    }

    pub(super) fn entity(&self, id: EntityId) -> Result<&'m Entity, SelectorError> {
        self.model.get(id).ok_or(SelectorError::MissingEntity(id))
    }

    pub(super) fn guids(&self) -> &GlobalIdIndex {
        self.guids
            .get_or_init(|| GlobalIdIndex::build(self.model, self.schema))
    }

    #[cfg(feature = "properties")]
    pub(super) fn properties(&self) -> &ifc_properties::PropertyIndex<'m> {
        self.properties
            .get_or_init(|| ifc_properties::PropertyIndex::build(self.model))
    }

    /// Whether `id` is `ancestor` or a subtype of it.
    pub(super) fn is_a(&self, id: EntityId, ancestor: &str) -> bool {
        self.model
            .get(id)
            .is_some_and(|entity| self.schema.is_a(&entity.type_name, ancestor))
    }

    /// Every entity of `class` or a subtype, through the type index.
    pub(super) fn of_class(&self, class: &str) -> impl Iterator<Item = EntityId> + 'm {
        let model = self.model;
        let names: Vec<&'m str> = self.schema.subtypes(class);
        let class = class.to_owned();
        model.ids_of_type(&class).iter().copied().chain(
            names
                .into_iter()
                .flat_map(move |name| model.ids_of_type(name).iter().copied()),
        )
    }

    /// The slot of attribute `name` (any case) in `type_name`'s record.
    pub(super) fn slot(&self, type_name: &str, name: &str) -> Option<usize> {
        let key = (type_name.to_ascii_uppercase(), name.to_owned());
        if let Some(found) = self.slots.borrow().get(&key) {
            return *found;
        }
        let found = self
            .schema
            .attribute_names(type_name)
            .iter()
            .position(|declared| declared.eq_ignore_ascii_case(name));
        self.slots.borrow_mut().insert(key, found);
        found
    }

    /// The value of attribute `name` of `entity`; `None` when its type
    /// declares no such attribute, `$` past the record's end.
    pub(super) fn attribute(&self, id: EntityId, name: &str) -> Option<&'m Value> {
        static NULL: Value = Value::Null;
        let entity = self.model.get(id)?;
        let slot = self.slot(&entity.type_name, name)?;
        Some(entity.attribute(slot).unwrap_or(&NULL))
    }

    /// An attribute's text, when it holds one.
    pub(super) fn text(&self, id: EntityId, name: &str) -> Option<&'m str> {
        self.attribute(id, name)
            .and_then(|value| value.unwrap_typed().as_text())
    }

    /// The links of one relationship kind, scanned on first use.
    pub(super) fn links(&self, rel: Rel) -> Result<&Links, SelectorError> {
        self.relations[rel.slot()]
            .get_or_init(|| self.scan(rel))
            .as_ref()
            .map_err(Clone::clone)
    }

    /// The one `to` entity `from` has, refused when it has several.
    pub(super) fn single(
        &self,
        rel: Rel,
        from: EntityId,
    ) -> Result<Option<EntityId>, SelectorError> {
        match self.links(rel)?.forward(from) {
            [] => Ok(None),
            [one] => Ok(Some(*one)),
            several => Err(SelectorError::Ambiguous {
                entity: from,
                relation: rel.noun(),
                candidates: several.to_vec(),
            }),
        }
    }

    fn scan(&self, rel: Rel) -> Result<Links, SelectorError> {
        let (type_name, from_end, from_many, to_end, to_many) = rel.layout();
        let mut links = Links::default();
        let mut seen = HashSet::new();
        for relationship in self.of_class(type_name) {
            let Some(record) = self.model.get(relationship) else {
                continue;
            };
            let from = self.end(relationship, record, from_end, from_many)?;
            let to = self.end(relationship, record, to_end, to_many)?;
            for &target in from.iter().chain(&to) {
                if !self.model.contains(target) {
                    return Err(SelectorError::MissingReference {
                        relationship,
                        target,
                    });
                }
            }
            for &a in &from {
                for &b in &to {
                    if seen.insert((a, b)) {
                        links.forward.entry(a).or_default().push(b);
                        links.backward.entry(b).or_default().push(a);
                    }
                }
            }
        }
        Ok(links)
    }

    /// The ids one end of a relationship names.
    fn end(
        &self,
        relationship: EntityId,
        record: &Entity,
        name: &'static str,
        many: bool,
    ) -> Result<Vec<EntityId>, SelectorError> {
        let malformed = SelectorError::Malformed {
            entity: relationship,
            attribute: name,
        };
        let slot = self
            .slot(&record.type_name, name)
            .ok_or_else(|| malformed.clone())?;
        match (record.attribute(slot), many) {
            (Some(Value::Ref(id)), false) => Ok(vec![*id]),
            (Some(Value::List(items)), true) => items
                .iter()
                .map(|item| item.as_ref_id().ok_or_else(|| malformed.clone()))
                .collect(),
            _ => Err(malformed),
        }
    }

    /// The type object of `id`: itself for a type object, else its
    /// `IfcRelDefinesByType` type.
    pub(super) fn type_object(&self, id: EntityId) -> Result<Option<EntityId>, SelectorError> {
        if self.is_a(id, "IfcTypeObject") {
            return Ok(Some(id));
        }
        self.single(Rel::Type, id)
    }

    /// The structure `id` sits in: its spatial container, else the whole
    /// it is a part of.
    pub(super) fn parent(&self, id: EntityId) -> Result<Option<EntityId>, SelectorError> {
        match self.single(Rel::Container, id)? {
            Some(container) => Ok(Some(container)),
            None => self.single(Rel::Aggregate, id),
        }
    }

    /// Every structure above `id`, nearest first, up to the project
    /// (excluded); stops at a repeated entity.
    pub(super) fn ancestors(&self, id: EntityId) -> Result<Vec<EntityId>, SelectorError> {
        let mut out = Vec::new();
        let mut current = self.parent(id)?;
        while let Some(parent) = current {
            if out.contains(&parent) || parent == id || self.is_a(parent, "IfcProject") {
                break;
            }
            out.push(parent);
            current = self.parent(parent)?;
        }
        Ok(out)
    }

    /// The material definitions associated with `id`, or, when none is,
    /// with its type object.
    pub(super) fn material_definitions(
        &self,
        id: EntityId,
    ) -> Result<Vec<EntityId>, SelectorError> {
        let links = self.links(Rel::Material)?;
        let own = links.forward(id);
        if !own.is_empty() {
            return Ok(own.to_vec());
        }
        match self.type_object(id)? {
            Some(type_object) if type_object != id => Ok(links.forward(type_object).to_vec()),
            _ => Ok(Vec::new()),
        }
    }

    /// The `IfcMaterial`s a material definition names, through usages,
    /// sets, layers, profiles, constituents and lists, each once.
    pub(super) fn materials_of(
        &self,
        definition: EntityId,
    ) -> Result<Vec<EntityId>, SelectorError> {
        let mut out = Vec::new();
        self.collect_materials(definition, &mut out, 0)?;
        Ok(out)
    }

    fn collect_materials(
        &self,
        id: EntityId,
        out: &mut Vec<EntityId>,
        depth: usize,
    ) -> Result<(), SelectorError> {
        // usage -> set -> layer -> material is the deepest chain IFC has.
        if depth > 8 {
            return Ok(());
        }
        let entity = self.entity(id)?;
        let is = |class: &str| self.schema.is_a(&entity.type_name, class);
        let follow: &[&'static str] = if is("IfcMaterial") {
            push_once(out, id);
            return Ok(());
        } else if is("IfcMaterialLayerSetUsage") {
            &["ForLayerSet"]
        } else if is("IfcMaterialProfileSetUsageTapering") {
            &["ForProfileSet", "ForProfileEndSet"]
        } else if is("IfcMaterialProfileSetUsage") {
            &["ForProfileSet"]
        } else if is("IfcMaterialLayerSet") {
            &["MaterialLayers"]
        } else if is("IfcMaterialProfileSet") {
            &["MaterialProfiles"]
        } else if is("IfcMaterialConstituentSet") {
            &["MaterialConstituents"]
        } else if is("IfcMaterialList") {
            &["Materials"]
        } else if is("IfcMaterialLayer") || is("IfcMaterialProfile") || is("IfcMaterialConstituent")
        {
            &["Material"]
        } else {
            return Err(SelectorError::Malformed {
                entity: id,
                attribute: "RelatingMaterial",
            });
        };
        for &name in follow {
            let mut targets = Vec::new();
            match self.attribute(id, name) {
                Some(Value::Ref(target)) => targets.push(*target),
                Some(Value::List(items)) => {
                    for item in items {
                        let target = item.as_ref_id().ok_or(SelectorError::Malformed {
                            entity: id,
                            attribute: name,
                        })?;
                        targets.push(target);
                    }
                }
                Some(Value::Null) | None => {}
                Some(_) => {
                    return Err(SelectorError::Malformed {
                        entity: id,
                        attribute: name,
                    })
                }
            }
            for target in targets {
                if !self.model.contains(target) {
                    return Err(SelectorError::MissingReference {
                        relationship: id,
                        target,
                    });
                }
                self.collect_materials(target, out, depth + 1)?;
            }
        }
        Ok(())
    }

    /// The classifications associated with `id` and with its type object.
    pub(super) fn classifications(&self, id: EntityId) -> Result<Vec<EntityId>, SelectorError> {
        let links = self.links(Rel::Classification)?;
        let mut out = links.forward(id).to_vec();
        if let Some(type_object) = self.type_object(id)? {
            if type_object != id {
                for &reference in links.forward(type_object) {
                    push_once(&mut out, reference);
                }
            }
        }
        Ok(out)
    }
}

fn push_once(list: &mut Vec<EntityId>, id: EntityId) {
    if !list.contains(&id) {
        list.push(id);
    }
}
