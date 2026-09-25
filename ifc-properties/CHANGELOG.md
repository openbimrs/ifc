# Changelog -- ifc-properties

All notable changes to the `ifc-properties` crate are documented here.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this crate follows Semantic Versioning independently of its siblings:
a release here does not imply a release of any other crate in the family.

Changes up to and including 0.2.0 are recorded family-wide in the
[repository changelog](../CHANGELOG.md); that file is the archive for
everything released before per-crate changelogs began.

## [Unreleased]

## [0.2.1] - 2026-09-25

### Added

- `exact_property` resolves IFC2X3 models (#48). It binds to the release the
  single `FILE_SCHEMA` token declares, IFC2X3 TC1 or IFC4 ADD2 TC1, and reads
  every structural fact from that release's bundled table: slot counts,
  entity domains (IFC2X3 `RelatedObjects` is `IfcObject`, IFC4's is
  `IfcObjectDefinition`), type objects (IFC2X3 `IfcDoorStyle` and
  `IfcWindowStyle` carry inherited properties like any other type), and the
  `IfcValue`/`IfcUnit` selects. An 8-slot IFC2X3 `IfcWall` resolves under an
  IFC2X3 header and is a slot mismatch under IFC4, and the reverse.
- `exact_schema(model)` reports the release a resolution binds to, so a
  consumer can bind vocabulary per release without re-parsing the header. It
  fails exactly where `exact_property` fails at model level.
  `SchemaVersion` is re-exported.
- `ExactPropertyError::NotInSchema` names a construct the declared release
  does not define, such as an `IfcDoorType`, an `IfcPropertySetDefinitionSet`,
  or an `IFCBINARY` value in an IFC2X3 file. A file that mixes releases fails
  closed instead of being read with the wrong table.
- `ExactPropertyError::UnsupportedRelationship`: a proper subtype of
  `IfcRelDefinesByProperties` or `IfcRelDefinesByType` that relates the
  queried object, such as IFC2X3 `IfcRelOverridesProperties`, is refused.
  Before, it was skipped as if absent, and the overridden value was returned.

### Changed

- IFC4X3 headers stay `UnsupportedSchema`: the table is bundled, but exact
  semantics for it are not verified yet (#33 tracks IFC4X1/IFC4X2).
- On five real IFC2X3 models, every sampled lookup failed with
  `UnsupportedSchema` before. Now 715 of 726 sampled lookups resolve; the other
  11 are `DuplicateMatchingSets`, where a property asked for by name alone is
  carried by two sets assigned to the same object. The same refusal occurs on
  the IFC4 control models.

## [0.2.0] - 2026-09-22

First release under per-crate versioning. See the
[repository changelog](../CHANGELOG.md) for the family-wide history
that produced this version.

[Unreleased]: https://github.com/openbimrs/ifc/compare/ifc-properties-v0.2.1...HEAD
[0.2.1]: https://github.com/openbimrs/ifc/releases/tag/ifc-properties-v0.2.1
[0.2.0]: https://github.com/openbimrs/ifc/releases/tag/v0.2.0
