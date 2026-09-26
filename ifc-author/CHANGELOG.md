# Changelog -- ifc-author

All notable changes to the `ifc-author` crate are documented here.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this crate follows Semantic Versioning independently of its siblings:
a release here does not imply a release of any other crate in the family.

Changes up to and including 0.2.0 are recorded family-wide in the
[repository changelog](../CHANGELOG.md); that file is the archive for
everything released before per-crate changelogs began.

## [Unreleased]

### Fixed

- An attribute declared as a defined type that aliases an aggregate is an
  aggregate (#17). `IfcSite.RefLatitude`/`RefLongitude`
  (`IfcCompoundPlaneAngleMeasure = LIST [3:4] OF INTEGER`) were refused with
  `AggregateMismatch`, which blocked georeferencing. Their elements are now
  checked against the alias's element type.
- A slot that the entity or a supertype redeclares as `DERIVE` is written `*`
  automatically (#18). `IfcSIUnit.Dimensions` and the four derived slots of
  `IfcGeometricRepresentationSubContext` reported `MissingRequired`, so no unit
  assignment or Body/Axis subcontext could be authored. Passing
  `Value::Derived` explicitly is also accepted.

### Added

- `AuthorError::DerivedAttribute` refuses a value or `$` in a derived slot.
  `AuthorError::NotDerived` refuses `*` in a slot the schema does not derive.
  Before, `*` was accepted in any slot and the file was invalid. Both apply to
  `EntityBuilder` and `EntityEditor`.

## [0.2.0] - 2026-09-22

First release under per-crate versioning. See the
[repository changelog](../CHANGELOG.md) for the family-wide history
that produced this version.

[Unreleased]: https://github.com/openbimrs/ifc/compare/ifc-author-v0.2.0...HEAD
[0.2.0]: https://github.com/openbimrs/ifc/releases/tag/v0.2.0
