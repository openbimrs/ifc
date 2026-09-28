# Changelog -- ifc-schedule

All notable changes to the `ifc-schedule` crate are documented here.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this crate follows Semantic Versioning independently of its siblings:
a release here does not imply a release of any other crate in the family.

Changes up to and including 0.2.0 are recorded family-wide in the
[repository changelog](../../CHANGELOG.md); that file is the archive for
everything released before per-crate changelogs began.

## [Unreleased]

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

[Unreleased]: https://github.com/openbimrs/ifc/compare/ifc-schedule-v0.2.0...HEAD
[0.2.0]: https://github.com/openbimrs/ifc/releases/tag/v0.2.0
