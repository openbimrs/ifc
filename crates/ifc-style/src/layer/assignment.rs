//! Presentation layer membership and layer-level styles.

use ifc_model::EntityId;
use ifc_schema::SchemaVersion;

use crate::assignment::{presentation_style_members, PresentationStyleMember};
use crate::error::StyleResult;
use crate::view::Record;

/// Borrowed projection of `IfcPresentationLayerAssignment` (and its
/// `IfcPresentationLayerWithStyle` subtype).
#[derive(Debug, Clone, Copy)]
pub struct PresentationLayer<'m, 's> {
    pub(crate) record: Record<'m, 's>,
}

impl<'m, 's> PresentationLayer<'m, 's> {
    pub(crate) fn from_record(record: Record<'m, 's>) -> Self {
        Self { record }
    }

    /// The entity id of this layer assignment.
    pub fn id(&self) -> EntityId {
        self.record.id
    }

    /// The `Name` attribute. Mandatory.
    pub fn name(&self) -> StyleResult<&'m str> {
        self.record.required_text("Name")
    }

    /// The `Description` attribute, when authored.
    pub fn description(&self) -> StyleResult<Option<&'m str>> {
        self.record.optional_text("Description")
    }

    /// The `Identifier` attribute, when authored.
    pub fn identifier(&self) -> StyleResult<Option<&'m str>> {
        self.record.optional_text("Identifier")
    }

    /// The `AssignedItems` select: the `IfcRepresentation`s and/or
    /// `IfcRepresentationItem`s placed on this layer.
    pub fn assigned_items(&self) -> StyleResult<Vec<EntityId>> {
        self.record.required_refs_select(
            "AssignedItems",
            "IfcLayeredItem",
            &["IfcRepresentation", "IfcRepresentationItem"],
            1,
            None,
        )
    }

    /// The `LayerStyles` attribute of an `IfcPresentationLayerWithStyle`.
    /// Returns an empty vector when the entity has no `LayerStyles` slot at
    /// all (a plain `IfcPresentationLayerAssignment`). On IFC2x3, resolves
    /// through the legacy `IfcPresentationStyleAssignment` select and drops
    /// `IfcNullStyle.NULL` members; on later schemas, reads direct
    /// `IfcPresentationStyle` references.
    pub fn layer_styles(&self) -> StyleResult<Vec<EntityId>> {
        if !self.record.has_attribute("LayerStyles") {
            return Ok(Vec::new());
        }
        if self.record.schema.version() == Some(SchemaVersion::Ifc2x3) {
            return Ok(presentation_style_members(self.record, "LayerStyles", 0)?
                .into_iter()
                .filter_map(|member| match member {
                    PresentationStyleMember::Style(id) => Some(id),
                    PresentationStyleMember::Null => None,
                })
                .collect());
        }
        self.record
            .required_refs("LayerStyles", "IfcPresentationStyle", 0, None)
    }
}
