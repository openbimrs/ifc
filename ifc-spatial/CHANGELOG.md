# Changelog -- ifc-spatial

All notable changes to the `ifc-spatial` crate are documented here.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this crate follows Semantic Versioning independently of its siblings:
a release here does not imply a release of any other crate in the family.

Changes up to and including 0.2.0 are recorded family-wide in the
[repository changelog](../CHANGELOG.md); that file is the archive for
everything released before per-crate changelogs began.

## [Unreleased]

### Added

- `SpatialTree::anomalies()` and `SpatialAnomaly` (#54). An element placed
  by two `IfcRelContainedInSpatialStructure`s is reported as
  `ContainedTwice`, and a container aggregated by two parents as
  `AggregatedTwice`. Each names the element or child, the kept and rejected
  parent, and the rejected relationship. The first relationship applied
  still wins. Restating the same parent is not reported.

### Changed

- For invalid files only: the rejected container no longer lists a doubly
  contained element in `SpatialNode::elements` / `elements_of`. Before, the
  element appeared in both containers while `container_of` returned only the
  first, so the two views disagreed. Valid files are unaffected.

## [0.2.0] - 2026-09-22

First release under per-crate versioning. See the
[repository changelog](../CHANGELOG.md) for the family-wide history
that produced this version.

[Unreleased]: https://github.com/openbimrs/ifc/compare/ifc-spatial-v0.2.0...HEAD
[0.2.0]: https://github.com/openbimrs/ifc/releases/tag/v0.2.0
