//! Entity attributes by name (#326).
//!
//! The positional operations ([`IfcModel::attribute`],
//! [`IfcModel::set_attribute`]) read and write a slot of the STEP record.
//! These resolve a name to its slot first, against the release the file's
//! header declares (`ifc::attribute_slots`): `IfcTask.Status` is slot 6 in
//! an IFC2X3 file and slot 7 in an IFC4 one, and asking an IFC4 task for
//! IFC2X3's `TaskId` is refused, never answered from another release.
//!
//! - Names match ASCII case-insensitively, as EXPRESS identifiers do;
//!   answers spell them as the schema declares (`GlobalId`).
//! - The slots are the explicit attributes, inherited first: exactly what a
//!   STEP record holds. `INVERSE` and new `DERIVE` attributes have no slot
//!   and are unknown names.
//! - An inherited attribute a subtype redeclares as derived keeps its slot,
//!   is listed with `derived` set, reads as stored (normally `*`) and is
//!   refused on write with `derived-attribute`.
//!
//! Refusals: `missing-entity`; `unsupported-schema` when the header names
//! no release this build bundles, or the release does not declare the
//! entity's type; `unknown-attribute`; `derived-attribute`.

use ifc::{EntityId, NamedAttributeError};

use crate::record::{Field, Record, ToRecord};
use crate::value::Tagged;
use crate::{BindingError, IfcModel};

/// One explicit attribute of an entity, as its declared release defines it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AttributeInfo {
    /// The declared name, in the schema's spelling.
    pub name: String,
    /// Zero-based slot in the STEP record: the `index` of
    /// [`IfcModel::attribute`].
    pub index: usize,
    /// The declared type, or an aggregate's innermost element type.
    pub type_name: String,
    /// Whether `$` is a valid value.
    pub optional: bool,
    /// Whether the type is a `LIST`, `SET`, `BAG` or `ARRAY`.
    pub aggregate: bool,
    /// Whether the slot is derived for this entity (written `*`; not
    /// writable).
    pub derived: bool,
    /// The entity whose declaration introduces the attribute.
    pub declared_by: String,
}

impl ToRecord for AttributeInfo {
    fn to_record(&self) -> Record {
        Record::new(
            "AttributeInfo",
            vec![
                ("name", Field::Text(self.name.clone())),
                ("index", Field::Count(self.index)),
                ("type_name", Field::Text(self.type_name.clone())),
                ("optional", Field::Bool(self.optional)),
                ("aggregate", Field::Bool(self.aggregate)),
                ("derived", Field::Bool(self.derived)),
                ("declared_by", Field::Text(self.declared_by.clone())),
            ],
        )
    }
}

pub(crate) fn refused(error: NamedAttributeError) -> BindingError {
    match error {
        NamedAttributeError::MissingEntity(EntityId(id)) => BindingError::MissingEntity(id),
        NamedAttributeError::UnknownEntity { .. } => {
            BindingError::UnsupportedSchema(error.to_string())
        }
        NamedAttributeError::UnknownAttribute { .. } => {
            BindingError::UnknownAttribute(error.to_string())
        }
        NamedAttributeError::DerivedAttribute { .. } => {
            BindingError::DerivedAttribute(error.to_string())
        }
        // `NamedAttributeError` is non-exhaustive; a new refusal is
        // reported, never dropped, until it earns a code of its own.
        other => BindingError::InvalidModel(other.to_string()),
    }
}

impl IfcModel {
    /// Every explicit attribute of entity `id` in slot order, inherited
    /// first, resolved against the declared release.
    pub fn attribute_names(&self, id: u64) -> Result<Vec<AttributeInfo>, BindingError> {
        let schema = self.declared_schema()?;
        let type_name = &self.entity(id)?.type_name;
        Ok(ifc::attribute_slots(schema, type_name)
            .map_err(refused)?
            .into_iter()
            .map(|slot| AttributeInfo {
                name: slot.name.to_owned(),
                index: slot.index,
                type_name: slot.type_name.to_owned(),
                optional: slot.optional,
                aggregate: slot.aggregate,
                derived: slot.derived,
                declared_by: slot.declared_by.to_owned(),
            })
            .collect())
    }

    /// Attribute `name` (any case) of entity `id`; `null` when the record
    /// stops before its slot, as for [`Self::attribute`].
    pub fn attribute_by_name(&self, id: u64, name: &str) -> Result<Tagged, BindingError> {
        let schema = self.declared_schema()?;
        Ok(
            ifc::attribute_by_name(&self.inner, schema, EntityId(id), name)
                .map_err(refused)?
                .map_or(Tagged::Null, Tagged::from_value),
        )
    }

    /// Set attribute `name` (any case) of entity `id`, returning the
    /// previous value. The value is checked as for [`Self::set_attribute`];
    /// every check runs before the write, so a refusal changes nothing.
    pub fn set_attribute_by_name(
        &mut self,
        id: u64,
        name: &str,
        value: Tagged,
    ) -> Result<Tagged, BindingError> {
        let value = value.into_value()?;
        let schema = self.declared_schema()?;
        ifc::set_attribute_by_name(&mut self.inner, schema, EntityId(id), name, value)
            .map(|previous| Tagged::from_value(&previous))
            .map_err(refused)
    }
}
