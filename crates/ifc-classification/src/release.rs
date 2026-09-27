//! The IFC release a model's classification records are read against.
//!
//! Every slot position, select, and domain comes from the bundled table of
//! one release, so an IFC2X3 record is never decoded with IFC4 positions.
//! Binding: a header declaring exactly `IFC2X3` binds the IFC2X3 table.
//! Any other single declaration, and an undeclared header (an in-memory
//! model), binds IFC4, which is the 0.2.0 behaviour. Several declarations
//! that include IFC2X3 cannot be bound to one release and fail closed.

use ifc_model::{Entity, EntityId, Model, Value};
use ifc_schema::{for_version, Schema, SchemaVersion};

use crate::{ClassificationError, ClassificationResult};

/// The release a view reads against, or why none could be bound.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Release {
    /// Reads resolve against this release's bundled table.
    Bound(SchemaVersion),
    /// The header declares this many schemas, one of them IFC2X3.
    Multiple(usize),
}

impl Release {
    /// The binding of projections built without a model (`try_new`).
    pub(crate) const LEGACY: Self = Self::Bound(SchemaVersion::Ifc4);

    /// The release `model`'s header binds.
    pub(crate) fn of(model: &Model) -> Self {
        let is_2x3 =
            |token: &String| SchemaVersion::from_header_token(token) == Some(SchemaVersion::Ifc2x3);
        match model.header().schema.as_slice() {
            [token] if is_2x3(token) => Self::Bound(SchemaVersion::Ifc2x3),
            tokens if tokens.len() > 1 && tokens.iter().any(is_2x3) => Self::Multiple(tokens.len()),
            _ => Self::LEGACY,
        }
    }

    /// The bound version and its bundled table.
    pub(crate) fn bound(self) -> ClassificationResult<(SchemaVersion, &'static Schema)> {
        match self {
            Self::Bound(version) => Ok((
                version,
                for_version(version).expect("the IFC2X3 and IFC4 tables are bundled"),
            )),
            Self::Multiple(schemas) => Err(ClassificationError::MultipleSchemas { schemas }),
        }
    }

    /// Position of `attribute` on `entity` in the bound release.
    ///
    /// `attribute` is the IFC4 name; IFC2X3 spells two of them differently
    /// (see [`release_name`]). An attribute or record the release does not
    /// declare is `NotInSchema`, never a silent `None` read past the record.
    pub(crate) fn slot(
        self,
        entity: &'static str,
        id: EntityId,
        attribute: &'static str,
    ) -> ClassificationResult<usize> {
        let (version, schema) = self.bound()?;
        let name = release_name(version, entity, attribute);
        schema
            .attributes(entity)
            .iter()
            .position(|declared| declared.name.eq_ignore_ascii_case(name))
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
}

/// The name `release` gives the attribute this crate knows by its IFC4 name.
///
/// IFC4 renamed IFC2X3 `IfcExternalReference.ItemReference` to
/// `Identification`, and `IfcDocumentInformation.DocumentId` to
/// `Identification`. Both keep their position and meaning, so the IFC4
/// accessor reads them. No other attribute is aliased.
fn release_name(release: SchemaVersion, entity: &str, attribute: &'static str) -> &'static str {
    match (release, entity, attribute) {
        (SchemaVersion::Ifc2x3, "IFCDOCUMENTINFORMATION", "Identification") => "DocumentId",
        (
            SchemaVersion::Ifc2x3,
            "IFCCLASSIFICATIONREFERENCE" | "IFCDOCUMENTREFERENCE" | "IFCLIBRARYREFERENCE",
            "Identification",
        ) => "ItemReference",
        _ => attribute,
    }
}

/// The IFC release `model`'s classification records are read against.
///
/// `IFC2X3` binds the IFC2X3 table. Any other single declaration, and a
/// header with none, binds IFC4 (IFC4X3 is not yet read against its own
/// table). A consumer binds its vocabulary with this answer instead of
/// re-parsing the header.
///
/// # Errors
///
/// [`ClassificationError::MultipleSchemas`] when the header declares several
/// schemas including IFC2X3, so no single release can be bound.
pub fn classification_schema(model: &Model) -> ClassificationResult<SchemaVersion> {
    Release::of(model).bound().map(|(version, _)| version)
}
