//! The exported model handle.
//!
//! `IfcModel` owns one `ifc::Model`. Every binding crate wraps this type and
//! converts host arguments to and from it; none re-implements an operation.

use ifc::{Codec, Entity, EntityId, Model, StepCodec};

use crate::value::Tagged;
use crate::BindingError;

/// An IFC model: entities keyed by their `#id`, in file order.
#[derive(Debug, Default)]
pub struct IfcModel {
    inner: Model,
}

impl IfcModel {
    /// An empty model.
    pub fn empty() -> Self {
        Self::default()
    }

    /// Parse a STEP (`.ifc`) file.
    ///
    /// Non-fatal problems do not fail the parse; they are reported by
    /// [`Self::diagnostics`], so a caller can tell a complete read from a
    /// partial one.
    pub fn parse(bytes: &[u8]) -> Result<Self, BindingError> {
        StepCodec
            .read_bytes(bytes)
            .map(|inner| Self { inner })
            .map_err(|error| BindingError::Parse(error.to_string()))
    }

    /// Serialize as STEP.
    pub fn write(&self) -> Result<Vec<u8>, BindingError> {
        StepCodec
            .write_bytes(&self.inner)
            .map_err(|error| BindingError::Write(error.to_string()))
    }

    /// Number of entities.
    pub fn len(&self) -> usize {
        self.inner.len()
    }

    /// Whether the model has no entities.
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    /// The first `FILE_SCHEMA` token, e.g. `IFC4`, if any.
    pub fn schema(&self) -> Option<&str> {
        self.inner.header().schema_token()
    }

    /// Every non-fatal problem found while reading, as display strings.
    pub fn diagnostics(&self) -> Vec<String> {
        self.inner
            .diagnostics()
            .iter()
            .map(ToString::to_string)
            .collect()
    }

    /// Every entity id, in file order.
    pub fn ids(&self) -> Vec<u64> {
        self.inner.ids().map(|EntityId(id)| id).collect()
    }

    /// Ids of every entity of exactly `type_name` (case-insensitive), in
    /// file order. Subtypes are not included.
    pub fn ids_of_type(&self, type_name: &str) -> Vec<u64> {
        self.inner
            .ids_of_type(&type_name.to_ascii_uppercase())
            .iter()
            .map(|EntityId(id)| *id)
            .collect()
    }

    /// Ids of every entity of `type_name` or any of its subtypes, in file
    /// order, using the schema the file's header declares.
    ///
    /// `IfcWall` then finds `IFCWALLSTANDARDCASE` too, which exact
    /// [`Self::ids_of_type`] does not. An undeclared name finds nothing.
    /// Fails when the header names no schema this crate bundles.
    pub fn ids_of_type_including_subtypes(
        &self,
        type_name: &str,
    ) -> Result<Vec<u64>, BindingError> {
        let token = self.schema().unwrap_or("");
        let schema = ifc::SchemaVersion::from_header_token(token)
            .and_then(ifc::schema::for_version)
            .ok_or_else(|| BindingError::UnsupportedSchema(token.to_owned()))?;
        Ok(
            ifc::ids_of_type_including_subtypes(&self.inner, schema, type_name)
                .into_iter()
                .map(|EntityId(id)| id)
                .collect(),
        )
    }

    /// The type name of entity `id`, upper-case.
    pub fn type_of(&self, id: u64) -> Result<&str, BindingError> {
        Ok(&self.entity(id)?.type_name)
    }

    /// Every attribute of entity `id`, in declaration order.
    pub fn attributes(&self, id: u64) -> Result<Vec<Tagged>, BindingError> {
        Ok(self
            .entity(id)?
            .attributes
            .iter()
            .map(Tagged::from_value)
            .collect())
    }

    /// Attribute `index` of entity `id`; `null` past the last attribute,
    /// the same as an unset `$`.
    pub fn attribute(&self, id: u64, index: usize) -> Result<Tagged, BindingError> {
        Ok(self
            .entity(id)?
            .attribute(index)
            .map_or(Tagged::Null, Tagged::from_value))
    }

    /// Set attribute `index` of entity `id`, returning the previous value.
    ///
    /// Writing past the end pads the gap with `$`, as the model does.
    pub fn set_attribute(
        &mut self,
        id: u64,
        index: usize,
        value: Tagged,
    ) -> Result<Tagged, BindingError> {
        let value = value.into_value()?;
        self.inner
            .set_attribute(EntityId(id), index, value)
            .map(|previous| Tagged::from_value(&previous))
            .ok_or(BindingError::MissingEntity(id))
    }

    /// Append a new entity, returning its id.
    pub fn add(&mut self, type_name: &str, attributes: Vec<Tagged>) -> Result<u64, BindingError> {
        let entity = entity(type_name, attributes)?;
        Ok(self.inner.push(entity).0)
    }

    /// Remove entity `id`. References to it are left dangling, as in the
    /// model; find them with [`Self::dangling_references`].
    pub fn remove(&mut self, id: u64) -> Result<(), BindingError> {
        self.inner
            .remove(EntityId(id))
            .map(drop)
            .ok_or(BindingError::MissingEntity(id))
    }

    /// Every `(from, to)` pair where `from` references a missing `to`.
    pub fn dangling_references(&self) -> Vec<(u64, u64)> {
        self.inner
            .dangling_references()
            .into_iter()
            .map(|(EntityId(from), EntityId(to))| (from, to))
            .collect()
    }

    fn entity(&self, id: u64) -> Result<&Entity, BindingError> {
        self.inner
            .get(EntityId(id))
            .ok_or(BindingError::MissingEntity(id))
    }
}

/// Build an entity, validating the type name as a STEP identifier.
fn entity(type_name: &str, attributes: Vec<Tagged>) -> Result<Entity, BindingError> {
    let probe = Tagged::Typed {
        type_name: type_name.to_owned(),
        value: Box::new(Tagged::Null),
    };
    // Reuse the typed-wrapper name check rather than duplicating it.
    probe.into_value()?;
    let values = attributes
        .into_iter()
        .map(Tagged::into_value)
        .collect::<Result<_, _>>()?;
    Ok(Entity::new(type_name.to_ascii_uppercase(), values))
}

/// Every host binding may move a model across threads (the Python GIL, a
/// C host's worker pool). Pin that here so a non-`Send` field fails the
/// core's build, not a binding's.
#[allow(dead_code)]
const fn assert_send_sync<T: Send + Sync>() {}
const _: () = assert_send_sync::<IfcModel>();

#[cfg(test)]
mod tests;
