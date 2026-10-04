//! Running a batch's operations against the overlay.

use std::collections::HashMap;
use std::sync::Arc;

use ifc_model::{Entity, EntityId, Value};
use ifc_schema::Schema;

use super::check::{build, recheck, relations};
use super::overlay::Overlay;
use super::{AuthorOp, AuthoringFailure as F, NamedValues};
use crate::name_guid::name_based_guid;

/// The batch's state: the overlay, and what each operation produced.
pub(super) struct Planner<'m, 's> {
    pub(super) schema: &'s Schema,
    pub(super) view: Overlay<'m>,
    seed: u128,
    /// Per operation run so far, the entity it produced.
    pub(super) ids: Vec<Option<EntityId>>,
    /// `GlobalId`s of the base model, read on first need.
    base_global_ids: Option<HashMap<Arc<str>, EntityId>>,
    /// `GlobalId`s the batch wrote.
    batch_global_ids: HashMap<Arc<str>, EntityId>,
}

/// `IFCROOT` and its `GlobalId`, slot 0 in every release.
const GLOBAL_ID: &str = "GlobalId";

impl<'m, 's> Planner<'m, 's> {
    pub(super) fn new(schema: &'s Schema, view: Overlay<'m>, seed: u128) -> Self {
        Self {
            schema,
            view,
            seed,
            ids: Vec::new(),
            base_global_ids: None,
            batch_global_ids: HashMap::new(),
        }
    }

    /// Run one operation, returning the entity it produced.
    pub(super) fn run(&mut self, op: AuthorOp) -> Result<Option<EntityId>, F> {
        Ok(match op {
            AuthorOp::Create {
                type_name,
                attributes,
            } => Some(self.create(&type_name, attributes)?),
            AuthorOp::Edit { entity, attributes } => Some(self.edit(entity, attributes)?),
            AuthorOp::Remove { entity } => {
                self.remove(entity)?;
                None
            }
            AuthorOp::Project {
                attributes,
                owner_history,
            } => Some(self.project(attributes, owner_history)?),
            AuthorOp::Spatial {
                type_name,
                parent,
                attributes,
                placement,
                owner_history,
            } => Some(self.spatial(&type_name, parent, attributes, placement, owner_history)?),
            AuthorOp::Product {
                type_name,
                container,
                attributes,
                placement,
                type_object,
                owner_history,
            } => {
                let product = self.product(&type_name, attributes, placement, owner_history)?;
                if let Some(container) = container {
                    self.contain(container, &[product], owner_history)?;
                }
                if let Some(type_object) = type_object {
                    self.assign_type(type_object, &[product], owner_history)?;
                }
                Some(product)
            }
            AuthorOp::TypeObject {
                type_name,
                attributes,
                owner_history,
            } => {
                self.require_kind(&type_name, "IfcTypeObject")?;
                Some(self.create(&type_name, with_owner(attributes, owner_history))?)
            }
            AuthorOp::AssignType {
                type_object,
                objects,
                owner_history,
            } => Some(self.assign_type(type_object, &objects, owner_history)?),
            AuthorOp::Contain {
                structure,
                elements,
                owner_history,
            } => Some(self.contain(structure, &elements, owner_history)?),
            AuthorOp::Aggregate {
                parent,
                parts,
                owner_history,
            } => Some(self.aggregate(parent, &parts, owner_history)?),
            AuthorOp::Placement {
                relative_to,
                location,
                axes,
            } => Some(self.placement(relative_to, location, axes)?),
            AuthorOp::OwnerHistory(fields) => Some(self.owner_history(&fields)?),
        })
    }

    /// Create one entity from named values: every check, a `GlobalId` for
    /// an `IfcRoot` that has none, then the overlay.
    pub(super) fn create(
        &mut self,
        type_name: &str,
        mut attributes: NamedValues,
    ) -> Result<EntityId, F> {
        let definition = self
            .schema
            .entity(type_name)
            .ok_or_else(|| F::UnknownEntity {
                schema: self.schema.name().to_owned(),
                type_name: type_name.to_owned(),
            })?;
        if definition.abstract_ {
            return Err(F::AbstractEntity {
                type_name: definition.name.clone(),
            });
        }
        let id = self.view.allocate();
        let supplied = attributes
            .iter()
            .any(|(name, _)| name.eq_ignore_ascii_case(GLOBAL_ID));
        if !supplied && self.schema.is_a(type_name, "IfcRoot") {
            attributes.push((GLOBAL_ID.to_owned(), Value::Text(self.global_id(id).into())));
        }
        let entity = build(self.schema, type_name, &attributes)?;
        relations(self.schema, &self.view, &entity, None)?;
        if supplied {
            self.claim_global_id(id, &entity)?;
        }
        self.view.create(id, entity);
        Ok(id)
    }

    /// Replace named attributes of `id`, then check the whole record.
    fn edit(&mut self, id: EntityId, attributes: NamedValues) -> Result<EntityId, F> {
        let mut entity = self.view.get(id).cloned().ok_or(F::MissingEntity(id))?;
        let slots = crate::attribute_slots(self.schema, &entity.type_name).map_err(|_| {
            F::UnknownEntity {
                schema: self.schema.name().to_owned(),
                type_name: entity.type_name.to_string(),
            }
        })?;
        let mut touched = Vec::new();
        for (name, value) in attributes {
            let Some(slot) = slots.iter().find(|s| s.name.eq_ignore_ascii_case(&name)) else {
                return Err(F::Author(ifc_author::AuthorError::UnknownAttribute {
                    entity: entity.type_name.to_string(),
                    attribute: name,
                    known: slots.iter().map(|s| s.name.to_owned()).collect(),
                }));
            };
            if touched.contains(&slot.index) {
                return Err(F::Author(ifc_author::AuthorError::DuplicateAttribute {
                    entity: entity.type_name.to_string(),
                    attribute: slot.name.to_owned(),
                }));
            }
            if slot.derived {
                return Err(F::Author(ifc_author::AuthorError::DerivedAttribute {
                    entity: entity.type_name.to_string(),
                    attribute: slot.name.to_owned(),
                    found: format!("{value:?}"),
                }));
            }
            if entity.attributes.len() <= slot.index {
                entity.attributes.resize(slot.index + 1, Value::Null);
            }
            entity.attributes[slot.index] = value;
            touched.push(slot.index);
        }
        recheck(self.schema, &entity)?;
        relations(self.schema, &self.view, &entity, Some(&touched))?;
        if touched.contains(&0) && self.schema.is_a(&entity.type_name, "IfcRoot") {
            self.claim_global_id(id, &entity)?;
        }
        self.view.replace(id, entity);
        Ok(id)
    }

    /// Refuse `type_name` unless it is a `kind`.
    pub(super) fn require_kind(&self, type_name: &str, kind: &str) -> Result<(), F> {
        if self.schema.entity(type_name).is_none() {
            return Err(F::UnknownEntity {
                schema: self.schema.name().to_owned(),
                type_name: type_name.to_owned(),
            });
        }
        if self.schema.is_a(type_name, kind) {
            Ok(())
        } else {
            Err(F::NotA {
                type_name: type_name.to_owned(),
                expected: kind.to_owned(),
            })
        }
    }

    /// The spatial element supertype of the release: `IfcSpatialElement`
    /// from IFC4 on, `IfcSpatialStructureElement` in IFC2X3.
    pub(super) fn spatial_kind(&self) -> &'static str {
        if self.schema.entity("IfcSpatialElement").is_some() {
            "IfcSpatialElement"
        } else {
            "IfcSpatialStructureElement"
        }
    }

    /// The `IfcProject`; one per model.
    fn project(
        &mut self,
        attributes: NamedValues,
        owner_history: Option<EntityId>,
    ) -> Result<EntityId, F> {
        if let Some(existing) = self.view.ids_of_type("IFCPROJECT").first() {
            return Err(F::AlreadyRelated {
                id: *existing,
                relationship: "IFCPROJECT",
                existing: *existing,
            });
        }
        self.create("IfcProject", with_owner(attributes, owner_history))
    }

    /// A spatial element, aggregated under `parent`.
    fn spatial(
        &mut self,
        type_name: &str,
        parent: EntityId,
        attributes: NamedValues,
        placement: Option<EntityId>,
        owner_history: Option<EntityId>,
    ) -> Result<EntityId, F> {
        self.require_kind(type_name, self.spatial_kind())?;
        let found = self
            .view
            .get(parent)
            .ok_or_else(|| F::MissingReference {
                entity: "IFCRELAGGREGATES".to_owned(),
                attribute: "RelatingObject".to_owned(),
                target: parent,
            })?
            .type_name
            .clone();
        if !self.schema.is_a(&found, "IfcProject") && !self.schema.is_a(&found, self.spatial_kind())
        {
            return Err(F::WrongReferenceType {
                entity: "IFCRELAGGREGATES".to_owned(),
                attribute: "RelatingObject".to_owned(),
                target: parent,
                actual: found.to_string(),
                expected: format!("IfcProject or {}", self.spatial_kind()),
            });
        }
        let attributes = with_placement(with_owner(attributes, owner_history), placement);
        let element = self.create(type_name, attributes)?;
        self.aggregate(parent, &[element], owner_history)?;
        Ok(element)
    }

    /// A product that is not a spatial element.
    fn product(
        &mut self,
        type_name: &str,
        attributes: NamedValues,
        placement: Option<EntityId>,
        owner_history: Option<EntityId>,
    ) -> Result<EntityId, F> {
        self.require_kind(type_name, "IfcProduct")?;
        if self.schema.is_a(type_name, self.spatial_kind()) {
            return Err(F::NotA {
                type_name: type_name.to_owned(),
                expected: format!("IfcProduct that is not an {}", self.spatial_kind()),
            });
        }
        let attributes = with_placement(with_owner(attributes, owner_history), placement);
        self.create(type_name, attributes)
    }

    /// A fresh `GlobalId` for entity `id`: name-based over the batch seed,
    /// so a seed drawn per batch keeps it unique, and a given seed
    /// reproduces the file.
    fn global_id(&self, id: EntityId) -> String {
        name_based_guid(&[
            "openbim-ifc/authoring",
            &format!("{:032x}", self.seed),
            &id.0.to_string(),
        ])
    }

    /// Record the `GlobalId` of `entity`, refusing one another entity holds.
    fn claim_global_id(&mut self, id: EntityId, entity: &Entity) -> Result<(), F> {
        let Some(Value::Text(global_id)) = entity.attributes.first() else {
            return Ok(());
        };
        let holder = self
            .batch_global_ids
            .get(global_id)
            .copied()
            .or_else(|| self.base_global_ids().get(global_id).copied())
            .filter(|holder| *holder != id && self.view.get(*holder).is_some());
        if let Some(holder) = holder {
            return Err(F::DuplicateGlobalId {
                global_id: global_id.to_string(),
                holder,
            });
        }
        self.batch_global_ids.insert(global_id.clone(), id);
        Ok(())
    }

    fn base_global_ids(&mut self) -> &HashMap<Arc<str>, EntityId> {
        let (schema, base) = (self.schema, self.view.base());
        self.base_global_ids.get_or_insert_with(|| {
            let mut roots: HashMap<Arc<str>, bool> = HashMap::new();
            base.iter()
                .filter(|(_, entity)| {
                    *roots
                        .entry(entity.type_name.clone())
                        .or_insert_with(|| schema.is_a(&entity.type_name, "IfcRoot"))
                })
                .filter_map(|(id, entity)| match entity.attributes.first() {
                    Some(Value::Text(text)) => Some((text.clone(), id)),
                    _ => None,
                })
                .collect()
        })
    }
}

/// `attributes` with `OwnerHistory` added when one is given.
pub(super) fn with_owner(mut attributes: NamedValues, owner: Option<EntityId>) -> NamedValues {
    if let Some(owner) = owner {
        attributes.push(("OwnerHistory".to_owned(), Value::Ref(owner)));
    }
    attributes
}

/// `attributes` with `ObjectPlacement` added when one is given.
fn with_placement(mut attributes: NamedValues, placement: Option<EntityId>) -> NamedValues {
    if let Some(placement) = placement {
        attributes.push(("ObjectPlacement".to_owned(), Value::Ref(placement)));
    }
    attributes
}
