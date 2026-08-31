//! `IfcStyledItem` projection.

use ifc_model::EntityId;
use ifc_schema::SchemaVersion;

use crate::error::StyleResult;
use crate::view::Record;

#[derive(Debug, Clone, Copy)]
pub struct StyledItem<'m, 's> {
    record: Record<'m, 's>,
}

impl<'m, 's> StyledItem<'m, 's> {
    pub(crate) fn from_record(record: Record<'m, 's>) -> Self {
        Self { record }
    }

    pub fn id(&self) -> EntityId {
        self.record.id
    }

    pub fn item(&self) -> StyleResult<Option<EntityId>> {
        self.record.optional_ref("Item", "IfcRepresentationItem")
    }

    pub fn styles(&self) -> StyleResult<Vec<EntityId>> {
        let (members, maximum): (&[&str], Option<usize>) = match self.record.schema.version() {
            Some(SchemaVersion::Ifc2x3) => (&["IfcPresentationStyleAssignment"], Some(1)),
            Some(SchemaVersion::Ifc4) => (
                &["IfcPresentationStyle", "IfcPresentationStyleAssignment"],
                None,
            ),
            _ => (&["IfcPresentationStyle"], None),
        };
        self.record
            .required_refs_select("Styles", "IfcStyleAssignmentSelect", members, 1, maximum)
    }

    pub fn name(&self) -> StyleResult<Option<&'m str>> {
        self.record.optional_text("Name")
    }
}
