//! Many writes by path from text, planned as one transaction (#472, #474):
//! [`SelectorIndex::plan_text_edits`] and [`apply_text_edits`].
//!
//! Each [`TextEdit`] names an entity, a [`ValuePath`] and the text to
//! write there, as a table cell or a command-line `--set` gives it. The
//! plan reads every edit against the model as it is, before any write:
//!
//! - an edit whose text is the text the path reads now ([`value_text`])
//!   changes nothing, so a table exported and imported unchanged is a
//!   no-op;
//! - otherwise the text is read as a value of each target's type (see
//!   [`text`](super::text)), and a target that already holds that value is
//!   left alone;
//! - two edits giving one target different values are refused
//!   ([`SelectorError::ConflictingWrite`]); equal values are written once.
//!
//! Every refusal is collected, each naming its edit, and a plan with any
//! refusal is refused whole: nothing is written unless everything can be.
//! The attribute writes and the property edits are staged on one
//! transaction, which [`EditPlan::commit`] commits or refuses as a unit.

use std::collections::HashMap;
use std::fmt;

use ifc_model::{Applied, EntityId, Model, Transaction, Value};
use ifc_schema::Schema;

use super::ast::ValuePath;
use super::error::SelectorError;
use super::index::SelectorIndex;
use super::set::SetTarget;
use super::text::{parse_declared, value_text};
use super::value::SelectedValue;
use crate::declared_type::attribute_type;

/// Property-edit refusals collected before a plan gives up looking for
/// more: each costs a full replan of the property edits.
#[cfg(feature = "properties")]
const PROPERTY_REFUSALS: usize = 16;

/// One write from text: `text` at `path` of `entity`.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct TextEdit {
    /// The entity the path starts from.
    pub entity: EntityId,
    /// What is written: `Name`, `type.Name`, `Pset_WallCommon.FireRating`.
    pub path: ValuePath,
    /// The value as text; the plan's null text writes `$`.
    pub text: String,
}

impl TextEdit {
    /// An edit writing `text` at `path` of `entity`.
    pub fn new(entity: EntityId, path: ValuePath, text: impl Into<String>) -> Self {
        Self {
            entity,
            path,
            text: text.into(),
        }
    }
}

/// One value a plan changes.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct PlannedChange {
    /// The position of the edit in the batch.
    pub edit: usize,
    /// The entity the edit's path starts from.
    pub entity: EntityId,
    /// The attribute or property written, which a path such as
    /// `type.Name` finds on another entity.
    pub target: SetTarget,
    /// The value it holds now; a property the object inherits from its
    /// type reads as the inherited value.
    pub before: SelectedValue,
    /// The value it will hold.
    pub after: SelectedValue,
    /// The value as the file will hold it: typed where its declaration
    /// asks for a type.
    pub value: Value,
}

/// A planned batch: what changes, and the transaction that changes it.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct EditPlan {
    /// Every value that changes, in edit order.
    pub changes: Vec<PlannedChange>,
    /// The writes, staged against the model the plan read; empty when
    /// nothing changes.
    pub transaction: Transaction,
}

impl EditPlan {
    /// Whether the plan changes nothing.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.changes.is_empty()
    }

    /// Commit the plan as one transaction; `None` when it changes nothing,
    /// which leaves the model and its revision untouched.
    ///
    /// # Errors
    ///
    /// [`SelectorError::Conflict`] when the transaction fails its
    /// preflight, such as for a model that changed since the plan was
    /// made. The model is unchanged then.
    pub fn commit(self, model: &mut Model) -> Result<Option<Applied>, SelectorError> {
        if self.transaction.is_empty() {
            return Ok(None);
        }
        self.transaction
            .commit(model)
            .map(Some)
            .map_err(SelectorError::Conflict)
    }
}

/// Why one edit of a batch, or the batch, was refused.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct EditRefusal {
    /// The position of the refused edit; `None` when the refusal concerns
    /// the batch, such as a transaction that failed its preflight.
    pub edit: Option<usize>,
    /// Why.
    pub error: SelectorError,
}

impl fmt::Display for EditRefusal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.edit {
            Some(edit) => write!(f, "write {edit}: {}", self.error),
            None => write!(f, "{}", self.error),
        }
    }
}

impl std::error::Error for EditRefusal {}

/// A target's identity, for finding two writes to one.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(not(feature = "properties"), allow(dead_code))]
enum TargetKey {
    Attribute(EntityId, usize),
    Property(EntityId, String, String),
}

/// What the plan collects while it reads the edits.
#[derive(Default)]
struct Collected {
    refusals: Vec<EditRefusal>,
    changes: Vec<PlannedChange>,
    attributes: Vec<(EntityId, usize, Value)>,
    #[cfg(feature = "properties")]
    properties: Vec<(usize, crate::PropertyEdit)>,
    written: HashMap<TargetKey, (usize, Value)>,
}

impl Collected {
    fn refuse(&mut self, edit: usize, error: SelectorError) {
        self.refusals.push(EditRefusal {
            edit: Some(edit),
            error,
        });
    }

    /// Record a write to `key`; `false` when an earlier write wrote the
    /// same value (nothing more to stage) or another one (refused).
    fn claim(
        &mut self,
        edit: usize,
        key: TargetKey,
        value: &Value,
        change: &PlannedChange,
    ) -> bool {
        if let Some((first, earlier)) = self.written.get(&key) {
            if earlier != value {
                let (entity, target) = describe(&change.target);
                let first = *first;
                self.refuse(
                    edit,
                    SelectorError::ConflictingWrite {
                        entity,
                        target,
                        first,
                    },
                );
            }
            return false;
        }
        self.written.insert(key, (edit, value.clone()));
        true
    }
}

impl SelectorIndex<'_> {
    /// Plan `edits` as one transaction, reading each text against the
    /// type of the attribute or property it writes; `null` is the text
    /// that writes `$`.
    ///
    /// `seed` is the batch's seed: the `GlobalId`s of the property sets and
    /// relationships a property write creates derive from it (see
    /// `apply_property_edits`); an attribute write ignores it.
    ///
    /// # Errors
    ///
    /// Every refusal found, each naming its edit: the refusals of
    /// [`Self::value`] and [`Self::set_targets`] for the path,
    /// [`SelectorError::InvalidText`] for text that is no value of the
    /// target's type, [`SelectorError::NotSettable`] for a list or
    /// composite value, the checks of [`Self::stage_set_value`],
    /// [`SelectorError::ConflictingWrite`], and the property edit's
    /// refusals. Nothing is planned when any edit is refused.
    pub fn plan_text_edits(
        &self,
        edits: &[TextEdit],
        null: &str,
        seed: u128,
    ) -> Result<EditPlan, Vec<EditRefusal>> {
        let mut collected = Collected::default();
        for (index, edit) in edits.iter().enumerate() {
            if let Err(error) = self.plan_one(index, edit, null, &mut collected) {
                collected.refuse(index, error);
            }
        }
        let mut transaction = self.stage_properties_of(&mut collected, seed);
        for (entity, slot, value) in collected.attributes.drain(..) {
            transaction.set_attribute(entity, slot, value);
        }
        if !collected.refusals.is_empty() {
            collected.refusals.sort_by_key(|refusal| refusal.edit);
            return Err(collected.refusals);
        }
        Ok(EditPlan {
            changes: collected.changes,
            transaction,
        })
    }

    fn plan_one(
        &self,
        index: usize,
        edit: &TextEdit,
        null: &str,
        collected: &mut Collected,
    ) -> Result<(), SelectorError> {
        let before = self.value(edit.entity, &edit.path)?;
        if value_text(&before, null) == edit.text {
            return Ok(());
        }
        for target in self.set_targets(edit.entity, &edit.path)? {
            let planned = match &target {
                SetTarget::Attribute { entity, name } => {
                    self.plan_attribute(*entity, name, &edit.text, null)?
                }
                SetTarget::Property {
                    object,
                    set,
                    property,
                } => self.plan_property(*object, set, property, &edit.text, null)?,
            };
            let Some((key, before, value)) = planned else {
                continue;
            };
            let change = PlannedChange {
                edit: index,
                entity: edit.entity,
                target: target.clone(),
                before,
                after: SelectedValue::from_model(&value),
                value: value.clone(),
            };
            if !collected.claim(index, key.clone(), &value, &change) {
                continue;
            }
            match key {
                TargetKey::Attribute(entity, slot) => {
                    collected.attributes.push((entity, slot, value));
                }
                #[cfg(feature = "properties")]
                TargetKey::Property(object, set, property) => {
                    collected.properties.push((
                        index,
                        crate::PropertyEdit::set(object, set, property, value),
                    ));
                }
                #[cfg(not(feature = "properties"))]
                TargetKey::Property(..) => {
                    return Err(SelectorError::FeatureDisabled("properties"));
                }
            }
            collected.changes.push(change);
        }
        Ok(())
    }

    /// The write of `text` to attribute `name` of `entity`: `None` when it
    /// holds that value already.
    fn plan_attribute(
        &self,
        entity: EntityId,
        name: &str,
        text: &str,
        null: &str,
    ) -> Result<Option<(TargetKey, SelectedValue, Value)>, SelectorError> {
        let record = self.entity(entity)?;
        let (slot, declared) = attribute_type(self.schema, &record.type_name, name)?;
        let current = record.attributes.get(slot.index);
        let value = if text == null {
            Value::Null
        } else {
            parse_declared(self.schema, &declared, current, text).map_err(|expected| {
                SelectorError::InvalidText {
                    entity,
                    target: slot.name.to_owned(),
                    text: text.to_owned(),
                    expected,
                }
            })?
        };
        let index = self.checked_slot(entity, name, &value)?;
        let held = current.unwrap_or(&Value::Null);
        if *held == value {
            return Ok(None);
        }
        Ok(Some((
            TargetKey::Attribute(entity, index),
            SelectedValue::from_model(held),
            value,
        )))
    }

    /// The write of `text` to property `set.property` of `object`: `None`
    /// when it holds that value already.
    #[cfg(feature = "properties")]
    fn plan_property(
        &self,
        object: EntityId,
        set: &str,
        property: &str,
        text: &str,
        null: &str,
    ) -> Result<Option<(TargetKey, SelectedValue, Value)>, SelectorError> {
        use ifc_properties::{ExactPropertyError, ExactResolution};

        let (stated, before) = match self
            .properties()
            .exact_property(object, Some(set), property)
        {
            Ok(ExactResolution::Present(found)) => (
                found.value_type.clone(),
                super::exact::selected(object, &found.value)?,
            ),
            Ok(_) | Err(ExactPropertyError::InvalidQueryObject { .. }) => {
                (None, SelectedValue::Null)
            }
            Err(error) => {
                return Err(SelectorError::Property {
                    entity: Some(object),
                    error,
                })
            }
        };
        if matches!(
            before,
            SelectedValue::List(_) | SelectedValue::Properties(_)
        ) {
            return Err(SelectorError::NotSettable {
                path: format!("{set}.{property}"),
                reason: "a list, bounded or complex value is not written from text".to_owned(),
            });
        }
        let value = if text == null {
            Value::Null
        } else {
            super::text::parse_property(self.schema, stated.as_deref(), text).map_err(
                |expected| SelectorError::InvalidText {
                    entity: object,
                    target: format!("{set}.{property}"),
                    text: text.to_owned(),
                    expected,
                },
            )?
        };
        if SelectedValue::from_model(&value) == before {
            return Ok(None);
        }
        Ok(Some((
            TargetKey::Property(object, set.to_owned(), property.to_owned()),
            before,
            value,
        )))
    }

    #[cfg(not(feature = "properties"))]
    #[allow(clippy::unused_self)]
    fn plan_property(
        &self,
        _object: EntityId,
        _set: &str,
        _property: &str,
        _text: &str,
        _null: &str,
    ) -> Result<Option<(TargetKey, SelectedValue, Value)>, SelectorError> {
        Err(SelectorError::FeatureDisabled("properties"))
    }

    /// The property edits staged on a transaction; an edit the property
    /// planner refuses is recorded and left out, and the rest replanned,
    /// up to [`PROPERTY_REFUSALS`] times.
    #[cfg(feature = "properties")]
    fn stage_properties_of(&self, collected: &mut Collected, seed: u128) -> Transaction {
        let mut pending = std::mem::take(&mut collected.properties);
        let mut refused = 0;
        while !pending.is_empty() {
            let edits: Vec<crate::PropertyEdit> =
                pending.iter().map(|(_, edit)| edit.clone()).collect();
            match crate::stage_property_edits(self.model, &edits, seed) {
                Ok(staged) => return staged.transaction,
                Err(error) => {
                    refused += 1;
                    let at = error.edit;
                    let edit = at.and_then(|at| pending.get(at)).map(|(edit, _)| *edit);
                    collected.refusals.push(EditRefusal {
                        edit,
                        error: SelectorError::PropertyEdit(error),
                    });
                    match at {
                        Some(at) if at < pending.len() && refused < PROPERTY_REFUSALS => {
                            pending.remove(at);
                        }
                        _ => break,
                    }
                }
            }
        }
        Transaction::new(self.model)
    }

    #[cfg(not(feature = "properties"))]
    fn stage_properties_of(&self, _collected: &mut Collected, _seed: u128) -> Transaction {
        Transaction::new(self.model)
    }
}

/// The entity a target writes and its attribute or `Set.Property`.
fn describe(target: &SetTarget) -> (EntityId, String) {
    match target {
        SetTarget::Attribute { entity, name } => (*entity, name.clone()),
        SetTarget::Property {
            object,
            set,
            property,
        } => (*object, format!("{set}.{property}")),
    }
}

/// Plan `edits` against `model` and commit them as one transaction: all of
/// them or none. Returns what changed; an empty list leaves the model and
/// its revision untouched.
///
/// `seed` is the batch's seed: the `GlobalId`s of the property sets and
/// relationships a property write creates derive from it (see
/// `apply_property_edits`); an attribute write ignores it.
///
/// # Errors
///
/// As [`SelectorIndex::plan_text_edits`], and a batch refusal carrying
/// [`SelectorError::Conflict`] when the transaction fails its preflight.
/// The model is unchanged on every error.
pub fn apply_text_edits(
    model: &mut Model,
    schema: &Schema,
    edits: &[TextEdit],
    null: &str,
    seed: u128,
) -> Result<Vec<PlannedChange>, Vec<EditRefusal>> {
    let plan = SelectorIndex::new(model, schema).plan_text_edits(edits, null, seed)?;
    let changes = plan.changes.clone();
    plan.commit(model)
        .map_err(|error| vec![EditRefusal { edit: None, error }])?;
    Ok(changes)
}
