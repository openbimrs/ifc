//! Running a batch's operations against the overlay.

use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;

use ifc_model::guid::Guid;
use ifc_model::{Entity, EntityId, Value};
use ifc_schema::Schema;

use super::check::{build, recheck, relations};
use super::identity::{derived_global_id, Scope};
use super::overlay::Overlay;
use super::report::{CopyReport, RemovalReport};
use super::{AssignTypeOptions, AuthorOp, AuthoringFailure as F, NamedValues};

/// The batch's state: the overlay, and what each operation produced.
pub(super) struct Planner<'m, 's> {
    pub(super) schema: &'s Schema,
    pub(super) view: Overlay<'m>,
    seed: u128,
    /// Where `GlobalId`s are derived now: `run` sets the operation; an append
    /// never calls `run`.
    scope: Scope,
    /// Per type (upper case), how many `GlobalId`s the current scope derived.
    drawn: BTreeMap<String, u32>,
    /// Per operation run so far, the entity it produced.
    pub(super) ids: Vec<Option<EntityId>>,
    /// Per operation run so far, the report of a removal or purge.
    pub(super) reports: Vec<Option<RemovalReport>>,
    /// The report of the operation running now.
    report: Option<RemovalReport>,
    /// Per operation run so far, the report of a copy (#479).
    pub(super) copies: Vec<Option<CopyReport>>,
    /// The copy report of the operation running now.
    copy_report: Option<CopyReport>,
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
            scope: Scope::Append,
            drawn: BTreeMap::new(),
            ids: Vec::new(),
            reports: Vec::new(),
            report: None,
            copies: Vec::new(),
            copy_report: None,
            batch_global_ids: HashMap::new(),
        }
    }

    /// Run one operation, returning the entity it produced; its report,
    /// if it makes one, is pushed on [`Self::reports`].
    pub(super) fn run(&mut self, index: usize, op: AuthorOp) -> Result<Option<EntityId>, F> {
        self.scope = Scope::Op(index);
        self.drawn.clear();
        let produced = self.run_op(op)?;
        let report = self.report.take();
        self.reports.push(report);
        let copied = self.copy_report.take();
        self.copies.push(copied);
        Ok(produced)
    }

    fn run_op(&mut self, op: AuthorOp) -> Result<Option<EntityId>, F> {
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
            AuthorOp::RemoveProduct { product, options } => {
                self.report = Some(self.remove_product(product, options)?);
                None
            }
            AuthorOp::Purge(options) => {
                self.report = Some(self.purge(&options)?);
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
                    self.contain(container, &[product], owner_history, false)?;
                }
                if let Some(type_object) = type_object {
                    self.assign_type(
                        type_object,
                        &[product],
                        owner_history,
                        &AssignTypeOptions::new(),
                    )?;
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
                options,
            } => Some(self.assign_type(type_object, &objects, owner_history, &options)?),
            AuthorOp::Contain {
                structure,
                elements,
                owner_history,
                options,
            } => Some(self.contain(structure, &elements, owner_history, options.replace)?),
            AuthorOp::Aggregate {
                parent,
                parts,
                owner_history,
                options,
            } => Some(self.aggregate(parent, &parts, owner_history, options.replace)?),
            AuthorOp::Placement {
                relative_to,
                location,
                axes,
            } => Some(self.placement(relative_to, location, axes)?),
            AuthorOp::OwnerHistory(fields) => Some(self.owner_history(&fields)?),
            AuthorOp::ShapeRepresentation {
                context,
                items,
                identifier,
                representation_type,
            } => {
                Some(self.shape_representation(context, &items, identifier, representation_type)?)
            }
            AuthorOp::AssignRepresentation { representation, to } => {
                Some(self.assign_representation(representation, to)?)
            }
            AuthorOp::UnassignRepresentation {
                representation,
                from,
            } => {
                self.report = Some(self.unassign_representation(representation, from)?);
                Some(representation)
            }
            AuthorOp::CopyRepresentation { representation } => {
                Some(self.copy_representation(representation)?)
            }
            AuthorOp::RemoveRepresentation { representation } => {
                self.report = Some(self.remove_representation(representation)?);
                None
            }
            AuthorOp::CopyProduct { product, options } => {
                let (copy, report) = self.copy_product(product, options)?;
                self.copy_report = Some(report);
                Some(copy)
            }
            AuthorOp::CopyEntity { entity, policy } => {
                let (copy, report) = self.copy_entity(entity, &policy)?;
                self.copy_report = Some(report);
                Some(copy)
            }
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
            let global_id = self.fresh_global_id(id, type_name)?;
            attributes.push((GLOBAL_ID.to_owned(), Value::Text(global_id.into())));
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
    pub(super) fn edit(&mut self, id: EntityId, attributes: NamedValues) -> Result<EntityId, F> {
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
        self.aggregate(parent, &[element], owner_history, false)?;
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

    /// A new `GlobalId` for `id`, an entity of `type_name` the running operation creates
    /// (see `identity`), recorded as the batch's. Refused with `SeedReused` when an entity
    /// the batch so far leaves in the model holds it.
    pub(super) fn fresh_global_id(&mut self, id: EntityId, type_name: &str) -> Result<String, F> {
        let key = type_name.to_ascii_uppercase();
        let count = self.drawn.entry(key.clone()).or_insert(0);
        let ordinal = *count;
        *count += 1;
        let global_id = derived_global_id(self.seed, self.scope, &key, ordinal);
        if let Some(holder) = self.global_id_holder(&global_id) {
            return Err(F::SeedReused { global_id, holder });
        }
        self.batch_global_ids
            .insert(Arc::from(global_id.as_str()), id);
        Ok(global_id)
    }

    /// Record the `GlobalId` of `entity`, refusing one another entity holds.
    pub(super) fn claim_global_id(&mut self, id: EntityId, entity: &Entity) -> Result<(), F> {
        let Some(Value::Text(global_id)) = entity.attributes.first() else {
            return Ok(());
        };
        let holder = match self.batch_global_ids.get(global_id) {
            Some(&holder) => Some(holder).filter(|holder| *holder != id && self.present(*holder)),
            None => self.base_holder(global_id, Some(id)),
        };
        if let Some(holder) = holder {
            return Err(F::DuplicateGlobalId {
                global_id: global_id.to_string(),
                holder,
            });
        }
        self.batch_global_ids.insert(global_id.clone(), id);
        Ok(())
    }

    /// The entity that holds `global_id` as the batch so far leaves the
    /// model, if any.
    pub(super) fn global_id_holder(&self, global_id: &str) -> Option<EntityId> {
        match self.batch_global_ids.get(global_id) {
            Some(&holder) => Some(holder).filter(|holder| self.present(*holder)),
            None => self.base_holder(global_id, None),
        }
    }

    fn present(&self, id: EntityId) -> bool {
        self.view.get(id).is_some()
    }

    /// The `IfcRoot` of the base model holding `text` that the batch leaves
    /// in the model, other than `except`; the lowest id when several do.
    ///
    /// Text of the GlobalId form is looked up in the model's GlobalId index;
    /// anything else is not indexed and found by a scan.
    fn base_holder(&self, text: &str, except: Option<EntityId>) -> Option<EntityId> {
        let base = self.view.base();
        let holds = |id: EntityId, entity: &Entity| {
            Some(id) != except
                && self.present(id)
                && self.schema.is_a(&entity.type_name, "IfcRoot")
                && matches!(entity.attributes.first(), Some(Value::Text(t)) if &**t == text)
        };
        if let Some(guid) = Guid::parse(text) {
            return base
                .global_id_candidates(&guid)
                .iter()
                .copied()
                .find(|id| base.get(*id).is_some_and(|entity| holds(*id, entity)));
        }
        let mut lowest: Option<EntityId> = None;
        for (type_name, _) in base.type_histogram() {
            if !self.schema.is_a(type_name, "IfcRoot") {
                continue;
            }
            for (id, entity) in base.of_type(type_name) {
                if holds(id, entity) && lowest.is_none_or(|lowest| id < lowest) {
                    lowest = Some(id);
                }
            }
        }
        lowest
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
