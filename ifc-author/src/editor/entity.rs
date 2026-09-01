//! Schema-checked updates of existing IFC entities.

use ifc_model::{Entity, EntityId, Model, Transaction, Value};
use ifc_schema::Schema;

use crate::builder::check_value;
use crate::error::{AuthorError, AuthorResult};

/// A schema-checked edit of one entity already present in a [`Model`].
///
/// Named edits are resolved against the entity's declaration and the entire
/// projected entity is validated before any write is staged. Reference
/// integrity, stale revisions, and atomic commit remain [`Transaction`]'s
/// responsibility.
#[derive(Debug, Clone)]
pub struct EntityEditor<'a> {
    schema: &'a Schema,
    id: EntityId,
    entity: Entity,
    set: Vec<(String, Value)>,
}

impl<'a> EntityEditor<'a> {
    /// Start editing `id` against the model snapshot used to open a transaction.
    ///
    /// # Errors
    ///
    /// Returns [`AuthorError::MissingEntity`] when `id` is absent. A caller
    /// should create its [`Transaction`] from the same model snapshot.
    pub fn new(schema: &'a Schema, model: &Model, id: EntityId) -> AuthorResult<Self> {
        let entity = model
            .get(id)
            .cloned()
            .ok_or(AuthorError::MissingEntity { id })?;
        Ok(Self {
            schema,
            id,
            entity,
            set: Vec::new(),
        })
    }

    /// Replace one attribute by its declared name.
    #[must_use]
    pub fn set(mut self, attribute: impl Into<String>, value: Value) -> Self {
        self.set.push((attribute.into(), value));
        self
    }

    /// Replace a text attribute.
    #[must_use]
    pub fn text(self, attribute: impl Into<String>, text: impl Into<std::sync::Arc<str>>) -> Self {
        self.set(attribute, Value::Text(text.into()))
    }

    /// Replace a real-valued attribute.
    #[must_use]
    pub fn real(self, attribute: impl Into<String>, value: f64) -> Self {
        self.set(attribute, Value::Real(value))
    }

    /// Replace an entity-reference attribute.
    #[must_use]
    pub fn reference(self, attribute: impl Into<String>, id: EntityId) -> Self {
        self.set(attribute, Value::Ref(id))
    }

    /// Replace an enumeration attribute.
    #[must_use]
    pub fn enumeration(
        self,
        attribute: impl Into<String>,
        constant: impl Into<std::sync::Arc<str>>,
    ) -> Self {
        self.set(attribute, Value::Enum(constant.into()))
    }

    /// Validate the projected entity and stage its changed slots.
    ///
    /// Validation completes before the first call to [`Transaction::set_attribute`],
    /// so any [`AuthorError`] leaves `transaction` unchanged.
    pub fn stage(self, transaction: &mut Transaction) -> AuthorResult<()> {
        let entity_name = self.entity.type_name.to_string();
        let declared = self.schema.attributes(&entity_name);
        if declared.is_empty() && self.schema.entity(&entity_name).is_none() {
            return Err(AuthorError::UnknownEntity {
                entity: entity_name,
            });
        }
        if self.entity.attributes.len() != declared.len() {
            return Err(AuthorError::ArityMismatch {
                entity: entity_name,
                expected: declared.len(),
                found: self.entity.attributes.len(),
            });
        }

        let mut projected = self.entity.attributes.clone();
        let mut edited = vec![false; declared.len()];
        let mut staged = Vec::with_capacity(self.set.len());
        for (name, value) in self.set {
            let Some(index) = declared
                .iter()
                .position(|attribute| attribute.name.eq_ignore_ascii_case(&name))
            else {
                return Err(AuthorError::UnknownAttribute {
                    entity: entity_name,
                    attribute: name,
                    known: declared
                        .iter()
                        .map(|attribute| attribute.name.clone())
                        .collect(),
                });
            };
            if edited[index] {
                return Err(AuthorError::DuplicateAttribute {
                    entity: entity_name,
                    attribute: declared[index].name.clone(),
                });
            }
            edited[index] = true;
            projected[index] = value.clone();
            staged.push((index, value));
        }

        for (index, attribute) in declared.iter().enumerate() {
            let value = &projected[index];
            if !attribute.optional && matches!(value, Value::Null) {
                return Err(AuthorError::MissingRequired {
                    entity: entity_name,
                    attribute: attribute.name.clone(),
                });
            }
            check_value(self.schema, &entity_name, attribute, value)?;
        }

        for (slot, value) in staged {
            if self.entity.attributes[slot] != value {
                transaction.set_attribute(self.id, slot, value);
            }
        }
        Ok(())
    }
}
