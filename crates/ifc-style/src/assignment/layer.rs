//! IFC2x3 `IfcPresentationStyleAssignment` wrapper projection.

use ifc_model::EntityId;

use crate::error::StyleResult;
use crate::view::Record;
use crate::{assignment::presentation_style_members, PresentationStyleMember};

/// Borrowed projection of the IFC2x3 `IfcPresentationStyleAssignment` wrapper
/// entity, which groups a list of presentation styles under one id.
#[derive(Debug, Clone, Copy)]
pub struct PresentationStyleAssignment<'m, 's> {
    record: Record<'m, 's>,
}

impl<'m, 's> PresentationStyleAssignment<'m, 's> {
    pub(crate) fn from_record(record: Record<'m, 's>) -> Self {
        Self { record }
    }

    /// The entity id of this `IfcPresentationStyleAssignment`.
    pub fn id(&self) -> EntityId {
        self.record.id
    }

    /// The `Styles` select, with any `IfcNullStyle.NULL` members dropped.
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

    /// The raw `Styles` select, one member per list entry, preserving
    /// `IfcNullStyle.NULL` placeholders.
    pub fn members(&self) -> StyleResult<Vec<PresentationStyleMember>> {
        presentation_style_members(self.record, "Styles", 1)
    }
}
