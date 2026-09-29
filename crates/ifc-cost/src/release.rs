//! The IFC release the cost readers bind to (#212).
//!
//! The same rule as the authoring binding in `mutation/release.rs`, from
//! `FILE_SCHEMA`:
//! - one declaration of a release this crate is verified against (IFC2X3,
//!   IFC4, IFC4X3) binds that release's bundled table;
//! - any other single declaration, IFC4X1 and IFC4X2 included, fails with
//!   [`CostError::UnsupportedSchema`], never read through a neighbour's
//!   layout;
//! - several fail with [`CostError::MultipleSchemas`];
//! - none at all (an in-memory [`Model::new`]) binds IFC4.
//!
//! Readers find an attribute's slot by name in that table, so an IFC2X3
//! `IfcCostSchedule` (thirteen attributes, `ID` at 11, `PredefinedType` at
//! 12) is never read through the IFC4 positions.

use ifc_model::{Entity, EntityId, Model, Value};
use ifc_schema::{for_version, Schema, SchemaVersion};

use crate::error::CostError;

/// Releases the cost readers are verified against.
const fn proven(version: SchemaVersion) -> bool {
    matches!(
        version,
        SchemaVersion::Ifc2x3 | SchemaVersion::Ifc4 | SchemaVersion::Ifc4x3
    )
}

/// The release a reader interprets records against.
#[derive(Debug, Clone, Copy)]
pub(crate) struct ReadRelease {
    version: SchemaVersion,
    schema: &'static Schema,
}

impl ReadRelease {
    /// Bind `version`, refusing a release the readers are not verified for.
    pub(crate) fn of_version(version: SchemaVersion) -> Result<Self, CostError> {
        if !proven(version) {
            return Err(CostError::UnsupportedSchema {
                schema: version.release_id().to_owned(),
            });
        }
        let schema = for_version(version).map_err(|_| CostError::UnsupportedSchema {
            schema: version.release_id().to_owned(),
        })?;
        Ok(Self { version, schema })
    }

    /// Bind `model`'s declared release.
    pub(crate) fn of(model: &Model) -> Result<Self, CostError> {
        match model.header().schema.as_slice() {
            [] => Self::of_version(SchemaVersion::Ifc4),
            [token] => SchemaVersion::from_header_token(token)
                .filter(|version| proven(*version))
                .ok_or_else(|| CostError::UnsupportedSchema {
                    schema: token.clone(),
                })
                .and_then(Self::of_version),
            tokens => Err(CostError::MultipleSchemas {
                schemas: tokens.len(),
            }),
        }
    }

    /// The bound release.
    pub(crate) const fn version(self) -> SchemaVersion {
        self.version
    }

    /// The value of `attribute`, named as `release` names it, on a record
    /// of `entity`; `None` when the release does not declare it or the
    /// record is too short to hold it.
    pub(crate) fn value<'m>(
        self,
        entity_type: &str,
        record: &'m Entity,
        attribute: &str,
    ) -> Option<&'m Value> {
        let slot = self
            .schema
            .attribute_names(entity_type)
            .iter()
            .position(|name| name.eq_ignore_ascii_case(attribute))?;
        match record.attribute(slot)? {
            Value::Null => None,
            value => Some(value),
        }
    }

    /// The text of `attribute`, or `None`.
    pub(crate) fn text<'m>(
        self,
        entity_type: &str,
        record: &'m Entity,
        attribute: &str,
    ) -> Option<&'m str> {
        self.value(entity_type, record, attribute)?
            .unwrap_typed()
            .as_text()
    }

    /// A date in the form the release declares it: IFC4 and IFC4X3
    /// `IfcDateTime` text, or an IFC2X3 `IfcDateTimeSelect` record.
    pub(crate) fn date_time<'m>(
        self,
        entity_type: &str,
        record: &'m Entity,
        attribute: &str,
    ) -> Option<AuthoredDateTime<'m>> {
        match self.value(entity_type, record, attribute)? {
            Value::Ref(target) => Some(AuthoredDateTime::Record(*target)),
            value => value.unwrap_typed().as_text().map(AuthoredDateTime::Text),
        }
    }
}

/// A date and time as the model's release types it.
///
/// IFC4 and IFC4X3 declare `IfcDateTime`, ISO 8601 text; IFC2X3 declares
/// `IfcDateTimeSelect`, a reference to an `IfcCalendarDate`, `IfcLocalTime`
/// or `IfcDateAndTime` record. Both are returned as authored.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum AuthoredDateTime<'m> {
    /// IFC4/IFC4X3 `IfcDateTime` text.
    Text(&'m str),
    /// IFC2X3 `IfcDateTimeSelect` record.
    Record(EntityId),
}

impl<'m> AuthoredDateTime<'m> {
    /// The text form, or `None` for a record.
    #[must_use]
    pub const fn text(self) -> Option<&'m str> {
        match self {
            Self::Text(text) => Some(text),
            Self::Record(_) => None,
        }
    }

    /// The record form, or `None` for text.
    #[must_use]
    pub const fn record(self) -> Option<EntityId> {
        match self {
            Self::Record(id) => Some(id),
            Self::Text(_) => None,
        }
    }
}
