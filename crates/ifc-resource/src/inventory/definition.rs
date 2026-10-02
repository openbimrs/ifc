//! `IfcInventory` — bounded asset-inventory metadata projection.
//!
//! IFC2X3 declares `InventoryType` (required), `Jurisdiction`,
//! `ResponsiblePersons` and `LastUpdateDate` as required, the last an
//! `IfcCalendarDate` entity rather than IFC4's `IfcDate` text. The shared
//! accessors read the IFC2X3 slots by their own names and refuse an unset
//! required one; the calendar date has its own accessor.

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
    ///
    /// Under IFC2X3 this reads the required `InventoryType`, declared with
    /// the same `IfcInventoryTypeEnum`; an unset value there is refused.
    pub fn predefined_type(&self) -> ResourceResult<Option<&'m str>> {
        if self.record.is_ifc2x3() {
            return self.record.required_enum("InventoryType").map(Some);
        }
        self.record.optional_enum("PredefinedType")
    }

    /// `IfcActorSelect`: an `IfcPerson`, `IfcOrganization`, or
    /// `IfcPersonAndOrganization`. Required under IFC2X3, so an unset value
    /// there is refused.
    pub fn jurisdiction(&self) -> ResourceResult<Option<EntityId>> {
        const MEMBERS: &[&str] = &["IfcPerson", "IfcOrganization", "IfcPersonAndOrganization"];
        if self.record.is_ifc2x3() {
            return self
                .record
                .required_ref_select("Jurisdiction", "IfcActorSelect", MEMBERS)
                .map(Some);
        }
        self.record
            .optional_ref_select("Jurisdiction", "IfcActorSelect", MEMBERS)
    }

    /// The `ResponsiblePersons` attribute, when authored. Required
    /// (`SET [1:?]`) under IFC2X3, so an unset value there is refused.
    pub fn responsible_person_ids(&self) -> ResourceResult<Vec<EntityId>> {
        let optional = !self.record.is_ifc2x3();
        self.record
            .refs("ResponsiblePersons", "IfcPerson", 1, optional, true)
    }

    /// The `LastUpdateDate` attribute (`IfcDate` text), when authored.
    ///
    /// IFC2X3 declares it as an `IfcCalendarDate` entity, so this refuses
    /// with [`ResourceError::NotInSchema`](crate::ResourceError::NotInSchema) there; read it with
    /// [`Inventory::last_update_calendar_date`].
    pub fn last_update_date(&self) -> ResourceResult<Option<&'m str>> {
        if self.record.is_ifc2x3() {
            return Err(self.record.not_in_schema("LastUpdateDate"));
        }
        self.record.optional_text("LastUpdateDate")
    }

    /// IFC2X3 `IfcInventory.LastUpdateDate`: the required `IfcCalendarDate`
    /// reference. [`ResourceError::NotInSchema`](crate::ResourceError::NotInSchema) under IFC4 and IFC4X3,
    /// where the attribute is `IfcDate` text read by
    /// [`Inventory::last_update_date`].
    pub fn last_update_calendar_date(&self) -> ResourceResult<EntityId> {
        if !self.record.is_ifc2x3() {
            return Err(self.record.not_in_schema("LastUpdateDate"));
        }
        self.record.required_ref("LastUpdateDate", "IfcCalendarDate")
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
