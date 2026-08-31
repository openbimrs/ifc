//! IFC2x3 `IfcPresentationStyleAssignment` wrapper projection.

use ifc_model::EntityId;

use crate::error::StyleResult;
use crate::view::Record;
use crate::{assignment::presentation_style_members, PresentationStyleMember};

#[derive(Debug, Clone, Copy)]
pub struct PresentationStyleAssignment<'m, 's> {
    record: Record<'m, 's>,
}

impl<'m, 's> PresentationStyleAssignment<'m, 's> {
    pub(crate) fn from_record(record: Record<'m, 's>) -> Self {
        Self { record }
    }

    pub fn id(&self) -> EntityId {
        self.record.id
    }

    pub fn styles(&self) -> StyleResult<Vec<EntityId>> {
        Ok(self
            .members()?
            .into_iter()
            .filter_map(|member| match member {
                PresentationStyleMember::Style(id) => Some(id),
                PresentationStyleMember::Null => None,
            })
            .collect())
    }

    pub fn members(&self) -> StyleResult<Vec<PresentationStyleMember>> {
        presentation_style_members(self.record, "Styles", 1)
    }
}
