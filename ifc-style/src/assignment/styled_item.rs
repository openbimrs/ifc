//! `IfcStyledItem` projection.

use ifc_model::EntityId;
use ifc_schema::SchemaVersion;

use crate::error::StyleResult;
use crate::view::Record;

/// Borrowed projection of `IfcStyledItem`: binds one or more presentation
/// styles to a single `IfcRepresentationItem`.
#[derive(Debug, Clone, Copy)]
pub struct StyledItem<'m, 's> {
    record: Record<'m, 's>,
}

impl<'m, 's> StyledItem<'m, 's> {
    pub(crate) fn from_record(record: Record<'m, 's>) -> Self {
        Self { record }
    }

    /// The entity id of this `IfcStyledItem`.
    pub fn id(&self) -> EntityId {
        self.record.id
    }

    /// The `Item` reference to the styled `IfcRepresentationItem`. `None`
    /// for an `IfcStyledItem` that only carries a `Name` with no target item.
    pub fn item(&self) -> StyleResult<Option<EntityId>> {
        self.record.optional_ref("Item", "IfcRepresentationItem")
    }

    /// The `Styles` select. IFC2x3 requires exactly one
    /// `IfcPresentationStyleAssignment` member; IFC4 also permits direct
    /// `IfcPresentationStyle` references alongside the legacy wrapper; later
    /// schemas accept only direct `IfcPresentationStyle` references.
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

    /// The `Name` attribute, when authored.
    pub fn name(&self) -> StyleResult<Option<&'m str>> {
        self.record.optional_text("Name")
    }
}
