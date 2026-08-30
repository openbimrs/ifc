//! `IfcWorkPlan` and `IfcWorkSchedule`: the documents that hold tasks.
//!
//! # Slots, verified against IFC4 EXPRESS
//!
//! Both are `IfcWorkControl` subtypes, which is `IfcControl` -> `IfcObject`
//! -> `IfcRoot`. The inherited chain contributes six slots before either type
//! adds anything:
//!
//! ```text
//! 0 GlobalId        1 OwnerHistory    2 Name
//! 3 Description     4 ObjectType      5 Identification   (IfcControl)
//! -- IfcWorkControl --
//! 6 CreationDate    7 Creators        8 Purpose
//! 9 Duration       10 TotalFloat     11 StartTime
//! 12 FinishTime
//! -- subtype --
//! 13 PredefinedType
//! ```
//!
//! `StartTime` is slot 11 and required by the schema; `FinishTime` is 12 and
//! optional. A reader that assumes the subtype's `PredefinedType` sits right
//! after `Identification` -- as it does on most `IfcControl` subtypes -- lands
//! on `CreationDate` instead and reports a date as a type token.
//!
//! # Dates are returned as authored
//!
//! `IfcDateTime` is an ISO 8601 string. This crate does not parse it into a
//! calendar type: doing so would force a date library into a crate whose only
//! dependency is `ifc-model`, and would have to decide what to do with the
//! offsets and partial dates real files carry. The string is returned intact
//! and a caller that needs arithmetic parses it with the library it already
//! uses.

use ifc_model::{Entity, EntityId, Model, Value};

/// `IfcWorkControl` slots, shared by plans and schedules.
mod slot {
    /// `GlobalId` (from `IfcRoot`).
    pub const GLOBAL_ID: usize = 0;
    /// `Name` (from `IfcRoot`).
    pub const NAME: usize = 2;
    /// `Description` (from `IfcRoot`).
    pub const DESCRIPTION: usize = 3;
    /// `Identification` (from `IfcControl`).
    pub const IDENTIFICATION: usize = 5;
    /// `CreationDate`.
    pub const CREATION_DATE: usize = 6;
    /// `Purpose`.
    pub const PURPOSE: usize = 8;
    /// `Duration`, an ISO 8601 duration.
    pub const DURATION: usize = 9;
    /// `TotalFloat`, an ISO 8601 duration.
    pub const TOTAL_FLOAT: usize = 10;
    /// `StartTime`, required by the schema.
    pub const START_TIME: usize = 11;
    /// `FinishTime`.
    pub const FINISH_TIME: usize = 12;
    /// `PredefinedType`, contributed by the subtype.
    pub const PREDEFINED_TYPE: usize = 13;
}

/// Whether a work control is a plan or a schedule.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkControlKind {
    /// `IfcWorkPlan`: a container for schedules.
    Plan,
    /// `IfcWorkSchedule`: a container for tasks.
    Schedule,
}

impl WorkControlKind {
    fn from_type(name: &str) -> Option<Self> {
        if name.eq_ignore_ascii_case("IFCWORKPLAN") {
            Some(Self::Plan)
        } else if name.eq_ignore_ascii_case("IFCWORKSCHEDULE") {
            Some(Self::Schedule)
        } else {
            None
        }
    }
}

/// A borrowed view of an `IfcWorkPlan` or `IfcWorkSchedule`.
#[derive(Debug, Clone, Copy)]
pub struct WorkControl<'m> {
    id: EntityId,
    entity: &'m Entity,
    kind: WorkControlKind,
}

impl<'m> WorkControl<'m> {
    /// Wrap an entity if it is a work plan or work schedule.
    #[must_use]
    pub fn new(id: EntityId, entity: &'m Entity) -> Option<Self> {
        let kind = WorkControlKind::from_type(&entity.type_name)?;
        Some(Self { id, entity, kind })
    }

    /// The entity id in the file.
    #[must_use]
    pub fn id(&self) -> EntityId {
        self.id
    }

    /// Whether this is a plan or a schedule.
    #[must_use]
    pub fn kind(&self) -> WorkControlKind {
        self.kind
    }

    /// The `GlobalId` string.
    #[must_use]
    pub fn global_id(&self) -> Option<&'m str> {
        self.entity.text(slot::GLOBAL_ID)
    }

    /// The name.
    #[must_use]
    pub fn name(&self) -> Option<&'m str> {
        self.entity.text(slot::NAME)
    }

    /// The description.
    #[must_use]
    pub fn description(&self) -> Option<&'m str> {
        self.entity.text(slot::DESCRIPTION)
    }

    /// The user-facing identification code.
    #[must_use]
    pub fn identification(&self) -> Option<&'m str> {
        self.entity.text(slot::IDENTIFICATION)
    }

    /// Why the plan or schedule exists, as authored.
    #[must_use]
    pub fn purpose(&self) -> Option<&'m str> {
        self.entity.text(slot::PURPOSE)
    }

    /// When it was created, as an authored ISO 8601 string.
    #[must_use]
    pub fn creation_date(&self) -> Option<&'m str> {
        self.entity.text(slot::CREATION_DATE)
    }

    /// The planned start, as authored. Required by the schema.
    #[must_use]
    pub fn start_time(&self) -> Option<&'m str> {
        self.entity.text(slot::START_TIME)
    }

    /// The planned finish, as authored.
    #[must_use]
    pub fn finish_time(&self) -> Option<&'m str> {
        self.entity.text(slot::FINISH_TIME)
    }

    /// The overall duration, as an authored ISO 8601 duration.
    #[must_use]
    pub fn duration(&self) -> Option<&'m str> {
        self.entity.text(slot::DURATION)
    }

    /// The total float, as an authored ISO 8601 duration.
    #[must_use]
    pub fn total_float(&self) -> Option<&'m str> {
        self.entity.text(slot::TOTAL_FLOAT)
    }

    /// The predefined type token, without its dots.
    #[must_use]
    pub fn predefined_type(&self) -> Option<&'m str> {
        match self.entity.attribute(slot::PREDEFINED_TYPE)? {
            Value::Enum(token) => Some(token),
            _ => None,
        }
    }
}

/// Every work plan in the model, in file order.
#[must_use]
pub fn work_plans(model: &Model) -> Vec<WorkControl<'_>> {
    model
        .of_type("IFCWORKPLAN")
        .filter_map(|(id, entity)| WorkControl::new(id, entity))
        .collect()
}

/// Every work schedule in the model, in file order.
#[must_use]
pub fn work_schedules(model: &Model) -> Vec<WorkControl<'_>> {
    model
        .of_type("IFCWORKSCHEDULE")
        .filter_map(|(id, entity)| WorkControl::new(id, entity))
        .collect()
}
