//! Writing a value by path: [`SelectorIndex::set_targets`] and
//! [`SelectorIndex::stage_set_value`].
//!
//! A path ending in an attribute name writes that attribute of every
//! entity the keys before it reach (`Name`, `type.Name`,
//! `materials.Category`); one ending in a set and a property name writes
//! that property (`Pset_WallCommon.FireRating`,
//! `type.Pset_WallCommon.FireRating`) through the checked property edit.
//! Every write is staged on one transaction, and every check runs first:
//! a refused write changes nothing.

use ifc_model::{EntityId, Model, Transaction, Value};

use super::ast::{Key, ValuePath};
use super::error::SelectorError;
use super::index::SelectorIndex;
use super::path::KEYWORDS;
use super::value::SelectedValue;
use crate::declared_type::{attribute_type, DeclaredType, SimpleType};
use crate::named_attribute::NamedAttributeError;

/// What a path writes.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum SetTarget {
    /// An explicit attribute of an entity.
    Attribute {
        /// The entity.
        entity: EntityId,
        /// The attribute, as the release spells it.
        name: String,
    },
    /// A property or quantity of an object's own set, created when the
    /// object states none.
    Property {
        /// The occurrence or type object.
        object: EntityId,
        /// The set name.
        set: String,
        /// The property or quantity name.
        property: String,
    },
}

impl SelectorIndex<'_> {
    /// What `path` would write on `entity`.
    ///
    /// # Errors
    ///
    /// [`SelectorError::NotSettable`] for a pattern, a computed key or a
    /// prefix that reaches no entity; [`SelectorError::Attribute`] for an
    /// attribute the entity's type does not declare; refusals met while
    /// following the prefix.
    pub fn set_targets(
        &self,
        entity: EntityId,
        path: &ValuePath,
    ) -> Result<Vec<SetTarget>, SelectorError> {
        self.entity(entity)?;
        let not_settable = |reason: &str| SelectorError::NotSettable {
            path: path.to_string(),
            reason: reason.to_owned(),
        };
        let mut names = Vec::with_capacity(path.keys.len());
        for key in &path.keys {
            match &key.key {
                Key::Text(text) => names.push(text.as_str()),
                Key::Regex(_) => return Err(not_settable("a pattern names no single value")),
            }
        }
        let last = *names.last().ok_or_else(|| not_settable("an empty path"))?;
        if KEYWORDS.contains(&last) {
            return Err(not_settable(
                "a relation or computed key holds no value of its own",
            ));
        }
        // `prefix.Set.Property`: the set key is no keyword and no
        // attribute of the entities the prefix reaches.
        if names.len() >= 2 {
            let set = names[names.len() - 2];
            if !KEYWORDS.contains(&set) {
                let objects = self.reach(entity, &names[..names.len() - 2], path)?;
                let attribute = objects.iter().any(|object| {
                    self.model
                        .get(*object)
                        .is_some_and(|record| self.slot(&record.type_name, set).is_some())
                });
                if !attribute {
                    return Ok(objects
                        .into_iter()
                        .map(|object| SetTarget::Property {
                            object,
                            set: set.to_owned(),
                            property: last.to_owned(),
                        })
                        .collect());
                }
            }
        }
        let mut targets = Vec::new();
        for target in self.reach(entity, &names[..names.len() - 1], path)? {
            let record = self.entity(target)?;
            let slot = crate::attribute_slot(self.schema, &record.type_name, last)?;
            targets.push(SetTarget::Attribute {
                entity: target,
                name: slot.name.to_owned(),
            });
        }
        Ok(targets)
    }

    /// The entities `keys` lead to from `entity`.
    fn reach(
        &self,
        entity: EntityId,
        keys: &[&str],
        path: &ValuePath,
    ) -> Result<Vec<EntityId>, SelectorError> {
        let mut current = SelectedValue::Entity(entity);
        for key in keys {
            current = self.step(&current, &Key::Text((*key).to_owned()))?;
        }
        let reached = match current {
            SelectedValue::Entity(id) => vec![id],
            SelectedValue::List(items) if !items.is_empty() => items
                .into_iter()
                .map(|item| match item {
                    SelectedValue::Entity(id) => Some(id),
                    _ => None,
                })
                .collect::<Option<Vec<_>>>()
                .unwrap_or_default(),
            _ => Vec::new(),
        };
        if reached.is_empty() {
            return Err(SelectorError::NotSettable {
                path: path.to_string(),
                reason: format!("{:?} reaches no entity", keys.join(".")),
            });
        }
        Ok(reached)
    }

    /// Stage writing `value` at `path` of `entity` on one transaction,
    /// without touching the model.
    ///
    /// An attribute takes `value` as the file would hold it, checked
    /// against the attribute's declared type in the release: bare for a
    /// defined type (`Value::Text` for an `IfcLabel`), typed in a SELECT
    /// (`IFCLABEL('x')`), an enumeration item by name, `$` only where the
    /// attribute is optional. A property takes a typed value; a bare one is
    /// wrapped in the type of the property already stated, and refused
    /// when none is.
    ///
    /// # Errors
    ///
    /// As [`Self::set_targets`]; [`SelectorError::TypeMismatch`];
    /// [`SelectorError::Attribute`] for a derived attribute;
    /// [`SelectorError::UntypedValue`];
    /// [`SelectorError::PropertyEdit`] when the property edit refuses.
    pub fn stage_set_value(
        &self,
        entity: EntityId,
        path: &ValuePath,
        value: &Value,
    ) -> Result<Transaction, SelectorError> {
        let targets = self.set_targets(entity, path)?;
        let mut transaction = Transaction::new(self.model);
        let mut edits = Vec::new();
        for target in targets {
            match target {
                SetTarget::Attribute { entity, name } => {
                    let index = self.checked_slot(entity, &name, value)?;
                    transaction.set_attribute(entity, index, value.clone());
                }
                SetTarget::Property {
                    object,
                    set,
                    property,
                } => edits.push((object, set, property)),
            }
        }
        if edits.is_empty() {
            return Ok(transaction);
        }
        self.stage_properties(edits, value)
    }

    /// The slot `value` may be written to, after every check.
    fn checked_slot(
        &self,
        entity: EntityId,
        name: &str,
        value: &Value,
    ) -> Result<usize, SelectorError> {
        let record = self.entity(entity)?;
        let (slot, declared) = attribute_type(self.schema, &record.type_name, name)?;
        if slot.derived {
            return Err(NamedAttributeError::DerivedAttribute {
                type_name: self
                    .schema
                    .entity(&record.type_name)
                    .map_or(record.type_name.to_string(), |definition| {
                        definition.name.clone()
                    }),
                name: slot.name.to_owned(),
            }
            .into());
        }
        let mismatch = |detail: String| SelectorError::TypeMismatch {
            entity,
            attribute: slot.name.to_owned(),
            detail,
        };
        if *value == Value::Null {
            if !slot.optional {
                return Err(mismatch("the attribute is not optional".to_owned()));
            }
        } else {
            conforms(self.model, self.schema, &declared, value, 0).map_err(mismatch)?;
        }
        Ok(slot.index)
    }

    #[cfg(feature = "properties")]
    fn stage_properties(
        &self,
        edits: Vec<(EntityId, String, String)>,
        value: &Value,
    ) -> Result<Transaction, SelectorError> {
        use ifc_properties::{ExactPropertyError, ExactResolution};

        let mut staged = Vec::with_capacity(edits.len());
        for (object, set, property) in edits {
            let typed = match value {
                Value::Typed { .. } | Value::Null | Value::List(_) => value.clone(),
                bare => {
                    let stated =
                        match self
                            .properties()
                            .exact_property(object, Some(&set), &property)
                        {
                            Ok(ExactResolution::Present(found)) => found.value_type,
                            Ok(_) | Err(ExactPropertyError::InvalidQueryObject { .. }) => None,
                            Err(error) => {
                                return Err(SelectorError::Property {
                                    entity: Some(object),
                                    error,
                                })
                            }
                        };
                    let Some(type_name) = stated else {
                        return Err(SelectorError::UntypedValue {
                            object,
                            set,
                            property,
                        });
                    };
                    Value::Typed {
                        type_name,
                        value: Box::new(bare.clone()),
                    }
                }
            };
            staged.push(crate::PropertyEdit::set(object, set, property, typed));
        }
        crate::stage_property_edits(self.model, &staged)
            .map(|staged| staged.transaction)
            .map_err(SelectorError::PropertyEdit)
    }

    #[cfg(not(feature = "properties"))]
    #[allow(clippy::unused_self, clippy::needless_pass_by_value)]
    fn stage_properties(
        &self,
        _edits: Vec<(EntityId, String, String)>,
        _value: &Value,
    ) -> Result<Transaction, SelectorError> {
        Err(SelectorError::FeatureDisabled("properties"))
    }
}

/// Write `value` at `path` of `entity` and commit, as one transaction.
///
/// # Errors
///
/// As [`SelectorIndex::stage_set_value`], and
/// [`SelectorError::Conflict`] when the transaction fails its preflight.
/// The model is unchanged on every error.
pub fn set_value(
    model: &mut Model,
    schema: &ifc_schema::Schema,
    entity: EntityId,
    path: &ValuePath,
    value: &Value,
) -> Result<ifc_model::Applied, SelectorError> {
    let transaction = SelectorIndex::new(model, schema).stage_set_value(entity, path, value)?;
    transaction.commit(model).map_err(SelectorError::Conflict)
}

/// Whether `value` fits `declared`; the reason when it does not.
fn conforms(
    model: &Model,
    schema: &ifc_schema::Schema,
    declared: &DeclaredType<'_>,
    value: &Value,
    depth: usize,
) -> Result<(), String> {
    if depth > 32 {
        return Err("the declared type nests too deeply".to_owned());
    }
    let refuse = || Err(format!("{value:?} does not fit {}", describe(declared)));
    match (declared, value) {
        (DeclaredType::Defined { underlying, .. }, _) => {
            conforms(model, schema, underlying, value, depth + 1)
        }
        (DeclaredType::Simple(simple), _) => {
            let fits = matches!(
                (simple, value),
                (SimpleType::String, Value::Text(_))
                    | (SimpleType::Integer, Value::Integer(_))
                    | (SimpleType::Real, Value::Real(_))
                    | (SimpleType::Number, Value::Integer(_) | Value::Real(_))
                    | (SimpleType::Boolean, Value::Bool(_))
                    | (SimpleType::Logical, Value::Bool(_) | Value::LogicalUnknown)
                    | (SimpleType::Binary, Value::Binary(_))
            );
            if fits {
                Ok(())
            } else {
                refuse()
            }
        }
        (DeclaredType::Enumeration { items, .. }, Value::Enum(item)) => {
            if items
                .iter()
                .any(|declared| declared.eq_ignore_ascii_case(item))
            {
                Ok(())
            } else {
                refuse()
            }
        }
        (DeclaredType::Entity(name), Value::Ref(id)) => match model.get(*id) {
            None => Err(format!("#{} is not in the model", id.0)),
            Some(target) if schema.is_a(&target.type_name, name) => Ok(()),
            Some(target) => Err(format!(
                "#{} is an {}, not an {name}",
                id.0, target.type_name
            )),
        },
        (DeclaredType::Select { members, .. }, _) => {
            let accepted = members.iter().any(|member| match (member, value) {
                (DeclaredType::Entity(_), Value::Ref(_)) | (DeclaredType::Select { .. }, _) => {
                    conforms(model, schema, member, value, depth + 1).is_ok()
                }
                (DeclaredType::Defined { name, underlying }, Value::Typed { type_name, value }) => {
                    name.eq_ignore_ascii_case(type_name)
                        && conforms(model, schema, underlying, value, depth + 1).is_ok()
                }
                (DeclaredType::Enumeration { name, .. }, Value::Typed { type_name, value }) => {
                    name.eq_ignore_ascii_case(type_name)
                        && conforms(model, schema, member, value, depth + 1).is_ok()
                }
                _ => false,
            });
            if accepted {
                Ok(())
            } else {
                refuse()
            }
        }
        (DeclaredType::Aggregate { element, .. }, Value::List(items)) => items
            .iter()
            .try_for_each(|item| conforms(model, schema, element, item, depth + 1)),
        (DeclaredType::Unresolved(token), _) => Err(format!(
            "the release does not resolve the declared type {token}"
        )),
        _ => refuse(),
    }
}

fn describe(declared: &DeclaredType<'_>) -> String {
    match declared {
        DeclaredType::Simple(simple) => simple.keyword().to_owned(),
        DeclaredType::Aggregate { element, .. } => format!("an aggregate of {}", describe(element)),
        other => other.name().unwrap_or("its declared type").to_owned(),
    }
}
