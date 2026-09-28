//! The IFC release quantity authoring writes against (#190).
//!
//! The same binding `ifc-properties` quantity authoring uses, so the two
//! writers of `IfcQuantity*` agree in every release (the facade test
//! `quantity_writer_agreement.rs` compares them byte for byte). Sibling
//! crates cannot share the code, only the rule, from `FILE_SCHEMA`:
//! - one recognised declaration binds that release's bundled table;
//! - one unrecognised declaration fails with
//!   [`CostAuthoringError::UnsupportedSchema`];
//! - several fail with [`CostAuthoringError::MultipleSchemas`];
//! - none at all (an in-memory [`Model::new`]) binds IFC4.
//!
//! Attributes are placed by name from that table, so an IFC2X3 quantity has
//! four attributes and no `Formula`, and `IfcQuantityNumber` exists only in
//! IFC4X3.

use ifc_model::{Entity, Model, Value};
use ifc_schema::{for_version, Schema, SchemaVersion};

use super::{CostAuthoringError, CostAuthoringResult};

/// The bound release and its table.
#[derive(Debug, Clone, Copy)]
pub(super) struct Release {
    version: SchemaVersion,
    schema: &'static Schema,
}

/// Bind `model`'s declared release.
pub(super) fn bind(model: &Model) -> CostAuthoringResult<Release> {
    let version = match model.header().schema.as_slice() {
        [] => SchemaVersion::Ifc4,
        [token] => SchemaVersion::from_header_token(token).ok_or_else(|| {
            CostAuthoringError::UnsupportedSchema {
                schema: token.clone(),
            }
        })?,
        tokens => {
            return Err(CostAuthoringError::MultipleSchemas {
                schemas: tokens.len(),
            })
        }
    };
    let schema = for_version(version).ok_or_else(|| CostAuthoringError::UnsupportedSchema {
        schema: format!("{version:?}"),
    })?;
    Ok(Release { version, schema })
}

impl Release {
    /// Build a record of `entity` in this release's layout from named
    /// values. A value for an attribute the release does not declare is
    /// refused unless it is `$`; unnamed slots are `$`.
    pub(super) fn record(
        self,
        entity: &'static str,
        values: Vec<(&'static str, Value)>,
    ) -> CostAuthoringResult<Entity> {
        if self.schema.entity(entity).is_none_or(|e| e.abstract_) {
            return Err(CostAuthoringError::EntityNotInSchema {
                entity,
                schema: self.version,
            });
        }
        let names = self.schema.attribute_names(entity);
        let mut attributes = vec![Value::Null; names.len()];
        for (attribute, value) in values {
            match names
                .iter()
                .position(|name| name.eq_ignore_ascii_case(attribute))
            {
                Some(slot) => attributes[slot] = value,
                None if value == Value::Null => {}
                None => {
                    return Err(CostAuthoringError::AuthoringNotInSchema {
                        entity,
                        attribute,
                        schema: self.version,
                    })
                }
            }
        }
        Ok(Entity::new(entity, attributes))
    }
}
