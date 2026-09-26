# Changelog -- ifc-validate

All notable changes to the `ifc-validate` crate are documented here.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this crate follows Semantic Versioning independently of its siblings:
a release here does not imply a release of any other crate in the family.

Changes up to and including 0.2.0 are recorded family-wide in the
[repository changelog](../CHANGELOG.md); that file is the archive for
everything released before per-crate changelogs began.

## [Unreleased]

This release is **breaking** (0.2 -> 0.3): `Severity` gains a variant,
`Summary` gains a field, and `Budget::max_depth` is removed.

### Added

- `Severity::EvaluationError` and `Finding::evaluation_error`: an
  implemented rule that applies to an instance but cannot be decided for it
  -- the schema tables lack an attribute the rule reads, an operand has a
  shape the rule cannot reason about, or a target the rule must type-test is
  absent -- is now reported under the rule's own id instead of being skipped
  silently (#115). `Summary::evaluation_errors` counts them, and
  `Report::is_conformant` is `false` while any is present. An unset (`$`)
  operand is still not an evaluation error: optional operands are guarded
  by the rules themselves and mandatory ones are `structure.required.missing`.
- Every rule id the crate emits is pinned by an adversarial pair of
  fixtures, and an inventory read from the crate's source fails the build
  when a new id ships without one (#114).

### Changed

- **Breaking:** `Severity` is `#[non_exhaustive]` and has the new
  `EvaluationError` variant, ordered after `Error`; exhaustive matches
  outside the crate must add a wildcard arm.
- **Breaking:** `Summary` is `#[non_exhaustive]` and has the new
  `evaluation_errors` field; its `Display` now also prints that count.
- `IfcExternalReference.WR1` reads `ItemReference` under IFC2X3 and
  `Identification` under IFC4/IFC4X3, as each release's EXPRESS declares,
  instead of whichever of the two resolved.

### Removed

- **Breaking:** `Budget::max_depth`. Nothing read it: every walk the crate
  performs is over the bundled schema's type graph, which a file cannot
  lengthen, and the SELECT walk's own bound is now proven sufficient for
  every SELECT the bundled schemas use. `Budget::max_findings` remains the
  one file-controlled limit.

### Fixed

- `type.scalar.mismatch` now checks bounded and fixed-width strings. The
  primitive was read from the trailing token of the resolved type, so
  `STRING(255)` -- IFC4's `IfcLabel` and `IfcIdentifier` -- recognised
  nothing and every such slot went unchecked; an integer in a `Name` slot
  was accepted.

## [0.2.0] - 2026-09-22

First release under per-crate versioning. See the
[repository changelog](../CHANGELOG.md) for the family-wide history
that produced this version.

[Unreleased]: https://github.com/openbimrs/ifc/compare/ifc-validate-v0.2.0...HEAD
[0.2.0]: https://github.com/openbimrs/ifc/releases/tag/v0.2.0
