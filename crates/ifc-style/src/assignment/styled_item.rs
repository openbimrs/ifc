//! `IfcStyledItem` projection.

use ifc_model::EntityId;
use ifc_schema::SchemaVersion;

use crate::error::{StyleError, StyleResult};
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
    /// `IfcPresentationStyleAssignment` member; IFC4, IFC4X1 and IFC4X2 also
    /// permit direct `IfcPresentationStyle` references alongside the legacy
    /// wrapper (`IfcStyleAssignmentSelect`); IFC4X3, and a schema of no
    /// recognised release, accept only direct `IfcPresentationStyle`
    /// references. A recognised release not listed here is refused with
    /// [`StyleError::UnsupportedSchema`] rather than read by a neighbour's
    /// rule.
    ///
    /// [`StyleError::UnsupportedSchema`]: crate::StyleError::UnsupportedSchema
    pub fn styles(&self) -> StyleResult<Vec<EntityId>> {
        let (members, maximum): (&[&str], Option<usize>) = match self.record.schema.version() {
            Some(SchemaVersion::Ifc2x3) => (&["IfcPresentationStyleAssignment"], Some(1)),
            Some(SchemaVersion::Ifc4 | SchemaVersion::Ifc4x1 | SchemaVersion::Ifc4x2) => (
                &["IfcPresentationStyle", "IfcPresentationStyleAssignment"],
                None,
            ),
            Some(SchemaVersion::Ifc4x3) | None => (&["IfcPresentationStyle"], None),
            Some(_) => {
                return Err(StyleError::UnsupportedSchema {
                    schema: self.record.schema.name().to_owned(),
                })
            }
        };
        self.record
            .required_refs_select("Styles", "IfcStyleAssignmentSelect", members, 1, maximum)
    }

    /// The `Name` attribute, when authored.
    pub fn name(&self) -> StyleResult<Option<&'m str>> {
        self.record.optional_text("Name")
    }
}
