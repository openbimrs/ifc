//! Planning a batch: each edit is applied to drafts of the sets it touches.
//!
//! Nothing here writes. A draft is one object's set as the batch leaves it:
//! where it came from (an existing set, held alone or shared, or a new
//! one) and its members, each kept as it is, changed in place, or created
//! fresh. Edits apply in order, each against the drafts the earlier ones
//! left, and every check that can refuse runs here, so `emit` only stages.

use std::collections::BTreeMap;

use ifc_model::{EntityId, Model, ReverseIndex, Value};
use ifc_properties::{exact_schema, ExactPropertyError, QuantityKind};

use super::checks::{
    agree, check_set_template, check_template, default_form, describe_payload, same_value,
    template_form,
};
use super::edit::{PropertyEdit, PropertyEditError, PropertyEditFailure, SetType};
use super::holder::{holding, sole_member, Holder, Link, Release};
use super::template::{template, SetTemplate};
use super::value::{check_ifc_value, describe, quantity_number, wrapper};

/// The member forms this writer writes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Form {
    /// `IfcPropertySingleValue`.
    Single,
    /// `IfcPropertyEnumeratedValue`.
    Enumerated,
    /// `IfcPropertyListValue`.
    List,
    /// A simple quantity of this kind.
    Quantity(QuantityKind),
}

/// One member of a draft set.
#[derive(Debug, Clone)]
pub(super) struct Member {
    pub(super) name: String,
    pub(super) state: MemberState,
    /// The last edit that changed it, for a refusal at emission.
    pub(super) edit: usize,
}

/// What becomes of a member.
#[derive(Debug, Clone)]
pub(super) enum MemberState {
    /// The existing entity, unchanged.
    Kept(EntityId),
    /// The existing entity, which the set alone holds, with a new value.
    InPlace {
        id: EntityId,
        form: Form,
        value: Value,
    },
    /// A new entity: a copy of `source` with the new value, or, without
    /// one, a new property or quantity.
    Fresh {
        form: Form,
        value: Value,
        source: Option<EntityId>,
    },
}

/// Where a draft set comes from.
#[derive(Debug, Clone, Copy)]
pub(super) enum Origin {
    /// A set the batch creates.
    New,
    /// An existing set of the holder, held as `link`; `exclusive` when no
    /// other object holds it.
    Existing {
        id: EntityId,
        link: Link,
        exclusive: bool,
    },
}

/// One object's set as the batch leaves it.
#[derive(Debug, Clone)]
pub(super) struct SetDraft {
    pub(super) holder: EntityId,
    pub(super) name: String,
    pub(super) set_type: SetType,
    pub(super) origin: Origin,
    pub(super) members: Vec<Member>,
    /// Whether the member list differs from the existing set's.
    pub(super) changed: bool,
    /// Existing members the batch removed from the list.
    pub(super) dropped: Vec<EntityId>,
    /// The first edit that touched it.
    pub(super) edit: usize,
}

/// The batch being planned.
pub(super) struct Planner<'m> {
    pub(super) model: &'m Model,
    pub(super) release: Release,
    reverse: Option<ReverseIndex>,
    pub(super) holders: BTreeMap<EntityId, Holder>,
    pub(super) sets: Vec<SetDraft>,
    index: BTreeMap<(EntityId, String), usize>,
    templates: BTreeMap<String, Option<SetTemplate>>,
    /// Per edit: the draft and member name a Set wrote; `None` for a Remove.
    pub(super) results: Vec<Option<(usize, String)>>,
}

impl<'m> Planner<'m> {
    /// Bind the batch to `model`'s release.
    pub(super) fn new(model: &'m Model) -> Result<Self, PropertyEditError> {
        let batch = |failure| PropertyEditError {
            edit: None,
            failure,
        };
        let version = exact_schema(model).map_err(|e| batch(PropertyEditFailure::Resolve(e)))?;
        let schema = ifc_schema::for_version(version).map_err(|_| {
            batch(PropertyEditFailure::Resolve(
                ExactPropertyError::UnsupportedSchema {
                    schema: format!("{version:?}"),
                },
            ))
        })?;
        Ok(Self {
            model,
            release: Release { version, schema },
            reverse: None,
            holders: BTreeMap::new(),
            sets: Vec::new(),
            index: BTreeMap::new(),
            templates: BTreeMap::new(),
            results: Vec::new(),
        })
    }

    /// The reverse index of the model as it was, built on first use.
    pub(super) fn reverse(&mut self) -> &ReverseIndex {
        self.reverse
            .get_or_insert_with(|| ReverseIndex::build(self.model))
    }

    /// Plan edit `index`.
    pub(super) fn apply(
        &mut self,
        index: usize,
        edit: &PropertyEdit,
    ) -> Result<(), PropertyEditFailure> {
        let result = match edit {
            PropertyEdit::Set {
                object,
                set,
                name,
                value,
                set_type,
            } => Some(self.set(index, *object, set, name, value, *set_type)?),
            PropertyEdit::Remove { object, set, name } => {
                self.remove(index, *object, set, name)?;
                None
            }
        };
        self.results.push(result);
        Ok(())
    }

    fn holder(&mut self, object: EntityId) -> Result<&Holder, PropertyEditFailure> {
        if !self.holders.contains_key(&object) {
            let holder = Holder::read(self.model, self.release, object)?;
            self.holders.insert(object, holder);
        }
        Ok(&self.holders[&object])
    }

    fn template(&mut self, set: &str) -> Result<Option<SetTemplate>, PropertyEditFailure> {
        if let Some(found) = self.templates.get(set) {
            return Ok(found.clone());
        }
        let found = template(self.release.version, set)?;
        self.templates.insert(set.to_owned(), found.clone());
        Ok(found)
    }

    fn set(
        &mut self,
        index: usize,
        object: EntityId,
        set: &str,
        name: &str,
        value: &Value,
        hint: Option<SetType>,
    ) -> Result<(usize, String), PropertyEditFailure> {
        for (what, text) in [("set", set), ("property", name)] {
            if text.trim().is_empty() {
                return Err(PropertyEditFailure::InvalidValue(format!(
                    "a {what} name must not be blank"
                )));
            }
        }
        // The object first: a missing or unfit object is the refusal even
        // where the set's template could not be read.
        let is_type = self.holder(object)?.is_type;
        let template = self.template(set)?;
        let draft = self
            .draft(index, object, set, hint, template.as_ref(), true)?
            .expect("a draft is created when asked to");
        if let Some(template) = &template {
            check_set_template(template, self.sets[draft].set_type, is_type)?;
        }
        let member_template = template.as_ref().and_then(|t| t.members.get(name));
        let position = self.sets[draft].members.iter().position(|m| m.name == name);
        match position {
            Some(position) => {
                let (form, source) = match &self.sets[draft].members[position].state {
                    MemberState::Kept(id) => (self.form_of(*id)?, Some(*id)),
                    MemberState::InPlace { id, form, .. } => (*form, Some(*id)),
                    MemberState::Fresh { form, source, .. } => (*form, *source),
                };
                self.check_value(form, value, source)?;
                if let Some(member) = member_template {
                    check_template(self.release, set, name, member, form, value, false)?;
                }
                let state = match self.sets[draft].members[position].state.clone() {
                    MemberState::Kept(id) | MemberState::InPlace { id, .. } => {
                        if self.alone(draft, id) {
                            MemberState::InPlace {
                                id,
                                form,
                                value: value.clone(),
                            }
                        } else {
                            self.sets[draft].changed = true;
                            MemberState::Fresh {
                                form,
                                value: value.clone(),
                                source: Some(id),
                            }
                        }
                    }
                    MemberState::Fresh { source, .. } => MemberState::Fresh {
                        form,
                        value: value.clone(),
                        source,
                    },
                };
                let member = &mut self.sets[draft].members[position];
                member.state = state;
                member.edit = index;
            }
            None => {
                if let (Some(template), None) = (&template, member_template) {
                    return Err(PropertyEditFailure::Template(format!(
                        "{name} is no property of {} in the {:?} catalog",
                        template.name, self.release.version
                    )));
                }
                let inherited = self.holders[&object]
                    .inherited
                    .get(&(set.to_owned(), name.to_owned()))
                    .copied()
                    .filter(|_| !is_type);
                let set_type = self.sets[draft].set_type;
                let (form, source) = match (inherited, member_template) {
                    (Some(inherited), _) => {
                        (self.form_of(inherited.property)?, Some(inherited.property))
                    }
                    (None, Some(member)) => (template_form(member, set, name)?, None),
                    (None, None) => (default_form(self.release, set_type, value)?, None),
                };
                if matches!(form, Form::Quantity(_)) != (set_type == SetType::ElementQuantity) {
                    return Err(PropertyEditFailure::InvalidValue(format!(
                        "{set} is an {} and cannot hold {name} as {form:?}",
                        set_type.entity()
                    )));
                }
                if let Some(member) = member_template {
                    check_template(self.release, set, name, member, form, value, true)?;
                }
                self.check_value(form, value, source)?;
                let draft_set = &mut self.sets[draft];
                draft_set.members.push(Member {
                    name: name.to_owned(),
                    state: MemberState::Fresh {
                        form,
                        value: value.clone(),
                        source,
                    },
                    edit: index,
                });
                draft_set.changed = true;
            }
        }
        Ok((draft, name.to_owned()))
    }

    fn remove(
        &mut self,
        index: usize,
        object: EntityId,
        set: &str,
        name: &str,
    ) -> Result<(), PropertyEditFailure> {
        self.holder(object)?;
        let draft = self.draft(index, object, set, None, None, false)?;
        let found = draft.and_then(|draft| {
            self.sets[draft]
                .members
                .iter()
                .position(|m| m.name == name)
                .map(|position| (draft, position))
        });
        let Some((draft, position)) = found else {
            let holder = &self.holders[&object];
            let inherited_from = (!holder.is_type)
                .then(|| holder.inherited.get(&(set.to_owned(), name.to_owned())))
                .flatten()
                .map(|inherited| inherited.type_object);
            return Err(PropertyEditFailure::MissingProperty {
                object,
                set: set.to_owned(),
                name: name.to_owned(),
                inherited_from,
            });
        };
        let draft = &mut self.sets[draft];
        let member = draft.members.remove(position);
        if let MemberState::Kept(id) | MemberState::InPlace { id, .. } = member.state {
            draft.dropped.push(id);
        }
        draft.changed = true;
        Ok(())
    }

    /// The draft of `object`'s set `set`, opened from its existing set, or
    /// created when `create` (`None` otherwise).
    fn draft(
        &mut self,
        index: usize,
        object: EntityId,
        set: &str,
        hint: Option<SetType>,
        template: Option<&SetTemplate>,
        create: bool,
    ) -> Result<Option<usize>, PropertyEditFailure> {
        let key = (object, set.to_owned());
        if let Some(draft) = self.index.get(&key).copied() {
            agree(set, self.sets[draft].set_type, hint, "the edit's set type")?;
            return Ok(Some(draft));
        }
        let holder = self.holders[&object].clone();
        let draft = if let Some(own) = holder.own.get(set) {
            let set_type = SetType::from_entity(&own.type_name).ok_or_else(|| {
                PropertyEditFailure::Unsupported(format!(
                    "{set} of {object} is an {}, a predefined set this writer does not edit",
                    own.type_name
                ))
            })?;
            agree(set, set_type, hint, "the edit's set type")?;
            let (model, release) = (self.model, self.release);
            let (link, exclusive) = holding(model, release, self.reverse(), &holder, own.id)?;
            let list = match set_type {
                SetType::PropertySet => "HasProperties",
                SetType::ElementQuantity => "Quantities",
            };
            let members = release
                .attribute(model, own.id, list)
                .and_then(Value::as_list)
                .unwrap_or_default()
                .iter()
                .filter_map(Value::as_ref_id)
                .map(|id| Member {
                    name: release
                        .text(model, id, "Name")
                        .unwrap_or_default()
                        .to_owned(),
                    state: MemberState::Kept(id),
                    edit: index,
                })
                .collect();
            SetDraft {
                holder: object,
                name: set.to_owned(),
                set_type,
                origin: Origin::Existing {
                    id: own.id,
                    link,
                    exclusive,
                },
                members,
                changed: false,
                dropped: Vec::new(),
                edit: index,
            }
        } else if create {
            let inherited = holder
                .inherited_sets
                .get(set)
                .filter(|_| !holder.is_type)
                .map(|(type_object, entity)| {
                    SetType::from_entity(entity).ok_or_else(|| {
                        PropertyEditFailure::Unsupported(format!(
                            "{set} of type {type_object} is an {entity}, a predefined set this writer does not override"
                        ))
                    })
                })
                .transpose()?;
            let catalog = template.map(|t| {
                if t.quantity {
                    SetType::ElementQuantity
                } else {
                    SetType::PropertySet
                }
            });
            let set_type = hint
                .or(inherited)
                .or(catalog)
                .unwrap_or(SetType::PropertySet);
            agree(
                set,
                set_type,
                inherited,
                "the type object's set of that name",
            )?;
            agree(set, set_type, catalog, "the catalog")?;
            SetDraft {
                holder: object,
                name: set.to_owned(),
                set_type,
                origin: Origin::New,
                members: Vec::new(),
                changed: false,
                dropped: Vec::new(),
                edit: index,
            }
        } else {
            return Ok(None);
        };
        self.sets.push(draft);
        self.index.insert(key, self.sets.len() - 1);
        Ok(Some(self.sets.len() - 1))
    }

    /// Whether member `id` of draft `draft` may change in place: the draft's
    /// set is the holder's alone and holds `id` alone.
    fn alone(&mut self, draft: usize, id: EntityId) -> bool {
        let Origin::Existing {
            id: set,
            exclusive: true,
            ..
        } = self.sets[draft].origin
        else {
            return false;
        };
        let model = self.model;
        sole_member(model, self.reverse(), set, id)
    }

    /// The form of an existing property or quantity entity.
    fn form_of(&self, id: EntityId) -> Result<Form, PropertyEditFailure> {
        let entity = self
            .model
            .get(id)
            .ok_or(PropertyEditFailure::MissingEntity(id))?;
        let type_name = entity.type_name.to_ascii_uppercase();
        Ok(match type_name.as_str() {
            "IFCPROPERTYSINGLEVALUE" => Form::Single,
            "IFCPROPERTYENUMERATEDVALUE" => Form::Enumerated,
            "IFCPROPERTYLISTVALUE" => Form::List,
            other => match QuantityKind::from_type_name(other) {
                Some(kind) => Form::Quantity(kind),
                None => {
                    return Err(PropertyEditFailure::Unsupported(format!(
                        "{id} is an {other}; this writer edits single, enumerated and list values and simple quantities"
                    )))
                }
            },
        })
    }

    /// The release's checks of `value` written as `form`, against the
    /// existing entity `source` it replaces or copies.
    fn check_value(
        &self,
        form: Form,
        value: &Value,
        source: Option<EntityId>,
    ) -> Result<(), PropertyEditFailure> {
        let schema = self.release.schema;
        let invalid = PropertyEditFailure::InvalidValue;
        match form {
            Form::Single => {
                check_ifc_value(schema, value, true).map_err(invalid)?;
                self.check_unit(source, "NominalValue", wrapper(value))
            }
            Form::Enumerated | Form::List => {
                let items = match value {
                    Value::List(items) if !items.is_empty() => items,
                    other => {
                        return Err(invalid(format!(
                            "an {} takes a non-empty list of IfcValues, not {}",
                            if form == Form::List {
                                "IfcPropertyListValue"
                            } else {
                                "IfcPropertyEnumeratedValue"
                            },
                            describe(other)
                        )))
                    }
                };
                for item in items {
                    check_ifc_value(schema, item, false).map_err(invalid)?;
                }
                if form == Form::List {
                    if items.iter().any(|item| wrapper(item) != wrapper(&items[0])) {
                        return Err(invalid(
                            "the values of a list must share one type".to_owned(),
                        ));
                    }
                    let attribute = "ListValues";
                    return self.check_unit(source, attribute, wrapper(&items[0]));
                }
                self.check_enumeration(source, items)
            }
            Form::Quantity(kind) => {
                if schema.entity(kind.type_name()).is_none() {
                    return Err(invalid(format!(
                        "{} is not an entity of {:?}",
                        kind.type_name(),
                        self.release.version
                    )));
                }
                quantity_number(kind.measure_type(), value)
                    .map(drop)
                    .map_err(invalid)
            }
        }
    }

    /// A stated `Unit` fixes the measure: refuse a value of another one.
    fn check_unit(
        &self,
        source: Option<EntityId>,
        attribute: &str,
        new: Option<&str>,
    ) -> Result<(), PropertyEditFailure> {
        let Some(source) = source else { return Ok(()) };
        let (model, release) = (self.model, self.release);
        let Some(unit) = release
            .attribute(model, source, "Unit")
            .and_then(Value::as_ref_id)
        else {
            return Ok(());
        };
        let old = release
            .attribute(model, source, attribute)
            .and_then(|value| match value {
                Value::List(items) => items.first().and_then(wrapper),
                other => wrapper(other),
            });
        match (old, new) {
            (Some(old), Some(new)) if !old.eq_ignore_ascii_case(new) => {
                Err(PropertyEditFailure::InvalidValue(format!(
                    "{source} states unit {unit} for an {old}; an {new} would be read in it"
                )))
            }
            _ => Ok(()),
        }
    }

    /// Every value of an enumerated property must be in its enumeration.
    fn check_enumeration(
        &self,
        source: Option<EntityId>,
        items: &[Value],
    ) -> Result<(), PropertyEditFailure> {
        let (model, release) = (self.model, self.release);
        let Some(enumeration) = source
            .and_then(|source| release.attribute(model, source, "EnumerationReference"))
            .and_then(Value::as_ref_id)
        else {
            return Ok(());
        };
        let allowed = release
            .attribute(model, enumeration, "EnumerationValues")
            .and_then(Value::as_list)
            .unwrap_or_default();
        for item in items {
            if !allowed.iter().any(|candidate| same_value(candidate, item)) {
                return Err(PropertyEditFailure::Template(format!(
                    "{} is not a value of IfcPropertyEnumeration {enumeration} ({})",
                    describe_payload(item),
                    release
                        .text(model, enumeration, "Name")
                        .unwrap_or("unnamed")
                )));
            }
        }
        Ok(())
    }
}
