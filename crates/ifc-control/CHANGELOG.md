# Changelog -- ifc-control

All notable changes to the `ifc-control` crate are documented here.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this crate follows Semantic Versioning independently of its siblings:
a release here does not imply a release of any other crate in the family.

Changes up to and including 0.2.0 are recorded family-wide in the
[repository changelog](../../CHANGELOG.md); that file is the archive for
everything released before per-crate changelogs began.

## [Unreleased]

### Changed (breaking)

- `ControlDraft` and `ControlAssignmentDraft` are `#[non_exhaustive]`
  (#214). Struct literals no longer compile outside the crate: build them
  with `ControlDraft::new()` and
  `ControlAssignmentDraft::new(global_id, control, related_objects)` plus
  field-named setters (`.name("Permit")`). Fields stay public.

## [0.2.2] - 2026-09-28

### Added

- `create_control_with_owner_history` and
  `assign_to_control_with_owner_history` (#198, #202). Each binds the
  model's declared release (a header without `FILE_SCHEMA` binds IFC4) and
  takes a caller-supplied `IfcOwnerHistory` id, which IFC2X3 requires on
  every `IfcRoot`. It must be in the model or staged on the transaction
  (`UnknownEntity` otherwise) and be an `IfcOwnerHistory`
  (`AuthoringInvalid`). None is ever invented. In IFC4 and IFC4X3 the
  record is the plain writer's with the reference in the optional slot.
- `ControlError::MultipleSchemas`, `UnsupportedSchema`,
  `AuthoringNotInSchema`, `AuthoringValueType` and `AuthoringRequired`,
  appended to the `#[non_exhaustive]` enum, so not breaking.

### Fixed

- `create_control` no longer panics on an IFC2X3 `IfcPermit`,
  `IfcActionRequest` or `IfcPerformanceHistory` (#198). It indexed IFC4
  positions into their six-attribute IFC2X3 records. `create_control` and
  `assign_to_control` now lay records out by attribute name from the table
  they are given, so a value that table does not declare is refused
  (`AuthoringNotInSchema`: an IFC2X3 permit has no `PredefinedType`,
  `Status` or `LongDescription`), one it cannot hold is refused
  (`AuthoringValueType`), and a required one left unset is refused
  (`AuthoringRequired`). In IFC2X3, `identification` is written as the
  entity's own identifier (`PermitID`, `RequestID`, `ID`), which the IFC4
  documentation records as renamed to `Identification`.
  **Behaviour change for IFC2X3 callers:** `IfcRoot.OwnerHistory` is
  mandatory there, so both plain writers refuse an IFC2X3 schema with
  `AuthoringRequired { attribute: "OwnerHistory", .. }` and stage nothing
  (an IFC2X3 project order used to be written with `$`); use the
  `*_with_owner_history` variants. IFC4 and IFC4X3 records are unchanged,
  record for record.

## [0.2.1] - 2026-09-27

### Added

- `assign_to_control` and `ControlAssignmentDraft` stage an
  `IfcRelAssignsToControl` whose relating control is a permit, project
  order, action request or performance history. Empty, duplicated and
  self-referencing `RelatedObjects`, members that are not
  `IfcObjectDefinition`s, and missing references are refused;
  `RelatedObjectsType` is left unset (#99).
- `ControlError::ForeignControl` refuses a relating control another crate
  owns (cost schedules, cost items, work controls).
- `ControlKind::ALL` lists the four owned controls.

## [0.2.0] - 2026-09-22

First release under per-crate versioning. See the
[repository changelog](../../CHANGELOG.md) for the family-wide history
that produced this version.

[Unreleased]: https://github.com/openbimrs/ifc/compare/ifc-control-v0.2.2...HEAD
[0.2.2]: https://github.com/openbimrs/ifc/releases/tag/ifc-control-v0.2.2
[0.2.1]: https://github.com/openbimrs/ifc/releases/tag/ifc-control-v0.2.1
[0.2.0]: https://github.com/openbimrs/ifc/releases/tag/v0.2.0
