//! Date and time inputs for authoring drafts, in the form each release
//! declares.

use ifc_model::EntityId;

/// A date and time for an authoring draft, in the form the release
/// declares it.
///
/// IFC4 and IFC4X3 declare `IfcDateTime`, ISO 8601 text written as given;
/// IFC2X3 declares `IfcDateTimeSelect`, a reference to an existing or
/// earlier-staged `IfcCalendarDate`, `IfcLocalTime` or `IfcDateAndTime`.
/// A form the release does not declare is refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum DateTimeInput<'a> {
    /// IFC4/IFC4X3 `IfcDateTime` text.
    Text(&'a str),
    /// An IFC2X3 `IfcDateTimeSelect` record.
    Record(EntityId),
}

impl<'a> From<&'a str> for DateTimeInput<'a> {
    fn from(text: &'a str) -> Self {
        Self::Text(text)
    }
}

impl From<EntityId> for DateTimeInput<'_> {
    fn from(record: EntityId) -> Self {
        Self::Record(record)
    }
}
