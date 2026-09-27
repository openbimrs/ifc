//! The IFC release quantity authoring writes against.
//!
//! Every slot comes from the bundled table of the model's declared release,
//! looked up by attribute name, so an IFC2X3 quantity is never written with
//! the IFC4 layout:
//!
//! ```text
//! IfcQuantity<Kind>   IFC2X3   Name, Description, Unit, <Kind>Value
//!                     IFC4     Name, Description, Unit, <Kind>Value, Formula
//!                     IFC4X3   as IFC4, plus IfcQuantityNumber (NumberValue)
//! IfcCountMeasure     IFC2X3, IFC4 = NUMBER;  IFC4X3 = INTEGER
//! ```
//!
//! Binding, from `FILE_SCHEMA`, as `ifc-material` and `ifc-classification`
//! bind their authoring:
//! - one recognised declaration binds that release's table;
//! - one unrecognised declaration fails with
//!   [`PropertyError::UnsupportedSchema`];
//! - several fail with [`PropertyError::MultipleSchemas`];
//! - none at all (an in-memory [`Model::new`]) binds IFC4.

use ifc_model::{Entity, Model, Value};
use ifc_schema::{for_version, Schema, SchemaVersion};

use crate::error::{PropertyError, PropertyResult};
use crate::quantity::set::QuantityKind;

/// The release a quantity edit is written in.
#[derive(Debug, Clone, Copy)]
pub(super) struct Layout {
    version: SchemaVersion,
    schema: &'static Schema,
}

/// Bind `model`'s declared release.
pub(super) fn bind(model: &Model) -> PropertyResult<Layout> {
    let version = match model.header().schema.as_slice() {
        [] => SchemaVersion::Ifc4,
        [token] => SchemaVersion::from_header_token(token).ok_or_else(|| {
            PropertyError::UnsupportedSchema {
                schema: token.clone(),
            }
        })?,
        tokens => {
            return Err(PropertyError::MultipleSchemas {
                schemas: tokens.len(),
            })
        }
    };
    let schema = for_version(version).ok_or_else(|| PropertyError::UnsupportedSchema {
        schema: format!("{version:?}"),
    })?;
    Ok(Layout { version, schema })
}

impl Layout {
    /// The upper-case entity name of `kind`, if this release declares it.
    pub(super) fn entity(self, kind: QuantityKind) -> PropertyResult<String> {
        let entity = kind.type_name().to_ascii_uppercase();
        if self.schema.entity(&entity).is_some_and(|e| !e.abstract_) {
            Ok(entity)
        } else {
            Err(PropertyError::EntityNotInSchema {
                entity: kind.type_name(),
                schema: self.version,
            })
        }
    }

    /// The attribute count this release declares for `entity`.
    pub(super) fn arity(self, entity: &str) -> usize {
        self.schema.attributes(entity).len()
    }

    /// The position of `attribute` on `entity`, by name.
    pub(super) fn slot(self, entity: &str, attribute: &str) -> Option<usize> {
        self.schema
            .attribute_names(entity)
            .iter()
            .position(|name| name.eq_ignore_ascii_case(attribute))
    }

    /// `value` as the scalar `kind`'s measure holds in this release.
    ///
    /// A count is written as an integer when it is whole. A fractional count
    /// is a REAL where `IfcCountMeasure` is `NUMBER` (IFC2X3, IFC4) and is
    /// refused where it is `INTEGER` (IFC4X3), never truncated.
    pub(super) fn scalar(self, kind: QuantityKind, value: f64) -> PropertyResult<Value> {
        let entity = kind.type_name();
        let attribute = kind.value_attribute();
        if !value.is_finite() {
            return Err(PropertyError::AuthoringInvalid {
                entity,
                attribute,
                value: value.to_string(),
            });
        }
        if kind != QuantityKind::Count {
            return Ok(Value::Real(value));
        }
        if value.fract() == 0.0 {
            #[allow(clippy::cast_possible_truncation)]
            return Ok(Value::Integer(value as i64));
        }
        if self.schema.resolve_defined("IFCCOUNTMEASURE") == "INTEGER" {
            return Err(PropertyError::AuthoringInvalid {
                entity,
                attribute,
                value: format!("{value} (IfcCountMeasure is INTEGER in {:?})", self.version),
            });
        }
        Ok(Value::Real(value))
    }

    /// Build a record of `entity` in this release's layout from named
    /// values. A value for an attribute the release does not declare is
    /// refused unless it is `$`; unnamed slots are `$`.
    pub(super) fn record(
        self,
        kind: QuantityKind,
        entity: &str,
        values: Vec<(&'static str, Value)>,
    ) -> PropertyResult<Entity> {
        let mut attributes = vec![Value::Null; self.arity(entity)];
        for (attribute, value) in values {
            match self.slot(entity, attribute) {
                Some(slot) => attributes[slot] = value,
                None if value == Value::Null => {}
                None => {
                    return Err(PropertyError::AuthoringNotInSchema {
                        entity: kind.type_name(),
                        attribute,
                        schema: self.version,
                    })
                }
            }
        }
        Ok(Entity::new(kind.type_name(), attributes))
    }
}
