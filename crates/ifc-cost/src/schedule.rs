//! `IfcCostSchedule` — the document containing cost items.
//!
//! # Read by name in the declared release (#212)
//!
//! The attribute layout differs between releases, from the EXPRESS sources:
//!
//! ```text
//! IFC2X3_TC1  (13)  0 GlobalId  1 OwnerHistory  2 Name  3 Description
//!                   4 ObjectType  5 SubmittedBy  6 PreparedBy
//!                   7 SubmittedOn (IfcDateTimeSelect)  8 Status
//!                   9 TargetUsers  10 UpdateDate (IfcDateTimeSelect)
//!                   11 ID  12 PredefinedType
//! IFC4 / IFC4X3 (10) 0 GlobalId  1 OwnerHistory  2 Name  3 Description
//!                   4 ObjectType  5 Identification  6 PredefinedType
//!                   7 Status  8 SubmittedOn (IfcDateTime)
//!                   9 UpdateDate (IfcDateTime)
//! ```
//!
//! Reading the IFC4 positions from an IFC2X3 record returns `PreparedBy`
//! as the predefined type and the `SubmittedOn` record as the status. Every
//! accessor here looks its attribute up by name in the model's declared
//! release instead. `Identification` is IFC2X3's `ID`: the IFC4 ADD2 TC1
//! documentation of `IfcCostSchedule` states "Attribute ID renamed to
//! Identification and promoted to supertype IfcControl".

use ifc_model::{Entity, EntityId, Value};

use crate::error::CostError;
use crate::release::{AuthoredDateTime, ReadRelease};
use crate::SchemaVersion;

const ENTITY: &str = "IFCCOSTSCHEDULE";

/// A borrowed view of an `IfcCostSchedule` entity, bound to a release.
#[derive(Debug, Clone, Copy)]
pub struct CostSchedule<'m> {
    id: EntityId,
    entity: &'m Entity,
    release: ReadRelease,
}

impl<'m> CostSchedule<'m> {
    /// Wrap an entity known to be an `IfcCostSchedule`, read against
    /// `release`.
    ///
    /// [`crate::CostView::schedules`] binds the model's declared release;
    /// use this when the release is known some other way.
    ///
    /// # Errors
    ///
    /// [`CostError::UnsupportedSchema`] for a release the cost readers are
    /// not verified against (IFC4X1, IFC4X2).
    pub fn new(
        id: EntityId,
        entity: &'m Entity,
        release: SchemaVersion,
    ) -> Result<Self, CostError> {
        Ok(Self::bound(id, entity, ReadRelease::of_version(release)?))
    }

    pub(crate) const fn bound(id: EntityId, entity: &'m Entity, release: ReadRelease) -> Self {
        Self {
            id,
            entity,
            release,
        }
    }

    /// The entity id in the file.
    pub fn id(&self) -> EntityId {
        self.id
    }

    /// The release this schedule is read against.
    pub fn release(&self) -> SchemaVersion {
        self.release.version()
    }

    fn text(&self, attribute: &str) -> Option<&'m str> {
        self.release.text(ENTITY, self.entity, attribute)
    }

    /// The `GlobalId` string.
    pub fn global_id(&self) -> Option<&'m str> {
        self.text("GlobalId")
    }

    /// The schedule name.
    pub fn name(&self) -> Option<&'m str> {
        self.text("Name")
    }

    /// The user-facing identification code: `Identification`, or IFC2X3's
    /// `ID`, which IFC4 renamed to it.
    pub fn identification(&self) -> Option<&'m str> {
        match self.release.version() {
            SchemaVersion::Ifc2x3 => self.text("ID"),
            _ => self.text("Identification"),
        }
    }

    /// The authored status label, e.g. `Draft`.
    pub fn status(&self) -> Option<&'m str> {
        self.text("Status")
    }

    /// `SubmittedOn`, as authored: IFC4/IFC4X3 ISO 8601 text or an IFC2X3
    /// date record.
    pub fn submitted_on(&self) -> Option<AuthoredDateTime<'m>> {
        self.release.date_time(ENTITY, self.entity, "SubmittedOn")
    }

    /// `UpdateDate`, as authored: IFC4/IFC4X3 ISO 8601 text or an IFC2X3
    /// date record.
    pub fn update_date(&self) -> Option<AuthoredDateTime<'m>> {
        self.release.date_time(ENTITY, self.entity, "UpdateDate")
    }

    /// The predefined type token, e.g. `BUDGET`, without its dots.
    pub fn predefined_type(&self) -> Option<&'m str> {
        match self.release.value(ENTITY, self.entity, "PredefinedType")? {
            Value::Enum(e) => Some(e),
            _ => None,
        }
    }
}
