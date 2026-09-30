# Changelog -- ifc-schedule

All notable changes to the `ifc-schedule` crate are documented here.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this crate follows Semantic Versioning independently of its siblings:
a release here does not imply a release of any other crate in the family.

Changes up to and including 0.2.0 are recorded family-wide in the
[repository changelog](../../CHANGELOG.md); that file is the archive for
everything released before per-crate changelogs began.

## [Unreleased]

### Added

- `RecurrenceDraft::time_periods` places `IfcTimePeriod` records in a
  pattern's `TimePeriods` (#233). `create_recurrence_pattern_in` and
  `create_time_period_in` bind the model's declared release: IFC2X3,
  which declares neither entity, is refused with `EntityNotInSchema`, and
  the pattern writer refuses a period reference that is absent
  (`MissingReference`) or not an `IfcTimePeriod` (`WrongReferenceType`).
  Nothing is staged on a refusal.
- `Recurrence` exposes `days` (`DayComponent`), `months`
  (`MonthComponent`) and `time_periods`, read as the new `TimePeriod`
  (`StartTime`, `EndTime` as authored); `recurrence_pattern(model, id)`
  reads a pattern referenced from outside a work calendar, such as
  `IfcTaskTimeRecurring.Recurrence`; `recurrence_slot::TIME_PERIODS` and
  `time_period_slot` name the slots (#233).
- `Lag` exposes `duration_type` (`IfcLagTime.DurationType`) and `name`
  (#236).
- `process_execution_order`: every `IfcProcess` (task, procedure, event)
  in a deterministic execution order (#236).
- `ScheduleReadError::SequenceDepthExceeded { start, limit }` (#236).

### Fixed

- `create_recurrence_pattern` writes all eight attributes IFC4 and IFC4X3
  declare for `IfcRecurrencePattern`; it wrote seven, so every authored
  pattern was a short record (#233).
- A sequence walk that reaches `MAX_SEQUENCE_DEPTH` is refused with
  `SequenceDepthExceeded` instead of returning a truncated result as if
  complete: `downstream_of` and `find_cycle` report it (#236). The walk
  uses an explicit stack, so the budget rather than the thread's stack
  bounds it.
- `find_cycle` walks from every `IfcProcess` in the declared release, not
  only `IfcTask`, so a cycle through events or procedures is found (#236).
- `execution_order` sorts over every `IfcProcess` and keeps the tasks, so a
  constraint through an event or procedure (task A, event E, task B) orders
  A before B, and a cycle through one is refused (#236). Its result is
  still tasks only.

## [0.3.0] - 2026-09-29

### Added

- `create_lag_time_in(tx, model, name, lag_value, duration_type)`: the
  model-bound `create_lag_time`. IFC2X3 declares no `IfcLagTime` and is
  refused with `EntityNotInSchema`, nothing staged; IFC4 and IFC4X3 stage
  exactly what `create_lag_time` stages. `create_lag_time` documents that
  it is for IFC4 and IFC4X3 only (#211).
- IFC2X3 work plans and work schedules can be authored (#214):
  `DateTimeValue` carries a date either as IFC4/IFC4X3 `IfcDateTime` text
  (`DateTimeValue::Text`, `From<&str>`) or as one of the IFC2X3
  `IfcDateTimeSelect` records (`Date(CalendarDate)`, `Time(LocalTime)`,
  `DateAndTime(CalendarDate, LocalTime)`). In IFC2X3
  `create_work_control_with_owner_history` stages the `IfcCalendarDate`,
  `IfcLocalTime` and `IfcDateAndTime` records and references them, only
  once the work control itself is accepted. Record forms are checked
  against the schema's rules before anything is staged
  (`IfcValidCalendarDate` with `IfcLeapYear`, `IfcValidTime`, and the
  ranges of `IfcMonthInYearNumber`, `IfcHourInDay`, `IfcMinuteInHour` and
  `IfcSecondInMinute`), refused with `InvalidValue`. `CalendarDate` and
  `LocalTime` are `#[non_exhaustive]`; `LocalTime` does not yet carry
  `Zone` or `DaylightSavingOffset`, which are written `$`.
- IFC2X3 sequences can be authored (#214): `TimeLag::Seconds` is the
  IFC2X3 `IfcRelSequence.TimeLag : IfcTimeMeasure`, `TimeLag::LagTime` the
  IFC4/IFC4X3 `IfcLagTime` reference. A non-finite lag is refused with
  `InvalidValue`.
- `ProcedureDraft::user_defined_procedure_type`, written as the IFC2X3
  `IfcProcedure.UserDefinedProcedureType` (#214), so a `USERDEFINED` IFC2X3
  procedure can be authored.
- Every draft has a `new` constructor taking its required fields
  (`TaskDraft::new(global_id)`, `WorkControlDraft::new(global_id,
  creation_date, start_time)`, `EventDraft::new(global_id)`,
  `ProcedureDraft::new(global_id)`, `RecurrenceDraft::new(recurrence_type)`,
  `EventTimeDraft::new()`, `TaskTimeDraft::new()`) and a setter per other
  field, named after it, as `ifc-resource`'s drafts are built.

### Fixed

- `create_work_control_with_owner_history`, `create_sequence_with_owner_history`
  and `create_procedure_with_owner_history` no longer refuse the IFC2X3
  records the drafts could not carry (#214). `IfcProcedure.WR4` is enforced
  instead: an IFC2X3 `USERDEFINED` procedure without a non-blank
  `user_defined_procedure_type` is refused with `InvalidValue`.
- The release-bound writers refuse a `SchemaVersion` this build carries no
  table for with `UnsupportedSchema` instead of panicking.

### Changed (breaking)

- The task, work-control and sequence readers bind the model's declared
  release and read every attribute by name from its table (#212). They
  read IFC4 slot constants from every file, so an IFC2X3 task answered its
  `Status` as the long description, `WorkMethod` as the status and
  `Priority` as the milestone flag, and an IFC2X3 work plan or schedule its
  `WorkControlType` as the predefined type. The header binds IFC2X3, IFC4
  or IFC4X3 (none reads as IFC4); IFC4X1, IFC4X2, unknown and multiple
  schemas are refused with the new `ScheduleReadError`. IFC4 and IFC4X3
  answers are unchanged.
- `tasks`, `work_plans`, `work_schedules`, `sequences`, `predecessors_of`,
  `successors_of`, `start_tasks`, `end_tasks`, `tasks_of_schedule` and
  `subtasks_of` return `Result<_, ScheduleReadError>`; `find_cycle` returns
  `Result<Option<SequenceCycle>, ScheduleReadError>`; `downstream_of` and
  `execution_order` return `ScheduleReadError::Cycle(SequenceCycle)` for a
  loop instead of a bare `SequenceCycle`.
- `Task::new(id, entity, release)` and `WorkControl::new(id, entity,
  release)` take the `SchemaVersion` to read against and return `Result`,
  refusing IFC4X1 and IFC4X2; `WorkControl::new` is `Ok(None)` for another
  entity. `Task::release` and `WorkControl::release` report the binding.
- An attribute IFC2X3 does not declare reads as `None`: `Task::
  long_description`, `predefined_type` and `task_time_ref`, and
  `WorkControl::predefined_type`. `Task::identification` reads IFC2X3's
  `TaskId` and `WorkControl::identification` its `Identifier`, which IFC4
  promoted to `Identification`. `WorkControl::work_control_type` reads
  IFC2X3's `WorkControlType`, which is not aliased to `PredefinedType`.
- `WorkControl::creation_date`, `start_time` and `finish_time` return
  `Option<AuthoredDateTime>` (IFC4/IFC4X3 text, or the IFC2X3
  `IfcDateTimeSelect` record), and `duration` and `total_float` return
  `Option<AuthoredDuration>` (IFC4/IFC4X3 text, or the IFC2X3
  `IfcTimeMeasure`), instead of `None` for a stated IFC2X3 value.
- `Sequence` gains `time_lag_measure`, IFC2X3's `IfcRelSequence.TimeLag`
  (an `IfcTimeMeasure` on the relationship); `lag` stays the IFC4/IFC4X3
  `IfcLagTime`.
- `TaskDraft`, `WorkControlDraft`, `EventDraft`, `EventTimeDraft`,
  `RecurrenceDraft`, `ProcedureDraft` and `TaskTimeDraft` are
  `#[non_exhaustive]`: build them with `new` and the setters instead of a
  struct literal. Their fields stay public to read and assign.
- `WorkControlDraft::creation_date` and `start_time` are
  `DateTimeValue<'a>` (were `&'a str`) and `finish_time` is
  `Option<DateTimeValue<'a>>` (was `Option<&'a str>`). The plain
  `create_work_control` refuses a record form with `InvalidValue`; with text
  its output is unchanged.
- `create_sequence_with_owner_history` takes `time_lag: Option<TimeLag>`
  (was `Option<EntityId>`); wrap an `IfcLagTime` id in `TimeLag::LagTime`.
  The plain `create_sequence` is unchanged.
- `ProcedureDraft` has the new `user_defined_procedure_type` field; the
  plain `create_procedure`, which writes IFC4/IFC4X3, refuses a value for
  it with `InvalidValue`, and the release-bound writer refuses it in IFC4
  and IFC4X3 with `AuthoringNotInSchema`.
- `#[non_exhaustive]` on the public enums and result structs a later
  release could extend: `SequenceType`, `DurationType`, `TaskTimeAnomaly`,
  `WorkControlKind`, `WorkTimeRole`, `RecurrenceType`, `Lag`, `Sequence`,
  `SequenceCycle`, `EventTime`, `Recurrence` and `WorkTime`. A `match`
  outside the crate needs a wildcard arm, and the structs can no longer be
  built outside it.

### Changed

- Depends on `ifc-schema` with its default features named explicitly
  (every bundled release), now that the workspace dependency turns them
  off for the facade's per-release features (#112).
- A model whose header declares `IFC4X1` or `IFC4X2` is refused with the
  existing unsupported-schema error. `ifc-schema` now bundles both
  releases, but no layout here is verified against them, so they are
  never read as IFC4 or IFC4X3.

## [0.2.1] - 2026-09-28

### Added

- `create_task_with_owner_history`, `create_sequence_with_owner_history`,
  `create_work_control_with_owner_history`,
  `assign_tasks_to_control_with_owner_history`,
  `nest_tasks_with_owner_history`, `create_work_calendar_with_owner_history`,
  `create_event_with_owner_history` and `create_procedure_with_owner_history`
  (#202). Each takes the model and a caller-supplied `IfcOwnerHistory` id,
  which IFC2X3 requires on every `IfcRoot`. The id must be in the model or
  staged on the transaction and must be an `IfcOwnerHistory`; a missing one
  is refused with `MissingReference`, another entity with
  `WrongReferenceType`. None is ever invented. Each binds the model's
  declared release (a header without `FILE_SCHEMA` binds IFC4; several or
  an unknown token are refused) and lays the record out by attribute name
  from that release's table. In IFC4 and IFC4X3 the record is the plain
  writer's with the reference in the optional slot. This follows
  `ifc-material` (#77), `ifc-properties` (#191) and `ifc-control` (#198).
- In IFC2X3 the variants write `Identification` as `IfcTask.TaskId` and
  `IfcProcedure.ProcedureID`, and `PredefinedType` as
  `IfcProcedure.ProcedureType` (renames stated in the IFC4 ADD2 TC1
  documentation). What IFC2X3 cannot hold is refused, never dropped:
  `IfcEvent` and `IfcWorkCalendar` (`EntityNotInSchema`); a task's
  `LongDescription`, `TaskTime` or `PredefinedType` (`AuthoringNotInSchema`);
  a sequence's `IfcLagTime` reference where IFC2X3 declares an
  `IfcTimeMeasure`, and a work control's ISO 8601 dates where it declares
  `IfcDateTimeSelect` (`AuthoringValueType`); a missing `TaskId`, `TimeLag`
  or `ProcedureType` (`AuthoringRequired`); and a `USERDEFINED` procedure,
  whose IFC2X3 WR4 needs a `UserDefinedProcedureType` the draft cannot
  carry (`InvalidValue`). Enumeration tokens are checked against the bound
  release, so an IFC4X3-only `IfcTaskTypeEnum` token is refused in IFC4.
- `ScheduleAuthoringError::MultipleSchemas`, `UnsupportedSchema`,
  `EntityNotInSchema`, `AuthoringNotInSchema`, `AuthoringValueType`,
  `AuthoringRequired`, `MissingReference` and `WrongReferenceType`, appended
  after `InvalidValue`. The enum is `#[non_exhaustive]`, so this is not
  breaking. `SchemaVersion` is re-exported, which they name. The crate now
  depends on `ifc-schema` for the bundled release tables.

### Changed

- The plain `IfcRoot` writers (`create_task`, `create_sequence`,
  `create_work_control`, `assign_tasks_to_control`, `nest_tasks`,
  `create_work_calendar`, `create_event`, `create_procedure`) take no model
  and so cannot see the declared release. Their signatures and output are
  unchanged: the layout IFC4 and IFC4X3 share, with `OwnerHistory` written
  `$`. They are now documented as IFC4/IFC4X3 only; in IFC2X3, where that
  record is invalid, use the `*_with_owner_history` variants (#202).
  `create_recurrence_pattern` moved to its own module; its path and
  behaviour are unchanged.
### Fixed

- `create_lag_time` writes `IfcLagTime.LagValue` as the member of
  `IfcTimeOrRatioSelect` it is (#201): a string as `IFCDURATION('P5D')` and
  a number as `IFCRATIOMEASURE(0.5)`, an integer as that REAL. A bare value
  in a SELECT slot does not say which member it is. A value already typed as
  `IFCDURATION` or `IFCRATIOMEASURE` is accepted; any other is refused, as
  before. `sequences` reads both forms, as before.

## [0.2.0] - 2026-09-22

First release under per-crate versioning. See the
[repository changelog](../../CHANGELOG.md) for the family-wide history
that produced this version.

[Unreleased]: https://github.com/openbimrs/ifc/compare/ifc-schedule-v0.3.0...HEAD
[0.3.0]: https://github.com/openbimrs/ifc/releases/tag/ifc-schedule-v0.3.0
[0.2.1]: https://github.com/openbimrs/ifc/releases/tag/ifc-schedule-v0.2.1
[0.2.0]: https://github.com/openbimrs/ifc/releases/tag/v0.2.0
