//! `IfcWorkPlan` and `IfcWorkSchedule`: the documents that hold tasks.
//!
//! # Read by name in the declared release (#212)
//!
//! Both are `IfcWorkControl` subtypes, which is `IfcControl` -> `IfcObject`
//! -> `IfcRoot`. IFC4 ADD2 TC1 and IFC4X3 ADD2 lay them out as below.
//! IFC2X3 TC1 names slot 5 `Identifier`, types the dates as
//! `IfcDateTimeSelect` records and `Duration` and `TotalFloat` as
//! `IfcTimeMeasure`, and ends with `WorkControlType`
//! (`IfcWorkControlTypeEnum`) and `UserDefinedControlType` instead of
//! `PredefinedType`. Every accessor looks its attribute up by name in the
//! model's declared release; a date or duration comes back in the form the
//! release declares it ([`AuthoredDateTime`], [`AuthoredDuration`]):
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
//! `IfcDateTime` is an ISO 8601 string, IFC2X3's `IfcDateTimeSelect` a
//! record. This crate does not parse either into a calendar type: doing so would force a date library into a crate whose only
//! dependency is `ifc-model`, and would have to decide what to do with the
//! offsets and partial dates real files carry. The string is returned intact
//! and a caller that needs arithmetic parses it with the library it already
//! uses.

use ifc_model::{Entity, EntityId, Model, Value};

use crate::error::ScheduleReadError;
use crate::release::ReadRelease;
use crate::SchemaVersion;

/// `IfcWorkControl` slots, shared by plans and schedules, in IFC4 and
/// IFC4X3. IFC2X3 differs from slot 13 on and in the types of 6 and 9..=12;
/// the accessors read by name, never through these.
pub mod slot {
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
#[non_exhaustive]
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

/// A date and time as the model's release types it.
///
/// IFC4 and IFC4X3 declare `IfcDateTime`, ISO 8601 text; IFC2X3 declares
/// `IfcDateTimeSelect`, a reference to an `IfcCalendarDate`, `IfcLocalTime`
/// or `IfcDateAndTime` record. Both are returned as authored.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum AuthoredDateTime<'m> {
    /// IFC4/IFC4X3 `IfcDateTime` text.
    Text(&'m str),
    /// IFC2X3 `IfcDateTimeSelect` record.
    Record(EntityId),
}

impl<'m> AuthoredDateTime<'m> {
    /// The text form, or `None` for a record.
    #[must_use]
    pub const fn text(self) -> Option<&'m str> {
        match self {
            Self::Text(text) => Some(text),
            Self::Record(_) => None,
        }
    }

    /// The record form, or `None` for text.
    #[must_use]
    pub const fn record(self) -> Option<EntityId> {
        match self {
            Self::Record(id) => Some(id),
            Self::Text(_) => None,
        }
    }
}

/// A duration as the model's release types it.
///
/// IFC4 and IFC4X3 declare `IfcDuration`, ISO 8601 duration text; IFC2X3
/// declares `IfcTimeMeasure`, a number in the project's time unit.
#[derive(Debug, Clone, Copy, PartialEq)]
#[non_exhaustive]
pub enum AuthoredDuration<'m> {
    /// IFC4/IFC4X3 `IfcDuration` text.
    Text(&'m str),
    /// IFC2X3 `IfcTimeMeasure`, in the project's time unit.
    TimeMeasure(f64),
}

impl<'m> AuthoredDuration<'m> {
    /// The text form, or `None` for a time measure.
    #[must_use]
    pub const fn text(self) -> Option<&'m str> {
        match self {
            Self::Text(text) => Some(text),
            Self::TimeMeasure(_) => None,
        }
    }
}

/// A borrowed view of an `IfcWorkPlan` or `IfcWorkSchedule`, read against
/// one release.
#[derive(Debug, Clone, Copy)]
pub struct WorkControl<'m> {
    id: EntityId,
    entity: &'m Entity,
    kind: WorkControlKind,
    release: ReadRelease,
}

impl<'m> WorkControl<'m> {
    /// Wrap an entity if it is a work plan or work schedule, read against
    /// `release`; `Ok(None)` for another entity.
    ///
    /// [`work_plans`] and [`work_schedules`] bind the model's declared
    /// release; use this when the release is known some other way.
    ///
    /// # Errors
    ///
    /// [`ScheduleReadError::UnsupportedSchema`] for a release the readers
    /// are not verified against (IFC4X1, IFC4X2).
    pub fn new(
        id: EntityId,
        entity: &'m Entity,
        release: SchemaVersion,
    ) -> Result<Option<Self>, ScheduleReadError> {
        let release = ReadRelease::of_version(release)?;
        Ok(Self::bound(id, entity, release))
    }

    fn bound(id: EntityId, entity: &'m Entity, release: ReadRelease) -> Option<Self> {
        let kind = WorkControlKind::from_type(&entity.type_name)?;
        Some(Self {
            id,
            entity,
            kind,
            release,
        })
    }

    /// The release this record is read against.
    #[must_use]
    pub fn release(&self) -> SchemaVersion {
        self.release.version()
    }

    fn entity_type(&self) -> &'static str {
        match self.kind {
            WorkControlKind::Plan => "IFCWORKPLAN",
            WorkControlKind::Schedule => "IFCWORKSCHEDULE",
        }
    }

    fn text(&self, attribute: &'static str) -> Option<&'m str> {
        self.release
            .text(self.entity_type(), self.entity, attribute)
    }

    fn date_time(&self, attribute: &'static str) -> Option<AuthoredDateTime<'m>> {
        match self
            .release
            .value(self.entity_type(), self.entity, attribute)?
        {
            Value::Ref(record) => Some(AuthoredDateTime::Record(*record)),
            value => value.unwrap_typed().as_text().map(AuthoredDateTime::Text),
        }
    }

    fn duration_value(&self, attribute: &'static str) -> Option<AuthoredDuration<'m>> {
        let value = self
            .release
            .value(self.entity_type(), self.entity, attribute)?
            .unwrap_typed();
        match value.as_text() {
            Some(text) => Some(AuthoredDuration::Text(text)),
            None => value.as_f64().map(AuthoredDuration::TimeMeasure),
        }
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
        self.text("GlobalId")
    }

    /// The name.
    #[must_use]
    pub fn name(&self) -> Option<&'m str> {
        self.text("Name")
    }

    /// The description.
    #[must_use]
    pub fn description(&self) -> Option<&'m str> {
        self.text("Description")
    }

    /// The user-facing identification code.
    #[must_use]
    pub fn identification(&self) -> Option<&'m str> {
        self.text("Identification")
    }

    /// Why the plan or schedule exists, as authored.
    #[must_use]
    pub fn purpose(&self) -> Option<&'m str> {
        self.text("Purpose")
    }

    /// When it was created, as authored: ISO 8601 text or an IFC2X3 date
    /// record.
    #[must_use]
    pub fn creation_date(&self) -> Option<AuthoredDateTime<'m>> {
        self.date_time("CreationDate")
    }

    /// The planned start, as authored. Required by the schema.
    #[must_use]
    pub fn start_time(&self) -> Option<AuthoredDateTime<'m>> {
        self.date_time("StartTime")
    }

    /// The planned finish, as authored.
    #[must_use]
    pub fn finish_time(&self) -> Option<AuthoredDateTime<'m>> {
        self.date_time("FinishTime")
    }

    /// The overall duration, as authored: an ISO 8601 duration or an IFC2X3
    /// time measure.
    #[must_use]
    pub fn duration(&self) -> Option<AuthoredDuration<'m>> {
        self.duration_value("Duration")
    }

    /// The total float, as authored: an ISO 8601 duration or an IFC2X3 time
    /// measure.
    #[must_use]
    pub fn total_float(&self) -> Option<AuthoredDuration<'m>> {
        self.duration_value("TotalFloat")
    }

    /// The predefined type token, without its dots. `None` in IFC2X3, which
    /// declares `WorkControlType` instead (see [`Self::work_control_type`]).
    #[must_use]
    pub fn predefined_type(&self) -> Option<&'m str> {
        self.release
            .token(self.entity_type(), self.entity, "PredefinedType")
    }

    /// IFC2X3's `WorkControlType` token (`IfcWorkControlTypeEnum`), without
    /// its dots. `None` in IFC4 and IFC4X3, which declare `PredefinedType`
    /// with their own enumerations instead; the two are not aliased.
    #[must_use]
    pub fn work_control_type(&self) -> Option<&'m str> {
        self.release
            .token(self.entity_type(), self.entity, "WorkControlType")
    }
}

/// Every work plan in the model, in file order, read against the model's
/// declared release.
///
/// # Errors
///
/// [`ScheduleReadError::UnsupportedSchema`] or
/// [`ScheduleReadError::MultipleSchemas`] for a header the readers cannot
/// bind; a header with no schema reads as IFC4.
pub fn work_plans(model: &Model) -> Result<Vec<WorkControl<'_>>, ScheduleReadError> {
    of_kind(model, "IFCWORKPLAN")
}

/// Every work schedule in the model, in file order, read against the
/// model's declared release.
///
/// # Errors
///
/// As [`work_plans`].
pub fn work_schedules(model: &Model) -> Result<Vec<WorkControl<'_>>, ScheduleReadError> {
    of_kind(model, "IFCWORKSCHEDULE")
}

fn of_kind<'m>(
    model: &'m Model,
    type_name: &str,
) -> Result<Vec<WorkControl<'m>>, ScheduleReadError> {
    let release = ReadRelease::of(model)?;
    Ok(model
        .of_type(type_name)
        .filter_map(|(id, entity)| WorkControl::bound(id, entity, release))
        .collect())
}
