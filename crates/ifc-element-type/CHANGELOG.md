# Changelog -- ifc-element-type

All notable changes to the `ifc-element-type` crate are documented here.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this crate follows Semantic Versioning independently of its siblings:
a release here does not imply a release of any other crate in the family.

Changes up to and including 0.2.0 are recorded family-wide in the
[repository changelog](../../CHANGELOG.md); that file is the archive for
everything released before per-crate changelogs began.

## [Unreleased]

### Changed

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

[Unreleased]: https://github.com/openbimrs/ifc/compare/ifc-element-type-v0.2.1...HEAD
[0.2.1]: https://github.com/openbimrs/ifc/releases/tag/ifc-element-type-v0.2.1
[0.2.0]: https://github.com/openbimrs/ifc/releases/tag/v0.2.0
