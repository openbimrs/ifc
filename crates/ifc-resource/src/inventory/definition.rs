//! `IfcInventory` — bounded IFC4 asset-inventory metadata projection.

use ifc_model::EntityId;

use crate::error::ResourceResult;
use crate::view::Record;

/// A borrowed, schema-resolved `IfcInventory` projection.
///
/// `IfcInventory` is an `IfcGroup`: its members are resolved separately
/// through `IfcRelAssignsToGroup` (see `items.rs`), never carried on this
/// projection directly.
#[derive(Debug, Clone, Copy)]
pub struct Inventory<'m, 's> {
    record: Record<'m, 's>,
}

impl<'m, 's> Inventory<'m, 's> {
    pub(crate) fn from_record(record: Record<'m, 's>) -> ResourceResult<Self> {
        Ok(Self { record })
    }

    /// The entity id of the projected `IfcInventory`.
    #[must_use]
    pub fn id(&self) -> EntityId {
        self.record.id
    }

    /// The `Name` attribute, when authored.
    pub fn name(&self) -> ResourceResult<Option<&'m str>> {
        self.record.optional_text("Name")
    }

    /// The `Description` attribute, when authored.
    pub fn description(&self) -> ResourceResult<Option<&'m str>> {
        self.record.optional_text("Description")
    }

    /// The `PredefinedType` enumeration, when authored.
    pub fn predefined_type(&self) -> ResourceResult<Option<&'m str>> {
        self.record.optional_enum("PredefinedType")
    }

    /// `IfcActorSelect`: an `IfcPerson`, `IfcOrganization`, or
    /// `IfcPersonAndOrganization`.
    pub fn jurisdiction(&self) -> ResourceResult<Option<EntityId>> {
        self.record.optional_ref_select(
            "Jurisdiction",
            "IfcActorSelect",
            &["IfcPerson", "IfcOrganization", "IfcPersonAndOrganization"],
        )
    }

    /// The `ResponsiblePersons` attribute, when authored.
    pub fn responsible_person_ids(&self) -> ResourceResult<Vec<EntityId>> {
        self.record
            .refs("ResponsiblePersons", "IfcPerson", 1, true, true)
    }

    /// The `LastUpdateDate` attribute, when authored.
    pub fn last_update_date(&self) -> ResourceResult<Option<&'m str>> {
        self.record.optional_text("LastUpdateDate")
    }

    /// The `CurrentValue` attribute, when authored.
    pub fn current_value(&self) -> ResourceResult<Option<EntityId>> {
        self.record.optional_ref("CurrentValue", "IfcCostValue")
    }

    /// The `OriginalValue` attribute, when authored.
    pub fn original_value(&self) -> ResourceResult<Option<EntityId>> {
        self.record.optional_ref("OriginalValue", "IfcCostValue")
    }
}
