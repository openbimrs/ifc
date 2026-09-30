//! `IfcEvent` and `IfcEventTime`.
//!
//! # Read by name in the declared release (#234)
//!
//! The readers bind the model's declared release like the task readers
//! (#212) and find every attribute by name in its table. IFC4 ADD2 TC1 and
//! IFC4X3 ADD2 declare both entities alike; IFC2X3 TC1 declares neither,
//! so an IFC2X3 model has no events. IFC4X1, IFC4X2 and a header with
//! several schemas are refused.
//!
//! # Slots, verified against IFC4 EXPRESS
//!
//! The slot modules below are the IFC4/IFC4X3 positions the modelless
//! writers lay records out with; the readers never go through them.
//!
//! `IfcEvent` is an `IfcProcess`, sharing the first seven slots with `IfcTask`:
//!
//! ```text
//! 0 GlobalId        1 OwnerHistory     2 Name
//! 3 Description     4 ObjectType       5 Identification    (IfcProcess)
//! 6 LongDescription                    7 PredefinedType
//! 8 EventTriggerType    9 UserDefinedEventTriggerType   10 EventOccurenceTime
//!
//! IfcEventTime  (IfcSchedulingTime)
//! 0 Name           1 DataOrigin       2 UserDefinedDataOrigin
//! 3 ActualDate     4 EarlyDate        5 LateDate           6 ScheduleDate
//! ```
//!
//! Note the schema's own spelling: `EventOccurenceTime`, with one `r`. It is
//! reproduced here because that is the attribute's name in the standard.
//!
//! `IfcEventTypeEnum` has no `MILESTONE` member; its tokens are `STARTEVENT`,
//! `ENDEVENT` and `INTERMEDIATEEVENT` (plus `USERDEFINED`/`NOTDEFINED`). A
//! milestone is an `IfcTask` with `IsMilestone` set.
//!
//! # An event is an instant
//!
//! Unlike a task, an event has no duration -- `IfcEventTime` states dates
//! only. This is why events are a separate projection rather than tasks with
//! a flag.

use ifc_model::{Entity, EntityId, Model};

use crate::error::ScheduleReadError;
use crate::release::ReadRelease;
use crate::SchemaVersion;

const EVENT: &str = "IFCEVENT";
const EVENT_TIME: &str = "IFCEVENTTIME";

/// `IfcEvent` slots.
pub mod slot {
    /// `GlobalId` (from `IfcRoot`).
    pub const GLOBAL_ID: usize = 0;
    /// `Name` (from `IfcRoot`).
    pub const NAME: usize = 2;
    /// `ObjectType` (from `IfcObject`). Required by the schema when
    /// `PredefinedType` is `USERDEFINED`.
    pub const OBJECT_TYPE: usize = 4;
    /// `Identification` (from `IfcProcess`).
    pub const IDENTIFICATION: usize = 5;
    /// `LongDescription` (from `IfcProcess`).
    pub const LONG_DESCRIPTION: usize = 6;
    /// `PredefinedType`.
    pub const PREDEFINED_TYPE: usize = 7;
    /// `EventTriggerType`.
    pub const EVENT_TRIGGER_TYPE: usize = 8;
    /// `UserDefinedEventTriggerType`. Required by the schema when
    /// `EventTriggerType` is `USERDEFINED`.
    pub const USER_DEFINED_EVENT_TRIGGER_TYPE: usize = 9;
    /// `EventOccurenceTime`, spelled as the schema spells it.
    pub const EVENT_OCCURENCE_TIME: usize = 10;
}

/// `IfcEventTime` slots.
pub mod time_slot {
    /// `Name` (from `IfcSchedulingTime`).
    pub const NAME: usize = 0;
    /// `ActualDate`.
    pub const ACTUAL_DATE: usize = 3;
    /// `EarlyDate`.
    pub const EARLY_DATE: usize = 4;
    /// `LateDate`.
    pub const LATE_DATE: usize = 5;
    /// `ScheduleDate`.
    pub const SCHEDULE_DATE: usize = 6;
}

/// When an event's dates are stated.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct EventTime {
    /// The `IfcEventTime` entity.
    pub id: EntityId,
    /// `Name` (from `IfcSchedulingTime`), as authored (#234).
    pub name: Option<String>,
    /// `DataOrigin` (`IfcDataOriginEnum`), the token without its dots
    /// (#234).
    pub data_origin: Option<String>,
    /// `UserDefinedDataOrigin`, as authored (#234).
    pub user_defined_data_origin: Option<String>,
    /// The actual date, as authored.
    pub actual: Option<String>,
    /// The earliest date, as authored.
    pub early: Option<String>,
    /// The latest date, as authored.
    pub late: Option<String>,
    /// The scheduled date, as authored.
    pub scheduled: Option<String>,
}

/// A borrowed view of an `IfcEvent`, read against one release.
#[derive(Debug, Clone, Copy)]
pub struct Event<'m> {
    id: EntityId,
    entity: &'m Entity,
    release: ReadRelease,
}

impl<'m> Event<'m> {
    /// The entity id in the file.
    #[must_use]
    pub fn id(&self) -> EntityId {
        self.id
    }

    /// The release this event is read against.
    #[must_use]
    pub fn release(&self) -> SchemaVersion {
        self.release.version()
    }

    fn text(&self, attribute: &'static str) -> Option<&'m str> {
        self.release.text(EVENT, self.entity, attribute)
    }

    /// The `GlobalId` string.
    #[must_use]
    pub fn global_id(&self) -> Option<&'m str> {
        self.text("GlobalId")
    }

    /// The event name.
    #[must_use]
    pub fn name(&self) -> Option<&'m str> {
        self.text("Name")
    }

    /// The object type, which the schema requires when `PredefinedType` is
    /// `USERDEFINED`.
    #[must_use]
    pub fn object_type(&self) -> Option<&'m str> {
        self.text("ObjectType")
    }

    /// The user-facing identification code.
    #[must_use]
    pub fn identification(&self) -> Option<&'m str> {
        self.text("Identification")
    }

    /// The long description.
    #[must_use]
    pub fn long_description(&self) -> Option<&'m str> {
        self.text("LongDescription")
    }

    /// The predefined type token, without its dots.
    #[must_use]
    pub fn predefined_type(&self) -> Option<&'m str> {
        self.release.token(EVENT, self.entity, "PredefinedType")
    }

    /// What triggers the event, as an enum token.
    #[must_use]
    pub fn trigger_type(&self) -> Option<&'m str> {
        self.release.token(EVENT, self.entity, "EventTriggerType")
    }

    /// `UserDefinedEventTriggerType`, which the schema requires when
    /// `EventTriggerType` is `USERDEFINED` (#234).
    #[must_use]
    pub fn user_defined_trigger_type(&self) -> Option<&'m str> {
        self.text("UserDefinedEventTriggerType")
    }

    /// The event's stated times, if any: `EventOccurenceTime`, when it
    /// references an `IfcEventTime`.
    #[must_use]
    pub fn time(&self, model: &Model) -> Option<EventTime> {
        let id = self
            .release
            .reference(EVENT, self.entity, "EventOccurenceTime")?;
        let entity = model.get(id)?;
        if !self.release.is_a(&entity.type_name, EVENT_TIME) {
            return None;
        }
        let text = |attribute| {
            self.release
                .text(EVENT_TIME, entity, attribute)
                .map(str::to_string)
        };
        Some(EventTime {
            id,
            name: text("Name"),
            data_origin: self
                .release
                .token(EVENT_TIME, entity, "DataOrigin")
                .map(str::to_string),
            user_defined_data_origin: text("UserDefinedDataOrigin"),
            actual: text("ActualDate"),
            early: text("EarlyDate"),
            late: text("LateDate"),
            scheduled: text("ScheduleDate"),
        })
    }
}

/// Every event in the model, in file order, read against the model's
/// declared release (#234).
///
/// IFC2X3 declares no `IfcEvent`, so an IFC2X3 model has none.
///
/// # Errors
///
/// [`ScheduleReadError::UnsupportedSchema`] or
/// [`ScheduleReadError::MultipleSchemas`] for a header the readers cannot
/// bind; a header with no schema reads as IFC4.
pub fn read_events(model: &Model) -> Result<Vec<Event<'_>>, ScheduleReadError> {
    Ok(events_in(model, ReadRelease::of(model)?))
}

/// Every event in the model, in file order.
///
/// Reads against the model's declared release when the readers can bind
/// it. A header they cannot bind (IFC4X1, IFC4X2, several schemas) is read
/// as IFC4, as this function always did; [`read_events`] refuses it
/// instead.
#[deprecated(
    note = "reads an unverified or multi-schema header as IFC4; use `read_events`, which refuses it (#234)"
)]
#[must_use]
pub fn events(model: &Model) -> Vec<Event<'_>> {
    ReadRelease::of(model)
        .or_else(|_| ReadRelease::of_version(SchemaVersion::Ifc4))
        .map_or_else(|_| Vec::new(), |release| events_in(model, release))
}

fn events_in(model: &Model, release: ReadRelease) -> Vec<Event<'_>> {
    if !release.declares(EVENT) {
        return Vec::new();
    }
    release
        .instances_of(model, EVENT)
        .into_iter()
        .filter_map(|id| {
            Some(Event {
                id,
                entity: model.get(id)?,
                release,
            })
        })
        .collect()
}
