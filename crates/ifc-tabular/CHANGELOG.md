# Changelog -- ifc-tabular

All notable changes to the `ifc-tabular` crate are documented here.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this crate follows Semantic Versioning independently of its siblings:
a release here does not imply a release of any other crate in the family.

Changes up to and including 0.2.0 are recorded family-wide in the
[repository changelog](../../CHANGELOG.md); that file is the archive for
everything released before per-crate changelogs began.

## [Unreleased]

## [0.2.1] - 2026-09-27

### Added

- Borrowed read views: `TabularView` reads `IfcTable` (rows and columns)
  and `IfcRegularTimeSeries`/`IfcIrregularTimeSeries` with their value
  records under a declared IFC4 or IFC4X3 schema, locating slots by name.
  WR1 (ragged row), WR2 (more than one heading), malformed slots, arity
  mismatches, empty lists and dangling or mistyped references are reported
  as `TabularIssue`s instead of being dropped. IFC2x3 is refused with
  `TabularReadError::UnsupportedSchema` (#120).

## [0.2.0] - 2026-09-22

First release under per-crate versioning. See the
[repository changelog](../../CHANGELOG.md) for the family-wide history
that produced this version.

[Unreleased]: https://github.com/openbimrs/ifc/compare/ifc-tabular-v0.2.1...HEAD
[0.2.1]: https://github.com/openbimrs/ifc/releases/tag/ifc-tabular-v0.2.1
[0.2.0]: https://github.com/openbimrs/ifc/releases/tag/v0.2.0
