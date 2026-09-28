//! The IFC release a type definition is written in (#202).
//!
//! The catalogue in [`crate::table`] is generated from IFC4X3 ADD2, and
//! [`crate::create_type`] and [`crate::create_supertype`], which take no
//! model, write that release's layout. The `*_in` and
//! `*_with_owner_history` writers take the model and write its declared
//! release instead: slots, the `PredefinedType` enumeration and the
//! attributes a type requires come from that release's table by attribute
//! name. `IfcRoot.OwnerHistory` is the difference that matters most:
//!
//! ```text
//! IFC2X3_TC1   OwnerHistory : IfcOwnerHistory;
//! IFC4         OwnerHistory : OPTIONAL IfcOwnerHistory;
//! IFC4X3_ADD2  OwnerHistory : OPTIONAL IfcOwnerHistory;
//! ```
//!
//! Binding, from `FILE_SCHEMA`, as `ifc-material` (#77), `ifc-properties`
//! (#191) and `ifc-classification` (#194) bind their authoring:
//! - one recognised declaration binds that release's table;
//! - one unrecognised declaration fails with
//!   [`ElementTypeError::UnsupportedSchema`];
//! - several fail with [`ElementTypeError::MultipleSchemas`];
//! - none at all (an in-memory [`Model::new`]) binds IFC4.

use ifc_model::{Edit, Entity, EntityId, Model, Transaction, Value};
use ifc_schema::{for_version, Attribute, Schema, SchemaVersion, TypeKind};

use crate::error::{ElementTypeError, ElementTypeResult};

/// The release a type definition is written in.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Layout {
    version: SchemaVersion,
    schema: &'static Schema,
    /// Refuse a required attribute left `$`. Off only for the catalogue
    /// layout of the writers that take no model, whose output is kept
    /// exactly as it was before #202.
    strict: bool,
}

/// Bind `model`'s declared release.
pub(crate) fn bind(model: &Model) -> ElementTypeResult<Layout> {
    let version = match model.header().schema.as_slice() {
        [] => SchemaVersion::Ifc4,
        [token] => SchemaVersion::from_header_token(token).ok_or_else(|| {
            ElementTypeError::UnsupportedSchema {
                schema: token.clone(),
            }
        })?,
        tokens => {
            return Err(ElementTypeError::MultipleSchemas {
                schemas: tokens.len(),
            })
        }
    };
    Layout::of(version)
}

impl Layout {
    /// The catalogue's own release, which the writers that take no model
    /// write, unchanged: a required attribute the draft has no field for,
    /// such as `IfcFurnitureType.AssemblyPlace`, stays `$` there. The
    /// model-bound writers refuse it with
    /// [`ElementTypeError::AuthoringRequired`] instead.
    pub(crate) fn catalogue() -> ElementTypeResult<Self> {
        Ok(Self {
            strict: false,
            ..Self::of(SchemaVersion::Ifc4x3)?
        })
    }

    fn of(version: SchemaVersion) -> ElementTypeResult<Self> {
        let schema = for_version(version).ok_or_else(|| ElementTypeError::UnsupportedSchema {
            schema: format!("{version:?}"),
        })?;
        Ok(Self {
            version,
            schema,
            strict: true,
        })
    }

    /// Fail unless this release declares `entity` as instantiable.
    pub(crate) fn require_entity(self, entity: &'static str) -> ElementTypeResult<()> {
        if self
            .schema
            .entity(entity)
            .is_some_and(|found| !found.abstract_)
        {
            return Ok(());
        }
        Err(ElementTypeError::EntityNotInSchema {
            entity,
            schema: self.version,
        })
    }

    /// `attribute` of `entity` as this release declares it.
    pub(crate) fn attribute(self, entity: &str, attribute: &str) -> Option<&'static Attribute> {
        self.schema
            .attributes(entity)
            .into_iter()
            .find(|found| found.name.eq_ignore_ascii_case(attribute))
    }

    /// The tokens of `attribute`'s enumeration on `entity`, if this release
    /// declares the attribute with an enumeration type.
    pub(crate) fn members(self, entity: &str, attribute: &str) -> Option<Vec<&'static str>> {
        let declared = self.attribute(entity, attribute)?;
        match &self.schema.type_def(&declared.type_name)?.kind {
            TypeKind::Enumeration(members) => Some(members.iter().map(String::as_str).collect()),
            _ => None,
        }
    }

    /// The release this layout binds.
    pub(crate) fn version(self) -> SchemaVersion {
        self.version
    }

    /// Build a record of `entity` in this release's layout from values named
    /// by attribute; unnamed slots are `$`, and a `$` for an attribute the
    /// release does not declare is dropped.
    ///
    /// Refused before anything is staged: a value for an attribute the
    /// release does not declare ([`ElementTypeError::AuthoringNotInSchema`])
    /// and a required attribute left `$`
    /// ([`ElementTypeError::AuthoringRequired`]), such as the IFC2X3
    /// `OwnerHistory`.
    pub(crate) fn named_record(
        self,
        entity: &'static str,
        values: Vec<(&'static str, Value)>,
    ) -> ElementTypeResult<Entity> {
        let declared = self.schema.attributes(entity);
        let mut attributes = vec![Value::Null; declared.len()];
        for (attribute, value) in values {
            let slot = declared
                .iter()
                .position(|found| found.name.eq_ignore_ascii_case(attribute));
            match slot {
                Some(slot) => attributes[slot] = value,
                None if value == Value::Null => {}
                None => {
                    return Err(ElementTypeError::AuthoringNotInSchema {
                        entity,
                        attribute,
                        schema: self.version,
                    })
                }
            }
        }
        for (declaration, value) in declared.iter().zip(&attributes) {
            if self.strict && *value == Value::Null && !declaration.optional {
                return Err(ElementTypeError::AuthoringRequired {
                    entity,
                    attribute: declaration.name.as_str(),
                    schema: self.version,
                });
            }
        }
        Ok(Entity::new(entity, attributes))
    }
}

/// Fail unless `id` is an `IfcOwnerHistory` in the model or staged on `tx`.
///
/// A missing one is [`ElementTypeError::MissingEntity`], another entity
/// [`ElementTypeError::Invalid`] on `OwnerHistory`. None is ever invented.
pub(crate) fn require_owner_history(
    tx: &Transaction,
    model: &Model,
    entity: &'static str,
    id: EntityId,
) -> ElementTypeResult<()> {
    let found = projected_type(tx, model, id).ok_or(ElementTypeError::MissingEntity { id })?;
    if found.eq_ignore_ascii_case("IFCOWNERHISTORY") {
        return Ok(());
    }
    Err(ElementTypeError::Invalid {
        entity,
        attribute: "OwnerHistory",
        value: format!("#{} is a {found}, not an IfcOwnerHistory", id.0),
    })
}

/// The type `id` has once the transaction's staged edits are applied.
fn projected_type(tx: &Transaction, model: &Model, id: EntityId) -> Option<String> {
    for edit in tx.edits().iter().rev() {
        match edit {
            Edit::Create {
                id: edit_id,
                entity,
            } if *edit_id == id => return Some(entity.type_name.to_string()),
            Edit::Remove { id: edit_id } if *edit_id == id => return None,
            Edit::Retype {
                id: edit_id,
                type_name,
            } if *edit_id == id => return Some(type_name.to_string()),
            _ => {}
        }
    }
    model.get(id).map(|entity| entity.type_name.to_string())
}
