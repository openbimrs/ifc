# Changelog -- ifc-element-type

All notable changes to the `ifc-element-type` crate are documented here.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this crate follows Semantic Versioning independently of its siblings:
a release here does not imply a release of any other crate in the family.

Changes up to and including 0.2.0 are recorded family-wide in the
[repository changelog](../../CHANGELOG.md); that file is the archive for
everything released before per-crate changelogs began.

## [Unreleased]

## [0.3.0] - 2026-09-29

### Added

- `TypeDraft` fields for the type-specific attributes IFC4 and IFC4X3
  require (#214): `operation_type` and `user_defined_operation_type`
  (`IfcDoorType`), `partitioning_type` and `user_defined_partitioning_type`
  (`IfcWindowType`), `parameter_takes_precedence` (both), `event_trigger_type`
  and `user_defined_event_trigger_type` (`IfcEventType`), and
  `assembly_place` (`IfcFurnitureType`, IFC2X3 too). Tokens are checked
  against the bound release's enumeration (`Invalid` otherwise); a value for
  an attribute the type does not declare is refused with
  `AuthoringNotInSchema`; `USERDEFINED` `event_trigger_type` without a
  non-blank `user_defined_event_trigger_type` is refused
  (`CorrectEventTriggerType`). These four types can now be authored in every
  release that declares them.
- `TypeDraft::new` and `SupertypeDraft::new`, and one builder setter per
  field, named after it (`TypeDraft::new().name("Beam").tag_or_long_description("B-1")`).

### Fixed

- `create_type` (and `create_supertype`), which take no model, no longer
  write `$` into an attribute IFC4X3 requires: a required attribute left
  unset is refused with `AuthoringRequired`, staging nothing, as the
  model-bound writers already did (#214).

### Changed (breaking)

- `TypeDraft` and `SupertypeDraft` are `#[non_exhaustive]`: struct literals
  outside the crate no longer compile. Use `new()` (or `default()`) and the
  setters; the fields stay public for reading and assignment.
- `create_type` refuses `IfcDoorType`, `IfcWindowType`, `IfcEventType` and
  `IfcFurnitureType` without their required type-specific attribute, where
  it wrote `$` before.
- `Slot6`, `Family`, `ElementType` and `SupertypeKind` are
  `#[non_exhaustive]`: a `match` on `Family` or `Slot6` needs a wildcard arm,
  and catalogue rows can no longer be built by struct literal outside the
  crate (use the generated constants).

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

- `create_type_in` and `create_supertype_in` (#202): `create_type` and
  `create_supertype` in the model's declared release. They bind it (a
  header without `FILE_SCHEMA` binds IFC4), lay the record out by attribute
  name from its table and check `predefined_type` against its enumeration.
  A type the release does not declare, or declares abstract, is refused
  with `EntityNotInSchema` (IFC2X3 has no `IfcDoorType` and no resource or
  process types; IFC4 has no `IfcBearingType` or `IfcBuiltElementType`). A
  token where the release declares no `PredefinedType` is refused with
  `AuthoringNotInSchema`. `OwnerHistory` is left `$`, which IFC2X3 forbids,
  so an IFC2X3 model is refused with `AuthoringRequired`.
- `create_type_with_owner_history` and
  `create_supertype_with_owner_history` (#202): the same, with a
  caller-supplied `IfcOwnerHistory`, which IFC2X3 requires. It must be in
  the model or staged on the transaction (`MissingEntity` otherwise) and be
  an `IfcOwnerHistory` (`Invalid` on `OwnerHistory` otherwise). None is ever
  invented. This follows `ifc-material` (#77) and `ifc-properties` (#191).
- `ElementTypeError::MultipleSchemas`, `UnsupportedSchema`,
  `EntityNotInSchema`, `AuthoringNotInSchema`, `AuthoringRequired` and
  `MissingEntity` (#202), appended. `ElementTypeError` is
  `#[non_exhaustive]`, so this is not breaking; its `Display` covers them.
- `ifc-schema` is now a dependency (it was a dev-dependency).

### Changed

- `create_type` and `create_supertype` are unchanged: they take no model
  and still write the catalogue's IFC4X3 layout with `OwnerHistory` `$`.
  That is never valid IFC2X3, and they cannot refuse a model that declares
  another release; their documentation now says so. They also still write
  `$` into the four required attributes `TypeDraft` has no field for
  (`IfcDoorType.OperationType`, `IfcWindowType.PartitioningType`,
  `IfcEventType.EventTriggerType`, `IfcFurnitureType.AssemblyPlace`), where
  the model-bound writers refuse with `AuthoringRequired`.
- `ElementTypeError` moved to its own module; the public path
  `ifc_element_type::ElementTypeError` is unchanged.

## [0.2.0] - 2026-09-22

First release under per-crate versioning. See the
[repository changelog](../../CHANGELOG.md) for the family-wide history
that produced this version.

[Unreleased]: https://github.com/openbimrs/ifc/compare/ifc-element-type-v0.3.0...HEAD
[0.3.0]: https://github.com/openbimrs/ifc/releases/tag/ifc-element-type-v0.3.0
[0.2.1]: https://github.com/openbimrs/ifc/releases/tag/ifc-element-type-v0.2.1
[0.2.0]: https://github.com/openbimrs/ifc/releases/tag/v0.2.0
