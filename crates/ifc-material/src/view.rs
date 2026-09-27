//! Shared borrowed-view mechanics and strict IFC slot decoders.

use ifc_model::{Entity, EntityId, Model, Value};

use crate::release::Release;
use crate::{LogicalValue, MaterialError, MaterialResult, SchemaVersion};

const MAX_TYPED_WRAPPERS: usize = 8;

/// Borrowed MaterialResource interpretation of a model.
///
/// Every projection the view hands out reads against the release the
/// model's header declares (see [`crate::material_schema`]): slot positions
/// come from that release's bundled table, never from IFC4 by assumption.
#[derive(Debug, Clone, Copy)]
pub struct MaterialView<'m> {
    model: &'m Model,
    release: Release<'m>,
}

impl<'m> MaterialView<'m> {
    /// Wraps a model reference for MaterialResource projection, bound to the
    /// release its header declares.
    pub fn new(model: &'m Model) -> Self {
        Self {
            model,
            release: Release::of(model),
        }
    }

    /// Returns the underlying model this view borrows from.
    pub fn model(self) -> &'m Model {
        self.model
    }

    /// The release this view's projections read against.
    ///
    /// # Errors
    ///
    /// [`MaterialError::MultipleSchemas`] or
    /// [`MaterialError::UnsupportedSchema`] when the header binds none.
    pub fn schema(self) -> MaterialResult<SchemaVersion> {
        self.release.bound().map(|(version, _)| version)
    }

    pub(crate) fn release(self) -> Release<'m> {
        self.release
    }

    pub(crate) fn entity(self, source: EntityId, target: EntityId) -> MaterialResult<&'m Entity> {
        self.model
            .get(target)
            .ok_or(MaterialError::DanglingReference {
                source_id: source,
                target,
            })
    }
}

macro_rules! borrowed_entity {
    ($name:ident, $ifc_name:literal) => {
        #[doc = concat!("Borrowed projection of `", $ifc_name, "`.")]
        #[doc = ""]
        #[doc = "Projections handed out by a [`crate::MaterialView`] read against the"]
        #[doc = "release the model declares; one built with `try_new` has no model and"]
        #[doc = "reads against IFC4, as in 0.2.0."]
        #[derive(Debug, Clone, Copy)]
        pub struct $name<'m> {
            id: ifc_model::EntityId,
            entity: &'m ifc_model::Entity,
            release: crate::release::Release<'m>,
        }

        impl<'m> $name<'m> {
            /// Wraps `entity` as this view after checking it is an instance
            /// of the expected IFC entity type; fails with
            /// [`crate::MaterialError::WrongEntityType`] otherwise.
            ///
            /// Without a model there is no header to bind, so the projection
            /// reads against IFC4. Use [`Self::try_from_view`] to read the
            /// release a model declares.
            pub fn try_new(
                id: ifc_model::EntityId,
                entity: &'m ifc_model::Entity,
            ) -> crate::MaterialResult<Self> {
                Self::check(id, entity, crate::release::Release::LEGACY)
            }

            /// Looks up `id` in the view's model and projects it against the
            /// release the model declares.
            ///
            /// # Errors
            ///
            /// [`crate::MaterialError::UnknownEntity`] when `id` is not in
            /// the model, and [`crate::MaterialError::WrongEntityType`] when
            /// it is another entity type.
            pub fn try_from_view(
                view: crate::MaterialView<'m>,
                id: ifc_model::EntityId,
            ) -> crate::MaterialResult<Self> {
                let entity = view
                    .model()
                    .get(id)
                    .ok_or(crate::MaterialError::UnknownEntity { id })?;
                Self::check(id, entity, view.release())
            }

            fn check(
                id: ifc_model::EntityId,
                entity: &'m ifc_model::Entity,
                release: crate::release::Release<'m>,
            ) -> crate::MaterialResult<Self> {
                if !entity.is_type($ifc_name) {
                    return Err(crate::MaterialError::WrongEntityType {
                        expected: $ifc_name,
                        actual: entity.type_name.to_string(),
                    });
                }
                Ok(Self {
                    id,
                    entity,
                    release,
                })
            }

            pub(crate) fn from_known(
                id: ifc_model::EntityId,
                entity: &'m ifc_model::Entity,
                release: crate::release::Release<'m>,
            ) -> Self {
                Self {
                    id,
                    entity,
                    release,
                }
            }

            /// The entity id of the wrapped record.
            pub fn id(self) -> ifc_model::EntityId {
                self.id
            }

            /// The raw, untyped entity record backing this view.
            pub fn entity(self) -> &'m ifc_model::Entity {
                self.entity
            }

            /// The release this projection reads against.
            ///
            /// # Errors
            ///
            /// [`crate::MaterialError::MultipleSchemas`] or
            /// [`crate::MaterialError::UnsupportedSchema`] when the model's
            /// header binds none.
            pub fn schema(self) -> crate::MaterialResult<crate::SchemaVersion> {
                self.release.bound().map(|(version, _)| version)
            }

            /// The release binding of this projection.
            #[allow(dead_code)]
            pub(crate) fn release(self) -> crate::release::Release<'m> {
                self.release
            }

            /// Slot of `attribute` (IFC4 name) on this record in the bound
            /// release; `NotInSchema` or `EntityNotInSchema` otherwise.
            #[allow(dead_code)]
            pub(crate) fn slot(self, attribute: &'static str) -> crate::MaterialResult<usize> {
                self.release.slot($ifc_name, self.id, attribute)
            }
        }
    };
}
pub(crate) use borrowed_entity;

fn kind(value: &Value) -> &'static str {
    match value {
        Value::Null => "null",
        Value::Derived => "derived marker",
        Value::Bool(_) => "boolean",
        Value::LogicalUnknown => "logical unknown",
        Value::Integer(_) => "integer",
        Value::Real(_) => "real",
        Value::Text(_) => "text",
        Value::Binary(_) => "binary",
        Value::Enum(_) => "enumeration",
        Value::Ref(_) => "reference",
        Value::List(_) => "aggregate",
        Value::Typed { .. } => "typed value",
    }
}

fn invalid(
    entity_type: &'static str,
    id: EntityId,
    attribute: &'static str,
    expected: &str,
    actual: &Value,
) -> MaterialError {
    MaterialError::InvalidValue {
        entity: entity_type,
        id,
        attribute,
        value: format!("expected {expected}, found {}", kind(actual)),
    }
}

fn optional_raw<'a>(
    entity_type: &'static str,
    id: EntityId,
    entity: &'a Entity,
    slot: usize,
    attribute: &'static str,
) -> MaterialResult<Option<&'a Value>> {
    match entity.attribute(slot) {
        None | Some(Value::Null) => Ok(None),
        Some(Value::Derived) => Err(invalid(
            entity_type,
            id,
            attribute,
            "an explicit value or $",
            &Value::Derived,
        )),
        Some(value) => Ok(Some(value)),
    }
}

fn required_raw<'a>(
    entity_type: &'static str,
    id: EntityId,
    entity: &'a Entity,
    slot: usize,
    attribute: &'static str,
) -> MaterialResult<&'a Value> {
    optional_raw(entity_type, id, entity, slot, attribute)?.ok_or(MaterialError::MissingAttribute {
        entity: entity_type,
        id,
        attribute,
    })
}

fn unwrap_scalar<'a>(
    entity_type: &'static str,
    id: EntityId,
    attribute: &'static str,
    value: &'a Value,
) -> MaterialResult<&'a Value> {
    let mut current = value;
    for _ in 0..MAX_TYPED_WRAPPERS {
        match current {
            Value::Typed { value, .. } => current = value,
            _ => return Ok(current),
        }
    }
    Err(MaterialError::InvalidValue {
        entity: entity_type,
        id,
        attribute,
        value: format!("typed-wrapper nesting exceeds {MAX_TYPED_WRAPPERS}"),
    })
}

pub(crate) fn required_text<'a>(
    entity_type: &'static str,
    id: EntityId,
    entity: &'a Entity,
    slot: usize,
    attribute: &'static str,
) -> MaterialResult<&'a str> {
    let raw = required_raw(entity_type, id, entity, slot, attribute)?;
    let value = unwrap_scalar(entity_type, id, attribute, raw)?;
    match value {
        Value::Text(value) => Ok(value),
        _ => Err(invalid(entity_type, id, attribute, "text", value)),
    }
}

pub(crate) fn optional_text<'a>(
    entity_type: &'static str,
    id: EntityId,
    entity: &'a Entity,
    slot: usize,
    attribute: &'static str,
) -> MaterialResult<Option<&'a str>> {
    let Some(raw) = optional_raw(entity_type, id, entity, slot, attribute)? else {
        return Ok(None);
    };
    let value = unwrap_scalar(entity_type, id, attribute, raw)?;
    match value {
        Value::Text(value) => Ok(Some(value)),
        _ => Err(invalid(entity_type, id, attribute, "text or $", value)),
    }
}

pub(crate) fn required_ref(
    entity_type: &'static str,
    id: EntityId,
    entity: &Entity,
    slot: usize,
    attribute: &'static str,
) -> MaterialResult<EntityId> {
    let value = required_raw(entity_type, id, entity, slot, attribute)?;
    match value {
        Value::Ref(target) => Ok(*target),
        _ => Err(invalid(entity_type, id, attribute, "reference", value)),
    }
}

pub(crate) fn optional_ref(
    entity_type: &'static str,
    id: EntityId,
    entity: &Entity,
    slot: usize,
    attribute: &'static str,
) -> MaterialResult<Option<EntityId>> {
    let Some(value) = optional_raw(entity_type, id, entity, slot, attribute)? else {
        return Ok(None);
    };
    match value {
        Value::Ref(target) => Ok(Some(*target)),
        _ => Err(invalid(entity_type, id, attribute, "reference or $", value)),
    }
}

fn refs_from_aggregate(
    entity_type: &'static str,
    id: EntityId,
    attribute: &'static str,
    value: &Value,
    minimum: usize,
) -> MaterialResult<Vec<EntityId>> {
    let Value::List(items) = value else {
        return Err(invalid(
            entity_type,
            id,
            attribute,
            "an immediate aggregate",
            value,
        ));
    };
    if items.len() < minimum {
        return Err(MaterialError::InvalidValue {
            entity: entity_type,
            id,
            attribute,
            value: format!("expected at least {minimum} item(s), found {}", items.len()),
        });
    }
    items
        .iter()
        .map(|item| match item {
            Value::Ref(target) => Ok(*target),
            _ => Err(invalid(
                entity_type,
                id,
                attribute,
                "an aggregate of direct references",
                item,
            )),
        })
        .collect()
}

pub(crate) fn required_refs(
    entity_type: &'static str,
    id: EntityId,
    entity: &Entity,
    slot: usize,
    attribute: &'static str,
    minimum: usize,
) -> MaterialResult<Vec<EntityId>> {
    refs_from_aggregate(
        entity_type,
        id,
        attribute,
        required_raw(entity_type, id, entity, slot, attribute)?,
        minimum,
    )
}

pub(crate) fn optional_refs(
    entity_type: &'static str,
    id: EntityId,
    entity: &Entity,
    slot: usize,
    attribute: &'static str,
    minimum: usize,
) -> MaterialResult<Option<Vec<EntityId>>> {
    let Some(value) = optional_raw(entity_type, id, entity, slot, attribute)? else {
        return Ok(None);
    };
    refs_from_aggregate(entity_type, id, attribute, value, minimum).map(Some)
}

fn number_value(
    entity_type: &'static str,
    id: EntityId,
    attribute: &'static str,
    raw: &Value,
) -> MaterialResult<f64> {
    let value = unwrap_scalar(entity_type, id, attribute, raw)?;
    let number = match value {
        Value::Real(value) => *value,
        Value::Integer(value) => *value as f64,
        _ => return Err(invalid(entity_type, id, attribute, "a number", value)),
    };
    if number.is_finite() {
        Ok(number)
    } else {
        Err(MaterialError::InvalidValue {
            entity: entity_type,
            id,
            attribute,
            value: "number must be finite".to_owned(),
        })
    }
}

pub(crate) fn required_number(
    entity_type: &'static str,
    id: EntityId,
    entity: &Entity,
    slot: usize,
    attribute: &'static str,
) -> MaterialResult<f64> {
    number_value(
        entity_type,
        id,
        attribute,
        required_raw(entity_type, id, entity, slot, attribute)?,
    )
}

pub(crate) fn optional_number(
    entity_type: &'static str,
    id: EntityId,
    entity: &Entity,
    slot: usize,
    attribute: &'static str,
) -> MaterialResult<Option<f64>> {
    let Some(value) = optional_raw(entity_type, id, entity, slot, attribute)? else {
        return Ok(None);
    };
    number_value(entity_type, id, attribute, value).map(Some)
}

fn integer_value(
    entity_type: &'static str,
    id: EntityId,
    attribute: &'static str,
    raw: &Value,
) -> MaterialResult<i64> {
    let value = unwrap_scalar(entity_type, id, attribute, raw)?;
    match value {
        Value::Integer(value) => Ok(*value),
        _ => Err(invalid(entity_type, id, attribute, "an integer", value)),
    }
}

pub(crate) fn optional_integer(
    entity_type: &'static str,
    id: EntityId,
    entity: &Entity,
    slot: usize,
    attribute: &'static str,
) -> MaterialResult<Option<i64>> {
    let Some(value) = optional_raw(entity_type, id, entity, slot, attribute)? else {
        return Ok(None);
    };
    integer_value(entity_type, id, attribute, value).map(Some)
}

pub(crate) fn required_enum<'a>(
    entity_type: &'static str,
    id: EntityId,
    entity: &'a Entity,
    slot: usize,
    attribute: &'static str,
) -> MaterialResult<&'a str> {
    let raw = required_raw(entity_type, id, entity, slot, attribute)?;
    let value = unwrap_scalar(entity_type, id, attribute, raw)?;
    match value {
        Value::Enum(token) => Ok(token),
        _ => Err(invalid(entity_type, id, attribute, "an enumeration", value)),
    }
}

pub(crate) fn optional_logical(
    entity_type: &'static str,
    id: EntityId,
    entity: &Entity,
    slot: usize,
    attribute: &'static str,
) -> MaterialResult<Option<LogicalValue>> {
    let Some(raw) = optional_raw(entity_type, id, entity, slot, attribute)? else {
        return Ok(None);
    };
    let value = unwrap_scalar(entity_type, id, attribute, raw)?;
    match value {
        Value::Bool(false) => Ok(Some(LogicalValue::False)),
        Value::Bool(true) => Ok(Some(LogicalValue::True)),
        Value::LogicalUnknown => Ok(Some(LogicalValue::Unknown)),
        _ => Err(invalid(
            entity_type,
            id,
            attribute,
            "a logical value or $",
            value,
        )),
    }
}

pub(crate) fn required_number_array_2(
    entity_type: &'static str,
    id: EntityId,
    entity: &Entity,
    slot: usize,
    attribute: &'static str,
) -> MaterialResult<[f64; 2]> {
    let value = required_raw(entity_type, id, entity, slot, attribute)?;
    let Value::List(items) = value else {
        return Err(invalid(
            entity_type,
            id,
            attribute,
            "an ARRAY [1:2] of two numbers",
            value,
        ));
    };
    if items.len() != 2 {
        return Err(MaterialError::InvalidValue {
            entity: entity_type,
            id,
            attribute,
            value: format!("expected 2 values, found {}", items.len()),
        });
    }
    Ok([
        number_value(entity_type, id, attribute, &items[0])?,
        number_value(entity_type, id, attribute, &items[1])?,
    ])
}
