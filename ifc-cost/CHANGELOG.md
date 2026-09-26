# Changelog -- ifc-cost

All notable changes to the `ifc-cost` crate are documented here.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this crate follows Semantic Versioning independently of its siblings:
a release here does not imply a release of any other crate in the family.

Changes up to and including 0.2.0 are recorded family-wide in the
[repository changelog](../CHANGELOG.md); that file is the archive for
everything released before per-crate changelogs began.

## [Unreleased]

### Added

- `nesting_anomalies(model)` and `CostAnomaly::NestedTwice { item, kept,
  rejected, relation }` (#57). A cost item that two `IfcRelNests` place
  under different parents is reported; `Nests` is `SET [0:1]`.

### Fixed

- A cost item nested under two parents is no longer counted twice (#57).
  `parent_of` already returned the first parent, but `children_of` listed
  the item under both. So `descendants_of` and `rolled_up_total` included it
  under each parent, and summing over `roots()` double-counted its value.
  Now only the kept parent (first `IfcRelNests` in file order) lists it,
  and a child listed twice under one parent is listed once. Output changes
  only for files that violate the schema.

## [0.2.0] - 2026-09-22

First release under per-crate versioning. See the
[repository changelog](../CHANGELOG.md) for the family-wide history
that produced this version.

[Unreleased]: https://github.com/openbimrs/ifc/compare/ifc-cost-v0.2.0...HEAD
[0.2.0]: https://github.com/openbimrs/ifc/releases/tag/v0.2.0
