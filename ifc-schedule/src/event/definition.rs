//! `IfcEvent` and `IfcEventTime`.
//!
//! # Slots, verified against IFC4 EXPRESS
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
//! # An event is an instant
//!
//! Unlike a task, an event has no duration -- `IfcEventTime` states dates
//! only. This is why events are a separate projection rather than tasks with
//! a flag.

use ifc_model::{Entity, EntityId, Model, Value};

/// `IfcEvent` slots.
mod slot {
    /// `Name` (from `IfcRoot`).
    pub const NAME: usize = 2;
    /// `Identification` (from `IfcProcess`).
    pub const IDENTIFICATION: usize = 5;
    /// `LongDescription` (from `IfcProcess`).
    pub const LONG_DESCRIPTION: usize = 6;
    /// `PredefinedType`.
    pub const PREDEFINED_TYPE: usize = 7;
    /// `EventTriggerType`.
    pub const EVENT_TRIGGER_TYPE: usize = 8;
    /// `EventOccurenceTime`, spelled as the schema spells it.
    pub const EVENT_OCCURENCE_TIME: usize = 10;
}

/// `IfcEventTime` slots.
mod time_slot {
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
pub struct EventTime {
    /// The `IfcEventTime` entity.
    pub id: EntityId,
    /// The actual date, as authored.
    pub actual: Option<String>,
    /// The earliest date, as authored.
    pub early: Option<String>,
    /// The latest date, as authored.
    pub late: Option<String>,
    /// The scheduled date, as authored.
    pub scheduled: Option<String>,
}

/// A borrowed view of an `IfcEvent`.
#[derive(Debug, Clone, Copy)]
pub struct Event<'m> {
    id: EntityId,
    entity: &'m Entity,
}

impl<'m> Event<'m> {
    /// The entity id in the file.
    #[must_use]
    pub fn id(&self) -> EntityId {
        self.id
    }

    /// The event name.
    #[must_use]
    pub fn name(&self) -> Option<&'m str> {
        self.entity.text(slot::NAME)
    }

    /// The user-facing identification code.
    #[must_use]
    pub fn identification(&self) -> Option<&'m str> {
        self.entity.text(slot::IDENTIFICATION)
    }

    /// The long description.
    #[must_use]
    pub fn long_description(&self) -> Option<&'m str> {
        self.entity.text(slot::LONG_DESCRIPTION)
    }

    /// The predefined type token, without its dots.
    #[must_use]
    pub fn predefined_type(&self) -> Option<&'m str> {
        match self.entity.attribute(slot::PREDEFINED_TYPE)? {
            Value::Enum(token) => Some(token),
            _ => None,
        }
    }

    /// What triggers the event, as an enum token.
    #[must_use]
    pub fn trigger_type(&self) -> Option<&'m str> {
        match self.entity.attribute(slot::EVENT_TRIGGER_TYPE)? {
            Value::Enum(token) => Some(token),
            _ => None,
        }
    }

    /// The event's stated times, if any.
    #[must_use]
    pub fn time(&self, model: &Model) -> Option<EventTime> {
        let id = match self.entity.attribute(slot::EVENT_OCCURENCE_TIME)? {
            Value::Ref(id) => *id,
            _ => return None,
        };
        let entity = model.get(id)?;
        if !entity.type_name.eq_ignore_ascii_case("IFCEVENTTIME") {
            return None;
        }
        Some(EventTime {
            id,
            actual: entity.text(time_slot::ACTUAL_DATE).map(str::to_string),
            early: entity.text(time_slot::EARLY_DATE).map(str::to_string),
            late: entity.text(time_slot::LATE_DATE).map(str::to_string),
            scheduled: entity.text(time_slot::SCHEDULE_DATE).map(str::to_string),
        })
    }
}

/// Every event in the model, in file order.
#[must_use]
pub fn events(model: &Model) -> Vec<Event<'_>> {
    model
        .of_type("IFCEVENT")
        .map(|(id, entity)| Event { id, entity })
        .collect()
}
