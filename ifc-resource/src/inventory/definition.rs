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

    #[must_use]
    pub fn id(&self) -> EntityId {
        self.record.id
    }

    pub fn name(&self) -> ResourceResult<Option<&'m str>> {
        self.record.optional_text("Name")
    }

    pub fn description(&self) -> ResourceResult<Option<&'m str>> {
        self.record.optional_text("Description")
    }

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

    pub fn responsible_person_ids(&self) -> ResourceResult<Vec<EntityId>> {
        self.record
            .refs("ResponsiblePersons", "IfcPerson", 1, true, true)
    }

    pub fn last_update_date(&self) -> ResourceResult<Option<&'m str>> {
        self.record.optional_text("LastUpdateDate")
    }

    pub fn current_value(&self) -> ResourceResult<Option<EntityId>> {
        self.record.optional_ref("CurrentValue", "IfcCostValue")
    }

    pub fn original_value(&self) -> ResourceResult<Option<EntityId>> {
        self.record.optional_ref("OriginalValue", "IfcCostValue")
    }
}
