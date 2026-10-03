//! Staging a planned batch on one transaction.
//!
//! Every refusal a caller can provoke was raised while planning; what can
//! still fail here are the `ifc-properties` writers' own checks and the
//! object records a new set derives from. Nothing is committed: the caller
//! commits the transaction, whose preflight is the last word on references.

use std::collections::BTreeMap;

use ifc_model::{Edit, Entity, EntityId, Transaction, Value};
use ifc_properties::{
    add_element_quantity, add_element_quantity_with_owner_history, add_property_enumerated_value,
    add_property_list_value, add_property_set, add_property_set_with_owner_history,
    add_property_single_value, attach_property_set, attach_property_set_with_owner_history,
    create_quantity_with, set_quantity_value, QuantityExtras, QuantityKind, SchemaVersion,
};

use super::edit::{PropertyEditError, PropertyEditFailure, SetType, StagedPropertyEdits};
use super::guid::derived_global_id;
use super::holder::{Holder, Link, Release};
use super::plan::{Form, Member, MemberState, Origin, Planner, SetDraft};
use super::value::quantity_number;

/// Stage `planner`'s drafts.
pub(super) fn emit(mut planner: Planner<'_>) -> Result<StagedPropertyEdits, PropertyEditError> {
    let model = planner.model;
    let release = planner.release;
    let mut stage = Stage {
        tx: Transaction::new(model),
        release,
        next_id: model.next_id().0,
        serial: 0,
        rels: BTreeMap::new(),
        types: BTreeMap::new(),
    };
    let mut finals: Vec<Vec<(String, EntityId)>> = Vec::with_capacity(planner.sets.len());
    let sets = std::mem::take(&mut planner.sets);
    for draft in &sets {
        let holder = planner.holders[&draft.holder].clone();
        let staged = stage
            .draft(&mut planner, &holder, draft)
            .map_err(|(edit, failure)| PropertyEditError::at(edit, failure))?;
        finals.push(staged);
    }
    stage.flush(model);
    let properties = planner
        .results
        .iter()
        .map(|result| {
            result.as_ref().and_then(|(draft, name)| {
                finals[*draft]
                    .iter()
                    .find(|(member, _)| member == name)
                    .map(|(_, id)| *id)
            })
        })
        .collect();
    Ok(StagedPropertyEdits {
        transaction: stage.tx,
        properties,
    })
}

type Failed = (usize, PropertyEditFailure);

/// The transaction and the relationship and type lists it rewrites once.
struct Stage {
    tx: Transaction,
    release: Release,
    next_id: u64,
    serial: u64,
    /// `RelatedObjects` of shared relationships the batch detaches from.
    rels: BTreeMap<EntityId, Vec<EntityId>>,
    /// `HasPropertySets` of type objects whose sets the batch changes.
    types: BTreeMap<EntityId, Vec<EntityId>>,
}

impl Stage {
    /// Stage one draft; its members' final ids by name.
    fn draft(
        &mut self,
        planner: &mut Planner<'_>,
        holder: &Holder,
        draft: &SetDraft,
    ) -> Result<Vec<(String, EntityId)>, Failed> {
        let model = planner.model;
        let mut members = Vec::with_capacity(draft.members.len());
        for member in &draft.members {
            let id = self.member(planner, member).map_err(|f| (member.edit, f))?;
            members.push((member.name.clone(), id));
        }
        let failed = |failure| (draft.edit, failure);
        match draft.origin {
            Origin::Existing {
                id,
                link,
                exclusive: true,
            } => {
                if members.is_empty() {
                    self.tx.remove(id);
                    match link {
                        Link::Rel(rel) => {
                            self.tx.remove(rel);
                        }
                        Link::Type => self.type_list(holder.id, model).retain(|set| *set != id),
                    }
                } else if draft.changed {
                    let slot = self.list_slot(model, id, draft.set_type).map_err(failed)?;
                    self.tx
                        .set_attribute(id, slot, refs(members.iter().map(|(_, id)| *id)));
                }
                for dropped in &draft.dropped {
                    let reverse = planner.reverse();
                    if reverse.referrers(*dropped).iter().all(|r| r.from == id) {
                        self.tx.remove(*dropped);
                    }
                }
            }
            Origin::Existing {
                id,
                link,
                exclusive: false,
            } => {
                if !draft.changed {
                    return Ok(members);
                }
                let copy = if members.is_empty() {
                    None
                } else {
                    Some(
                        self.new_set(planner, holder, draft, &members, Some(id))
                            .map_err(failed)?,
                    )
                };
                match link {
                    Link::Rel(rel) => {
                        let release = self.release;
                        let related = self.rels.entry(rel).or_insert_with(|| {
                            release
                                .attribute(model, rel, "RelatedObjects")
                                .and_then(Value::as_list)
                                .map(|items| items.iter().filter_map(Value::as_ref_id).collect())
                                .unwrap_or_default()
                        });
                        related.retain(|object| *object != holder.id);
                        if let Some(copy) = copy {
                            self.attach(model, holder, &draft.name, copy)
                                .map_err(failed)?;
                        }
                    }
                    Link::Type => {
                        let list = self.type_list(holder.id, model);
                        match (list.iter().position(|set| *set == id), copy) {
                            (Some(position), Some(copy)) => list[position] = copy,
                            (Some(position), None) => {
                                list.remove(position);
                            }
                            (None, Some(copy)) => list.push(copy),
                            (None, None) => {}
                        }
                    }
                }
            }
            Origin::New => {
                if members.is_empty() {
                    return Ok(members);
                }
                let set = self
                    .new_set(planner, holder, draft, &members, None)
                    .map_err(failed)?;
                if holder.is_type {
                    self.type_list(holder.id, model).push(set);
                } else {
                    self.attach(model, holder, &draft.name, set)
                        .map_err(failed)?;
                }
            }
        }
        Ok(members)
    }

    /// Stage one member; its final id.
    fn member(
        &mut self,
        planner: &Planner<'_>,
        member: &Member,
    ) -> Result<EntityId, PropertyEditFailure> {
        let model = planner.model;
        let authoring = PropertyEditFailure::Authoring;
        match &member.state {
            MemberState::Kept(id) => Ok(*id),
            MemberState::InPlace { id, form, value } => {
                match form {
                    Form::Quantity(kind) => {
                        let number = quantity_number(kind.measure_type(), value)
                            .map_err(PropertyEditFailure::InvalidValue)?;
                        set_quantity_value(&mut self.tx, model, *id, number).map_err(authoring)?;
                    }
                    _ => {
                        let slot = self.value_slot(model, *id, *form)?;
                        self.tx.set_attribute(*id, slot, value.clone());
                    }
                }
                Ok(*id)
            }
            MemberState::Fresh {
                form,
                value,
                source: Some(source),
            } => {
                if let Form::Quantity(kind) = form {
                    let release = self.release;
                    let text = |attribute: &str| release.text(model, *source, attribute);
                    let extras = QuantityExtras {
                        description: text("Description"),
                        unit: self
                            .release
                            .attribute(model, *source, "Unit")
                            .and_then(Value::as_ref_id),
                        formula: text("Formula"),
                    };
                    let number = quantity_number(kind.measure_type(), value)
                        .map_err(PropertyEditFailure::InvalidValue)?;
                    let created =
                        quantity(&mut self.tx, model, *kind, &member.name, number, extras);
                    return created.map_err(authoring);
                }
                let entity = model
                    .get(*source)
                    .ok_or(PropertyEditFailure::MissingEntity(*source))?;
                let slot = self.value_slot(model, *source, *form)?;
                let mut attributes = entity.attributes.clone();
                attributes[slot] = value.clone();
                Ok(self
                    .tx
                    .create(Entity::new(entity.type_name.clone(), attributes)))
            }
            MemberState::Fresh {
                form,
                value,
                source: None,
            } => {
                let tx = &mut self.tx;
                let name = member.name.as_str();
                let list = || match value {
                    Value::List(items) => Some(items.clone()),
                    _ => None,
                };
                match form {
                    Form::Single => {
                        let value = (!matches!(value, Value::Null)).then(|| value.clone());
                        add_property_single_value(tx, name, None, value, None)
                    }
                    Form::Enumerated => add_property_enumerated_value(tx, name, None, list(), None),
                    Form::List => add_property_list_value(tx, name, None, list(), None),
                    Form::Quantity(kind) => {
                        let number = quantity_number(kind.measure_type(), value)
                            .map_err(PropertyEditFailure::InvalidValue)?;
                        quantity(tx, model, *kind, name, number, QuantityExtras::default())
                    }
                }
                .map_err(authoring)
            }
        }
    }

    /// Stage a new set of `draft`'s type holding `members`, copying the
    /// description (and method of measurement) of `copy_of`.
    fn new_set(
        &mut self,
        planner: &Planner<'_>,
        holder: &Holder,
        draft: &SetDraft,
        members: &[(String, EntityId)],
        copy_of: Option<EntityId>,
    ) -> Result<EntityId, PropertyEditFailure> {
        let model = planner.model;
        let global_id = self.global_id(holder, &draft.name, "set")?;
        let owner_history = self.owner_history(holder)?;
        let release = self.release;
        let text = |attribute: &str| copy_of.and_then(|set| release.text(model, set, attribute));
        let description = text("Description");
        let tx = &mut self.tx;
        let created = match draft.set_type {
            SetType::PropertySet => {
                let named: Vec<(&str, EntityId)> = members
                    .iter()
                    .map(|(name, id)| (name.as_str(), *id))
                    .collect();
                match owner_history {
                    Some(history) => add_property_set_with_owner_history(
                        tx,
                        model,
                        &global_id,
                        &draft.name,
                        description,
                        &named,
                        history,
                    ),
                    None => add_property_set(tx, &global_id, &draft.name, description, &named),
                }
            }
            SetType::ElementQuantity => {
                let method = text("MethodOfMeasurement");
                let ids: Vec<EntityId> = members.iter().map(|(_, id)| *id).collect();
                match owner_history {
                    Some(history) => add_element_quantity_with_owner_history(
                        tx,
                        model,
                        &global_id,
                        &draft.name,
                        method,
                        &ids,
                        history,
                    ),
                    None => add_element_quantity(tx, &global_id, &draft.name, method, &ids),
                }
            }
        };
        created.map_err(PropertyEditFailure::Authoring)
    }

    /// Relate `set` to the occurrence `holder`.
    fn attach(
        &mut self,
        model: &ifc_model::Model,
        holder: &Holder,
        name: &str,
        set: EntityId,
    ) -> Result<EntityId, PropertyEditFailure> {
        let global_id = self.global_id(holder, name, "rel")?;
        match self.owner_history(holder)? {
            Some(history) => attach_property_set_with_owner_history(
                &mut self.tx,
                model,
                &global_id,
                &[holder.id],
                set,
                history,
            ),
            None => attach_property_set(&mut self.tx, model, &global_id, &[holder.id], set),
        }
        .map_err(PropertyEditFailure::Authoring)
    }

    fn global_id(
        &mut self,
        holder: &Holder,
        set: &str,
        role: &str,
    ) -> Result<String, PropertyEditFailure> {
        let owner = holder
            .global_id
            .as_deref()
            .filter(|text| ifc_model::guid::Guid::parse(text).is_some())
            .ok_or_else(|| {
                PropertyEditFailure::InvalidModel(format!(
                    "{} has no valid GlobalId to derive a new record's from",
                    holder.id
                ))
            })?;
        self.serial += 1;
        Ok(derived_global_id(
            owner,
            set,
            role,
            self.next_id,
            self.serial,
        ))
    }

    /// The holder's `OwnerHistory` for new records; IFC2X3 requires one.
    fn owner_history(&self, holder: &Holder) -> Result<Option<EntityId>, PropertyEditFailure> {
        match (holder.owner_history, self.release.version) {
            (None, SchemaVersion::Ifc2x3) => Err(PropertyEditFailure::InvalidModel(format!(
                "{} states no OwnerHistory, which IFC2X3 requires of the records an edit creates",
                holder.id
            ))),
            (history, _) => Ok(history),
        }
    }

    fn list_slot(
        &self,
        model: &ifc_model::Model,
        set: EntityId,
        set_type: SetType,
    ) -> Result<usize, PropertyEditFailure> {
        let attribute = match set_type {
            SetType::PropertySet => "HasProperties",
            SetType::ElementQuantity => "Quantities",
        };
        self.slot_of(model, set, attribute)
    }

    fn value_slot(
        &self,
        model: &ifc_model::Model,
        id: EntityId,
        form: Form,
    ) -> Result<usize, PropertyEditFailure> {
        let attribute = match form {
            Form::Single => "NominalValue",
            Form::Enumerated => "EnumerationValues",
            Form::List => "ListValues",
            Form::Quantity(_) => unreachable!("quantities are written by ifc-properties"),
        };
        self.slot_of(model, id, attribute)
    }

    fn slot_of(
        &self,
        model: &ifc_model::Model,
        id: EntityId,
        attribute: &str,
    ) -> Result<usize, PropertyEditFailure> {
        let entity = model
            .get(id)
            .ok_or(PropertyEditFailure::MissingEntity(id))?;
        let slot = self
            .release
            .slot(&entity.type_name, attribute)
            .ok_or_else(|| {
                PropertyEditFailure::InvalidModel(format!(
                    "{id} {} declares no {attribute}",
                    entity.type_name
                ))
            })?;
        if slot >= entity.attributes.len() {
            return Err(PropertyEditFailure::InvalidModel(format!(
                "{id} {} has {} attributes, too few for {attribute}",
                entity.type_name,
                entity.attributes.len()
            )));
        }
        Ok(slot)
    }

    /// The type object's `HasPropertySets` as the batch leaves it.
    fn type_list(&mut self, type_object: EntityId, model: &ifc_model::Model) -> &mut Vec<EntityId> {
        let release = self.release;
        self.types.entry(type_object).or_insert_with(|| {
            release
                .attribute(model, type_object, "HasPropertySets")
                .and_then(Value::as_list)
                .map(|items| items.iter().filter_map(Value::as_ref_id).collect())
                .unwrap_or_default()
        })
    }

    /// Stage the rewritten relationship and type lists.
    fn flush(&mut self, model: &ifc_model::Model) {
        let release = self.release;
        for (rel, related) in std::mem::take(&mut self.rels) {
            if related.is_empty() {
                self.tx.remove(rel);
            } else if let Some(slot) = model
                .get(rel)
                .and_then(|entity| release.slot(&entity.type_name, "RelatedObjects"))
            {
                self.tx.set_attribute(rel, slot, refs(related));
            }
        }
        for (type_object, sets) in std::mem::take(&mut self.types) {
            if let Some(slot) = model
                .get(type_object)
                .and_then(|entity| release.slot(&entity.type_name, "HasPropertySets"))
            {
                // `OPTIONAL SET [1:?]`: an emptied list is unset, not `()`.
                let value = if sets.is_empty() {
                    Value::Null
                } else {
                    refs(sets)
                };
                self.tx.set_attribute(type_object, slot, value);
            }
        }
    }
}

/// [`create_quantity_with`], with the entity name upper-case as the model
/// stores every other one: the writer names it in schema casing
/// (`IfcQuantityLength`), which `IfcModel::type_of` would report until the
/// file is read back.
fn quantity(
    tx: &mut Transaction,
    model: &ifc_model::Model,
    kind: QuantityKind,
    name: &str,
    number: f64,
    extras: QuantityExtras<'_>,
) -> Result<EntityId, ifc_properties::PropertyError> {
    let mut scratch = Transaction::new(model);
    create_quantity_with(&mut scratch, model, kind, name, number, extras)?;
    let Some(Edit::Create { entity, .. }) = scratch.edits().first() else {
        unreachable!("create_quantity_with stages exactly one create on success")
    };
    Ok(tx.create(Entity::new(
        entity.type_name.to_ascii_uppercase(),
        entity.attributes.clone(),
    )))
}

fn refs(ids: impl IntoIterator<Item = EntityId>) -> Value {
    Value::List(ids.into_iter().map(Value::Ref).collect())
}
