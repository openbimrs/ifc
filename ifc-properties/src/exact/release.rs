//! The release an exact resolution binds to, and model-level validation.

use ifc_model::{Entity, EntityId, Model};
use ifc_schema::{for_version, Schema, SchemaVersion};

use super::refs::require_exact_slots;
use super::ExactPropertyError;
use std::sync::Arc;

/// The release a resolution runs against: its version and bundled table.
#[derive(Clone, Copy)]
pub(super) struct Release {
    pub(super) version: SchemaVersion,
    pub(super) schema: &'static Schema,
}

impl Release {
    /// Fail closed on an entity the declared release does not define, then
    /// on a record whose slot count is not the release's arity.
    pub(super) fn require_exact_slots(
        self,
        entity_id: EntityId,
        entity: &Entity,
    ) -> Result<(), ExactPropertyError> {
        if self.schema.entity(entity.type_name.as_ref()).is_none() {
            return Err(self.not_in_schema(entity_id, entity.type_name.clone()));
        }
        require_exact_slots(self.schema, entity_id, entity)
    }

    pub(super) fn not_in_schema(self, entity: EntityId, name: Arc<str>) -> ExactPropertyError {
        ExactPropertyError::NotInSchema {
            entity,
            name,
            schema: self.version,
        }
    }

    /// Whether `candidate` is a legal member of attribute `slot` of `entity`,
    /// as this release declares it.
    pub(super) fn slot_accepts(self, entity: &str, slot: usize, candidate: &str) -> bool {
        self.schema
            .attributes(entity)
            .get(slot)
            .is_some_and(|attribute| self.schema.accepts_type(&attribute.type_name, candidate))
    }
}

pub(super) fn validate_model(model: &Model) -> Result<Release, ExactPropertyError> {
    if !model.diagnostics().is_empty() {
        return Err(ExactPropertyError::IncompleteModel {
            diagnostics: model.diagnostics().len(),
        });
    }
    match model.header().schema.as_slice() {
        [] => Err(ExactPropertyError::MissingSchema),
        [token] => match SchemaVersion::from_header_token(token) {
            // IFC4X3 is bundled, but its exact semantics are not verified
            // here yet (#48 scope); it stays unsupported until they are.
            Some(version @ (SchemaVersion::Ifc2x3 | SchemaVersion::Ifc4)) => Ok(Release {
                version,
                schema: for_version(version).expect("IFC2X3 and IFC4 are bundled"),
            }),
            _ => Err(ExactPropertyError::UnsupportedSchema {
                schema: token.clone(),
            }),
        },
        schemas => Err(ExactPropertyError::MultipleSchemas {
            schemas: schemas.len(),
        }),
    }
}
