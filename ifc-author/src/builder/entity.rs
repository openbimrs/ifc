//! Build one entity by naming its attributes.
//!
//! # The problem this solves
//!
//! `Model::push` takes a positional `Vec<Value>`. Authoring an `IfcAnnotation`
//! therefore means knowing that it has seven slots, that slot 0 is the
//! GlobalId, slot 2 the name, and that slots 1 and 3..6 are optional. Get the
//! count wrong and the file still writes -- and is rejected downstream.
//!
//! Here the caller names attributes and the schema decides the positions.

use ifc_model::{Entity, Value};
use ifc_schema::Schema;

use crate::check::{describe_value, value_matches};
use crate::error::{AuthorError, AuthorResult};

/// A partially-specified entity, checked against the schema on [`build`].
///
/// [`build`]: EntityBuilder::build
#[derive(Debug, Clone)]
pub struct EntityBuilder<'a> {
    schema: &'a Schema,
    entity: String,
    /// Attribute name and value, in the order the caller set them. Kept as a
    /// list rather than a map so a duplicate set is detectable and the error
    /// can name the attribute.
    set: Vec<(String, Value)>,
}

impl<'a> EntityBuilder<'a> {
    /// Start building `entity`, whose type must be declared by `schema`.
    ///
    /// The type is not checked here: an unknown type is reported by [`build`],
    /// so that a caller assembling several entities gets every failure at the
    /// same point in its code rather than some at construction and some later.
    ///
    /// [`build`]: EntityBuilder::build
    pub fn new(schema: &'a Schema, entity: impl Into<String>) -> Self {
        Self {
            schema,
            entity: entity.into(),
            set: Vec::new(),
        }
    }

    /// Set one attribute by its declared name.
    ///
    /// Names are matched case-insensitively, because EXPRESS declares
    /// `GlobalId` while STEP files and much prose write `GLOBALID`.
    #[must_use]
    pub fn set(mut self, attribute: impl Into<String>, value: Value) -> Self {
        self.set.push((attribute.into(), value));
        self
    }

    /// Set a text attribute.
    pub fn text(self, attribute: impl Into<String>, text: impl Into<std::sync::Arc<str>>) -> Self {
        self.set(attribute, Value::Text(text.into()))
    }

    /// Set a real-valued attribute.
    pub fn real(self, attribute: impl Into<String>, value: f64) -> Self {
        self.set(attribute, Value::Real(value))
    }

    /// Set an entity-reference attribute.
    pub fn reference(self, attribute: impl Into<String>, id: ifc_model::EntityId) -> Self {
        self.set(attribute, Value::Ref(id))
    }

    /// Set an enumeration attribute (written `.PLAN_VIEW.` in STEP).
    pub fn enumeration(
        self,
        attribute: impl Into<String>,
        constant: impl Into<std::sync::Arc<str>>,
    ) -> Self {
        self.set(attribute, Value::Enum(constant.into()))
    }
}

impl EntityBuilder<'_> {
    /// Resolve every named attribute to its STEP slot and produce the entity.
    ///
    /// # Errors
    ///
    /// Refuses an unknown entity type, an attribute the schema does not
    /// declare, an attribute set twice, a required attribute left unset, a
    /// scalar/aggregate confusion, a value whose shape contradicts the declared
    /// type, and a malformed GlobalId.
    pub fn build(self) -> AuthorResult<Entity> {
        let declared = self.schema.attributes(&self.entity);
        if declared.is_empty() && self.schema.entity(&self.entity).is_none() {
            return Err(AuthorError::UnknownEntity {
                entity: self.entity,
            });
        }

        // Positional order comes from the schema: inherited attributes first,
        // which is what makes a STEP record readable by anything else.
        let mut slots = vec![Value::Null; declared.len()];
        let mut filled = vec![false; declared.len()];

        for (name, value) in &self.set {
            let Some(index) = declared
                .iter()
                .position(|a| a.name.eq_ignore_ascii_case(name))
            else {
                return Err(AuthorError::UnknownAttribute {
                    entity: self.entity.clone(),
                    attribute: name.clone(),
                    known: declared.iter().map(|a| a.name.clone()).collect(),
                });
            };
            if filled[index] {
                return Err(AuthorError::DuplicateAttribute {
                    entity: self.entity.clone(),
                    attribute: declared[index].name.clone(),
                });
            }
            check_value(self.schema, &self.entity, declared[index], value)?;
            slots[index] = value.clone();
            filled[index] = true;
        }

        for (index, attribute) in declared.iter().enumerate() {
            if !filled[index] && !attribute.optional {
                return Err(AuthorError::MissingRequired {
                    entity: self.entity.clone(),
                    attribute: attribute.name.clone(),
                });
            }
        }

        Ok(Entity::new(self.entity.to_ascii_uppercase(), slots))
    }

    /// Build the entity and append it to `model`, returning its new id.
    ///
    /// The model is left untouched when construction fails, so a rejected
    /// entity cannot leave a half-written record behind.
    ///
    /// # Errors
    ///
    /// Any failure from [`build`](EntityBuilder::build).
    pub fn insert(self, model: &mut ifc_model::Model) -> AuthorResult<ifc_model::EntityId> {
        let entity = self.build()?;
        Ok(model.push(entity))
    }
}

/// Check one value against one declared attribute.
///
/// Split out as a free function because it borrows the schema and the entity
/// name while the builder's `set` list is being consumed.
fn check_value(
    schema: &Schema,
    entity: &str,
    attribute: &ifc_schema::Attribute,
    value: &Value,
) -> AuthorResult<()> {
    // An aggregate declaration wants a list and a scalar declaration does not.
    // `$` is exempt: an unset optional aggregate is still `$`, not `()`.
    if !matches!(value, Value::Null | Value::Derived) {
        let supplied_aggregate = matches!(value, Value::List(_));
        if supplied_aggregate != attribute.aggregate {
            return Err(AuthorError::AggregateMismatch {
                entity: entity.to_owned(),
                attribute: attribute.name.clone(),
                expected_aggregate: attribute.aggregate,
            });
        }
    }

    // GlobalId is the one attribute whose *content* is worth checking here: it
    // is the only stable cross-file identity an element has, and a malformed
    // one silently breaks diffing and issue tracking rather than failing loudly.
    if attribute.name.eq_ignore_ascii_case("GlobalId") {
        if let Value::Text(text) = value {
            if ifc_model::guid::Guid::parse(text).is_none() {
                return Err(AuthorError::InvalidGlobalId {
                    entity: entity.to_owned(),
                    found: text.to_string(),
                });
            }
        }
    }

    // Aggregate element types are checked per item; the declaration names the
    // element type, not the container.
    let admissible = match value {
        Value::List(items) => items
            .iter()
            .all(|item| value_matches(schema, &attribute.type_name, item)),
        scalar => value_matches(schema, &attribute.type_name, scalar),
    };
    if !admissible {
        return Err(AuthorError::TypeMismatch {
            entity: entity.to_owned(),
            attribute: attribute.name.clone(),
            expected: attribute.type_name.clone(),
            found: describe_value(value),
        });
    }
    Ok(())
}
