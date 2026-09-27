//! `IfcCostSchedule` — the document containing cost items.

use ifc_model::{Entity, EntityId};

/// A borrowed view of an `IfcCostSchedule` entity.
#[derive(Debug, Clone, Copy)]
pub struct CostSchedule<'m> {
    id: EntityId,
    entity: &'m Entity,
}

/// `IfcCostSchedule` slots.
///
/// Same `IfcControl` prefix as `IfcCostItem`:
///
/// ```text
/// 0 GlobalId   1 OwnerHistory  2 Name      3 Description  4 ObjectType
/// 5 Identification  6 PredefinedType  7 Status  8 SubmittedOn  9 UpdateDate
/// ```
///
/// Slot 8 is `SubmittedOn`, NOT `PredefinedType`: reading the type from 8
/// returns a date string for any file that states one.
mod slot {
    /// `GlobalId` (from `IfcRoot`).
    pub const GLOBAL_ID: usize = 0;
    /// `Name` (from `IfcRoot`).
    pub const NAME: usize = 2;
    /// `Identification` (from `IfcControl`).
    pub const IDENTIFICATION: usize = 5;
    /// `PredefinedType`, e.g. `.BUDGET.`
    pub const PREDEFINED_TYPE: usize = 6;
    /// `Status`.
    pub const STATUS: usize = 7;
    /// `SubmittedOn`.
    pub const SUBMITTED_ON: usize = 8;
    /// `UpdateDate`.
    pub const UPDATE_DATE: usize = 9;
}

impl<'m> CostSchedule<'m> {
    /// Wrap an entity known to be an `IfcCostSchedule`.
    pub fn new(id: EntityId, entity: &'m Entity) -> Self {
        Self { id, entity }
    }

    /// The entity id in the file.
    pub fn id(&self) -> EntityId {
        self.id
    }

    /// The `GlobalId` string.
    pub fn global_id(&self) -> Option<&'m str> {
        self.entity.text(slot::GLOBAL_ID)
    }

    /// The schedule name.
    pub fn name(&self) -> Option<&'m str> {
        self.entity.text(slot::NAME)
    }

    /// The user-facing identification code.
    pub fn identification(&self) -> Option<&'m str> {
        self.entity.text(slot::IDENTIFICATION)
    }

    /// The authored status label, e.g. `Draft`.
    pub fn status(&self) -> Option<&'m str> {
        self.entity.text(slot::STATUS)
    }

    /// `SubmittedOn`, as the authored ISO-8601 string.
    pub fn submitted_on(&self) -> Option<&'m str> {
        self.entity.text(slot::SUBMITTED_ON)
    }

    /// `UpdateDate`, as the authored ISO-8601 string.
    pub fn update_date(&self) -> Option<&'m str> {
        self.entity.text(slot::UPDATE_DATE)
    }

    /// The predefined type token, e.g. `BUDGET`, without its dots.
    pub fn predefined_type(&self) -> Option<&'m str> {
        match self.entity.attribute(slot::PREDEFINED_TYPE)? {
            ifc_model::Value::Enum(e) => Some(e),
            _ => None,
        }
    }
}
