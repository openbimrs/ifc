//! The IFC release a model's classification records are read and written
//! against.
//!
//! Every slot position, select, domain and enumeration comes from the
//! bundled table of one release, looked up by attribute name, so a record is
//! never decoded or authored with another release's positions.
//!
//! Binding, from `FILE_SCHEMA`, as `ifc-material` binds (#77):
//! - one recognised declaration (`IFC2X3`, `IFC4`, `IFC4X3`/`IFC4X3_ADD2`)
//!   binds that release's own table;
//! - one unrecognised declaration fails closed with
//!   [`ClassificationError::UnsupportedSchema`];
//! - several declarations fail closed with
//!   [`ClassificationError::MultipleSchemas`];
//! - no declaration at all (an in-memory [`Model::new`]) binds IFC4, the
//!   0.2.0 behaviour.

use ifc_model::{Entity, EntityId, Model, Value};
use ifc_schema::{for_version, Attribute, Schema, SchemaVersion, TypeKind};

use crate::{ClassificationError, ClassificationResult};

/// The release a view or authoring call binds to, or why none could be.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Release<'m> {
    /// Reads and writes resolve against this release's bundled table.
    Bound(SchemaVersion),
    /// The header declares this many schemas.
    Multiple(usize),
    /// The header declares one schema that has no bundled table.
    Unsupported(&'m str),
}

impl<'m> Release<'m> {
    /// The binding of projections built without a model (`try_new`), of
    /// authoring calls that take no model, and of a model whose header
    /// declares no schema.
    pub(crate) const LEGACY: Release<'static> = Release::Bound(SchemaVersion::Ifc4);

    /// The release `model`'s header binds.
    pub(crate) fn of(model: &'m Model) -> Self {
        match model.header().schema.as_slice() {
            [] => Release::LEGACY,
            [token] => SchemaVersion::from_header_token(token)
                .map_or(Self::Unsupported(token.as_str()), Self::Bound),
            tokens => Self::Multiple(tokens.len()),
        }
    }

    /// The bound version and its bundled table.
    pub(crate) fn bound(self) -> ClassificationResult<(SchemaVersion, &'static Schema)> {
        match self {
            Self::Bound(version) => Ok((
                version,
                for_version(version).expect("every SchemaVersion has a bundled table"),
            )),
            Self::Multiple(schemas) => Err(ClassificationError::MultipleSchemas { schemas }),
            Self::Unsupported(schema) => Err(ClassificationError::UnsupportedSchema {
                schema: schema.to_owned(),
            }),
        }
    }

    /// Position of `attribute` (IFC4 name) on `entity` in the bound release.
    ///
    /// An attribute or record the release does not declare is `NotInSchema`,
    /// never a silent `None` read past the record or from a slot the release
    /// gives another meaning.
    pub(crate) fn slot(
        self,
        entity: &'static str,
        id: EntityId,
        attribute: &'static str,
    ) -> ClassificationResult<usize> {
        let (version, schema) = self.bound()?;
        position(schema, version, entity, attribute)
            .map(|(slot, _)| slot)
            .ok_or(ClassificationError::NotInSchema {
                entity,
                id,
                attribute,
                schema: version,
            })
    }

    /// [`Self::slot`] for a text accessor.
    ///
    /// IFC2X3 types several dates and formats as entity records (for example
    /// `IfcClassification.EditionDate` is an `IfcCalendarDate`). Such a value
    /// is valid, not malformed, so it is reported as `StructuredValue` with
    /// the record id instead of being rejected as an invalid string.
    pub(crate) fn text_slot(
        self,
        entity: &'static str,
        id: EntityId,
        record: &Entity,
        attribute: &'static str,
    ) -> ClassificationResult<usize> {
        let slot = self.slot(entity, id, attribute)?;
        if let Some(Value::Ref(target)) = record.attribute(slot) {
            let declared = self.declared_type(entity, id, attribute)?;
            let (_, schema) = self.bound()?;
            if schema.entity(declared).is_some() {
                return Err(ClassificationError::StructuredValue {
                    entity,
                    id,
                    attribute,
                    target: *target,
                });
            }
        }
        Ok(slot)
    }

    /// The type the bound release declares for `attribute` on `entity`,
    /// for example `IfcClassificationNotationSelect` for IFC2X3
    /// `IfcRelAssociatesClassification.RelatingClassification`.
    pub(crate) fn declared_type(
        self,
        entity: &'static str,
        id: EntityId,
        attribute: &'static str,
    ) -> ClassificationResult<&'static str> {
        let slot = self.slot(entity, id, attribute)?;
        let (_, schema) = self.bound()?;
        Ok(schema.attributes(entity)[slot].type_name.as_str())
    }

    /// Whether `candidate` is a legal value of `attribute` on `entity`.
    pub(crate) fn accepts(
        self,
        entity: &'static str,
        id: EntityId,
        attribute: &'static str,
        candidate: &str,
    ) -> ClassificationResult<bool> {
        let declared = self.declared_type(entity, id, attribute)?;
        let (_, schema) = self.bound()?;
        Ok(schema.accepts_type(declared, candidate))
    }

    /// The enumerators the bound release declares for the enumeration-typed
    /// `attribute` on `entity`, in declaration order. Empty when the
    /// attribute is not an enumeration, so every value is refused.
    pub(crate) fn enumerators(
        self,
        entity: &'static str,
        id: EntityId,
        attribute: &'static str,
    ) -> ClassificationResult<Vec<&'static str>> {
        let declared = self.declared_type(entity, id, attribute)?;
        let (_, schema) = self.bound()?;
        Ok(enumeration(schema, declared))
    }

    /// Fail with [`ClassificationError::EntityNotInSchema`] unless the bound
    /// release can instantiate `entity`.
    pub(crate) fn require_entity(
        self,
        entity: &'static str,
    ) -> ClassificationResult<(SchemaVersion, &'static Schema)> {
        let (version, schema) = self.bound()?;
        if schema.entity(entity).is_some_and(|e| !e.abstract_) {
            Ok((version, schema))
        } else {
            Err(ClassificationError::EntityNotInSchema {
                entity,
                schema: version,
            })
        }
    }

    /// The bound release's declaration of `attribute` (IFC4 name) on
    /// `entity`, for authoring: `EntityNotInSchema` or
    /// `AuthoringNotInSchema` when the release lacks either.
    pub(crate) fn declared(
        self,
        entity: &'static str,
        attribute: &'static str,
    ) -> ClassificationResult<&'static Attribute> {
        let (version, schema) = self.require_entity(entity)?;
        position(schema, version, entity, attribute)
            .map(|(_, declared)| declared)
            .ok_or(ClassificationError::AuthoringNotInSchema {
                entity,
                attribute,
                schema: version,
            })
    }

    /// Build `entity`'s record in the bound release's layout from values
    /// named by their IFC4 attribute names.
    ///
    /// A non-null value for an attribute the release does not declare is
    /// refused with `AuthoringNotInSchema` rather than dropped; text for an
    /// attribute the release types as an entity record with
    /// `AuthoringValueType`; and a null for an attribute the release
    /// requires with `AuthoringRequired`. References are checked by the
    /// caller against [`Self::declared`], which sees staged edits.
    pub(crate) fn record(
        self,
        entity: &'static str,
        values: Vec<(&'static str, Value)>,
    ) -> ClassificationResult<Entity> {
        let (version, schema) = self.require_entity(entity)?;
        let declared = schema.attributes(entity);
        let mut slots = vec![None; declared.len()];
        for (attribute, value) in values {
            match position(schema, version, entity, attribute) {
                Some((slot, declaration)) => {
                    if holds_text(&value) && !is_text_type(schema, &declaration.type_name) {
                        return Err(ClassificationError::AuthoringValueType {
                            entity,
                            attribute,
                            declared: declaration.type_name.as_str(),
                            schema: version,
                        });
                    }
                    slots[slot] = Some(value);
                }
                None if value == Value::Null => {}
                None => {
                    return Err(ClassificationError::AuthoringNotInSchema {
                        entity,
                        attribute,
                        schema: version,
                    })
                }
            }
        }
        let mut attributes = Vec::with_capacity(slots.len());
        for (slot, value) in slots.into_iter().enumerate() {
            match value.unwrap_or(Value::Null) {
                Value::Null if !declared[slot].optional => {
                    return Err(ClassificationError::AuthoringRequired {
                        entity,
                        attribute: declared[slot].name.as_str(),
                        schema: version,
                    })
                }
                value => attributes.push(value),
            }
        }
        Ok(Entity::new(entity, attributes))
    }
}

/// Position and declaration of the attribute this crate knows by its IFC4
/// name `attribute`, in `version`'s table.
fn position(
    schema: &'static Schema,
    version: SchemaVersion,
    entity: &str,
    attribute: &'static str,
) -> Option<(usize, &'static Attribute)> {
    let name = release_name(version, entity, attribute);
    schema
        .attributes(entity)
        .into_iter()
        .enumerate()
        .find(|(_, declared)| declared.name.eq_ignore_ascii_case(name))
}

fn holds_text(value: &Value) -> bool {
    match value {
        Value::Text(_) => true,
        Value::List(items) => items.iter().any(holds_text),
        _ => false,
    }
}

/// Whether `declared` resolves to an EXPRESS `STRING` (a label, identifier,
/// URI, IFC4 date string), rather than an entity record such as IFC2X3
/// `IfcCalendarDate`.
fn is_text_type(schema: &Schema, declared: &str) -> bool {
    schema
        .resolve_defined(declared)
        .to_ascii_uppercase()
        .starts_with("STRING")
}

/// The enumerators of the enumeration type `declared`, in declaration
/// order; empty when `declared` is not an enumeration.
pub(crate) fn enumeration(schema: &'static Schema, declared: &str) -> Vec<&'static str> {
    match schema.type_def(declared).map(|definition| &definition.kind) {
        Some(TypeKind::Enumeration(values)) => values.iter().map(String::as_str).collect(),
        _ => Vec::new(),
    }
}

/// The name `release` gives the attribute this crate knows by its IFC4 name.
///
/// Each alias keeps its position and meaning, so the IFC4-named accessor and
/// draft field read and write it:
/// - IFC2X3 `IfcExternalReference.ItemReference` became `Identification`;
/// - IFC2X3 `IfcDocumentInformation.DocumentId` became `Identification`;
/// - IFC4X3 ADD2 renamed IFC4 `IfcClassification.Location` to
///   `Specification` (both `OPTIONAL IfcURIReference`, sixth attribute).
///
/// No other attribute this crate reads or writes is renamed.
pub(crate) fn release_name(
    release: SchemaVersion,
    entity: &str,
    attribute: &'static str,
) -> &'static str {
    let entity = entity.to_ascii_uppercase();
    match (release, entity.as_str(), attribute) {
        (SchemaVersion::Ifc2x3, "IFCDOCUMENTINFORMATION", "Identification") => "DocumentId",
        (
            SchemaVersion::Ifc2x3,
            "IFCCLASSIFICATIONREFERENCE" | "IFCDOCUMENTREFERENCE" | "IFCLIBRARYREFERENCE",
            "Identification",
        ) => "ItemReference",
        (SchemaVersion::Ifc4x3, "IFCCLASSIFICATION", "Location") => "Specification",
        _ => attribute,
    }
}

/// The IFC release `model`'s classification records are read and written
/// against.
///
/// A header declaring one recognised schema binds that release: `IFC2X3`,
/// `IFC4`, or `IFC4X3`/`IFC4X3_ADD2` (the IFC4X3 ADD2 table). A header with
/// no declaration binds IFC4 (an in-memory model; the 0.2.0 behaviour). A
/// consumer binds its vocabulary with this answer instead of re-parsing the
/// header.
///
/// # Errors
///
/// [`ClassificationError::MultipleSchemas`] when the header declares several
/// schemas, and [`ClassificationError::UnsupportedSchema`] when it declares
/// one with no bundled table. Neither is read as IFC4.
pub fn classification_schema(model: &Model) -> ClassificationResult<SchemaVersion> {
    Release::of(model).bound().map(|(version, _)| version)
}

#[cfg(test)]
mod tests;
