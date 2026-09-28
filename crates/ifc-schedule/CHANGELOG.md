# Changelog -- ifc-schedule

All notable changes to the `ifc-schedule` crate are documented here.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this crate follows Semantic Versioning independently of its siblings:
a release here does not imply a release of any other crate in the family.

Changes up to and including 0.2.0 are recorded family-wide in the
[repository changelog](../../CHANGELOG.md); that file is the archive for
everything released before per-crate changelogs began.

## [Unreleased]

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

[Unreleased]: https://github.com/openbimrs/ifc/compare/ifc-schedule-v0.2.1...HEAD
[0.2.1]: https://github.com/openbimrs/ifc/releases/tag/ifc-schedule-v0.2.1
[0.2.0]: https://github.com/openbimrs/ifc/releases/tag/v0.2.0
